// A window holds a life of the daemon of another machine of the tailnet as it holds a local one: the second daemon is a
// process of its own that listens on loopback as on the tailnet, with a fake of the local API of Tailscale that names the
// login of each peer, as host/tests/suite/net.rs does. The window dials its address through the one dial, lists its lives, opens
// one and sends a message that the model answers. It lists the records that the machine names, and refuses to delete a session that stands on the machine.
import { expect } from "bun:test";
import { type ChildProcess, spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, realpath, rm, writeFile } from "node:fs/promises";
import { createServer, type Server } from "node:net";
import { dirname, join } from "node:path";
import { type Daemon, furbProgram, daemon as local, machine } from "../src/client.ts";
import { type Rule, scenario, test, Window, type Yard } from "./harness.ts";

// The program furb of the run, read before the yard names its own wrapper in FURB_BIN, which gives its own grace.
const PROGRAM = furbProgram();

const LOGIN = "alice@github";

/** A fake of the local API of Tailscale on a port of loopback: it gives this machine its address and each peer a login. */
async function tailscale(): Promise<{ server: Server; port: number }> {
  const server = createServer((socket) => {
    socket.once("data", (data) => {
      const path = String(data).split(" ")[1] ?? "";
      const body = path.startsWith("/localapi/v0/status")
        ? { TailscaleIPs: ["127.0.0.1"] }
        : { UserProfile: { ID: 1, LoginName: LOGIN } };
      const text = JSON.stringify(body);
      socket.end(
        `HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: ${text.length}\r\nConnection: close\r\n\r\n${text}`,
      );
    });
  });
  await new Promise<void>((done) => server.listen(0, "127.0.0.1", done));
  const address = server.address();
  return { server, port: typeof address === "object" && address ? address.port : 0 };
}

/** A free port of loopback. */
async function free(): Promise<number> {
  const server = createServer();
  await new Promise<void>((done) => server.listen(0, "127.0.0.1", done));
  const address = server.address();
  const port = typeof address === "object" && address ? address.port : 0;
  await new Promise<void>((done) => server.close(() => done()));
  return port;
}

/** The script of the model of each scenario below. */
const SCRIPT: Rule[] = [{ cue: "Machine: say hi.", word: 'close("Hi from the machine.")' }];

/** A second daemon, a process of its own on a port of loopback, which `use` is given by its address, the connection to
 * it and the record that its life of the yard holds. The windows of the yard hold their lives in it meanwhile. */
async function onMachine(
  yard: Yard,
  use: (address: string, link: Daemon, remote: string) => Promise<void>,
): Promise<void> {
  const api = await tailscale();
  const port = await free();
  // The second daemon has a cache, a config and a socket of its own, and listens on the port of the tailnet.
  const cache = await realpath(await mkdtemp(join("/tmp", "furb-machine-")));
  const configs = join(cache, "config");
  await mkdir(configs, { recursive: true });
  await writeFile(
    join(configs, "host.json"),
    JSON.stringify({ tailnet: { tcp: port }, roles: { [LOGIN]: "write" } }),
  );
  // The list of workspaces of that machine, which its daemon reads to name the records of the sessions.
  const listed = { directory: yard.work, name: "work", collapsed: false, records: [], archived: [] };
  await writeFile(join(configs, "workspaces.json"), JSON.stringify({ workspaces: [listed] }));
  let second: ChildProcess | undefined;
  try {
    second = spawn(PROGRAM, ["host", "--grace", "2"], {
      env: {
        ...process.env,
        XDG_CACHE_HOME: cache,
        FURB_CONFIG_DIR: configs,
        FURB_TAILSCALE_API: `${api.port}:proof`,
      },
      stdio: ["ignore", "pipe", "pipe"],
    });
    const address = `127.0.0.1:${port}`;
    const link = machine(address);
    // The daemon listens once a window can dial its address.
    const until = Date.now() + 60000;
    for (;;) {
      let why = "";
      const reached = await link.connect().then(
        () => true,
        (error: unknown) => {
          why = String(error);
          return false;
        },
      );
      if (reached) break;
      if (Date.now() > until) throw new Error(`The second daemon never answered on its address: ${why}`);
      await Bun.sleep(100);
    }
    expect(link.peer?.login).toBe(LOGIN);
    yard.machine = address;
    await use(address, link, join(yard.work, ".furb", "sessions", "remote.jsonl"));
  } finally {
    second?.kill();
    api.server.close();
  }
}

test("a window dials the daemon of another machine by its address, lists its lives, opens one and sends a message that its model answers", () =>
  scenario(SCRIPT, (_first, yard) =>
    onMachine(yard, async (_address, link, remote) => {
      const window = await Window.open(yard, remote);
      try {
        await window.type("Machine: say hi.");
        await window.shows(
          (frame) => frame.includes("Hi from the machine."),
          "the answer of the other machine",
        );
        // The life stands in the daemon of the other machine, and in none of this one.
        const lives = await link.ask({ verb: "list" });
        expect(lives.answer === "lives" && lives.data.some((one) => one.life === remote && one.held)).toBe(
          true,
        );
      } finally {
        await window.close();
      }
    }),
  ));

test("a window of another machine refuses to delete a session that its daemon holds, and neither daemon changes", () =>
  scenario(SCRIPT, (_first, yard) =>
    onMachine(yard, async (address, link, remote) => {
      const window = await Window.open(yard, remote);
      try {
        await window.type("Machine: say hi.");
        await window.shows(
          (frame) => frame.includes("Hi from the machine."),
          "the answer of the other machine",
        );
        const entry = window.library.groups
          .flatMap((group) => group.sessions)
          .find((one) => one.path === remote);
        if (!entry) throw new Error("The window lists no session of the other machine.");
        const error = await window.library.delete(entry).catch((error: unknown) => error);
        expect(String(error)).toContain(address);
        // The record, which the second daemon wrote, stays, and its life stays held there and is none of this machine's.
        expect(existsSync(remote)).toBe(true);
        const there = await link.ask({ verb: "list" });
        expect(there.answer === "lives" && there.data.some((one) => one.life === remote && one.held)).toBe(
          true,
        );
        const here = await local().ask({ verb: "list" });
        expect(here.answer === "lives" && here.data.some((one) => one.life === remote)).toBe(false);
      } finally {
        await window.close();
      }
    }),
  ));

test("a window of a machine lists the records of the workspaces that the machine names, with no list of this machine on that machine", () =>
  scenario(SCRIPT, (_first, yard) =>
    onMachine(yard, async (_address, _link, remote) => {
      // A record that stands on the machine, and that no window of this machine opened.
      const far = join(dirname(remote), "far.jsonl");
      await mkdir(dirname(remote), { recursive: true });
      await writeFile(far, "");
      const window = await Window.open(yard, remote);
      try {
        const paths = () => window.library.groups.flatMap((group) => group.sessions).map((one) => one.path);
        await window.library.refresh();
        expect(paths()).toContain(far);
        // The list of this machine, which the machine does not hold, as another machine does not.
        await rm(join(yard.root, "ui", "workspaces.json"));
        await window.library.refresh();
        expect(paths()).toContain(far);
      } finally {
        await window.close();
      }
    }),
  ));

// A window holds a life of the daemon of another machine of the tailnet as it holds a local one: the second daemon is a
// process of its own that listens on loopback as on the tailnet, with a fake of the local API of Tailscale that names the
// login of each peer, as host/tests/suite/net.rs does. The window dials its address through the one dial, lists its lives, opens
// one and sends a message that the model answers.
import { expect } from "bun:test";
import { type ChildProcess, spawn } from "node:child_process";
import { mkdir, mkdtemp, realpath, writeFile } from "node:fs/promises";
import { createServer, type Server } from "node:net";
import { join } from "node:path";
import { furbProgram, machine } from "../src/client.ts";
import { scenario, test, Window } from "./harness.ts";

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

test("a window dials the daemon of another machine by its address, lists its lives, opens one and sends a message that its model answers", () =>
  scenario([{ cue: "Machine: say hi.", word: 'close("Hi from the machine.")' }], async (_first, yard) => {
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
    let daemon: ChildProcess | undefined;
    try {
      daemon = spawn(PROGRAM, ["host", "--grace", "2"], {
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
      const remote = join(yard.work, ".furb", "sessions", "remote.jsonl");
      yard.machine = address;
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
    } finally {
      daemon?.kill();
      api.server.close();
    }
  }));

// A window of a machine reaches the daemon of that machine alone, which it dials again while that machine is not up and
// after its connection dropped, and it refuses what only this machine does: a delete moves the files of this machine,
// and the files of the session stand on the other one.
import { expect, test } from "bun:test";
import { mkdir, mkdtemp, realpath, rm, stat, writeFile } from "node:fs/promises";
import { createServer, type Server, type Socket } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { type Daemon, daemon, machine } from "../src/client.ts";
import { Preferences } from "../src/preferences.ts";
import { Workspaces } from "../src/workspaces.ts";

/** The verbs that a daemon is asked, answered as a daemon holding the given lives and no window answers them, until
 * restore. */
function listening(
  link: Daemon,
  lives: { life: string; workspace: string; held: boolean; open: boolean }[] = [],
): { asked: string[]; restore: () => void } {
  const ask = link.ask.bind(link);
  const asked: string[] = [];
  link.ask = (async (verb: { verb: string }) => {
    asked.push(verb.verb);
    return verb.verb === "presence" ? { answer: "presence", data: [] } : { answer: "lives", data: lives };
  }) as typeof link.ask;
  return {
    asked,
    restore: () => {
      link.ask = ask;
    },
  };
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

/** A socket of a fake daemon that welcomes each hello. */
function welcome(socket: Socket): void {
  socket.on("data", (chunk) => {
    for (const line of String(chunk).split("\n").filter(Boolean)) {
      const { id } = JSON.parse(line) as { id: number };
      const answer = { answer: "welcome", data: { peer: {} } };
      socket.write(`${JSON.stringify({ frame: "reply", id, answer })}\n`);
    }
  });
}

test("a window of a machine asks the daemon of that machine alone, and refuses to delete a session whose files stand on that machine", async () => {
  const directory = await realpath(await mkdtemp(join(tmpdir(), "furb-machine-window-")));
  const address = "127.0.0.1:9";
  const here = listening(daemon());
  const there = listening(machine(address));
  const library = new Workspaces(new Preferences(join(directory, "config/ui.json")), { machine: address });
  try {
    const group = await library.add(directory);
    // The record of the other machine has a twin at its path on this one, as two machines of one layout have.
    const record = join(directory, ".furb", "sessions", "remote.jsonl");
    await mkdir(join(directory, ".furb", "sessions"), { recursive: true });
    await writeFile(record, "");
    const entry = { path: record, name: "remote", status: "saved" as const, unread: false };
    group.sessions.push(entry);
    const error = await library.delete(entry).catch((error: unknown) => error);
    expect(error).toBeInstanceOf(Error);
    expect(String(error)).toContain(address);
    expect((await stat(record)).isFile()).toBe(true);
    expect(group.sessions).toContain(entry);
    await library.refresh();
    expect(here.asked).toEqual([]);
    expect(there.asked.length).toBeGreaterThan(0);
  } finally {
    here.restore();
    there.restore();
    machine(address).close();
    await library.dispose();
    await rm(directory, { recursive: true, force: true });
  }
});

test("a window of a machine lists the records that the daemon of that machine names, whose files this machine does not hold", async () => {
  const directory = await realpath(await mkdtemp(join(tmpdir(), "furb-machine-list-")));
  const address = "127.0.0.1:9";
  // The record stands on the other machine alone: no file of it stands here.
  const record = join(directory, ".furb", "sessions", "far.jsonl");
  const lives = [{ life: record, workspace: directory, held: false, open: false }];
  const here = listening(daemon());
  const there = listening(machine(address), lives);
  const library = new Workspaces(new Preferences(join(directory, "config/ui.json")), { machine: address });
  try {
    const group = await library.add(directory);
    expect(group.sessions.map((entry) => entry.path)).toEqual([record]);
    // A record that the daemon names no more leaves the list.
    lives.length = 0;
    await library.refresh();
    expect(group.sessions).toEqual([]);
    expect(here.asked).toEqual([]);
    expect(there.asked.length).toBeGreaterThan(0);
  } finally {
    here.restore();
    there.restore();
    machine(address).close();
    await library.dispose();
    await rm(directory, { recursive: true, force: true });
  }
});

test("a window dials a machine that is not up yet again, and reaches it once it listens", async () => {
  const port = await free();
  const link = machine(`127.0.0.1:${port}`);
  const reaching = link.connect();
  await Bun.sleep(1200);
  const server = createServer(welcome);
  await new Promise<void>((done) => server.listen(port, "127.0.0.1", done));
  try {
    await reaching;
    expect(link.peer).toBeDefined();
  } finally {
    link.close();
    server.close();
  }
});

test("a window redials a machine whose connection dropped, and tells reconnected", async () => {
  const port = await free();
  const sockets = new Set<Socket>();
  const serve = async (): Promise<Server> => {
    const server = createServer((socket) => {
      sockets.add(socket);
      welcome(socket);
    });
    await new Promise<void>((done) => server.listen(port, "127.0.0.1", done));
    return server;
  };
  let server = await serve();
  const link = machine(`127.0.0.1:${port}`);
  try {
    await link.connect();
    const back = new Promise<void>((done) => link.once("reconnected", done));
    // The machine goes: its sockets end and its port closes, so the window dials it again until it listens.
    for (const socket of sockets) socket.destroy();
    await new Promise<void>((done) => server.close(() => done()));
    await Bun.sleep(600);
    server = await serve();
    await back;
    expect(link.peer).toBeDefined();
  } finally {
    link.close();
    server.close();
  }
}, 40000);

test("a window gives up on a machine that never listens, and names its address", async () => {
  const address = `127.0.0.1:${await free()}`;
  const link = machine(address);
  try {
    const error = await link.connect().catch((error: unknown) => error);
    expect(String(error)).toContain(`No daemon of furb listens at ${address}`);
  } finally {
    link.close();
  }
}, 40000);

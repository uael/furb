// A window of a machine reaches the daemon of that machine alone, which it dials again while that machine is not up, and
// it refuses what only this machine does: a delete moves the files of this machine, and the files of the session stand
// on the other one.
import { expect, test } from "bun:test";
import { mkdir, mkdtemp, realpath, rm, stat, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { type Daemon, daemon, machine } from "../src/client.ts";
import { Preferences } from "../src/preferences.ts";
import { Workspaces } from "../src/workspaces.ts";

/** The verbs that a daemon is asked, answered as a daemon with no life answers them, until restore. */
function listening(link: Daemon): { asked: string[]; restore: () => void } {
  const ask = link.ask.bind(link);
  const asked: string[] = [];
  link.ask = (async (verb: { verb: string }) => {
    asked.push(verb.verb);
    return verb.verb === "presence" ? { answer: "presence", data: [] } : { answer: "lives", data: [] };
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

test("a window dials a machine that is not up yet again, and reaches it once it listens", async () => {
  const port = await free();
  const link = machine(`127.0.0.1:${port}`);
  const reaching = link.connect();
  await Bun.sleep(1200);
  const server = createServer((socket) =>
    socket.on("data", (chunk) => {
      for (const line of String(chunk).split("\n").filter(Boolean)) {
        const { id } = JSON.parse(line) as { id: number };
        const answer = { answer: "welcome", data: { peer: {} } };
        socket.write(`${JSON.stringify({ frame: "reply", id, answer })}\n`);
      }
    }),
  );
  await new Promise<void>((done) => server.listen(port, "127.0.0.1", done));
  try {
    await reaching;
    expect(link.peer).toBeDefined();
  } finally {
    link.close();
    server.close();
  }
});

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

// A window holds what it shows of a life, not the length of the life: it keeps no fact, slims the texts of each act to
// what a view draws, and reads the whole act from the daemon when a card shows it.
import { afterEach, expect, test } from "bun:test";
import { mkdtempSync, rmSync } from "node:fs";
import { createServer, type Server, type Socket } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Daemon, RemoteLife } from "../src/client.ts";

type Asked = { id: number; verb: string; life?: string; act?: string };

const made: { folder: string; server: Server; daemon: Daemon }[] = [];
afterEach(() => {
  for (const { folder, server, daemon } of made.splice(0)) {
    daemon.close();
    server.close();
    rmSync(folder, { recursive: true, force: true });
  }
});

function frame(socket: Socket, one: unknown): void {
  socket.write(`${JSON.stringify(one)}\n`);
}

const BIG = "x".repeat(2 ** 19);
const exit = {
  is: "Exit",
  code: 0,
  stdout: { is: "Text", content: BIG },
  stderr: { is: "Text", content: "" },
};
const word = `notes = ${JSON.stringify(BIG)}`;

/** A life of many commands that each printed half a megabyte, a run whose word is half a megabyte and runs still, and a prefix
 * that carries a whole transcript, as a long life tells them. */
function told(socket: Socket, life: string, commands: number): void {
  let seq = 0;
  const event = (event: unknown) => frame(socket, { frame: "event", life, seq: seq++, event });
  for (let n = 1; n <= commands; n++) {
    const id = `bash${n}`;
    const act = { id, kind: "bash", chain: "chain1", by: "rung1", words: ["cat big"], began: n };
    event({ event: "fact", data: ["bash", id, "rung1", "chain1", "cat big"] });
    event({ event: "fact", data: ["done", id, "bash", exit] });
    event({ event: "act", data: { ...act, phase: { phase: "done", value: exit } } });
  }
  event({ event: "fact", data: ["prefix", "chain1", "chain1", [["tell", "chain1", "x", [BIG]]]] });
  event({ event: "fact", data: ["run", "run1", "chain1", "chain1", "rung1", word, ""] });
  const run = {
    id: "run1",
    kind: "run",
    chain: "chain1",
    by: "chain1",
    words: ["rung1", word, ""],
    began: 0,
  };
  event({ event: "act", data: { ...run, phase: { phase: "running" } } });
  event({ event: "changed" });
}

test("a window keeps no fact and slims each act, and reads the whole act from the daemon on demand", async () => {
  const folder = mkdtempSync(join(tmpdir(), "furb-memory-"));
  const place = join(folder, "host.sock");
  const commands = 24;
  const server = createServer((socket) => {
    let held = "";
    socket.setEncoding("utf8");
    socket.on("error", () => {});
    socket.on("data", (chunk: string) => {
      held += chunk;
      for (let end = held.indexOf("\n"); end >= 0; end = held.indexOf("\n")) {
        const asked = JSON.parse(held.slice(0, end)) as Asked;
        held = held.slice(end + 1);
        const answer =
          asked.verb === "hello"
            ? { answer: "welcome", data: { peer: { login: "test", role: "write", kind: "window" } } }
            : asked.verb === "pending"
              ? { answer: "values", data: [] }
              : asked.verb === "peek"
                ? { answer: "value", data: exit }
                : asked.verb === "get"
                  ? { answer: "value", data: ["bash", asked.act, "rung1", "chain1", "cat big"] }
                  : { answer: "done" };
        frame(socket, { frame: "reply", id: asked.id, answer });
        if (asked.verb === "open") told(socket, String(asked.life), commands);
      }
    });
  });
  server.listen(place);
  const daemon = new Daemon(folder, place);
  made.push({ folder, server, daemon });
  const life = new RemoteLife(daemon, join(folder, "life.jsonl"));
  await life.open();
  const deadline = Date.now() + 10000;
  while (!life.act("run1") && Date.now() < deadline) await Bun.sleep(10);
  expect(life.act("run1")).toBeDefined();
  // The window heard each fact and kept none of them: the facts of this life weigh more than 12 MB.
  expect(life.count).toBe(2 * commands + 2);
  expect((life as unknown as { facts?: unknown }).facts).toBeUndefined();
  // Each act holds what a view draws of it, whatever the life printed.
  const held = JSON.stringify([...Array(commands).keys()].map((n) => life.act(`bash${n + 1}`)));
  expect(held.length).toBeLessThan(commands * 4096);
  expect(JSON.stringify(life.act("run1")).length).toBeLessThan(70000);
  // A card reads the whole act from the daemon, so nothing is lost.
  const whole = await life.whole("bash7");
  expect((whole?.phase as { value?: typeof exit }).value?.stdout.content).toBe(BIG);
  expect(await life.result("bash7")).toEqual(exit);
});

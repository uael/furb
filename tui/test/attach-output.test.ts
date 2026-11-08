// A window that attaches to a life while a command runs reads what the command printed before the attach: the log of
// the daemon keeps each out fact, and holds of the act of the command only its last event, which comes after them.
import { afterEach, expect, test } from "bun:test";
import { mkdtempSync, rmSync } from "node:fs";
import { createServer, type Server, type Socket } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Daemon, RemoteLife } from "../src/client.ts";

type Asked = { id: number; verb: string; life?: string };

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

/** The events of a life as a window that attaches late reads them: the facts of a command and its output, then the
 * only event of its act that the log still holds, as it runs. */
function attached(socket: Socket, life: string): void {
  const facts = [
    ["bash", "bash1", "rung1", "chain1", "for i in 1 2; do echo line$i; done; sleep 30"],
    ["started", "bash1", "shell"],
    ["out", "bash1", "shell", "line1\n", "stdout"],
    ["out", "bash1", "shell", "line2\n", "stdout"],
    ["out", "bash1", "shell", "warn\n", "stderr"],
  ];
  let seq = 0;
  for (const fact of facts)
    frame(socket, { frame: "event", life, seq: seq++, event: { event: "fact", data: fact } });
  const act = {
    id: "bash1",
    kind: "bash",
    chain: "chain1",
    by: "rung1",
    words: ["for i in 1 2; do echo line$i; done; sleep 30"],
    phase: { phase: "running" },
    began: 1,
  };
  frame(socket, { frame: "event", life, seq: seq++, event: { event: "act", data: act } });
  frame(socket, { frame: "event", life, seq: seq++, event: { event: "changed" } });
}

test("a window that attaches while a command runs shows each line that the command printed before the attach", async () => {
  const folder = mkdtempSync(join(tmpdir(), "furb-attach-"));
  const place = join(folder, "host.sock");
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
              : { answer: "done" };
        frame(socket, { frame: "reply", id: asked.id, answer });
        if (asked.verb === "open") attached(socket, String(asked.life));
      }
    });
  });
  server.listen(place);
  const daemon = new Daemon(folder, place);
  made.push({ folder, server, daemon });
  const life = new RemoteLife(daemon, join(folder, "life.jsonl"));
  await life.open();
  const deadline = Date.now() + 5000;
  while (!life.act("bash1") && Date.now() < deadline) await Bun.sleep(10);
  expect(life.output("bash1")).toEqual({ stdout: "line1\nline2\n", stderr: "warn\n" });
  await life.detach();
});

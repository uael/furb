import { expect, test } from "bun:test";
import { mkdirSync, rmSync } from "node:fs";
import { createServer, type Socket } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { daemonPlace, HOST, LIFE, SKILL, Skill } from "../src/skill.ts";

test("a skill answers each open act of its kind with a close, and skips one that is done", async () => {
  // A socket path has a short bound, which the temporary folder of macOS passes.
  const folder = join(process.platform === "win32" ? tmpdir() : "/tmp", `furb-answers-${process.pid}`);
  const said: Record<string, unknown>[] = [];
  let peer: Socket | undefined;
  const server = createServer((socket) => {
    peer = socket;
    let rest = "";
    socket.on("data", (chunk) => {
      rest += String(chunk);
      const lines = rest.split("\n");
      rest = lines.pop() ?? "";
      for (const line of lines.filter(Boolean)) {
        const verb = JSON.parse(line) as Record<string, unknown> & { id: number };
        said.push(verb);
        const frame =
          verb.verb === "hello"
            ? { frame: "reply", id: verb.id, answer: { answer: "welcome", data: { peer: {} } } }
            : verb.verb === "peek" && verb.act !== "k.add1"
              ? {
                  frame: "refused",
                  id: verb.id,
                  error: { error: "refused" },
                  why: `No act named ${verb.act} is done.`,
                }
              : { frame: "reply", id: verb.id, answer: { answer: "done" } };
        socket.write(`${JSON.stringify(frame)}\n`);
      }
    });
  });
  mkdirSync(folder, { recursive: true });
  const place = daemonPlace(folder);
  await new Promise<void>((resolve) => server.listen(place, resolve));
  try {
    const skill = await Skill.attach({
      kinds: ["prompt"],
      env: { [HOST]: folder, [LIFE]: "a.jsonl", [SKILL]: "k" },
    });
    skill.answers("k.add", async (fact) => `added ${String(fact[4])}`);
    skill.answers("k.drop", () => {
      throw new Error("no room");
    });
    skill.answers("k.gone", () => 1)();
    const acts = [
      ["k.add", "k.add1"],
      ["k.add", "k.add2"],
      ["k.drop", "k.drop1"],
      ["k.gone", "x"],
    ];
    acts.forEach(([kind, act], seq) => {
      const fact = [kind, act, "rung4", "chain1", "milk"];
      peer?.write(
        `${JSON.stringify({ frame: "event", life: "a.jsonl", seq, event: { event: "fact", data: fact } })}\n`,
      );
    });
    const closes = () => said.filter((verb) => verb.verb === "close");
    for (let tries = 0; closes().length < 2 && tries < 500; tries++) await Bun.sleep(10);
    const got = closes()
      .map((verb) => [verb.act, verb.value])
      .sort();
    expect(got).toEqual([
      ["k.add2", JSON.stringify("added milk")],
      ["k.drop1", JSON.stringify({ is: "Refused", args: ["no room"] })],
    ]);
    const peeked = said.filter((verb) => verb.verb === "peek").map((verb) => verb.act);
    expect(peeked.sort()).toEqual(["k.add1", "k.add2", "k.drop1"]);
    skill.close();
  } finally {
    server.close();
    rmSync(folder, { recursive: true, force: true });
  }
});

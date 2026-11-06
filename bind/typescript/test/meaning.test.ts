import { expect, test } from "bun:test";
import { copyFileSync, mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { Meaning, retold } from "../index.cjs";

/** The meaning of a recorded life, folded in the crate from each fact that the record tells, in order. The record
 * is read from a copy, since the crate retells it in a file beside it. */
function folded(source: string): Meaning {
  const record = join(mkdtempSync(join(tmpdir(), "furb-meaning-")), "record.jsonl");
  copyFileSync(source, record);
  const meaning = new Meaning();
  for (const line of readFileSync(retold(record).file, "utf8").split("\n")) {
    if (!line.startsWith("[")) continue;
    const fact = JSON.parse(line) as unknown[];
    if (fact[0] !== "keep") meaning.hear(fact);
  }
  return meaning;
}

test("a recorded life folds its acts, its desk, its threads and its moments in the crate", () => {
  const meaning = folded(fileURLToPath(new URL("../../../tests/replay/refused.jsonl", import.meta.url)));
  const moments = meaning.moments();
  expect(moments.length).toBe(11);
  expect(moments).toEqual([...moments].sort((one, other) => one - other));
  expect(meaning.desk()).toEqual({ asks: [], questions: 0, notices: 0 });
  const threads = meaning.threads("chain1");
  expect([threads.threads.length, threads.state, threads.attention]).toEqual([
    1,
    "idle",
    { asks: 0, failed: 0, paused: 0 },
  ]);
  expect(meaning.asOf(moments.at(-1) ?? 0).map((act) => [act.kind, act.id, act.state])).toEqual([
    ["chain", "chain1", "idle"],
    ["stand", "stand1", "done"],
    ["prompt", "prompt1", "runs"],
    ["rung", "rung1", "idle"],
    ["reply", "reply1", "delegates"],
    ["chain", "chain2", "thinks"],
    ["prompt", "prompt2", "runs"],
    ["rung", "rung2", "thinks"],
    ["reply", "reply2", "delegates"],
  ]);
  expect(meaning.asOf(moments[0] ?? 0).map((act) => act.id)).toEqual(["chain1"]);
});

test("a record folds a paused prompt, an act born under its pause and a later wake as the live life did", () => {
  // Each fact, with what the engine of the live life finds for it: the acts that a pause or a wake covers, and the
  // pause that holds a new act. A reader of the record gives neither, and the crate finds both.
  const pause = ["pause", "prompt1", "operator", "held"];
  const facts: [unknown[], string[], unknown[] | null][] = [
    [["chain", "chain1", "operator", "", "root", ""], [], null],
    [["prompt", "prompt1", "operator", "chain1", "int", "count", ""], [], null],
    [pause, ["prompt1"], null],
    [["rung", "rung1", "prompt1", "chain1", "x = 1", "", ""], [], pause],
    [["wake", "prompt1", "operator"], ["prompt1", "rung1"], null],
  ];
  const live = new Meaning();
  const recorded = new Meaning();
  for (const [fact, covered, born] of facts) {
    expect(recorded.hear(fact)).toEqual(live.hear(fact, covered, born));
    expect(recorded.open(false)).toEqual(live.open(false));
  }
  expect(recorded.open(false)).toEqual([]);
});

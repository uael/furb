import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { Meaning, retold } from "../index.cjs";

/** The meaning of a recorded life, folded in the crate from each fact that the record tells, in order. */
function folded(record: string): Meaning {
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

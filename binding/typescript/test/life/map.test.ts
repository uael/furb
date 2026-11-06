import { expect, test } from "bun:test";
import { messages, places, route, you } from "../../src/life/map.ts";
import { act } from "./acts.ts";

const ask = (
  id: string,
  on: string,
  by: string,
  began: number,
  ended?: number,
  words: unknown[] = ["str", id, ""],
) =>
  act(id, "prompt", on, by, began, ended, {
    words,
    ...(ended === undefined ? {} : { value: `${id} closed` }),
  });

test("a chain takes its place at its birth and keeps it: the column of its depth, and the first free row at or under the chain that hired it", () => {
  const born = [
    act("chain1", "chain", "", "operator", 1),
    act("chain2", "chain", "", "rung1", 2),
    act("chain3", "chain", "", "rung1", 3),
  ];
  const parent = (chain: { id: string }) => (chain.id === "chain1" ? undefined : "chain1");
  const first = places(born, parent);
  expect([...first]).toEqual([
    [you, { column: 0, row: 0 }],
    ["chain1", { column: 0, row: 1 }],
    ["chain2", { column: 1, row: 1 }],
    ["chain3", { column: 1, row: 2 }],
  ]);
  const later = places([...born, act("chain4", "chain", "", "rung2", 4)], (chain) =>
    chain.id === "chain4" ? "chain2" : parent(chain),
  );
  for (const [id, place] of first) expect(later.get(id)).toEqual(place);
  expect(later.get("chain4")).toEqual({ column: 2, row: 1 });
});

test("an open prompt flows from the chain of the rung that made it to its chain, a question flows to the operator, and an answer flows back for a moment", () => {
  const acts = [
    act("chain1", "chain", "", "operator", 1),
    act("rung1", "rung", "chain1", "operator", 2),
    act("chain2", "chain", "", "rung1", 3),
    ask("prompt1", "chain2", "rung1", 4),
    ask("prompt2", "chain2", "rung1", 5, 6),
    ask("prompt3", "chain2", "rung1", 7, undefined, ["str", "Which?", "operator"]),
  ];
  expect(messages(acts, (one) => one.id === "prompt2")).toEqual([
    { act: "prompt1", from: "chain1", to: "chain2", kind: "ask" },
    { act: "prompt2", from: "chain2", to: "chain1", kind: "answer" },
    { act: "prompt3", from: "chain2", to: you, kind: "asks you" },
    { act: "prompt3", from: "chain1", to: "chain2", kind: "ask" },
  ]);
  const parents: Record<string, string> = { chain2: "chain1", chain3: "chain1", chain1: you };
  expect(route("chain2", "chain3", (id) => parents[id])).toEqual(["chain2", "chain1", "chain3"]);
});

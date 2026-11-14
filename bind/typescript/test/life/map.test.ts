import { expect, test } from "bun:test";
import { messages, you } from "../../src/life/map.ts";
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
});

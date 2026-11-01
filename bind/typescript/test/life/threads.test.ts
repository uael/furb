import { expect, test } from "bun:test";
import type { LiveAct } from "../../src/life/acts.ts";
import { chainThreads } from "../../src/life/threads.ts";
import { act } from "./acts.ts";

const prompt = (id: string, began: number, more: Partial<LiveAct> = {}) =>
  act(id, "prompt", "chain1", "operator", began, undefined, { words: ["str", `${id} says`, ""], ...more });

test("what a host keeps of a prompt belongs to that act, so a prompt of the same id in another session or another branch is read again and joins its thread", () => {
  const known = new WeakMap<LiveAct, { message: string | undefined; title: string }>();
  const list = (acts: LiveAct[]) =>
    chainThreads(acts, new Map(acts.map((one) => [one.id, one])), new Set(), known).map((thread) => [
      thread.act.id,
      thread.messages.map((one) => one.id),
    ]);
  // In one session, prompt2 starts a thread of its own.
  expect(list([prompt("prompt1", 1), prompt("prompt2", 2)])).toEqual([
    ["prompt1", []],
    ["prompt2", []],
  ]);
  // In another session, or on another branch, prompt2 is a message to prompt1, and it stands in that thread at once.
  expect(list([prompt("prompt1", 1), prompt("prompt2", 2, { thread: "prompt1" })])).toEqual([
    ["prompt1", ["prompt2"]],
  ]);
});

test("a question and a notice that a thread made for the operator stand in that thread, with the questions that wait, and one that no thread made stands in none", () => {
  const acts = [
    prompt("prompt1", 1),
    act("rung1", "rung", "chain1", "prompt1", 2),
    act("prompt2", "prompt", "chain1", "rung1", 3, undefined, { words: ["bool", "Keep it?", "operator"] }),
    act("prompt3", "prompt", "chain1", "rung1", 4, undefined, {
      words: ["None", "It is 14 lines.", "operator"],
    }),
    act("prompt4", "prompt", "chain1", "chain1", 5, undefined, {
      words: ["str", "A question of no thread.", "operator"],
    }),
  ];
  const [thread, ...others] = chainThreads(
    acts,
    new Map(acts.map((one) => [one.id, one])),
    new Set(["prompt2", "prompt4"]),
  );
  expect(others).toEqual([]);
  expect(thread?.asked.map((one) => one.id)).toEqual(["prompt2", "prompt3"]);
  expect(thread?.asks).toEqual(["prompt2"]);
});

test("the order in which the acts land changes no verdict: each read with what a host kept gives what a fresh read gives", () => {
  const all = [
    act("chain1", "chain", "", "", 0),
    prompt("prompt1", 1, { words: ["str", "Hire two workers.", "claude"] }),
    prompt("prompt2", 2, { thread: "prompt1" }),
    prompt("prompt3", 3, { thread: "prompt2" }),
    act("rung1", "rung", "chain1", "prompt1", 4),
    act("bash5", "bash", "chain1", "rung1", 5),
    prompt("prompt9", 6, { by: "chain1", words: ["None", "bash5 done", ""] }),
  ];
  const read = (
    landed: LiveAct[],
    known?: WeakMap<LiveAct, { message: string | undefined; title: string }>,
  ) => {
    const on = all.filter((one) => landed.includes(one));
    return chainThreads(on, new Map(landed.map((one) => [one.id, one])), new Set(), known).map((thread) => [
      thread.act.id,
      thread.messages.map((one) => one.id),
    ]);
  };
  const orders = (rest: LiveAct[]): LiveAct[][] =>
    rest.length
      ? rest.flatMap((one) => orders(rest.filter((x) => x !== one)).map((tail) => [one, ...tail]))
      : [[]];
  let checked = 0;
  for (const order of orders(all)) {
    const known = new WeakMap<LiveAct, { message: string | undefined; title: string }>();
    for (let n = 1; n <= order.length; n++) {
      const landed = order.slice(0, n);
      expect(read(landed, known)).toEqual(read(landed));
      checked++;
    }
  }
  expect(checked).toBe(5040 * 7);
  expect(read(all)).toEqual([["prompt1", ["prompt2", "prompt3", "prompt9"]]]);
});

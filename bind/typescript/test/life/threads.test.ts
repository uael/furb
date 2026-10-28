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

import { expect, test } from "bun:test";
import { actState, busy, rollUp, STATES, working } from "../../src/life/states.ts";
import { act } from "./acts.ts";

test("each act takes one state of the one set: it asks, a pause holds it, a cancel or a failure ended it, its model thinks, it runs, it delegates, or it is done", () => {
  const owed = new Set(["prompt9"]);
  const rung = act("rung1", "rung", "chain1", "prompt1", 1);
  const worded = { ...rung, word: "x = 1" };
  const table = [
    [act("prompt9", "prompt", "chain1", "operator", 1), "asks"],
    [{ ...act("bash1", "bash", "chain1", "rung1", 1), paused: true }, "paused"],
    [
      { ...act("bash2", "bash", "chain1", "rung1", 1, 2), value: { is: "ValueError", args: ["no"] } },
      "failed",
    ],
    [rung, "thinks"],
    [worded, "runs"],
    [act("bash3", "bash", "chain1", "rung1", 1), "runs"],
    [act("chain2", "chain", "chain1", "rung1", 1), "delegates"],
    [act("read1", "read", "chain1", "rung1", 1, 2), "done"],
  ] as const;
  expect(table.map(([one]) => actState(one, owed))).toEqual(table.map(([, state]) => state));
});

test("a whole takes the first state of its parts in the order asks, unread, failed, paused, thinks, runs, delegates, done, cancelled, idle", () => {
  expect(STATES).toEqual([
    "asks",
    "unread",
    "failed",
    "paused",
    "thinks",
    "runs",
    "delegates",
    "done",
    "cancelled",
    "idle",
  ]);
  for (const [at, state] of STATES.entries()) expect(rollUp(STATES.slice(at).toReversed())).toBe(state);
  expect(rollUp([])).toBe("idle");
  expect(["thinks", "runs", "delegates", "done"].map(busy)).toEqual([true, true, true, false]);
});

test("the work of a chain thinks while a model writes a word, runs while a command or a wait runs, and delegates while its word awaits only another chain", () => {
  const thinking = act("rung1", "rung", "chain1", "prompt1", 1);
  const running = { ...act("rung2", "rung", "chain1", "prompt1", 2), word: "await bash1" };
  const command = act("bash1", "bash", "chain1", "rung2", 3);
  const asked = act("prompt5", "prompt", "chain2", "rung2", 4);
  expect(working([thinking, running, command], [asked])).toBe("thinks");
  expect(working([running, command], [asked])).toBe("runs");
  expect(working([running], [asked])).toBe("delegates");
  expect(working([running], [])).toBe("runs");
});

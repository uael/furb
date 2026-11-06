import { expect, test } from "bun:test";
import { busy, rollUp, STATES, working } from "../../src/life/states.ts";
import { act } from "./acts.ts";

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
  const thinking = act("rung1", "rung", "chain1", "prompt1", 1, undefined, { asking: true });
  const running = act("rung2", "rung", "chain1", "prompt1", 2, undefined, { word: "await bash1" });
  const command = act("bash1", "bash", "chain1", "rung2", 3);
  const asked = act("prompt5", "prompt", "chain2", "rung2", 4);
  expect(working([thinking, running, command], [asked])).toBe("thinks");
  expect(working([running, command], [asked])).toBe("runs");
  expect(working([running], [asked])).toBe("delegates");
  expect(working([running], [])).toBe("runs");
});
test("a rung that waits for its turn, with no reply of it that runs, rests: the work of its chain is what else runs, or rests", () => {
  const waiting = act("rung1", "rung", "chain1", "prompt1", 1);
  const prompt = act("prompt1", "prompt", "chain1", "operator", 1);
  const command = act("bash1", "bash", "chain1", "rung0", 2);
  expect(working([prompt, waiting, command], [])).toBe("runs");
  expect(working([prompt, waiting], [])).toBe("idle");
  expect(
    working([prompt, act("rung1", "rung", "chain1", "prompt1", 1, undefined, { asking: true })], []),
  ).toBe("thinks");
});

import { expect, test } from "bun:test";
import { busy, rollUp, STATES } from "../../src/life/states.ts";

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

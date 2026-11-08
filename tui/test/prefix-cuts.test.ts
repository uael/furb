import { expect, test } from "bun:test";
import { cuts } from "../src/client.ts";

test("a prefix cuts its chain only where a word compacts or clears it", () => {
  expect(cuts(["prefix", "chain1", "chain1", []], "")).toBe(false);
  expect(
    cuts(["prefix", "chain1", "rung4", []], "with site.set(scope(acting())): say('prefix', site.get(), [])"),
  ).toBe(true);
  expect(cuts(["prefix", "chain1", "rung4", []], "clear()")).toBe(true);
  const seeded = "say('prefix', scope(acting()), [*transcript(of='module')[-1:]])\nx = 1";
  expect(cuts(["prefix", "chain1", "rung50", []], seeded)).toBe(false);
});

import { expect, test } from "bun:test";
import { compacts } from "furb/life";

test("a cut compacts its chain, but the cut of a birth, which keeps the fact carried of its checkpoint", () => {
  expect(compacts(["cut", "chain1", "rung4", []])).toBe(true);
  expect(compacts(["cut", "chain1", "rung4", [["tell", "chain1", "chain1", ["chain()"]]]])).toBe(true);
  expect(compacts(["cut", "chain1", "rung50", [["carried", "chain1", "journal", [], {}, []]]])).toBe(false);
  const born = [
    ["chain", "chain1", "operator", "", "root", ""],
    ["carried", "chain1", "journal", [], {}, []],
  ];
  expect(compacts(["cut", "chain1", "rung50", born])).toBe(false);
  expect(compacts(["prefix", "chain2", "chain2", []])).toBe(false);
});

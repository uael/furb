import { expect, test } from "bun:test";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { compacts } from "furb/life";
import { Reader } from "../src/reading.ts";

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
  expect(compacts(["prefix", "chain1", "rung2", []])).toBe(true);
});

test("a reader folds the turns of a chain again from a cut, or from a prefix that another said on it", () => {
  const turn = ["assistant", "x = 1", [1, 2, 3, 4, 0.5], null];
  const said = [
    ["rung", "rung1", "operator", "chain1", "", "", ""],
    ["tell", "rung1", "chain1", ["first words"]],
    ["reply", "reply1", "rung1", "chain1", "m"],
    ["done", "reply1", "world", turn],
  ];
  for (const kind of ["prefix", "cut"]) {
    const at = mkdtempSync(join(tmpdir(), "furb-cut-"));
    const lines = [["chain", "chain1", "operator", "", "root", ""], ...said, [kind, "chain1", "rung2", said]];
    writeFileSync(join(at, "record.jsonl"), `${lines.map((line) => JSON.stringify(line)).join("\n")}\n`);
    const reader = Reader.recorded(join(at, "record.jsonl"), at);
    expect(reader.turns().map((one) => one[1])).toEqual(["first words", "x = 1", ""]);
    expect(reader.activity.compactions.length).toBe(1);
    reader.dispose();
    rmSync(at, { recursive: true, force: true });
  }
});

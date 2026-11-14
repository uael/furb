import { expect, test } from "bun:test";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import { LIMIT, longestFirst, weight } from "../../script/order.ts";

const limited = (limit: string) => `test("a", async () => {}, ${limit});`;

test("a file weighs as much as the longest limit that it declares for a test, and never less than the runner gives", () => {
  expect(weight(limited("90000"))).toBe(90000);
  expect(weight(`test(\n  "b",\n  async () => {},\n  ${"150_000"},\n);`)).toBe(150000);
  expect(weight(limited("7000"))).toBe(LIMIT);
  expect(weight("test('c', () => {}); setSystemTime(new Date(1_000_000));")).toBe(LIMIT);
});

test("the runner starts the longest tests first, whatever the size of their files", () => {
  const folder = mkdtempSync(join(tmpdir(), "furb-order-"));
  try {
    const padding = "// padding\n".repeat(500);
    const file = (name: string, source: string) => {
      const path = join(folder, name);
      writeFileSync(path, source);
      return path;
    };
    const long = file("long.test.ts", limited("120000"));
    const shorter = file("shorter.test.ts", `${limited("60000")}\n${padding}`);
    const big = file("big.test.ts", `test("b", () => {});\n${padding}`);
    const small = file("small.test.ts", 'test("c", () => {});');
    // The longest limit stands first even in the smallest file, and the larger file of two equal limits stands first.
    expect(longestFirst([small, big, shorter, long]).map((path) => basename(path))).toEqual([
      "long.test.ts",
      "shorter.test.ts",
      "big.test.ts",
      "small.test.ts",
    ]);
  } finally {
    rmSync(folder, { recursive: true, force: true });
  }
});

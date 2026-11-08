import { expect, test } from "bun:test";
import { join } from "node:path";
import { Glob } from "bun";
import { longestFirst, weight } from "../script/order.ts";

const root = join(import.meta.dir, "../..");

test("a file weighs as much as the longest limit that it declares for a test", () => {
  expect(weight('test("a", async () => {}, 7000);')).toBe(7000);
  expect(weight("test(\n  'b',\n  async () => {},\n  1_500,\n);")).toBe(1500);
  expect(weight("test('c', () => {}); setSystemTime(new Date(1_000_000));")).toBe(0);
});

test("the runner starts the longest tests first, whatever the size of their files", () => {
  const paths = ["bind/typescript/test", "tui/test", "tui/scenarios"].flatMap((dir) =>
    [...new Glob("**/*.test.ts").scanSync(join(root, dir))].map((file) => join(root, dir, file)),
  );
  const first = longestFirst(paths)
    .slice(0, 2)
    .map((path) => path.slice(root.length + 1))
    .sort();
  // The gallery lasts 380 seconds on Linux and the walk 250 seconds on macOS, and their files are the smallest.
  expect(first).toEqual(["tui/scenarios/walk.test.ts", "tui/test/gallery.test.ts"]);
});

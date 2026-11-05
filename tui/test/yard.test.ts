import { afterAll, expect, test } from "bun:test";
import { closeSync, mkdirSync, openSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

// On Windows an open file keeps its folder busy, so the removal of the yard folder after this test raises EBUSY. The
// cleanup of tui/script/preload.ts must leave the folder with one line on stderr, and this test must still pass.
let held: number | undefined;
afterAll(() => {
  if (held !== undefined) closeSync(held);
});

test("a folder of the yard that stays busy does not fail the test", () => {
  const folder = join(tmpdir(), "furb-busy-folder");
  mkdirSync(folder, { recursive: true });
  const file = join(folder, "held.txt");
  writeFileSync(file, "held");
  held = openSync(file, "r");
  expect(held).toBeGreaterThan(0);
});

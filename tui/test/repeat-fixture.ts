// A test that fails on one run of the many, as a flake does, for tui/test/repeat.test.ts. The name of the file has no
// .test, so that the runner of the suite never runs it by itself.
import { expect, test } from "bun:test";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

test("fails on a chosen run", () => {
  const counter = process.env.FURB_REPEAT_COUNTER ?? "";
  const run = (existsSync(counter) ? Number(readFileSync(counter, "utf8")) : 0) + 1;
  writeFileSync(counter, String(run));
  writeFileSync(join(process.env.FURB_YARD ?? ".", "trace.txt"), `run ${run}`);
  expect(run).not.toBe(Number(process.env.FURB_REPEAT_FAIL));
});

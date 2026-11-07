// The counts of lines that a write added and removed come from the hunks of the review, one calculation for the
// review and for the feed: a body line that starts with --- or +++ is a line, and a missing final newline counts exactly.
import { expect, test } from "bun:test";
import { reviewed } from "../src/review.ts";

const counts = (before: string, after: string) => {
  const { added, removed } = reviewed({ path: "f", before, after, act: "write1" } as never);
  return [added, removed];
};

test("a body line that starts with --- or +++ counts as a line", () => {
  expect(counts("--old\n", "++new\n")).toEqual([1, 1]);
  expect(counts("---\nkeep\n", "+++\nkeep\n")).toEqual([1, 1]);
});

test("a missing final newline counts exactly", () => {
  expect(counts("a\nb", "a\nb\n")).toEqual([1, 1]);
  expect(counts("a\nb\n", "a\nb")).toEqual([1, 1]);
  expect(counts("", "x\ny\n")).toEqual([2, 0]);
});

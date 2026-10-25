import { expect, test } from "bun:test";
import { matchingHistory } from "../src/history.ts";

test("history matches the typed prefix, deduplicates, and ranks recent entries first", () => {
  expect(matchingHistory(["open file", "other", "open folder", "open file"], "open")).toEqual([
    "open file",
    "open folder",
  ]);
});

test("history matching is case insensitive, bounded, and returns no empty entries", () => {
  expect(matchingHistory(["Alpha", "alpine", "", "beta"], "AL", 1)).toEqual(["alpine"]);
  expect(matchingHistory(["Alpha", "beta"], "missing")).toEqual([]);
});

test("history keeps distinct spelling and whitespace while deduplicating exact entries", () => {
  expect(matchingHistory(["find  file", "Find file", "find file", "find  file"], "find")).toEqual([
    "find  file",
    "find file",
    "Find file",
  ]);
});

test("history prefix matching preserves leading whitespace in the typed prefix", () => {
  expect(matchingHistory(["  echo", "echo"], " ")).toEqual(["  echo"]);
});

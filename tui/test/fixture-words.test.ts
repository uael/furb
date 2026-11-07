// The words of the operator never leave his machine, so no fixture of the repository holds the words or the traces of a
// person: each prompt of the operator in the long record of the conductor is one that this set scripts, and no line of
// it holds an account or a path of a home.
import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { scripted } from "../script/scripted.ts";

const SCRIPTED = new Set([
  "(removed)",
  "You are the conductor of a scripted life, in its third round. The second round stopped at noon, and its record cannot open again, so notes.md at the root hands over what it knew.\n1. Read notes.md whole first. Then read plan.md, board.md, rules.md and the guide at the root, which hold the rules of the scripted work and the way each agent reports.\n2. List the open items of the board, order them by their rank, and hire one agent for each of the first three, with a brief that names its files and its checks.\n3. Watch each agent until its pull request is green, then merge it, and tell the next agent on the list to start.\n4. Report the state of the board to the operator each time that an item ends, in one short line.\n… 9 more lines",
  "Have a worker loading the sample skill and renaming its panel, the labels are unclear.",
  "A second task is a bit more complex: a sample file that you read, and a note to hire a worker on this thread.",
  "tell the sample lane to add colors to the minimal panels, then report the state of the lane to the cond…",
  "the sample flow of how an answer is given to a question is confusing and hard to follow",
  "Report progress.",
]);

const record = readFileSync(join(import.meta.dir, "..", "fixtures", "conductor.jsonl"), "utf8");

test("each prompt of the operator in the record of the conductor is a scripted one", () => {
  const prompts = record
    .split("\n")
    .filter((line) => line.startsWith("["))
    .map((line) => JSON.parse(line) as unknown[])
    .filter((fact) => fact[0] === "prompt" && fact[2] === "operator");
  expect(prompts.length).toBeGreaterThan(5);
  expect(prompts.filter((fact) => !SCRIPTED.has(String(fact[5]))).map((fact) => fact[1])).toEqual([]);
});

test("the record of the conductor holds no account, no private host and no path of a home", () => {
  // The public repositories (uael/furb, uael/monty) may stand: the handle of a person stands nowhere else.
  const rest = record.replace(/uael\/(furb|monty)/g, "");
  expect(rest).not.toContain("uael");
  expect(record).not.toContain("/Users/");
  expect(record).not.toContain("/home/");
  expect(record).not.toMatch(/\b192\.168\.|DESKTOP-|\.local\b/);
  expect(record).not.toMatch(/scratch\/[0-9a-f]{16}\b(?<!0{16})(?<!1{16})/);
});

test("the record of the conductor is what the scripted words make: a second pass of the script writes it again", () => {
  // Each text of the record that is no token of the engine is a line of the scripted vocabulary, as long as the line that
  // it stands for, so the script that makes the record from its shape gives the same record from it.
  expect(scripted(record) === record).toBe(true);
});

test("the record of the conductor holds no address of a repository of a person and no number of a run or a pull request", () => {
  expect(record).not.toMatch(/github\.com\/(?!example)/);
  expect(record).not.toMatch(/\bactions\/runs\/\d|\/pull\/\d|\bjob\/\d/);
  expect(record).not.toMatch(/gh (?:run|pr|api)\b/);
});

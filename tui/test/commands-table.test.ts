// The table of commands is the one home of each name and alias: the alias reads as its command, and the README lists
// each command, by its name and its other name, from that table.
import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { canonical, slashes } from "../src/commands.ts";

const readme = readFileSync(join(import.meta.dir, "..", "README.md"), "utf8");

test("each alias of the table reads as its command, and no name or alias is taken twice", () => {
  const words = slashes.flatMap(({ name, alias }) => [name, ...(alias ? [alias] : [])]);
  expect(new Set(words).size).toBe(words.length);
  for (const { name, alias, argument } of slashes) {
    if (!alias) continue;
    expect(canonical(`/${alias}`)).toBe(`/${name}`);
    expect(canonical(`/${alias} some words`)).toBe(`/${name} some words`);
    expect(argument).toBeDefined();
  }
  // The commands that an operator expects stand in the table.
  for (const name of ["copy", "status", "help", "new", "model", "resume", "rename", "quit"])
    expect(slashes.some((one) => one.name === name || one.alias === name)).toBe(true);
});

test("the README lists each command of the table with its detail, in the row that bun run docs writes", () => {
  const cell = (text: string) => text.replaceAll("|", "\\|");
  const missing = slashes.filter(
    ({ forms, detail }) => !readme.includes(`| \`${cell(forms)}\` | ${cell(detail)} |`),
  );
  expect(missing.map((one) => one.name)).toEqual([]);
});

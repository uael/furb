import { expect, test } from "bun:test";
import { mkdtemp, rm } from "node:fs/promises";
import { homedir, tmpdir } from "node:os";
import { join } from "node:path";
import { FileChanges } from "../src/changes.ts";

test("a write of a path that the World writes with ~ for the home of the user is kept at its place in the home", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-changes-home-"));
  const journal = new FileChanges(join(directory, "life.jsonl"));
  try {
    const name = `furb-changes-home-${process.pid}-none`;
    const text = { is: "Text", path: `~/${name}/note.txt`, content: "one\n" };
    const ear = journal.ear("/nowhere");
    ear.next();
    ear.next(["write", "write1", "rung1", "chain1", text]);
    ear.next("~");
    ear.next(["done", "write1", "", text]);
    expect(journal.read()).toEqual([
      { path: join(homedir(), name, "note.txt"), before: "", after: "one\n", act: "write1", made: true },
    ]);
  } finally {
    journal.dispose();
    await rm(directory, { recursive: true, force: true });
  }
});

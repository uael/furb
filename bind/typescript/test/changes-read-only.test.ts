import { expect, test } from "bun:test";
import { mkdir, mkdtemp, readdir, readFile, rename, rm, stat, utimes, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { DamagedJournal, FileChanges } from "../src/changes.ts";

const change = (after: string) => JSON.stringify({ path: "note.txt", before: "", after });

test("a journal open to read keeps its bytes and time and refuses every write", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-read-journal-"));
  try {
    const record = join(directory, "life.jsonl");
    const path = `${record}.changes.jsonl`;
    const text = `${change("one")}\n${change("two")}\n`;
    await writeFile(path, text);
    const before = await stat(path);
    const journal = new FileChanges(record, true);
    try {
      expect(journal.length).toBe(2);
      expect(journal.read().map((one) => one.after)).toEqual(["one", "two"]);
      expect(journal.readable(1, 1).map((one) => one.after)).toEqual(["two"]);
      expect(() => journal.append({ path: "note.txt", before: "", after: "three" })).toThrow("open to read");
      expect(() => journal.repair()).toThrow("open to read");
      expect(() => journal.renew()).toThrow("open to read");
    } finally {
      journal.dispose();
    }
    expect(await readFile(path, "utf8")).toBe(text);
    expect((await stat(path)).mtimeMs).toBe(before.mtimeMs);
    expect(await readdir(directory)).toEqual(["life.jsonl.changes.jsonl"]);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("a read-only journal reports damaged rows and an incomplete tail without truncating either", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-read-journal-tail-"));
  try {
    const record = join(directory, "life.jsonl");
    const path = `${record}.changes.jsonl`;
    // Even valid JSON without its closing newline is not a committed change.
    const text = `${change("one")}\n{ damaged }\n${change("two")}\n${change("tail")}`;
    await writeFile(path, text);
    const before = await stat(path);
    const journal = new FileChanges(record, true);
    try {
      expect(journal.length).toBe(4);
      expect(journal.read(0, 1).map((one) => one.after)).toEqual(["one"]);
      expect(() => journal.read(1, 1)).toThrow(new DamagedJournal(path, 1));
      expect(() => journal.read(3, 1)).toThrow(new DamagedJournal(path, 3));
      expect(journal.readable().map((one) => one.after)).toEqual(["one", "two"]);
    } finally {
      journal.dispose();
    }
    expect(await readFile(path, "utf8")).toBe(text);
    expect((await stat(path)).mtimeMs).toBe(before.mtimeMs);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("a read-only journal creates neither a missing record folder nor a missing journal", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-read-journal-missing-"));
  try {
    const journal = new FileChanges(join(directory, "missing", "life.jsonl"), true);
    try {
      expect(journal.length).toBe(0);
      expect(journal.read()).toEqual([]);
    } finally {
      journal.dispose();
    }
    expect(await readdir(directory)).toEqual([]);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("opening a writable journal keeps a cut tail until an actual write replaces it", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-journal-deferred-tail-"));
  try {
    const record = join(directory, "life.jsonl");
    const path = `${record}.changes.jsonl`;
    const text = `${change("one")}\n{"path":"cut`;
    await writeFile(path, text);
    const journal = new FileChanges(record);
    try {
      expect(await readFile(path, "utf8")).toBe(text);
      expect(journal.read().map((one) => one.after)).toEqual(["one"]);
      journal.append({ path: "note.txt", before: "one", after: "two" });
      expect(journal.read().map((one) => one.after)).toEqual(["one", "two"]);
      expect((await readFile(path, "utf8")).split("\n")).toHaveLength(3);
    } finally {
      journal.dispose();
    }
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("a journal that cannot be read reports its error even on an empty page", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-read-journal-error-"));
  try {
    const record = join(directory, "life.jsonl");
    await mkdir(`${record}.changes.jsonl`);
    const journal = new FileChanges(record, true);
    try {
      expect(journal.length).toBe(0);
      expect(() => journal.read(0, 0)).toThrow();
      expect(() => journal.readable()).toThrow();
    } finally {
      journal.dispose();
    }
    expect(await readdir(directory)).toEqual(["life.jsonl.changes.jsonl"]);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("a journal row with a path but no text is damaged, not a file change", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-read-journal-fields-"));
  const record = join(directory, "life.jsonl");
  const path = `${record}.changes.jsonl`;
  const bytes = `${JSON.stringify({ path: "notes.txt", before: 42, after: null })}\n`;
  try {
    await writeFile(path, bytes);
    const journal = new FileChanges(record, true);
    try {
      expect(() => journal.read()).toThrow(DamagedJournal);
      expect(journal.readable()).toEqual([]);
    } finally {
      journal.dispose();
    }
    expect(await readFile(path, "utf8")).toBe(bytes);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("a failed journal repair keeps its readable changes and can be retried", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-journal-repair-failed-"));
  const record = join(directory, "life.jsonl");
  const path = `${record}.changes.jsonl`;
  const bytes = `${change("one")}\n{ damaged }\n`;
  let journal: FileChanges | undefined;
  try {
    await writeFile(path, bytes);
    await mkdir(`${path}.repair`);
    journal = new FileChanges(record);
    const opened = journal;
    expect(() => opened.repair()).toThrow();
    expect(await readFile(path, "utf8")).toBe(bytes);
    expect(opened.read(0, 1).map((one) => one.after)).toEqual(["one"]);
    expect(() => opened.read(1, 1)).toThrow(DamagedJournal);
    await rm(`${path}.repair`, { recursive: true });
    expect(opened.repair()).toEqual({ kept: 1, removed: 1 });
    opened.append({ path: "note.txt", before: "one", after: "two" });
    expect(opened.read().map((one) => one.after)).toEqual(["one", "two"]);
    opened.dispose();
    journal = undefined;
    expect(() => opened.dispose()).not.toThrow();
  } finally {
    try {
      journal?.dispose();
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  }
});

test("a journal open to read does not prevent its writer from repairing it", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-journal-reader-repair-"));
  const record = join(directory, "life.jsonl");
  const path = `${record}.changes.jsonl`;
  let writer: FileChanges | undefined;
  let reader: FileChanges | undefined;
  try {
    await writeFile(path, `${change("one")}\n{ damaged }\n${change("two")}\n`);
    writer = new FileChanges(record);
    reader = new FileChanges(record, true);
    expect(reader.read(0, 1).map((one) => one.after)).toEqual(["one"]);
    expect(writer.repair()).toEqual({ kept: 2, removed: 1 });
    expect(writer.read().map((one) => one.after)).toEqual(["one", "two"]);
    expect(reader.read().map((one) => one.after)).toEqual(["one", "two"]);
  } finally {
    try {
      reader?.dispose();
      writer?.dispose();
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  }
});

test("a read-only journal sees a same-size atomic replacement", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-read-journal-replaced-"));
  const record = join(directory, "life.jsonl");
  const path = `${record}.changes.jsonl`;
  const replacement = `${path}.replacement`;
  const reader = new FileChanges(record, true);
  try {
    await writeFile(path, `${change("one")}\n`);
    expect(reader.read().map((one) => one.after)).toEqual(["one"]);
    const before = await stat(path);
    await writeFile(replacement, `${change("two")}\n`);
    await utimes(replacement, before.atime, before.mtime);
    await rename(replacement, path);
    expect((await stat(path)).size).toBe(before.size);
    expect(reader.read().map((one) => one.after)).toEqual(["two"]);
  } finally {
    reader.dispose();
    await rm(directory, { recursive: true, force: true });
  }
});

test("a disposed read-only journal does not reopen its file", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-read-journal-disposed-"));
  const record = join(directory, "life.jsonl");
  const path = `${record}.changes.jsonl`;
  await writeFile(path, `${change("one")}\n`);
  const reader = new FileChanges(record, true);
  try {
    expect(reader.read().map((one) => one.after)).toEqual(["one"]);
    reader.dispose();
    expect(reader.read()).toEqual([]);
    expect(reader.readable()).toEqual([]);
    expect(await readFile(path, "utf8")).toBe(`${change("one")}\n`);
  } finally {
    reader.dispose();
    await rm(directory, { recursive: true, force: true });
  }
});

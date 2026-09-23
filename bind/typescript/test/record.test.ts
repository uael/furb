import { expect, test } from "bun:test";
import { existsSync } from "node:fs";
import { mkdtemp, readdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { RecordLock } from "../src/index.ts";
import { RecordFile } from "../src/record.ts";

test("a record of more entries than one call takes as arguments opens whole", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-big-record-"));
  const path = join(cwd, "record.jsonl");
  try {
    const count = 700_000;
    await writeFile(path, '["x",["done","x","world",null]]\n'.repeat(count));
    const record = new RecordFile(path);
    try {
      expect(record.entries).toHaveLength(count);
    } finally {
      record.dispose();
    }
    expect(new RecordFile(path, true).entries).toHaveLength(count);
  } finally {
    await rm(cwd, { recursive: true });
  }
}, 60000);

test("a lease left by a process that ended is taken, whatever process its number names now", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-lease-"));
  const path = join(cwd, "record.jsonl");
  try {
    // A lease file of an earlier life of this very process, or of a process that reused its number.
    await writeFile(`${path}.lock`, String(process.pid));
    const record = new RecordFile(path);
    record.dispose();
  } finally {
    await rm(cwd, { recursive: true });
  }
});

test("one holder at a time owns a record, and a holder that ends ends only its own lease", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-lease-"));
  const path = join(cwd, "record.jsonl");
  try {
    const first = new RecordLock(path);
    expect(first.path).toBe(path);
    expect(() => new RecordLock(path)).toThrow(`Another process owns ${path}.`);
    first.dispose();
    const second = new RecordLock(path);
    first.dispose();
    expect(() => new RecordLock(path)).toThrow(`Another process owns ${path}.`);
    second.dispose();
    expect(existsSync(`${path}.lock`)).toBe(true);
    new RecordLock(path).dispose();
  } finally {
    await rm(cwd, { recursive: true });
  }
});

test("a holder may move the lock file, and two processes never own the record at once", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-lease-"));
  const path = join(cwd, "record.jsonl");
  const racer = join(import.meta.dir, "lease-racer.ts");
  const roles = ["", "", "", "", "", "", "move", "move"];
  try {
    const racers = roles.map((role) =>
      Bun.spawn([process.execPath, racer, path, "50", role], { stdout: "pipe", stderr: "pipe" }),
    );
    // The race starts once every contender has loaded, however long a system takes to start a process.
    const ready = async () => (await readdir(cwd)).filter((name) => name.includes(".ready.")).length;
    for (const deadline = Date.now() + 20000; (await ready()) < roles.length; await Bun.sleep(20))
      if (Date.now() > deadline) throw new Error(`${await ready()} of ${roles.length} contenders are ready.`);
    await writeFile(`${path}.go`, "");
    const ends = await Promise.all(
      racers.map(async (child) => ({
        code: await child.exited,
        taken: Number(await new Response(child.stdout).text()),
        error: await new Response(child.stderr).text(),
      })),
    );
    expect(ends.map(({ code, error }) => (code ? error : ""))).toEqual(ends.map(() => ""));
    expect(ends.map(({ taken }) => taken)).toEqual(ends.map(() => 50));
  } finally {
    await rm(cwd, { recursive: true });
  }
}, 60000);

import { expect, test } from "bun:test";
import {
  appendFile,
  mkdtemp,
  readdir,
  readFile,
  rename,
  rm,
  stat,
  utimes,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createTestRenderer } from "@opentui/core/testing";
import { retold } from "furb";
import { App } from "../src/app.ts";
import { openEngine } from "../src/bridge.ts";
import { Preferences } from "../src/preferences.ts";
import { guard, Reader } from "../src/reading.ts";
import { Workspaces } from "../src/workspaces.ts";

/** What the folder of a record holds that belongs to it: each file by name, with its time and its bytes. */
async function folder(record: string): Promise<Map<string, { time: number; bytes: string }>> {
  const directory = join(record, "..");
  const base = record.slice(directory.length + 1);
  const held = new Map<string, { time: number; bytes: string }>();
  for (const name of (await readdir(directory)).sort()) {
    if (name !== base && !name.startsWith(`${base}.`)) continue;
    const info = await stat(join(directory, name));
    // A file is held by its time and its bytes, since a read changes neither.
    held.set(
      name,
      info.isFile()
        ? { time: info.mtimeMs, bytes: (await readFile(join(directory, name))).toString("base64") }
        : { time: 0, bytes: "folder" },
    );
  }
  return held;
}

/** Whether a folder is as it was. A time that is put back goes through seconds in a double, so a file system that
 * keeps the time finer than a double reads it back a few microseconds off: the times agree within a millisecond. */
function expectKept(
  got: Map<string, { time: number; bytes: string }>,
  was: Map<string, { time: number; bytes: string }>,
) {
  expect([...got.keys()]).toEqual([...was.keys()]);
  for (const [name, one] of was) {
    expect(got.get(name)?.bytes).toBe(one.bytes);
    expect(Math.abs((got.get(name)?.time ?? Number.NaN) - one.time)).toBeLessThan(1);
  }
}

/** A record of a demo life with a message of the operator, a write and a command, which an earlier engine could keep. */
async function lived(directory: string, name: string): Promise<string> {
  const record = join(directory, `${name}.jsonl`);
  const first = await openEngine({ cwd: directory, record, demo: true });
  const { engine } = first;
  await engine.result(await engine.prompt("str", { message: "Say hello.", on: engine.root }));
  await engine.result(
    await engine.rung({ word: 'write(Text("note.txt", "x"))\nawait bash("echo hi")', on: engine.root }),
  );
  await first.host.dispose();
  return record;
}

/** The record as a build that made it before would keep the command: with a word that this engine does not make. */
async function drifting(record: string): Promise<void> {
  const text = await readFile(record, "utf8");
  expect(text).toContain('"echo hi",""');
  await writeFile(record, text.replace('"echo hi",""', '"echo hi",false'));
}

/** The record as another engine made it, which it names, or which it does not name. */
async function made(record: string, named: boolean): Promise<void> {
  const text = await readFile(record, "utf8");
  const fact = /\["made","chain1","store","[0-9a-f]{64}",\d+\]/.exec(text)?.[0];
  if (!fact) throw new Error("The record names no engine.");
  const without = text
    .split("\n")
    .filter((line) => !line.includes(fact))
    .join("\n");
  await writeFile(
    record,
    named ? text.replace(fact, `["made","chain1","store","${"0".repeat(64)}"]`) : without,
  );
}

/** A record open to read in an App of a width, and the frame that the App draws first. */
async function shown(record: string, directory: string, width: number) {
  const library = new Workspaces(new Preferences(join(directory, "config/ui.json")), { demo: true });
  await library.import(record, await library.add(directory));
  const session = library.current?.session;
  if (!session) throw new Error("The record did not open.");
  const screen = await createTestRenderer({ width, height: 44, exitSignals: [] });
  const app = new App(screen.renderer, session, { quit() {}, workspaces: library });
  await screen.renderOnce();
  return {
    library,
    session,
    screen,
    frame: () => screen.captureCharFrame(),
    close: async () => {
      app.dispose();
      screen.renderer.destroy();
      await library.dispose();
    },
  };
}

for (const width of [80, 160]) {
  test(`the dialog of a record of a named other engine names both engines at ${width} columns`, async () => {
    const directory = await mkdtemp(join(tmpdir(), "furb-engines-"));
    try {
      const record = await lived(directory, "other");
      await made(record, true);
      const open = await shown(record, directory, width);
      try {
        const reading = open.session.host.reading;
        const frame = open.frame();
        expect(frame).toContain("This record was made by another build");
        expect(frame).toContain(`Made by: ${"0".repeat(12)}`);
        expect(frame).toContain(`This build: ${reading?.engine.slice(0, 12)}`);
        expect(frame).toMatch(/A new life carries \d+ turns? of its root chain and the 1 file/);
        expect(frame).toContain("Start a new life from it");
      } finally {
        await open.close();
      }
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });

  test(`the dialog of a record of an unnamed earlier engine says so, with this build, at ${width} columns`, async () => {
    const directory = await mkdtemp(join(tmpdir(), "furb-engines-"));
    try {
      const record = await lived(directory, "earlier");
      await made(record, false);
      await drifting(record);
      const open = await shown(record, directory, width);
      try {
        const reading = open.session.host.reading;
        const frame = open.frame();
        expect(reading?.made).toBeNull();
        expect(frame).toContain("Made by: an earlier engine, not named");
        expect(frame).toContain(`This build: ${reading?.engine.slice(0, 12)}`);
      } finally {
        await open.close();
      }
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });
}

test("a record open to read says so in its footer and its composer, and shows no spinner and no running work", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-footer-"));
  try {
    const record = await lived(directory, "drifts");
    // With no log, the record shows only the facts that it kept, and the work that had no end stays so.
    await rm(`${record}.log`, { force: true });
    await drifting(record);
    const open = await shown(record, directory, 120);
    try {
      open.screen.mockInput.pressEscape();
      await open.screen.renderOnce();
      await Bun.sleep(50);
      await open.screen.renderOnce();
      const frame = open.frame();
      expect(frame).toContain("Read only: its replay drifts");
      expect(frame).toContain("Start a new life");
      expect(frame).not.toMatch(/[⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏]/);
      expect(frame).not.toContain("running");
      expect(frame).toContain("not finished when the record ends");
      expect(open.session.status()).toBe("reading");
      await open.screen.mockInput.typeText("zzqq");
      await open.screen.renderOnce();
      expect(open.frame()).not.toContain("zzqq");
    } finally {
      await open.close();
    }
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("a read leaves the record and the files beside it as they were", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-same-"));
  try {
    const record = await lived(directory, "drifts");
    await drifting(record);
    const before = await folder(record);
    const open = await shown(record, directory, 120);
    try {
      expect(open.session.host.reading).toBeDefined();
      open.session.save();
      expectKept(await folder(record), before);
      expect([...before.keys()].filter((name) => /\.log\.new$|\.ui\.json$/.test(name))).toEqual([]);
    } finally {
      await open.close();
    }
    expectKept(await folder(record), before);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("a new life from a record starts with the turns of its root chain and the files that it wrote", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-carry-"));
  try {
    const record = await lived(directory, "drifts");
    await drifting(record);
    const open = await shown(record, directory, 120);
    try {
      const carry = await open.session.host.carry();
      expect(carry.turns).toBeGreaterThanOrEqual(2);
      expect(carry.files).toHaveLength(1);
      expect(carry.word).toContain("Say hello.");
      const group = open.library.groupOf();
      if (!group) throw new Error("No workspace.");
      const fresh = await open.library.create(group, undefined, carry);
      expect(fresh.session?.host.reading).toBeUndefined();
      const kept = await readFile(fresh.path, "utf8");
      expect(kept).toContain("earlier_files");
      expect(kept).toContain("Say hello.");
      expect(kept).toContain("note.txt");
    } finally {
      await open.close();
    }
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("a chain that the record does not keep takes its first prompt as its label and its maker's chain as its parent", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-labels-"));
  try {
    const record = join(directory, "kept.jsonl");
    const kept = [
      ["chain", "chain1", "operator", "", "root", ""],
      ["prompt", "prompt1", "operator", "chain1", "str", "Go.", ""],
      ["reply", "reply1", "rung1", "chain1", "opus"],
      ["reply", "reply2", "rung2", "chain1", "opus"],
      ["prompt", "prompt2", "rung1", "chain2", "str", "Review the diff.\nThen the tests.", ""],
      ["bash", "bash1", "rung7", "chain3", "ls", "", 600],
      ["prompt", "prompt3", "operator", "chain4", "str", "Other.", ""],
    ];
    await writeFile(record, `${kept.map((fact) => JSON.stringify([fact])).join("\n")}\n`);
    const got = retold(record);
    const reader = new Reader(got.facts as never, directory, record, {
      made: got.made,
      engine: got.engine,
      version: got.version,
      current: got.current,
      why: "x",
      logged: got.logged,
    });
    // A rung that the record ended shows as done, and the work with no end stays unfinished.
    const rungs = [...reader.activity.acts.values()].filter((act) => act.kind === "rung");
    expect(reader.activity.acts.get("rung1")?.done).toBe(true);
    expect(rungs.filter((act) => act.done && act.run?.status === "running")).toEqual([]);
    expect(rungs.filter((act) => !act.done && !act.unfinished && act.run?.status === "running")).toEqual([]);
    const labels = [...reader.activity.acts.values()]
      .filter((act) => act.kind === "chain")
      .map((act) => String(act.words[0]));
    expect(labels).toEqual(["root", "Review the diff.", "chain3", "Other."]);
    expect(new Set(labels).size).toBe(labels.length);
    const two = (got.facts as unknown[][]).find((fact) => fact[0] === "chain" && fact[1] === "chain2");
    expect(two?.[2]).toBe("rung1");
    expect(two?.[3]).toBe("chain1");
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("the guard gives back a record that a life wrote beside before its replay drifted", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-guard-"));
  try {
    for (const way of ["replaced", "appended", "absent"]) {
      const record = join(directory, `${way}.jsonl`);
      await writeFile(record, "one\ntwo\n");
      if (way !== "absent") await writeFile(`${record}.log`, "log\n");
      await writeFile(`${record}.ui.json`, "{}");
      await utimes(record, 1_700_000_000, 1_700_000_000);
      const before = await folder(record);
      const held = guard(record);
      await Bun.sleep(5);
      await appendFile(record, "three\n");
      await writeFile(`${record}.log.new`, "fresh log\n");
      if (way === "replaced" || way === "absent") await rename(`${record}.log.new`, `${record}.log`);
      else await appendFile(`${record}.log`, "more\n");
      await writeFile(`${record}.snapshot`, "state");
      held.undo();
      expectKept(await folder(record), before);
      expect((await stat(record)).mtimeMs).toBe(1_700_000_000_000);
      const clean = guard(record);
      clean.keep();
      expectKept(await folder(record), before);
    }
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

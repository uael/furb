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
    // The file of what the record tells is a cache that a read derives from the record and writes again when it is
    // stale, so it is no change of the record.
    if (name === `${base}.told`) continue;
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
    app,
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
      expect(open.session.host.reading).toBeDefined();
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

test("a record of an earlier engine offers a resume that names its open prompts, and gives the text where no extension runs", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-resume-"));
  try {
    const record = await lived(directory, "older");
    await made(record, true);
    // The earlier life left a prompt of a model open when it ended.
    await appendFile(
      record,
      `${JSON.stringify([["prompt", "prompt90", "operator", "chain1", "str", "Finish the note.", ""], Date.now()])}\n`,
    );
    const open = await shown(record, directory, 120);
    try {
      // The prompt of the demo that its model never closed stands open in the record beside the one that was added.
      expect(open.session.host.reading?.carries?.prompts).toBe(2);
      expect(open.frame()).toContain("Resume it");
      expect(open.frame()).toContain("2 open prompts");
      const carry = await open.session.host.carry();
      expect(carry.prompts).toBe(2);
      // The path is the one the reader resolved, so a system that spells a folder two ways gives either spelling.
      expect(carry.resume).toMatch(/^await inherit\(".*older\.jsonl", "chain1"\)$/);
      const group = open.library.groupOf();
      if (!group) throw new Error("No workspace.");
      const fresh = await open.library.create(group, undefined, carry, true);
      expect(fresh.session?.host.reading).toBeUndefined();
      const kept = await readFile(fresh.path, "utf8");
      // The demo life runs no extension, so it has no `inherit`, and the earlier work comes as text.
      expect(kept).toContain("earlier_files");
      expect(kept).toContain("Say hello.");
      expect(fresh.session?.notice).toBe(
        "The extension compaction is off, so the earlier work came as text.",
      );
    } finally {
      await open.close();
    }
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("a record of an earlier engine resumes in a real life: its turns go on and its closed work is not asked again", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-resume-real-"));
  try {
    const record = await lived(directory, "older");
    await made(record, true);
    // The earlier life closed its only prompt, so the resume has no prompt to ask again.
    await appendFile(record, `${JSON.stringify([["done", "prompt1", "chain1", "Done."], Date.now()])}\n`);
    const library = new Workspaces(new Preferences(join(directory, "config/ui.json")), {});
    try {
      await library.import(record, await library.add(directory));
      const session = library.current?.session;
      expect(session?.host.reading?.carries?.prompts).toBe(0);
      const carry = await session?.host.carry();
      if (!carry) throw new Error("The record did not open.");
      const group = library.groupOf();
      if (!group) throw new Error("No workspace.");
      const fresh = await library.create(group, undefined, carry, true);
      expect(fresh.session?.host.reading).toBeUndefined();
      expect(fresh.session?.notice).toBe("Resumed from the turns of the record.");
      const kept = await readFile(fresh.path, "utf8");
      // The word is a rung of the new life, and the carry answers it with the turns of the record.
      expect(kept).toContain("await inherit(");
      expect(kept).toContain('["done","carry1"');
      // A record with no log keeps the replies of the root chain, which the carry hands to the new life as its turns.
      expect(kept).toContain("The change is small");
      expect(kept).not.toContain("earlier_files");
    } finally {
      await library.dispose();
    }
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("a recorded answer keeps its cost when its tuples have the record wrapper", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-record-cost-"));
  try {
    const record = join(directory, "cost.jsonl");
    const tuple = (args: unknown[]) => ({ is: "tuple", args: [args] });
    const facts = [
      ["chain", "chain1", "operator", "", "root", ""],
      ["reply", "reply1", "rung1", "chain1", "opus"],
      ["done", "reply1", "journal", tuple(["assistant", "", tuple([10, 2, 0, 8, 0.25]), null])],
    ];
    await writeFile(record, `${facts.map((fact) => JSON.stringify(fact)).join("\n")}\n`);
    const reading = { made: null, engine: "old", version: null, current: 1, why: "drift", logged: false };
    const reader = new Reader(directory, record, reading);
    expect(reader.activity.cost).toBe(0.25);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("a close in the record ends the work it closed without ending truncated work", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-record-close-"));
  try {
    const record = join(directory, "close.jsonl");
    const facts = [
      ["chain", "chain1", "operator", "", "root", ""],
      ["prompt", "prompt1", "operator", "chain1", "str", "Do the work.", ""],
      ["rung", "rung1", "prompt1", "chain1", "closed word", "", ""],
      ["close", "prompt1", "console", "finished"],
      ["rung", "rung2", "operator", "chain1", "unfinished word", "", ""],
    ];
    await writeFile(record, `${facts.map((fact) => JSON.stringify(fact)).join("\n")}\n`);
    const reading = { made: null, engine: "old", version: null, current: 1, why: "drift", logged: false };
    const reader = new Reader(directory, record, reading);
    expect(reader.activity.acts.get("prompt1")).toMatchObject({ done: true, value: "finished" });
    const closed = reader.activity.acts.get("rung1");
    expect(closed).toMatchObject({ done: true });
    expect(closed?.unfinished).toBeUndefined();
    expect(reader.activity.acts.get("rung2")).toMatchObject({ done: false, unfinished: true });
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
    const reader = new Reader(
      directory,
      record,
      {
        made: got.made,
        engine: got.engine,
        version: got.version,
        current: got.current,
        why: "x",
        logged: got.logged,
      },
      got.file,
    );
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
    const facts = (await readFile(got.file, "utf8"))
      .split("\n")
      .filter((line) => line.startsWith("["))
      .map((line) => JSON.parse(line) as unknown[]);
    const two = facts.find((fact) => fact[0] === "chain" && fact[1] === "chain2");
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

test("a record whose preservation guard cannot be made refuses before replay", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-record-guard-failed-"));
  try {
    const record = await lived(directory, "drifts");
    await drifting(record);
    await writeFile(`${record}.log.held`, "This file belongs to another operation.\n");
    const before = await folder(record);
    let opened: Awaited<ReturnType<typeof openEngine>> | undefined;
    let failure: unknown;
    try {
      opened = await openEngine({ cwd: directory, record, demo: true });
    } catch (error) {
      failure = error;
    } finally {
      await opened?.host.dispose();
    }
    expect(String(failure)).toMatch(/EEXIST|already exists/i);
    expectKept(await folder(record), before);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("a record name cannot add Python statements to the word that carries its work", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-carry-name-"));
  try {
    const file = join(directory, "facts.jsonl");
    await writeFile(file, `${JSON.stringify(["chain", "chain1", "operator", "", "root", ""])}\n`);
    const name = 'earlier\nraise RuntimeError("name is data")';
    const reader = new Reader(
      directory,
      join(directory, name),
      { made: null, engine: "old", version: null, current: 1, why: "drift", logged: false },
      file,
    );
    const carried = reader.carry().word.split("\n");
    expect(carried[0]).toContain("\\nraise RuntimeError");
    expect(carried[1]).toBe("<s:earlier>");
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

for (const width of [80, 160]) {
  test(`a record's Life review reads its journal without live controls at ${width} columns`, async () => {
    const directory = await mkdtemp(join(tmpdir(), "furb-reading-changes-"));
    try {
      const record = await lived(directory, "journal");
      await drifting(record);
      await appendFile(
        `${record}.changes.jsonl`,
        `${JSON.stringify({ path: join(directory, "legacy.txt"), before: "old\n", after: "new\n" })}\n`,
      );
      const before = await folder(record);
      const open = await shown(record, directory, width);
      try {
        open.app.render();
        open.app.closeOverlay();
        open.session.review("life");
        await open.session.refresh();
        expect(open.session.view).toBe("changes");
        open.app.render();
        await open.screen.flush();
        await open.app.colorsDone();
        expect(open.session.changes.map((change) => change.path)).toEqual([
          join(directory, "note.txt"),
          join(directory, "legacy.txt"),
        ]);
        expect(open.frame()).toContain("2 files");
        expect(open.frame()).toContain("note.txt");
        expect(open.frame()).toContain("legacy.txt");
        expect(open.frame()).not.toContain("Revert");
        expect(open.frame()).not.toContain("Comment a line");
        expect(open.frame()).not.toContain("revert hunk");
      } finally {
        await open.close();
      }
      expectKept(await folder(record), before);
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });
}

test("a record's damaged Life journal remains whole and offers no repair or new journal", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-reading-damaged-changes-"));
  try {
    const record = await lived(directory, "damaged");
    await drifting(record);
    await appendFile(`${record}.changes.jsonl`, '{"path":"cut');
    const before = await folder(record);
    const open = await shown(record, directory, 120);
    try {
      open.app.render();
      open.app.closeOverlay();
      open.session.review("life");
      await open.session.refresh();
      expect(open.session.view).toBe("changes");
      open.app.render();
      await open.screen.flush();
      expect(open.session.journalDamaged).toBe(true);
      expect(open.frame()).toContain("The journal of file changes is damaged");
      expect(open.frame()).toMatch(/Changes\s+\?/);
      expect(open.frame()).not.toContain("Repair the journal");
      expect(open.frame()).not.toContain("Start a new journal");
    } finally {
      await open.close();
    }
    expectKept(await folder(record), before);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("a record with no journal gains no journal when its replay drifts", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-reading-no-journal-"));
  try {
    const record = await lived(directory, "missing");
    await rm(`${record}.changes.jsonl`);
    await drifting(record);
    const before = await folder(record);
    const open = await shown(record, directory, 80);
    try {
      expect(open.session.host.reading).toBeDefined();
      expect(await open.session.host.netChanges(null)).toEqual([]);
      open.app.render();
      open.app.closeOverlay();
      open.session.review("life");
      await open.session.refresh();
      open.app.render();
      await open.screen.flush();
      expect(open.frame()).toContain("No file-change snapshots are available");
      expect(open.frame()).toMatch(/Changes\s+\?/);
      expect(open.frame()).not.toContain("No net change");
    } finally {
      await open.close();
    }
    expectKept(await folder(record), before);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("a guard with no log keeps a held file that it did not make", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-record-other-held-"));
  try {
    const record = join(directory, "life.jsonl");
    await writeFile(record, "");
    await writeFile(`${record}.log.held`, "another operation\n");
    const before = await folder(record);
    guard(record).keep();
    expectKept(await folder(record), before);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

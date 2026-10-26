import { expect, test } from "bun:test";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { makers } from "../../bind/typescript/src/life/ladder.ts";
import { threadProgram } from "../../bind/typescript/src/life/program.ts";
import { executable, programs } from "../../bind/typescript/test/executable.ts";
import { act } from "../../bind/typescript/test/life/acts.ts";
import { Formatter } from "../src/ruff.ts";

const ruff = Bun.which("ruff");
const label = (id: string) => id;
export const longWord =
  'result = await bash("printf original-source", stdin="keep the whole payload", timeout=600, on="chain1")';

test.skipIf(!ruff)(
  "actual Ruff-100 wraps a completed word, preserving ownership, refusal coordinates, quotes and original source",
  async () => {
    let changes = 0;
    const formatter = new Formatter(
      () => changes++,
      () => ruff,
    );
    try {
      expect(formatter.shape(longWord)).toBeUndefined();
      await formatter.idle();
      const shape = formatter.shape(longWord);
      expect(shape?.size).toBeGreaterThan(0);
      const prompt = act("prompt1", "prompt", "chain1", "operator", 1);
      const rung = act("rung1", "rung", "chain1", "prompt1", 2, 5, {
        run: { status: "failed", reason: "line 1: original refusal" },
      });
      const bash = act("bash1", "bash", "chain1", "rung1", 3, 4);
      const next = act("rung2", "rung", "chain1", "prompt1", 6, 7);
      const made = makers([prompt, rung, bash, next]);
      const rows = threadProgram(
        [prompt],
        made,
        (step) => (step === rung ? longWord : "print(1)"),
        new Set(),
        label,
        undefined,
        undefined,
        (step) => (step === rung ? shape : undefined),
      );
      const code = rows.filter((row) => row.kind === "code");
      expect(code.length).toBeGreaterThan(2);
      expect(code.every((row) => Bun.stringWidth(row.text) <= 100)).toBe(true);
      expect(code.map((row) => row.display)).toEqual(code.map((_, at) => at + 1));
      expect(code[0]?.acts).toEqual([bash]);
      expect(code[0]?.fails).toBe("line 1: original refusal");
      expect(code.slice(0, -1).every((row) => row.source === longWord && row.number === 1)).toBe(true);
      expect(code.at(-1)?.number).toBe(1);
      expect(changes).toBe(1);
      const raw = threadProgram([prompt], made, () => longWord, new Set(), label);
      expect(raw[0]?.text).toBe(longWord);
      // No formatter call is made for a live word, even when it happens to parse as complete Python.
      const live = { ...rung, done: false, run: undefined };
      let requested = 0;
      threadProgram(
        [prompt],
        makers([prompt, live]),
        () => longWord,
        new Set(),
        label,
        undefined,
        undefined,
        () => {
          requested++;
          return shape;
        },
      );
      expect(requested).toBe(0);
      const quoted = '<s:brief>\n雪😀 "\\n"\n</s:brief>\n' + longWord;
      formatter.shape(quoted);
      await formatter.idle();
      const quoteRows = threadProgram(
        [prompt],
        makers([prompt, rung]),
        () => quoted,
        new Set(),
        label,
        undefined,
        undefined,
        () => formatter.shape(quoted),
      );
      expect(quoteRows[0]).toMatchObject({ kind: "quote", value: '雪😀 "\\n"\n', number: 1 });
      expect(quoteRows.filter((row) => row.kind === "code").every((row) => row.number === 4)).toBe(true);
      const separate = act("prompt2", "prompt", "chain1", "operator", 10);
      const separateRung = { ...next, by: separate.id };
      expect(
        threadProgram([separate], makers([separate, separateRung]), () => "x=1", new Set(), label)[0],
      ).toMatchObject({ number: 1, display: 1 });
    } finally {
      formatter.dispose();
    }
  },
);

test.skipIf(!ruff)(
  "actual Ruff truthfully falls back for partial and invalid words; marker-like Unicode strings and multiline literals survive",
  async () => {
    const formatter = new Formatter(
      () => {},
      () => ruff,
    );
    try {
      for (const word of ['text = """unfinished\n雪', "x = (", "if invalid ! syntax"]) {
        formatter.shape(word);
        await formatter.idle();
        expect(formatter.shape(word)).toBeUndefined();
      }
      const word =
        '# furb:statement\ntext="""雪😀\n# furb:statement-1\nmore"""\nraw = r"""C:\\new\n\\path"""';
      formatter.shape(word);
      await formatter.idle();
      const shape = formatter.shape(word);
      const prompt = act("prompt1", "prompt", "chain1", "operator", 1);
      const rung = act("rung1", "rung", "chain1", "prompt1", 2, 5);
      const rows = threadProgram(
        [prompt],
        makers([prompt, rung]),
        () => word,
        new Set(),
        label,
        undefined,
        undefined,
        () => shape,
      );
      expect(rows.some((row) => row.text.includes("雪😀\\n# furb:statement-1\\nmore"))).toBe(true);
      expect(rows.some((row) => row.kind === "code" && row.source?.includes('r"""C:\\new'))).toBe(true);
    } finally {
      formatter.dispose();
    }
  },
);

async function formatterExecutable(
  directory: string,
  name: string,
  settings: { blocked?: boolean; started?: string; gate?: string; log?: string } = {},
): Promise<string> {
  const command = executable(join(programs, "ruff.ts"), directory, name);
  await writeFile(`${command}.json`, JSON.stringify(settings));
  return command;
}

async function until(check: () => Promise<boolean>): Promise<void> {
  for (let at = 0; at < 300; at++) {
    if (await check()) return;
    await Bun.sleep(10);
  }
  throw new Error("The formatter fixture did not reach its observable state");
}

test("per-window callbacks, changed commands, a full FIFO and disposal settle without starvation or late redraw", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-ruff-"));
  const log = join(directory, "calls");
  const started = join(directory, "started");
  const blocked = await formatterExecutable(directory, "blocked", { blocked: true, started });
  const fast = await formatterExecutable(directory, "fast", { log });
  const missing = join(directory, "missing");
  let command: string | null = blocked;
  let first = 0;
  let second = 0;
  const a = new Formatter(
    () => first++,
    () => command,
  );
  const b = new Formatter(
    () => second++,
    () => fast,
  );
  try {
    a.shape("x=0");
    await until(async () => Number(await readFile(started, "utf8").catch(() => "")) > 1);
    const pid = Number(await readFile(started, "utf8"));
    for (let n = 1; n <= 70; n++) a.shape(`x=${n}`);
    command = fast;
    a.begin();
    // The command change cancels the old active work AND invalidates its queued words.
    a.shape("x=0");
    b.shape("x=0");
    await Promise.all([a.idle(), b.idle()]);
    expect(a.shape("x=0")?.get(0)?.lines).toEqual(["x = 0"]);
    expect(b.shape("x=0")?.get(0)?.lines).toEqual(["x = 0"]);
    expect(first).toBe(1);
    expect(second).toBe(1);
    expect(() => process.kill(pid, 0)).toThrow();
    expect((await readFile(log, "utf8")).match(/x=0/g)).toHaveLength(2);
    a.begin();
    for (let n = 1; n <= 70; n++) a.shape(`x=${n}`);
    await a.idle();
    // Retry the overflow as the App does on the completion notification; it was not cached as a failure.
    a.begin();
    for (let n = 1; n <= 70; n++) a.shape(`x=${n}`);
    await a.idle();
    expect(a.shape("x=70")?.get(0)?.lines).toEqual(["x = 70"]);
    command = missing;
    a.begin();
    expect(a.shape("x=0")).toBeUndefined();
    await a.idle();
    expect(a.shape("x=0")).toBeUndefined();
    command = null;
    a.begin();
    expect(a.shape("x=0")).toBeUndefined();
    command = blocked;
    await rm(started);
    a.begin();
    a.shape("x=9");
    await until(async () => Number(await readFile(started, "utf8").catch(() => "")) > 1);
    const disposedPid = Number(await readFile(started, "utf8"));
    const before = first;
    a.dispose();
    await a.idle();
    expect(() => process.kill(disposedPid, 0)).toThrow();
    expect(first).toBe(before);
    expect(a.shape("x=10")).toBeUndefined();
  } finally {
    a.dispose();
    b.dispose();
    await Promise.all([a.idle(), b.idle()]);
    await rm(directory, { recursive: true, force: true });
  }
});

test.skipIf(!ruff)(
  "cache eviction cannot renumber or replace an existing step's chosen projection on append",
  async () => {
    const formatter = new Formatter(
      () => {},
      () => ruff,
    );
    const prompt = act("prompt1", "prompt", "chain1", "operator", 1);
    const first = act("rung1", "rung", "chain1", "prompt1", 2, 3);
    const words = new Map([[first.id, longWord]]);
    const steps = [first];
    try {
      formatter.shape(longWord, first, first.began);
      await formatter.idle();
      const before = threadProgram(
        [prompt],
        makers([prompt, first]),
        () => longWord,
        new Set(),
        label,
        undefined,
        undefined,
        (step) => formatter.shape(words.get(step.id) ?? "", step, step.began),
      );
      expect(before.length).toBeGreaterThan(1);
      // More distinct completed words than the whole-word cache can retain.
      for (let batch = 0; batch < 5; batch++) {
        for (let n = 0; n < 55; n++) {
          const index = batch * 55 + n + 2;
          const step = act(`rung${index}`, "rung", "chain1", "prompt1", index * 2, index * 2 + 1);
          const word = `value_${index}= ${index}`;
          steps.push(step);
          words.set(step.id, word);
          formatter.shape(word, step, step.began);
        }
        await formatter.idle();
        formatter.begin();
      }
      const after = threadProgram(
        [prompt],
        // Session.refresh replaces row objects; stable step IDs must keep the chosen layout.
        makers([prompt, ...steps.map((step) => ({ ...step }))]),
        (step) => words.get(step.id) ?? "",
        new Set(),
        label,
        undefined,
        undefined,
        (step) => formatter.shape(words.get(step.id) ?? "", step, step.began),
      );
      expect(after.slice(0, before.length)).toEqual(before);
      const code = after.filter((row) => row.kind === "code");
      expect(code.at(-1)?.display).toBe(code.length);
      expect(code[before.length]?.display).toBe(before.length + 1);
      // Editing the original invalidates only that step's layout.
      expect(formatter.shape("edited = (", first, first.began)).toBeUndefined();
      await formatter.idle();
      expect(formatter.shape("edited = (", first, first.began)).toBeUndefined();
    } finally {
      formatter.dispose();
    }
  },
);

test.skipIf(!ruff)(
  "complex valid rewrites keep a stable raw fallback instead of unbounded layout metadata",
  async () => {
    let changes = 0;
    const formatter = new Formatter(
      () => changes++,
      () => ruff,
    );
    const owner = { id: "rung-complex" };
    // Ruff accepts this, but its hundreds of semicolon splits exceed the bounded display edit distance.
    const source = Array.from({ length: 180 }, (_, at) => `v${at}=${at}`).join("; ");
    try {
      formatter.shape(source, owner);
      await formatter.idle();
      expect(changes).toBe(1);
      expect(formatter.shape(source, { ...owner })).toBeUndefined();
      await formatter.idle();
      expect(changes).toBe(1);
      const prompt = act("prompt1", "prompt", "chain1", "operator", 1);
      const rung = act(owner.id, "rung", "chain1", prompt.id, 2, 3);
      expect(
        threadProgram(
          [prompt],
          makers([prompt, rung]),
          () => source,
          new Set(),
          label,
          undefined,
          undefined,
          (step) => formatter.shape(source, step),
        )[0],
      ).toMatchObject({
        kind: "code",
        text: source,
        number: 1,
        display: 1,
      });
    } finally {
      formatter.dispose();
    }
  },
);

test("a full cross-card queue admits the newest completed word before older pending history", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-ruff-priority-"));
  const gate = join(directory, "gate");
  const started = join(directory, "started");
  const log = join(directory, "calls");
  const command = await formatterExecutable(directory, "formatter", { gate, started, log });
  const formatter = new Formatter(
    () => {},
    () => command,
  );
  try {
    formatter.shape("x=0", { id: "first-card" }, 0);
    await until(async () => Number(await readFile(started, "utf8").catch(() => "")) > 1);
    for (let n = 1; n <= 64; n++) formatter.shape(`x=${n}`, { id: `old-${n}` }, n);
    formatter.shape("x=999", { id: "new-card" }, 999);
    await writeFile(gate, "go");
    await formatter.idle();
    expect(formatter.shape("x=999", { id: "new-card" })?.get(0)?.lines).toEqual(["x = 999"]);
    const calls = await readFile(log, "utf8");
    expect(calls.indexOf("x=999")).toBeLessThan(calls.indexOf("x=64"));
    expect(calls).not.toMatch(/x=1\n/);
  } finally {
    formatter.dispose();
    await formatter.idle();
    await rm(directory, { recursive: true, force: true });
  }
});

test("a completion callback can queue the next word in a microtask without orphaning it", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-ruff-handoff-"));
  const command = await formatterExecutable(directory, "formatter");
  let changes = 0;
  let queued = false;
  const formatter = new Formatter(
    () => {
      changes++;
      if (changes === 1)
        queueMicrotask(() => {
          queued = true;
          formatter.shape("x=2");
        });
    },
    () => command,
  );
  try {
    formatter.shape("x=1");
    await formatter.idle();
    expect(queued).toBe(true);
    expect(formatter.shape("x=1")?.get(0)?.lines).toEqual(["x = 1"]);
    expect(formatter.shape("x=2")?.get(0)?.lines).toEqual(["x = 2"]);
    expect(changes).toBe(2);
  } finally {
    formatter.dispose();
    await formatter.idle();
    await rm(directory, { recursive: true, force: true });
  }
});

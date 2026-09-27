import { expect, test } from "bun:test";
import { mkdir, mkdtemp, readFile, rm, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { actorParts, boot, type Ear, type Fact, inspectRecord, Session, type Turn } from "../src/index.ts";
import { alive, printPid, remove } from "./processes.ts";
import { until } from "./until.ts";

// A session with no model puts every prompt to the operator, so a test that asks a model names one of the catalog of
// the crate, and its `answer` replaces the request, so no model is asked.
const modeled = { model: "claude-cli:sonnet" };
/** A turn of a model whose word is the one given. */
const said = (word: string): Turn => ["assistant", word, null, null];
/** What a command has printed so far, as the act table holds it. */
const printed = (session: Session, id: string) =>
  (session.activity.acts.get(id)?.value as { stdout?: { content: string } } | undefined)?.stdout?.content ??
  "";
/** The kinds of the facts a record holds, in order. */
const kinds = async (record: string) =>
  (await readFile(record, "utf8"))
    .split("\n")
    .filter(Boolean)
    .map((line) => JSON.parse(line)[0][0] as string);
/** One turn of the loop of the host, so that every fact said before it has reached the session. */
const tick = () => new Promise((resolve) => setTimeout(resolve, 0));

test("record inspection derives pending work without taking its lock, writing files, or starting commands", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-inspect-"));
  const record = join(cwd, "session.jsonl");
  const session = new Session({ cwd, record });
  try {
    const engine = session.open();
    const on = engine.root;
    await engine.rung({ word: 'write(Text("value.txt", "once"))', on });
    const waiting = engine.wait({ seconds: 60, on }).id;
    const prompt = engine.prompt("str", { message: "Question", to: "operator", on }).id;
    const before = await readFile(record, "utf8");
    const metadata = await readFile(`${record}.session.json`, "utf8");
    // The lock file holds no text, and Windows refuses a read of a file that another handle has locked, so the test
    // reads what the system keeps of it.
    const locked = async () => {
      const { size, mtimeMs } = await stat(`${record}.lock`);
      return { size, mtimeMs };
    };
    const lock = await locked();
    // The session holds the lease while the inspection runs, so an inspection that took it would be refused.
    const first = await inspectRecord(record);
    expect(first.pending.map(([id]) => id)).toEqual([waiting, prompt]);
    expect(await readFile(record, "utf8")).toBe(before);
    expect(await readFile(`${record}.session.json`, "utf8")).toBe(metadata);
    expect(await locked()).toEqual(lock);
    expect(await readFile(join(cwd, "value.txt"), "utf8")).toBe("once");
    engine.cancel(waiting);
    engine.close("answered", { id: prompt });
    expect((await inspectRecord(record)).pending).toEqual([]);
  } finally {
    await session.dispose();
    await rm(cwd, { recursive: true, force: true });
  }
});

test("the act table holds an act paused while the last pause or wake that covers puts over it is a pause", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-paused-"));
  const session = new Session({ cwd });
  try {
    const engine = session.open();
    const root = engine.root;
    const two = engine.chain({ label: "two" }).id;
    const here = engine.wait({ seconds: 60, on: root }).id;
    const there = engine.wait({ seconds: 60, on: two }).id;
    const paused = () => [root, two, here, there].filter((id) => session.isPaused(id));
    engine.pause(two);
    expect(paused()).toEqual([two, there]);
    const later = engine.wait({ seconds: 60, on: two }).id;
    expect(session.isPaused(later)).toBe(true);
    engine.pause(here);
    expect(paused()).toEqual([two, here, there]);
    engine.wake(two);
    expect(paused()).toEqual([here]);
    expect(session.isPaused(later)).toBe(false);
    engine.pause(root);
    engine.wake(here);
    expect(paused()).toEqual([root]);
    expect(session.isPaused(engine.wait({ seconds: 60, on: root }).id)).toBe(true);
    expect(session.isPaused(engine.wait({ seconds: 60, on: two }).id)).toBe(false);
    // An act of a kind the file does not make is a row of the table, which a pause holds as it holds any other.
    await engine.rung({
      word: 'def takes(id):\n  yield "started", id\n  while True:\n    yield\nnote = act("note", "", takes)',
      on: two,
    });
    const note = String(engine.inspect("note", two).value);
    expect(session.activity.acts.get(note)?.kind).toBe("note");
    engine.pause(two);
    expect(session.isPaused(note)).toBe(true);
  } finally {
    await session.dispose();
    await rm(cwd, { recursive: true, force: true });
  }
});

test("an actor names its model and its effort after the last slash that follows the name of a model", () => {
  expect(actorParts("claude-cli:org/plain")).toEqual({ model: "claude-cli:org/plain", effort: "off" });
  expect(actorParts("claude-cli:org/plain/high")).toEqual({ model: "claude-cli:org/plain", effort: "high" });
  expect(actorParts("claude-cli:org/high", ["claude-cli:org/high"])).toEqual({
    model: "claude-cli:org/high",
    effort: "off",
  });
});

test("a session offers the models its host names, and keeps the model and the effort chosen last as a preference", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-roster-"));
  const record = join(cwd, "session.jsonl");
  const alone = new Session({ cwd, roster: [] });
  try {
    expect(alone.actor).toBe("operator");
    const engine = alone.open();
    const [roster, , actor] = engine.standing() as [[string][], string, string];
    expect([roster.map(([name]) => name), actor, alone.actor]).toEqual([
      ["operator"],
      "operator",
      "operator",
    ]);
  } finally {
    await alone.dispose();
  }
  const first = new Session({ cwd, record, model: "opus", effort: "high", roster: ["claude-cli:sonnet"] });
  try {
    const engine = first.open();
    const [roster] = engine.standing() as [[string, string[], number][]];
    expect(roster.map(([name, efforts]) => [name, efforts])).toEqual([
      ["claude-cli:opus", ["low", "medium", "high", "xhigh", "max"]],
      ["claude-cli:sonnet", ["low", "medium", "high", "xhigh", "max"]],
      ["operator", []],
    ]);
    expect(first.actor).toBe("claude-cli:opus/high");
  } finally {
    await first.dispose();
  }
  // The record holds the standing of the life, so the companion keeps no roster, and the actor as its parts.
  expect(JSON.parse(await readFile(`${record}.session.json`, "utf8")).options).toEqual({
    cwd,
    model: "claude-cli:opus",
    effort: "high",
  });
  const second = new Session({ record, roster: ["claude-cli:fable"], effort: "minimal" });
  try {
    const engine = second.open();
    const [roster] = engine.standing() as [[string][]];
    expect(roster.map(([name]) => name)).toEqual(["claude-cli:opus", "claude-cli:fable", "operator"]);
    expect(second.actor).toBe("claude-cli:opus/low");
  } finally {
    await second.dispose();
  }
  expect((await inspectRecord(record)).pending).toEqual([]);
  expect(() => new Session({ cwd, roster: ["nothing"] }).open()).toThrow("No model is nothing.");
  await rm(cwd, { recursive: true, force: true });
});

test("what a model writes streams into the session under its rung until its reply is done", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-streams-"));
  const record = join(cwd, "life.jsonl");
  let release: () => void = () => {};
  const held = new Promise<void>((resolve) => {
    release = resolve;
  });
  const seen: unknown[] = [];
  const session = new Session({
    cwd,
    record,
    ...modeled,
    answer: async (request, write) => {
      seen.push(request);
      write({ thinking: "counting" });
      write({ text: "close(" });
      write({ text: "3)" });
      await held;
      return ["assistant", "close(3)", [10, 2, 0, 0, 0], null];
    },
  });
  try {
    const engine = session.open();
    const prompt = engine.prompt("int", { message: "count", on: engine.root });
    // The rungs of the two official extensions come first.
    await until(session, () => session.streams.get("rung3")?.text === "close(3)");
    expect(session.streams.get("rung3")).toEqual({
      chain: engine.root,
      text: "close(3)",
      thinking: "counting",
    });
    expect(seen).toEqual([
      {
        actor: "claude-cli:sonnet/high",
        chain: engine.root,
        messages: [expect.objectContaining({ role: "user" })],
        settings: { effort: "high", session: expect.stringMatching(/\/chain1$/) },
      },
    ]);
    release();
    expect(await prompt).toBe(3);
    expect(session.streams.size).toBe(0);
    expect(session.activity.cost).toBe(0);
  } finally {
    await session.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("unfinished model work and waits reopen pending until the host resumes them", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-resume-"));
  const record = join(cwd, "life.jsonl");
  let calls = 0;
  const first = new Session({ cwd, record, ...modeled, answer: () => new Promise(() => {}) });
  const engine = first.open();
  const prompt = engine.prompt("str", { message: "pending", on: engine.root }).id;
  const wait = engine.wait({ seconds: 0.1, on: engine.root }).id;
  await first.dispose();
  const second = new Session({
    record,
    answer: async () => {
      calls++;
      return ["assistant", 'close("resumed")', null, null];
    },
  });
  const resumed = second.open();
  try {
    // The engine starts no pending work until a wake that this life says, so no model is asked however long it
    // stands.
    await tick();
    expect(calls).toBe(0);
    expect(second.pending.has(prompt)).toBe(true);
    expect(resumed.outcome(prompt).done).toBe(false);
    await second.resume();
    expect(await resumed.result<string>(prompt)).toBe("resumed");
    expect(await resumed.result(wait)).toBeNull();
    expect(calls).toBe(1);
  } finally {
    await second.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("a function of the host that throws answers nothing, and two in a row pause the chain with the reason", async () => {
  let calls = 0;
  const broken = boot({
    ...modeled,
    answer: async () => {
      calls++;
      throw new Error("unavailable");
    },
  });
  try {
    const prompt = broken.engine.prompt("str", { message: "try", on: broken.engine.root });
    await until(broken, () => broken.facts.some((fact) => fact[0] === "pause"));
    expect(calls).toBe(2);
    expect(broken.engine.outcome(prompt.id).done).toBe(false);
    expect(broken.facts.filter((fact) => fact[0] === "pause")).toHaveLength(1);
    const reasons = [...broken.activity.acts.values()]
      .filter((act) => act.kind === "rung")
      .map((act) => act.run?.reason);
    expect(reasons).toContain("Refused: claude-cli:sonnet/high answered nothing: ProviderError: unavailable");
  } finally {
    await broken.dispose();
  }
});

test("file changes append once and reopen in pages without growing the session metadata", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-changes-test-"));
  const record = join(cwd, "life.jsonl");
  const first = new Session({ cwd, record });
  const engine = first.open();
  const metadata = await readFile(`${record}.session.json`, "utf8");
  for (let number = 0; number < 25; number++)
    engine.write({ path: "file", content: String(number) }, { on: engine.root });
  expect(await readFile(`${record}.session.json`, "utf8")).toBe(metadata);
  await first.dispose();
  const second = new Session({ record });
  try {
    second.open();
    expect(second.changes.length).toBe(25);
    expect(second.changes.read(20, 20).map((change) => [change.before, change.after])).toEqual([
      ["19", "20"],
      ["20", "21"],
      ["21", "22"],
      ["22", "23"],
      ["23", "24"],
    ]);
  } finally {
    await second.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("a whole number from the operator answers a float prompt as a float, typed or from its callback", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-float-"));
  const typed = boot({ cwd });
  const called = boot({ cwd, operator: async () => 2 });
  try {
    const asked = (session: typeof typed) =>
      session.engine.prompt("float", { message: "A number?", to: "operator", on: session.engine.root });
    const one = asked(typed);
    await until(typed, () => typed.console.prompts.has(one.id));
    typed.console.answer(one.id, "1.0");
    const two = asked(called);
    expect([await one, await two]).toEqual([1, 2]);
    for (const [{ engine }, question, shown] of [
      [typed, one, "1.0"],
      [called, two, "2.0"],
    ] as const) {
      await engine.rung({ word: `x = peek(${JSON.stringify(question.id)})`, on: engine.root });
      expect(engine.inspect("x").representation).toBe(shown);
    }
  } finally {
    await typed.dispose();
    await called.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("an operator question survives a resume and validates its answer", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-operator-resume-"));
  const record = join(cwd, "life.jsonl");
  const first = new Session({ cwd, record });
  const opened = first.open();
  const question = opened.prompt("bool", { message: "Continue?", to: "operator", on: opened.root }).id;
  await until(first, () => first.console.prompts.has(question));
  await first.dispose();
  const second = new Session({ record });
  try {
    const engine = second.open();
    expect(second.console.prompts.size).toBe(0);
    await second.resume();
    await until(second, () => second.console.prompts.has(question));
    expect(second.console.prompts.get(question)?.message).toBe("Continue?");
    expect(() => second.console.answer(question, "maybe")).toThrow("neither yes nor no");
    second.console.answer(question, "yes");
    expect(await engine.result<boolean>(question)).toBe(true);
  } finally {
    await second.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("resume wakes no work that a pause of the operator holds, so that pause stands", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-own-pause-"));
  const record = join(cwd, "life.jsonl");
  const first = new Session({ cwd, record, ...modeled, answer: () => new Promise(() => {}) });
  const engine = first.open();
  engine.pause(engine.root);
  engine.prompt("str", { message: "later", on: engine.root });
  await first.dispose();
  const second = new Session({ record, answer: async () => said('close("x")') });
  try {
    const again = second.open();
    expect(second.pending.size).toBe(0);
    await second.resume();
    expect(second.isPaused(again.root)).toBe(true);
    expect(await kinds(record)).not.toContain("wake");
  } finally {
    await second.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("pending work that a cancel ends is pending no more", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-drained-"));
  const record = join(cwd, "life.jsonl");
  const first = new Session({ cwd, record });
  const opened = first.open();
  const prompt = opened.prompt("str", { message: "first", to: "operator", on: opened.root }).id;
  await first.dispose();
  const second = new Session({ record });
  try {
    const again = second.open();
    expect(second.pending.has(prompt)).toBe(true);
    again.cancel(prompt);
    await tick();
    expect(second.pending.size).toBe(0);
  } finally {
    await second.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("record inspection reports pending work when its replay starts or feeds a command", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-inspect-commands-"));
  const record = join(cwd, "life.jsonl");
  const first = new Session({ cwd, record });
  const engine = first.open();
  const on = engine.root;
  engine.rung({ word: 'await bash("exec sleep 30")', on });
  engine.rung({
    word: 'c = bash("head -c 6; exec sleep 30", fed=True)\nwrite(Text(c + "/stdin", "hello\\n"))\nawait c',
    on,
  });
  await until(first, () => printed(first, "bash2") === "hello\n");
  await first.dispose();
  try {
    expect((await inspectRecord(record)).pending.map(([id]) => id)).toEqual([
      "rung3",
      "bash1",
      "rung4",
      "bash2",
    ]);
  } finally {
    await remove(cwd);
  }
});

test("a close of the session whose save fails still ends its life and its commands", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-failed-save-"));
  const record = join(cwd, "life.jsonl");
  const session = new Session({ cwd, record });
  const engine = session.open();
  const command = engine.bash(`${printPid}; exec sleep 30`, { on: engine.root }).id;
  await until(session, () => printed(session, command) !== "");
  const pid = Number(printed(session, command));
  // A directory where the save writes its file makes the save fail on every system.
  await mkdir(`${record}.session.json.tmp`);
  try {
    await expect(session.dispose()).rejects.toThrow("life.jsonl.session.json.tmp");
    expect(engine.disposed).toBe(true);
    // A process the session ended ends a moment after, once the system hears of it.
    for (let tries = 0; alive(pid) && tries < 100; tries++) await Bun.sleep(20);
    expect(alive(pid)).toBe(false);
  } finally {
    await remove(cwd);
  }
});

test("a host that asks the life at each change hears no change of its own asking", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-own-query-"));
  const session = new Session({ cwd });
  const engine = session.open();
  try {
    await tick();
    let calls = 0;
    session.on("change", () => {
      calls++;
      engine.cwd({ on: engine.root });
    });
    engine.cwd({ on: engine.root });
    await tick();
    expect(calls).toBe(0);
    expect(session.facts.filter(([kind, id]) => kind === "done" && id.startsWith("cwd@"))).toEqual([]);
  } finally {
    await session.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("a host that reads the life at a change of a file leaves the write whole", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-write-change-"));
  const session = new Session({ cwd });
  const engine = session.open();
  try {
    await tick();
    session.on("change", () => engine.cwd({ on: engine.root }));
    const word = 't = write(Text("note.txt", "hello\\n"))\nclose(t.content)';
    expect(await engine.rung({ word, on: engine.root })).toBe("hello\n");
  } finally {
    await session.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("an operator answer of a list keeps its numbers exact, and a map in it that holds the key is stays a map", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-list-answer-"));
  const session = new Session({ cwd });
  const engine = session.open();
  try {
    const on = engine.root;
    const question = engine.prompt("list", { message: "Numbers?", to: "operator", on });
    await until(session, () => session.console.prompts.has(question.id));
    expect(() => session.console.answer(question.id, "{}")).toThrow('"{}" is no list');
    session.console.answer(question.id, '[2.0, 9007199254740993, {"is": "str", "args": [1.0]}]');
    const kept = {
      is: "dict",
      args: [
        [
          ["is", "str"],
          ["args", [1]],
        ],
      ],
    };
    const big = { is: "int", args: ["9007199254740993"] };
    expect(await question).toEqual([2, big, kept]);
    await engine.rung({ word: `x = peek(${JSON.stringify(question.id)})`, on });
    expect(engine.inspect("x").representation).toBe("[2.0, 9007199254740993, {'is': 'str', 'args': [1.0]}]");
    expect(engine.inspect("x").value).toEqual([2, big, kept]);
  } finally {
    await session.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("a session with no model puts to the operator every prompt that names no actor, the acknowledgment among them", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-operator-alone-"));
  let acknowledged: (message: string) => void = () => {};
  const acknowledgment = new Promise<string>((resolve) => {
    acknowledged = resolve;
  });
  // The first example of the README, whose callback answers for the operator.
  const session = boot({
    cwd,
    record: join(cwd, "work.jsonl"),
    roster: [],
    operator: async ({ shape, message }) => {
      if (shape === "None") acknowledged(message);
      return shape === "str" ? `You asked: ${message}` : null;
    },
  });
  try {
    const { engine } = session;
    const on = engine.root;
    expect(await engine.prompt("str", { message: "What is this project?", on })).toBe(
      "You asked: What is this project?",
    );
    engine.rung({ word: 'b = bash("true")', on });
    expect(await acknowledgment).toBe("bash1 done");
  } finally {
    await session.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("an ear of the host that comes before the files takes a read in their place", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-first-ear-"));
  function* mine(): Ear {
    for (;;) {
      const fact = (yield null) as Fact | undefined;
      if (fact?.[0] === "read" && String(fact[4]).startsWith("mine://"))
        yield ["done", fact[1], { is: "Text", path: fact[4], content: "mine\n" }];
    }
  }
  const session = boot({ cwd, ears: [["mine", mine()]] });
  try {
    const { engine } = session;
    expect(engine.read("mine://a", { on: engine.root })).toEqual({
      is: "Text",
      path: "mine://a",
      content: "mine\n",
    } as never);
  } finally {
    await session.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("a session opens a record in a directory that does not stand yet, and keeps its changes beside it", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-deep-"));
  const record = join(cwd, "a", "b", "life.jsonl");
  const session = boot({ cwd, record });
  try {
    const { engine } = session;
    engine.write({ path: "note.txt", content: "one\n" }, { on: engine.root });
    await until(session, () => session.changes.length === 1);
    expect(session.changes.read()).toEqual([{ path: join(cwd, "note.txt"), before: "", after: "one\n" }]);
    expect((await stat(`${record}.changes.jsonl`)).isFile()).toBe(true);
  } finally {
    await session.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("a session enables the extensions of the configs, and a later session on its record runs them whatever it says", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-extensions-"));
  const record = join(cwd, "life.jsonl");
  const config = join(cwd, "config");
  await writeFile(join(cwd, "CLAUDE.md"), "Use two spaces.\n");
  /** The names of the extensions that the life of a session runs, which the root says it enabled. */
  const enabled = (session: Session) =>
    (session.engine?.transcript({ on: "chain1" }) ?? [])
      .filter(([kind]) => kind === "enable")
      .map((fact) => fact[3]);
  const first = new Session({ cwd, record, config });
  try {
    first.open();
    expect(enabled(first)).toEqual(["memory", "skills"]);
    const told = first.engine?.turns({ on: "chain1" }).at(-1)?.[1] ?? "";
    expect(told).toContain(
      `#memory ${join(cwd, "CLAUDE.md")}\n# ${join(cwd, "CLAUDE.md")}, 0 known\n# 1 Use two spaces.`,
    );
  } finally {
    await first.dispose();
  }
  const later = new Session({ cwd, record, config, extensions: false });
  const fresh = new Session({ cwd, config, extensions: false });
  try {
    later.open();
    fresh.open();
    expect([enabled(later), enabled(fresh)]).toEqual([["memory", "skills"], []]);
  } finally {
    await later.dispose();
    await fresh.dispose();
    await rm(cwd, { recursive: true, force: true });
  }
});

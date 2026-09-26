import { afterAll, expect, test } from "bun:test";
import { mkdir, mkdtemp, readFile, rm, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createModels, getSupportedThinkingLevels } from "@earendil-works/pi-ai";
import {
  actorParts,
  boot,
  type Ear,
  type Fact,
  inspectRecord,
  isQuestion,
  Session,
  type Turn,
} from "../src/index.ts";
import { claudeProvider, cliModel } from "../src/providers/claude.ts";
import { executable } from "./executable.ts";
import { alive, printPid, remove } from "./processes.ts";
import { until } from "./until.ts";

// A session with no model puts every prompt to the operator, so a test that asks a model names one, and its `answer`
// replaces the request, so no CLI runs. A later session that is given the models takes the saved model as its own.
const offer = claudeProvider();
const offered = createModels();
offered.setProvider(offer.provider);
const modeled = { models: offered, model: "claude-cli:sonnet" };
afterAll(() => offer.dispose());
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

test("the standing takes each model's efforts from its pi-ai metadata", async () => {
  expect(getSupportedThinkingLevels(cliModel("sonnet"))).toEqual(["low", "medium", "high", "xhigh", "max"]);
  expect(actorParts("claude-cli:org/plain")).toEqual({ model: "claude-cli:org/plain", effort: "off" });
  expect(actorParts("claude-cli:org/plain/high")).toEqual({ model: "claude-cli:org/plain", effort: "high" });
  expect(actorParts("claude-cli:org/high", ["claude-cli:org/high"])).toEqual({
    model: "claude-cli:org/high",
    effort: "off",
  });
  const cli = claudeProvider();
  const models = createModels();
  models.setProvider({
    ...cli.provider,
    getModels: () => [
      { ...cliModel("org/plain"), reasoning: false },
      {
        ...cliModel("focused"),
        thinkingLevelMap: {
          off: null,
          minimal: null,
          low: "low",
          medium: null,
          high: "high",
          xhigh: null,
          max: null,
        },
      },
    ],
  });
  const session = new Session({ models, model: "claude-cli:org/plain", roster: ["claude-cli:focused"] });
  try {
    const engine = session.open();
    const [roster] = engine.standing() as [[string, string[], number][], string, string];
    expect(roster.map(([name, efforts]) => [name, efforts])).toEqual([
      ["claude-cli:org/plain", ["off"]],
      ["claude-cli:focused", ["low", "high"]],
      ["operator", []],
    ]);
    expect(session.provider.effort).toBe("off");
    expect(session.provider.route("claude-cli:org/plain/off").id).toBe("org/plain");
  } finally {
    await session.dispose();
    cli.dispose();
  }
});

test("a session offers the models its host names, and keeps the model and the effort chosen last as a preference", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-roster-"));
  const record = join(cwd, "session.jsonl");
  const cli = claudeProvider();
  const models = createModels();
  models.setProvider(cli.provider);
  const alone = new Session({ cwd });
  try {
    expect(alone.provider.roster).toEqual([]);
    const engine = alone.open();
    const [, , actor] = engine.standing() as [unknown, string, string];
    expect(actor).toBe("operator");
    expect(alone.provider.actor).toBe("operator");
  } finally {
    await alone.dispose();
  }
  const first = new Session({
    cwd,
    record,
    models,
    model: "claude-cli:opus",
    effort: "high",
    roster: ["claude-cli:sonnet"],
  });
  try {
    first.open();
  } finally {
    await first.dispose();
  }
  // The record holds the standing of the life, so the companion keeps no roster and no actor.
  expect(JSON.parse(await readFile(`${record}.session.json`, "utf8")).options).toEqual({
    cwd,
    model: "claude-cli:opus",
    effort: "high",
  });
  const second = new Session({ record, models, roster: ["claude-cli:fable"] });
  try {
    expect(second.provider.roster).toEqual(["claude-cli:opus", "claude-cli:fable"]);
    expect(second.provider.actor).toBe("claude-cli:opus/high");
    expect(second.provider.route("haiku").id).toBe("haiku");
    expect(() => second.provider.route("nothing")).toThrow("Name one as provider:model");
  } finally {
    await second.dispose();
    cli.dispose();
  }
  // A preference the session cannot route falls away, so an inspection needs no model of the record.
  expect((await inspectRecord(record)).pending).toEqual([]);
  const third = new Session({ record });
  try {
    expect(third.provider.model).toBeUndefined();
    expect(third.provider.roster).toEqual([]);
    expect(third.provider.actor).toBe("operator");
    expect(() => new Session({ cwd, roster: ["claude-cli:opus"] })).toThrow("No model claude-cli:opus");
    // An answer replaces the request of a model, and a session with no model has none to replace.
    expect(() => new Session({ cwd, answer: async () => said("close(1)") })).toThrow("offers no model");
  } finally {
    await third.dispose();
    await rm(cwd, { recursive: true, force: true });
  }
});

test("a fenced reply is no python: the gate refuses it and the prompt asks again", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-fence-"));
  const cli = claudeProvider({ bin: executable(join(import.meta.dir, "fake-claude.ts"), cwd, "claude") });
  const models = createModels();
  models.setProvider(cli.provider);
  const session = new Session({ cwd, models, roster: ["claude-cli:sonnet"] });
  try {
    const engine = session.open();
    const on = engine.root;
    expect(await engine.prompt("str", { message: "FENCE", on })).toBe("reply 2");
    const transcript = engine
      .turns({ on })
      .map(([, python]) => python)
      .join("\n");
    expect(transcript).toContain("```python");
    expect(transcript).toMatch(/^#rung\d+ refused$/m);
  } finally {
    await session.dispose();
    cli.dispose();
    await remove(cwd);
  }
}, 30000);

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
    models: offered,
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

test("one failed reply is asked again, while two failures in a row pause with a reason", async () => {
  let calls = 0;
  const session = boot({
    ...modeled,
    answer: async () => {
      if (++calls === 1) throw new Error("temporary outage");
      return ["assistant", 'close("recovered")', null, null];
    },
  });
  try {
    expect(await session.engine.prompt("str", { message: "try", on: session.engine.root })).toBe("recovered");
    expect(session.facts.some((fact) => fact[0] === "pause")).toBe(false);
    expect(calls).toBe(2);
  } finally {
    await session.dispose();
  }
  calls = 0;
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
    expect(reasons).toContain(`Refused: ${modeled.model}/low answered nothing: Error: unavailable`);
  } finally {
    await broken.dispose();
  }
});

test("the provider hands the model the python of a user turn as the engine wrote it, and chain accepts its parent", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-python-turn-"));
  const log = join(cwd, "cli.jsonl");
  process.env.FURB_FAKE_LOG = log;
  const cli = claudeProvider({ bin: executable(join(import.meta.dir, "fake-claude.ts"), cwd, "claude") });
  const models = createModels();
  models.setProvider(cli.provider);
  const session = boot({ cwd, models, model: "claude-cli:sonnet" });
  try {
    const { engine } = session;
    const on = engine.root;
    const child = engine.chain({ label: "child", on });
    expect(engine.get(child.id)?.[3]).toBe(on);
    expect(await engine.prompt("str", { message: 'say "hi" <b> && \\n', on })).toBe("reply 1");
    const [turn] = engine.turns({ on });
    expect(turn?.[1]).toContain('#prompt1 say "hi" <b> && \\n');
    const requests = (await readFile(log, "utf8"))
      .split("\n")
      .filter(Boolean)
      .map((line) => JSON.parse(line))
      .filter((line) => line.type === "user");
    expect(requests.map((line) => line.message.content)).toEqual([[{ type: "text", text: turn?.[1] }]]);
  } finally {
    await session.dispose();
    cli.dispose();
    delete process.env.FURB_FAKE_LOG;
    await remove(cwd);
  }
}, 30000);

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
    expect(() => second.console.answer(question, "maybe")).toThrow("yes or no");
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
  const second = new Session({ record, models: offered, answer: async () => said('close("x")') });
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
      "rung1",
      "bash1",
      "rung2",
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
    expect(() => session.console.answer(question.id, "[9007199254740993]")).toThrow("safe integer");
    session.console.answer(question.id, '[2.0, 3, {"is": "str", "args": [1.0]}]');
    const kept = {
      is: "dict",
      args: [
        [
          ["is", "str"],
          ["args", [1]],
        ],
      ],
    };
    expect(await question).toEqual([2, 3, kept]);
    await engine.rung({ word: `x = peek(${JSON.stringify(question.id)})`, on });
    expect(engine.inspect("x").representation).toBe("[2.0, 3, {'is': 'str', 'args': [1.0]}]");
    expect(engine.inspect("x").value).toEqual([2, 3, kept]);
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

test("a reply that a cancel ends asks its model for nothing more, and comes to a CancelledError", async () => {
  const stopped: string[] = [];
  const session = boot({
    ...modeled,
    answer: (_actor, _chain, _turns, signal) =>
      new Promise<Turn>((_, reject) =>
        signal.addEventListener(
          "abort",
          () => {
            stopped.push("aborted");
            reject(new Error("aborted"));
          },
          { once: true },
        ),
      ),
  });
  try {
    const { engine } = session;
    const act = engine.prompt("int", { message: "count", on: engine.root });
    await until(session, () => session.provider.streams.size > 0);
    engine.cancel(String(act));
    expect(stopped).toEqual(["aborted"]);
    const dones = engine
      .transcript({ on: engine.root })
      .filter(([kind, id]) => kind === "done" && isQuestion("reply", id));
    expect(dones.map((fact) => (fact[3] as { is?: string }).is)).toEqual(["CancelledError"]);
  } finally {
    await session.dispose();
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

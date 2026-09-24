import { afterAll, expect, test } from "bun:test";
import { mkdir, mkdtemp, readFile, rm, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createModels, getSupportedThinkingLevels } from "@earendil-works/pi-ai";
import {
  actorParts,
  boot,
  builtinExtensions,
  decodeRecord,
  type Ear,
  Ears,
  type Fact,
  inspectRecord,
  type Turn,
  unwrapped,
  World,
} from "../src/index.ts";
import { claudeProvider, cliModel } from "../src/providers/claude.ts";
import { RecordFile } from "../src/record.ts";
import { executable } from "./executable.ts";
import { alive, printPid, remove } from "./processes.ts";
import { until } from "./until.ts";
import { bash, exited, read, verb, write } from "./verbs.ts";

// A World with no model puts every prompt to the operator, so a test that asks a model names one, and its `answer`
// replaces the request, so no CLI runs. A later World that is given the models takes the saved model as its own.
const offer = claudeProvider();
const offered = createModels();
offered.setProvider(offer.provider);
const modeled = { models: offered, model: "claude-cli:sonnet" };
afterAll(() => offer.dispose());
/** A turn of a model whose word is the one given. */
const said = (word: string): Turn => ["assistant", word, null, null];

test("record inspection derives pending work without taking its lock, writing files, or starting commands", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-inspect-"));
  const record = join(cwd, "session.jsonl");
  const world = new World({ cwd, record });
  try {
    const life = world.open();
    await life.rung('write(Text("value.txt", "once"))');
    const waiting = life.wait(60).id;
    const prompt = life.prompt("str", "Question", { to: "operator" }).id;
    const before = await readFile(record, "utf8");
    const metadata = await readFile(`${record}.world.json`, "utf8");
    // The lock file holds no text, and Windows refuses a read of a file that another handle has locked, so the test
    // reads what the system keeps of it.
    const locked = async () => {
      const { size, mtimeMs } = await stat(`${record}.lock`);
      return { size, mtimeMs };
    };
    const lock = await locked();
    // The World holds the lease while the inspection runs, so an inspection that took it would be refused.
    const first = await inspectRecord(record);
    expect(first.pending.map(([id]) => id)).toEqual([waiting, prompt]);
    expect(await readFile(record, "utf8")).toBe(before);
    expect(await readFile(`${record}.world.json`, "utf8")).toBe(metadata);
    expect(await locked()).toEqual(lock);
    expect(await readFile(join(cwd, "value.txt"), "utf8")).toBe("once");
    life.cancel(waiting);
    life.close("answered", prompt);
    expect((await inspectRecord(record)).pending).toEqual([]);
  } finally {
    await world.dispose();
    await rm(cwd, { recursive: true, force: true });
  }
});

test("the act table holds an act paused while the last pause or wake that covers puts over it is a pause", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-paused-"));
  const world = new World({ cwd });
  try {
    const life = world.open();
    const two = life.chain("two").id;
    const here = life.wait(60).id;
    const there = life.wait(60, two).id;
    const paused = () => [life.root, two, here, there].filter((id) => world.isPaused(id));
    life.pause(two);
    expect(paused()).toEqual([two, there]);
    const later = life.wait(60, two).id;
    expect(world.isPaused(later)).toBe(true);
    life.pause(here);
    expect(paused()).toEqual([two, here, there]);
    life.wake(two);
    expect(paused()).toEqual([here]);
    expect(world.isPaused(later)).toBe(false);
    life.pause(life.root);
    life.wake(here);
    expect(paused()).toEqual([life.root]);
    expect(world.isPaused(life.wait(60).id)).toBe(true);
    expect(world.isPaused(life.wait(60, two).id)).toBe(false);
    // An act of a kind the file does not make derives the table again from every fact, with the same answers.
    const generation = world.activity.generation;
    await life.rung('note = act("note", "", idle)', { on: two });
    await until(world, () => world.activity.generation > generation);
    expect(paused()).toEqual([life.root]);
  } finally {
    await world.dispose();
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
  const world = new World({ models, model: "claude-cli:org/plain", roster: ["claude-cli:focused"] });
  try {
    const life = world.open();
    const [, [roster]] = life.call<[unknown, [[string, string[], number][], string, string]]>(
      "ask",
      ["stand", life.root],
      {},
    );
    expect(roster.map(([name, efforts]) => [name, efforts])).toEqual([
      ["claude-cli:org/plain", ["off"]],
      ["claude-cli:focused", ["low", "high"]],
      ["operator", []],
    ]);
    expect(world.effort).toBe("off");
    expect(world.route("claude-cli:org/plain/off").id).toBe("org/plain");
  } finally {
    await world.dispose();
    cli.dispose();
  }
});

test("a World offers the models its host names, and keeps the model and the effort chosen last as a preference", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-roster-"));
  const record = join(cwd, "session.jsonl");
  const cli = claudeProvider();
  const models = createModels();
  models.setProvider(cli.provider);
  const alone = new World({ cwd });
  try {
    expect(alone.roster).toEqual([]);
    const life = alone.open();
    const [, [, , actor]] = life.call<[unknown, [unknown, string, string]]>("ask", ["stand", life.root], {});
    expect(actor).toBe("operator");
    expect(alone.actor).toBe("operator");
  } finally {
    await alone.dispose();
  }
  const first = new World({
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
  expect(JSON.parse(await readFile(`${record}.world.json`, "utf8")).options).toEqual({
    cwd,
    model: "claude-cli:opus",
    effort: "high",
  });
  const second = new World({ record, models, roster: ["claude-cli:fable"] });
  try {
    expect(second.roster).toEqual(["claude-cli:opus", "claude-cli:fable"]);
    expect(second.actor).toBe("claude-cli:opus/high");
    expect(second.route("haiku").id).toBe("haiku");
    expect(() => second.route("nothing")).toThrow("Name one as provider:model");
  } finally {
    await second.dispose();
    cli.dispose();
  }
  // A preference the World cannot route falls away, so an inspection needs no model of the record.
  expect((await inspectRecord(record)).pending).toEqual([]);
  const third = new World({ record });
  try {
    expect(third.model).toBeUndefined();
    expect(third.roster).toEqual([]);
    expect(third.actor).toBe("operator");
    expect(() => new World({ cwd, roster: ["claude-cli:opus"] })).toThrow("No model claude-cli:opus");
    // An answer replaces the request of a model, and a World with no model has none to replace.
    expect(() => new World({ cwd, answer: async () => said("close(1)") })).toThrow("offers no model");
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
  const world = new World({ cwd, models, roster: ["claude-cli:sonnet"] });
  try {
    const life = world.open();
    expect(await life.prompt<string>("str", "FENCE")).toBe("reply 2");
    const transcript = life
      .turns()
      .map(([, python]) => python)
      .join("\n");
    expect(transcript).toContain("```python");
    expect(transcript).toMatch(/^#rung\d+ refused$/m);
  } finally {
    await world.dispose();
    cli.dispose();
    await remove(cwd);
  }
}, 30000);

test("the default World serves files and streams commands without any TUI", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-world-"));
  const session = await boot({ cwd });
  try {
    const { life, world } = session;
    if (!world) throw new Error("The session has no World.");
    expect(verb<string>(life, "cwd")).toBe(cwd);
    write(life, "hello.txt", "hello\n");
    expect(read(life, "hello.txt").content).toBe("hello\n");
    const command = bash(life, "printf 'snow: 雪\\n'; printf 'problem\\n' >&2", {
      show_err: life.held("modules", [life.root, "TAIL"], "at"),
    });
    const result = await exited(world, life, command);
    expect(result.code).toBe(0);
    expect(result.stdout.content).toBe("snow: 雪\n");
    expect(result.stderr.content).toBe("problem\n");
    const input = bash(life, "cat", { fed: true });
    write(life, `${input}/stdin`, "fed\n");
    write(life, `${input}/stdin`, "");
    expect((await exited(world, life, input)).stdout.content).toBe("fed\n");
  } finally {
    await session.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("unfinished model work and waits reopen pending until the host resumes them", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-resume-"));
  const record = join(cwd, "life.jsonl");
  let calls = 0;
  const first = new World({ cwd, record, ...modeled, answer: () => new Promise(() => {}) });
  const life = first.open();
  const prompt = life.prompt("str", "pending").id;
  const wait = life.wait(0.1).id;
  await first.dispose();
  const second = new World({
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

test("record ownership, a torn last line, and a damaged complete line are distinct", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-record-"));
  const path = join(cwd, "record.jsonl");
  try {
    await writeFile(path, '[["done","x","world",null]]\n["partial"');
    const record = new RecordFile(path);
    expect(record.entries).toHaveLength(1);
    expect(() => new RecordFile(path)).toThrow("Another process");
    record.keep([["done", "y", "world", 1]]);
    record.dispose();
    expect((await readFile(path, "utf8")).split("\n").filter(Boolean)).toHaveLength(2);
    await writeFile(path, "broken\n");
    expect(() => new RecordFile(path)).toThrow("Invalid record entry");
  } finally {
    await rm(cwd, { recursive: true });
  }
});

test("one failed model ask retries, while two consecutive failures pause with a reason", async () => {
  let calls = 0;
  const session = await boot({
    ...modeled,
    answer: async () => {
      if (++calls === 1) throw new Error("temporary outage");
      return ["assistant", 'close("recovered")', null, null];
    },
  });
  try {
    expect(await session.life.prompt<string>("str", "try")).toBe("recovered");
    expect(session.world?.facts.some((fact) => fact[0] === "pause")).toBe(false);
    expect(calls).toBe(2);
  } finally {
    await session.dispose();
  }
  calls = 0;
  const broken = await boot({
    ...modeled,
    answer: async () => {
      calls++;
      throw new Error("unavailable");
    },
  });
  try {
    const prompt = broken.life.prompt("str", "try");
    const world = broken.world;
    if (!world) throw new Error("The session has no World.");
    await until(world, () => world.facts.some((fact) => fact[0] === "pause"));
    expect(calls).toBe(2);
    expect(broken.life.outcome(prompt.id).done).toBe(false);
    expect(broken.world?.facts.filter((fact) => fact[0] === "pause")).toHaveLength(1);
    expect(
      broken.life
        .turns()
        .map(([, python]) => python)
        .join("\n"),
    ).toContain("answered nothing: Error: unavailable");
  } finally {
    await broken.dispose();
  }
});

test("reopening unfinished work does not add another pause to the record", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-pause-"));
  const record = join(cwd, "life.jsonl");
  try {
    const first = new World({ cwd, record });
    first.open().wait(60);
    await first.dispose();
    const original = await readFile(record, "utf8");
    for (let index = 0; index < 2; index++) {
      const later = new World({ record });
      later.open();
      await later.dispose();
    }
    expect(await readFile(record, "utf8")).toBe(original);
  } finally {
    await rm(cwd, { recursive: true });
  }
});

test("the record, not stale saved metadata, decides whether a reopened life has pending work", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-stale-pending-"));
  const record = join(cwd, "life.jsonl");
  const first = new World({
    cwd,
    record,
    ...modeled,
    answer: async () => ["assistant", 'close("done")', null, null],
  });
  const prompt = first.open().prompt<string>("str", "finish");
  await prompt;
  await first.dispose();
  const saved = JSON.parse(await readFile(`${record}.world.json`, "utf8"));
  saved.pending = [[prompt.id, "prompt"]];
  await writeFile(`${record}.world.json`, JSON.stringify(saved));
  let calls = 0;
  const second = new World({
    record,
    models: offered,
    answer: async () => {
      calls++;
      return ["assistant", 'close("new answer")', null, null];
    },
  });
  try {
    const life = second.open();
    expect(second.pending.size).toBe(0);
    expect(life.outcome(prompt.id).done).toBe(true);
    expect(await life.prompt<string>("str", "continue")).toBe("new answer");
    expect(calls).toBe(1);
  } finally {
    await second.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("the World hands the provider the python of a user turn as the engine wrote it, and chain accepts its parent", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-python-turn-"));
  const log = join(cwd, "cli.jsonl");
  process.env.FURB_FAKE_LOG = log;
  const cli = claudeProvider({ bin: executable(join(import.meta.dir, "fake-claude.ts"), cwd, "claude") });
  const models = createModels();
  models.setProvider(cli.provider);
  const session = await boot({ cwd, models, model: "claude-cli:sonnet" });
  try {
    const { life } = session;
    const child = life.chain("child", null, null, life.root);
    expect(life.get(child.id)[3]).toBe(life.root);
    expect(await life.prompt<string>("str", 'say "hi" <b> && \\n')).toBe("reply 1");
    const [turn] = life.turns();
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

test("input sent to a pending command before it starts reaches its process after resume", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-fed-"));
  const record = join(cwd, "life.jsonl");
  const first = new World({ cwd, record });
  const command = bash(first.open(), "cat", { fed: true });
  await first.dispose();
  const second = new World({ record });
  try {
    const life = second.open();
    expect(second.pending.has(command)).toBe(true);
    write(life, `${command}/stdin`, "before start\n");
    write(life, `${command}/stdin`, "");
    await second.resume();
    expect((await exited(second, life, command)).stdout.content).toBe("before start\n");
  } finally {
    await second.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("file changes append once and reopen in pages without growing the World metadata", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-changes-test-"));
  const record = join(cwd, "life.jsonl");
  const first = new World({ cwd, record });
  const life = first.open();
  const metadata = await readFile(`${record}.world.json`, "utf8");
  for (let number = 0; number < 25; number++) write(life, "file", String(number));
  expect(await readFile(`${record}.world.json`, "utf8")).toBe(metadata);
  await first.dispose();
  const second = new World({ record });
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

test("record numbers refuse precision loss and map keys keep their identity", () => {
  expect(decodeRecord('{"$serde_json::private::Number":"10"}')).toEqual({
    "$serde_json::private::Number": "10",
  });
  expect(() => decodeRecord("9007199254740993")).toThrow("safe integer");
  expect(() => decodeRecord("99999999999999999999999999999999")).toThrow("safe integer");
  expect(decodeRecord('"99999999999999999999999999999999"')).toBe("99999999999999999999999999999999");
});

test("integral floats keep their Python type in operator replies and records", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-float-"));
  const record = join(cwd, "life.jsonl");
  const first = new World({ cwd, record });
  const life = first.open();
  const question = life.prompt<number>("float", "A number", { to: "operator" });
  await until(first, () => first.prompts.has(question.id));
  first.answer(question.id, "1.0");
  expect(await question).toBe(1);
  const id = question.id;
  await first.dispose();
  expect(await readFile(record, "utf8")).toContain('"is":"float"');
  const second = new World({ record });
  try {
    const next = second.open();
    expect(await next.result<number>(id)).toBe(1);
    expect(second.pending.size).toBe(0);
  } finally {
    await second.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("host ears yield nested bus calls and host shows remain callable", () => {
  const ears = new Ears({
    world: (function* (): Ear {
      for (;;) {
        const fact = (yield null) as Fact;
        if (fact?.[0] === "stand") yield ["done", fact[1], [[["operator", [], 200000]], "/tmp", "operator"]];
        if (fact?.[0] === "read") {
          const cwd = yield { verb: "cwd", kwargs: { on: fact[3] } };
          yield ["done", fact[1], { path: `${cwd}/file`, content: "one\ntwo\n" }];
        }
      }
    })(),
  });
  const words = builtinExtensions().flatMap((one) => (one.word ? [one.word] : []));
  const life = ears.boot([], words);
  try {
    const show = ears.callable((lines: string[]) => lines.map((_, index) => index + 1));
    expect(unwrapped<{ path: string }>(verb(life, "read", ["file", show])).path).toBe("/tmp/file");
    expect(() => life.clock()).toThrow("no number");
  } finally {
    life.dispose();
  }
});

test("a command an earlier World started and did not end runs again once, at the resume", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-command-resume-"));
  const record = join(cwd, "life.jsonl");
  const first = new World({ cwd, record });
  const life = first.open();
  const command = bash(life, "printf x >> count; [ -e done ] || { printf ready; exec sleep 30; }");
  const output = () =>
    (first.activity.acts.get(command)?.value as { stdout?: { content: string } }).stdout?.content;
  await until(first, () => output() === "ready");
  await first.dispose();
  await writeFile(join(cwd, "done"), "");
  const second = new World({ record });
  try {
    const resumed = second.open();
    await tick();
    expect(second.pending.has(command)).toBe(true);
    expect(await readFile(join(cwd, "count"), "utf8")).toBe("x");
    await second.resume();
    // What the command told before the death of its process stands in its door.
    expect(await exited(second, resumed, command)).toMatchObject({
      code: 0,
      stdout: { content: "ready" },
    });
    expect(await readFile(join(cwd, "count"), "utf8")).toBe("xx");
    resumed.wake(resumed.root);
    await tick();
    expect(await readFile(join(cwd, "count"), "utf8")).toBe("xx");
  } finally {
    await second.dispose();
    await remove(cwd);
  }
});

test("an operator question survives a resume and validates its answer", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-operator-resume-"));
  const record = join(cwd, "life.jsonl");
  const first = new World({ cwd, record });
  const question = first.open().prompt("bool", "Continue?", { to: "operator" }).id;
  await until(first, () => first.prompts.has(question));
  await first.dispose();
  const second = new World({ record });
  try {
    const life = second.open();
    expect(second.prompts.size).toBe(0);
    await second.resume();
    await until(second, () => second.prompts.has(question));
    expect(second.prompts.get(question)?.message).toBe("Continue?");
    expect(() => second.answer(question, "maybe")).toThrow("yes or no");
    second.answer(question, "yes");
    expect(await life.result<boolean>(question)).toBe(true);
  } finally {
    await second.dispose();
    await rm(cwd, { recursive: true });
  }
});

/** What a command has printed so far, as the act table holds it. */
const printed = (world: World, id: string) =>
  (world.activity.acts.get(id)?.value as { stdout?: { content: string } } | undefined)?.stdout?.content ?? "";
/** The kinds of the facts a record holds, in order. */
const kinds = async (record: string) =>
  (await readFile(record, "utf8"))
    .split("\n")
    .filter(Boolean)
    .map((line) => JSON.parse(line)[0][0] as string);
/** One turn of the loop of the host, so that every fact said before it has reached the World. */
const tick = () => new Promise((resolve) => setTimeout(resolve, 0));

test("a command a rung started runs in no later life before a wake, and once at the wake", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-silent-"));
  const record = join(cwd, "life.jsonl");
  const first = new World({ cwd, record });
  const life = first.open();
  life.rung('await bash("printf once >> count; exec sleep 30")');
  // A second command waits for the first to write, so the first stands silent in a record that holds its start.
  await life.result(bash(life, "while [ ! -s count ]; do sleep 0.01; done"));
  await first.dispose();
  const second = new World({ record });
  try {
    const resumed = second.open();
    await tick();
    expect(second.pending.has("bash1")).toBe(true);
    expect(await readFile(join(cwd, "count"), "utf8")).toBe("once");
    await second.resume();
    for (let tries = 0; (await readFile(join(cwd, "count"), "utf8")) === "once" && tries < 250; tries++)
      await Bun.sleep(20);
    expect(await readFile(join(cwd, "count"), "utf8")).toBe("onceonce");
    resumed.wake(resumed.root);
    await Bun.sleep(100);
    expect(await readFile(join(cwd, "count"), "utf8")).toBe("onceonce");
  } finally {
    await second.dispose();
    await remove(cwd);
  }
});

test("resume wakes no work that a pause of the operator holds, so that pause stands", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-own-pause-"));
  const record = join(cwd, "life.jsonl");
  const first = new World({ cwd, record, ...modeled, answer: () => new Promise(() => {}) });
  const life = first.open();
  life.pause(life.root);
  life.prompt("str", "later");
  await first.dispose();
  const second = new World({ record, models: offered, answer: async () => said('close("x")') });
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

test("a reopened World pauses no chain, so new work asks once the pending work is gone", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-drained-"));
  const record = join(cwd, "life.jsonl");
  const first = new World({ cwd, record, ...modeled, answer: () => new Promise(() => {}) });
  const prompt = first.open().prompt("str", "first").id;
  await first.dispose();
  const second = new World({ record, models: offered, answer: async () => said('close("x")') });
  try {
    const again = second.open();
    expect(second.pending.has(prompt)).toBe(true);
    again.cancel(prompt);
    await tick();
    expect(second.pending.size).toBe(0);
    await second.resume();
    expect(await again.prompt<string>("str", "next")).toBe("x");
    expect(second.isPaused(again.root)).toBe(false);
  } finally {
    await second.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("a question a rung put to the operator is asked once in a later life", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-asked-once-"));
  const record = join(cwd, "life.jsonl");
  const first = new World({ cwd, record });
  first.open().rung('x = await prompt(str, "Q?", to="operator")\nclose(x)');
  const question = "prompt1";
  await until(first, () => first.prompts.has(question));
  await first.dispose();
  const asked: string[] = [];
  const second = new World({
    record,
    operator: async ({ id }) => {
      asked.push(id);
      return `answer ${asked.length}`;
    },
  });
  try {
    const again = second.open();
    await second.resume();
    expect(await again.result<string>(question)).toBe("answer 1");
    expect(asked.filter((id) => id === question)).toEqual([question]);
  } finally {
    await second.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("a wait the World starts again on resume says nothing after a cancel", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-wait-cancel-"));
  const record = join(cwd, "life.jsonl");
  const first = new World({ cwd, record });
  const wait = first.open().wait(0.2).id;
  await first.dispose();
  const second = new World({ record });
  try {
    const again = second.open();
    await second.resume();
    again.cancel(wait);
    // A later wait outlives the deadline of the first one.
    await again.result(again.wait(0.4).id);
    expect(second.activity.acts.get(wait)?.value).toEqual({ is: "CancelledError", args: [] });
    expect(
      (await readFile(record, "utf8"))
        .split("\n")
        .filter((line) => line.includes(`["done","${wait}","world"`)),
    ).toEqual([]);
  } finally {
    await second.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("an act that never ends by design holds nothing, and a close of the World leaves the record as it was", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-door-"));
  const record = join(cwd, "life.jsonl");
  const door = [
    "def note(id):",
    "  while True:",
    "    match (yield):",
    "      case ('read', qid, _, _, path) if path.startswith('note://'):",
    "        yield 'done', qid, Text(path, 'kept')",
    "",
    "act('note', '', note)",
    "close(1)",
  ].join("\n");
  const first = new World({ cwd, record, ...modeled, answer: async () => said(door) });
  expect(await first.open().prompt<number>("int", "make a door")).toBe(1);
  const before = await readFile(record, "utf8");
  await first.dispose();
  const second = new World({ record });
  try {
    expect(await readFile(record, "utf8")).toBe(before);
    expect((await inspectRecord(record)).pending).toEqual([]);
    second.open();
    expect(second.pending.size).toBe(0);
  } finally {
    await second.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("record inspection reports pending work when its replay starts or feeds a command", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-inspect-commands-"));
  const record = join(cwd, "life.jsonl");
  const first = new World({ cwd, record });
  const life = first.open();
  life.rung('await bash("exec sleep 30")');
  life.rung('c = bash("head -c 6; exec sleep 30", fed=True)\nwrite(Text(c + "/stdin", "hello\\n"))\nawait c');
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

test("a later life stands on what its host offers now, and a stood tells its chains, though its work stays pending", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-gone-model-"));
  const record = join(cwd, "life.jsonl");
  let calls = 0;
  const first = new World({
    cwd,
    record,
    ...modeled,
    answer: async () => said(++calls === 1 ? "close(5)" : "await wait(60)\nclose(6)"),
  });
  const life = first.open();
  const answered = life.prompt<number>("int", "five");
  expect(await answered).toBe(5);
  life.prompt("int", "later");
  await until(first, () => first.activity.acts.has("wait1"));
  await first.dispose();
  const pending = ["prompt2", "rung3", "wait1"];
  const second = new World({ record });
  try {
    expect((await inspectRecord(record)).pending.map(([id]) => id)).toEqual(pending);
    const again = second.open();
    expect(second.roster).toEqual([]);
    const [, [, , actor]] = again.call<[unknown, [unknown, string, string]]>(
      "ask",
      ["stand", again.root],
      {},
    );
    expect(actor).toBe("operator");
    expect(second.facts.filter(([kind]) => kind === "stood").map(([, id]) => id)).toContain(again.root);
    expect(again.outcome(answered.id)).toEqual({ done: true, value: 5 });
    expect([...second.pending.keys()]).toEqual(pending);
  } finally {
    await second.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("the World ends a command only at a control that covers it, so a close of the prompt lets its command run", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-covers-"));
  const words = ["c = bash('sleep 30')\nawait wait(60)", "c = bash('sleep 0.3; echo late')\nclose(1)"];
  const session = await boot({ cwd, ...modeled, answer: async () => said(words.shift() ?? "close(0)") });
  const { life } = session;
  try {
    const world = session.world;
    if (!world) throw new Error("The session has no World.");
    const cancelled = life.prompt<number>("int", "go");
    await until(world, () => world.activity.acts.has("bash1"));
    life.cancel(cancelled.id);
    await expect(life.result("bash1")).rejects.toThrow("CancelledError");
    expect(await life.prompt<number>("int", "go")).toBe(1);
    const late = await exited(world, life, "bash2");
    expect([late.code, late.stdout.content]).toEqual([0, "late\n"]);
  } finally {
    await session.dispose();
    await remove(cwd);
  }
});

test("a timeout or a wait past the longest timer runs its full time, and a command with no timeout runs to its end", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-long-"));
  const session = await boot({ cwd });
  const { life } = session;
  const month = 30 * 86400;
  try {
    const wait = life.wait(month).id;
    const long = bash(life, "sleep 1; echo finished", { timeout: month });
    const endless = bash(life, "sleep 1; echo finished", { timeout: null });
    const world = session.world;
    if (!world) throw new Error("The session has no World.");
    const exits = await Promise.all([exited(world, life, long), exited(world, life, endless)]);
    expect(exits.map((exit) => exit.code)).toEqual([0, 0]);
    expect(life.outcome(wait).done).toBe(false);
  } finally {
    await session.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("a close of the World whose save fails still ends its life and its commands", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-failed-save-"));
  const record = join(cwd, "life.jsonl");
  const world = new World({ cwd, record });
  const life = world.open();
  const command = bash(life, `${printPid}; exec sleep 30`);
  await until(world, () => printed(world, command) !== "");
  const pid = Number(printed(world, command));
  // A directory where the save writes its file makes the save fail on every system.
  await mkdir(`${record}.world.json.tmp`);
  try {
    await expect(world.dispose()).rejects.toThrow("life.jsonl.world.json.tmp");
    expect(life.disposed).toBe(true);
    // A process the World killed ends a moment after, once the loop of this process hears of it.
    for (let tries = 0; alive(pid) && tries < 100; tries++) await Bun.sleep(20);
    expect(alive(pid)).toBe(false);
  } finally {
    await remove(cwd);
  }
});

test("a host that asks the life at each change hears no change of its own asking", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-own-query-"));
  const world = new World({ cwd });
  const life = world.open();
  try {
    await tick();
    let calls = 0;
    world.on("change", () => {
      calls++;
      life.clock();
    });
    life.clock();
    await tick();
    expect(calls).toBe(0);
    expect(world.facts.filter(([kind, id]) => kind === "done" && id.startsWith("clock@"))).toEqual([]);
  } finally {
    await world.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("a host that reads the life at a change of a file leaves the write whole", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-write-change-"));
  const world = new World({ cwd });
  const life = world.open();
  try {
    await tick();
    world.on("change", () => verb(life, "cwd"));
    expect(await life.rung('t = write(Text("note.txt", "hello\\n"))\nclose(t.content)')).toBe("hello\n");
  } finally {
    await world.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("a told text that names a failure is no failure: only two failed asks in a row pause the chain", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-mute-text-"));
  await writeFile(
    join(cwd, "notes.txt"),
    "Last run: claude-cli:sonnet/low answered nothing: Error: 529 overloaded\n",
  );
  let calls = 0;
  const session = await boot({
    cwd,
    ...modeled,
    answer: async () => {
      calls++;
      if (calls === 2) throw new Error("first outage");
      return said(calls === 1 ? 'read("notes.txt")' : 'close("ok")');
    },
  });
  try {
    expect(await session.life.prompt<string>("str", "work")).toBe("ok");
    expect(calls).toBe(3);
    expect(session.world?.facts.some((fact) => fact[0] === "pause")).toBe(false);
  } finally {
    await session.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("a whole number from the operator answers a float prompt as a float", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-float-operator-"));
  const session = await boot({ cwd, operator: async () => 2 });
  try {
    const question = session.life.prompt<number>("float", "A number?", { to: "operator" });
    expect(await question).toBe(2);
    await session.life.rung(`x = peek(${JSON.stringify(question.id)})`);
    expect(session.life.inspect("x").representation).toBe("2.0");
  } finally {
    await session.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("an operator answer of a list keeps its numbers exact, and a map in it that holds the key is stays a map", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-list-answer-"));
  const world = new World({ cwd });
  const life = world.open();
  try {
    const question = life.prompt("list", "Numbers?", { to: "operator" });
    await until(world, () => world.prompts.has(question.id));
    expect(() => world.answer(question.id, "[9007199254740993]")).toThrow("safe integer");
    world.answer(question.id, '[2.0, 3, {"is": "str", "args": [1.0]}]');
    expect(await question).toEqual([
      2,
      3,
      {
        is: "dict",
        args: [
          [
            ["is", "str"],
            ["args", [1]],
          ],
        ],
      },
    ]);
    await life.rung(`x = peek(${JSON.stringify(question.id)})`);
    expect(life.inspect("x").representation).toBe("[2.0, 3, {'is': 'str', 'args': [1.0]}]");
    expect(life.inspect("x").value).toEqual([
      2,
      3,
      {
        is: "dict",
        args: [
          [
            ["is", "str"],
            ["args", [1]],
          ],
        ],
      },
    ]);
  } finally {
    await world.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("a host carries a filter of its own into the life through the ears of its World", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-host-filter-"));
  const world = new World({ cwd });
  const session = await boot({ cwd });
  try {
    for (const [life, ears] of [
      [world.open(), world.ears],
      [session.life, session.ears],
    ] as const) {
      let calls = 0;
      const filter = ears.callable((acts: unknown[]) => {
        calls++;
        return acts;
      });
      life.chain("fork", life.root, filter);
      expect(calls).toBe(1);
    }
  } finally {
    await world.dispose();
    await session.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("a read keeps the byte order mark of a file, as a write does", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-bom-"));
  await writeFile(join(cwd, "table.csv"), "﻿name,value\n");
  const session = await boot({ cwd });
  try {
    const text = read(session.life, "table.csv");
    expect(text.content.codePointAt(0)).toBe(0xfeff);
    write(session.life, "table.csv", text.content.replace("value", "amount"));
    expect([...(await readFile(join(cwd, "table.csv"))).subarray(0, 3)]).toEqual([0xef, 0xbb, 0xbf]);
  } finally {
    await session.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("a World with no model puts to the operator every prompt that names no actor, the acknowledgment among them", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-operator-alone-"));
  let acknowledged: (message: string) => void = () => {};
  const acknowledgment = new Promise<string>((resolve) => {
    acknowledged = resolve;
  });
  // The first example of the README, whose callback answers for the operator.
  const session = await boot({
    cwd,
    record: join(cwd, "work.jsonl"),
    operator: async ({ shape, message }) => {
      if (shape === "None") acknowledged(message);
      return shape === "str" ? `You asked: ${message}` : null;
    },
  });
  try {
    expect(await session.life.prompt<string>("str", "What is this project?")).toBe(
      "You asked: What is this project?",
    );
    session.life.rung('b = bash("true")');
    expect(await acknowledgment).toBe("bash1 done");
  } finally {
    await session.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("a record of 0.1.0 opens with no drift, though it answers a read and a write with a text", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-old-record-"));
  const record = join(cwd, "record.jsonl");
  await writeFile(record, await readFile(join(import.meta.dir, "../../../test/outside/record-0.1.0.jsonl")));
  await writeFile(join(cwd, "a.txt"), "one\ntwo\nthree\n");
  const world = new World({ cwd, record });
  try {
    const life = world.open();
    expect(life.raised).toBeNull();
    expect(life.root).toBe("chain1");
    // The word of 0.1.0 runs again, and its read and its write take the text the record answers them with.
    expect(life.outcome("rung1")).toEqual({ done: true, value: [2, "one\ntwo\nthree\n"] });
    expect(read(life, "a.txt")).toEqual({
      path: join(cwd, "a.txt"),
      content: "one\ntwo\nthree\n",
      before: null,
    });
  } finally {
    await world.dispose();
    await rm(cwd, { recursive: true, force: true });
  }
});

import { afterAll, expect, test } from "bun:test";
import { chmod, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createModels, getSupportedThinkingLevels } from "@earendil-works/pi-ai";
import {
  actorParts,
  boot,
  decodeRecord,
  type Ear,
  Ears,
  type Fact,
  inspectRecord,
  type Turn,
  World,
} from "../src/index.ts";
import { claudeProvider, cliModel } from "../src/providers/claude.ts";
import { RecordFile } from "../src/record.ts";
import { until } from "./until.ts";

// A World with no model puts every prompt to the operator, so a test that asks a model names one, and its `answer`
// replaces the request, so no CLI runs.
const offer = claudeProvider();
const offered = createModels();
offered.setProvider(offer.provider);
const modeled = { models: offered, model: "claude-cli:sonnet" };
afterAll(() => offer.dispose());
/** A turn of a model whose word is the one given. */
const said = (word: string): Turn => ["assistant", [word], null, null];

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
    const lock = await readFile(`${record}.lock`, "utf8");
    const first = await inspectRecord(record);
    expect(first.held.map(([id]) => id)).toEqual([waiting, prompt]);
    expect(await readFile(record, "utf8")).toBe(before);
    expect(await readFile(`${record}.world.json`, "utf8")).toBe(metadata);
    expect(await readFile(`${record}.lock`, "utf8")).toBe(lock);
    expect(await readFile(join(cwd, "value.txt"), "utf8")).toBe("once");
    life.cancel(waiting);
    life.close("answered", prompt);
    expect((await inspectRecord(record)).held).toEqual([]);
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

test("a World holds the models its host gives it: a saved roster gains what the host offers since, and an id alone routes", async () => {
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
  const first = new World({ cwd, record, models, model: "claude-cli:opus", roster: ["claude-cli:sonnet"] });
  try {
    first.open();
  } finally {
    await first.dispose();
  }
  const second = new World({ record, models, roster: ["claude-cli:fable"] });
  try {
    expect(second.model).toBe("claude-cli:opus");
    expect(second.roster).toEqual(["claude-cli:opus", "claude-cli:sonnet", "claude-cli:fable"]);
    expect(second.route("haiku").id).toBe("haiku");
    expect(() => second.route("nothing")).toThrow("Name one as provider:model");
  } finally {
    await second.dispose();
    cli.dispose();
  }
  // Without the provider that offered them, the saved models stay, since the record was lived on them; only an ask
  // of one fails.
  expect((await inspectRecord(record)).held).toEqual([]);
  const third = new World({ record });
  try {
    expect(third.model).toBe("claude-cli:opus");
    expect(third.roster).toEqual(["claude-cli:opus", "claude-cli:sonnet", "claude-cli:fable"]);
    expect(third.actor).toBe("claude-cli:opus/low");
    expect(() => third.route("claude-cli:opus")).toThrow("No model claude-cli:opus");
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
  const bin = new URL("fake-claude.ts", import.meta.url).pathname;
  await chmod(bin, 0o755);
  const cli = claudeProvider({ bin });
  const models = createModels();
  models.setProvider(cli.provider);
  const world = new World({ cwd, models, roster: ["claude-cli:sonnet"] });
  try {
    const life = world.open();
    expect(await life.prompt<string>("str", "FENCE")).toBe("reply 2");
    const transcript = life.rendered().join("\n");
    expect(transcript).toContain("```python");
    expect(transcript).toContain("<refused");
  } finally {
    await world.dispose();
    cli.dispose();
    await rm(cwd, { recursive: true, force: true });
  }
});

test("the default World serves files and streams commands without any TUI", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-world-"));
  const session = boot({ cwd });
  try {
    expect(session.life.cwd()).toBe(cwd);
    session.life.write({ path: "hello.txt", content: "hello\n" });
    expect(session.life.read("hello.txt").content).toBe("hello\n");
    const command = session.life.bash("printf 'snow: 雪\\n'; printf 'problem\\n' >&2", {
      showErr: { is: "name", name: "TAIL" },
    });
    const result = (await command) as {
      code: number;
      stdout: { content: string };
      stderr: { content: string };
    };
    expect(result.code).toBe(0);
    expect(result.stdout.content).toBe("snow: 雪\n");
    expect(result.stderr.content).toBe("problem\n");
    const input = session.life.bash("cat", { fed: true });
    session.life.write({ path: `${input.id}/stdin`, content: "fed\n" });
    session.life.write({ path: `${input.id}/stdin`, content: "" });
    expect(((await input) as { stdout: { content: string } }).stdout.content).toBe("fed\n");
  } finally {
    await session.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("unfinished model work and waits reopen held until the host confirms resume", async () => {
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
    answer: async () => {
      calls++;
      return ["assistant", ['close("resumed")'], null, null];
    },
  });
  const resumed = second.open();
  try {
    // A reopened World holds every ask until the host resumes it, so no model is asked however long it stands.
    expect(calls).toBe(0);
    expect(second.held.has(prompt)).toBe(true);
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
    await writeFile(path, '["x",["done","x","world",null]]\n["partial"');
    const record = new RecordFile(path);
    expect(record.entries).toHaveLength(1);
    expect(() => new RecordFile(path)).toThrow("Another process");
    record.keep(["y", ["done", "y", "world", 1]]);
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
  const session = boot({
    ...modeled,
    answer: async () => {
      if (++calls === 1) throw new Error("temporary outage");
      return ["assistant", ['close("recovered")'], null, null];
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
  const broken = boot({
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
    expect(broken.life.rendered().join("\n")).toContain("answered nothing: Error: unavailable");
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

test("the record, not stale saved metadata, decides whether a reopened life has held work", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-stale-held-"));
  const record = join(cwd, "life.jsonl");
  const first = new World({
    cwd,
    record,
    ...modeled,
    answer: async () => ["assistant", ['close("done")'], null, null],
  });
  const prompt = first.open().prompt<string>("str", "finish");
  await prompt;
  await first.dispose();
  const saved = JSON.parse(await readFile(`${record}.world.json`, "utf8"));
  saved.held = [[prompt.id, "prompt"]];
  await writeFile(`${record}.world.json`, JSON.stringify(saved));
  let calls = 0;
  const second = new World({
    record,
    answer: async () => {
      calls++;
      return ["assistant", ['close("new answer")'], null, null];
    },
  });
  try {
    const life = second.open();
    expect(second.held.size).toBe(0);
    expect(life.outcome(prompt.id).done).toBe(true);
    expect(await life.prompt<string>("str", "continue")).toBe("new answer");
    expect(calls).toBe(1);
  } finally {
    await second.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("native transcript rendering retains Python values and chain accepts its parent", async () => {
  const rendered: string[] = [];
  const session = boot({
    ...modeled,
    answer: async (_actor, _chain, _turns, _signal, texts) => {
      rendered.push(...texts);
      return ["assistant", ['close("ok")'], null, null];
    },
  });
  try {
    const child = session.life.chain("child", null, null, session.life.root);
    expect(session.life.get(child.id)[3]).toBe(session.life.root);
    await session.life.rung(
      `tell("values", ("f", 1.0), ("t", (1, 2)), body=[['a', 'b']])\nresult = await bash("printf hi")\ntell("exit", body=result)`,
      { on: child.id },
    );
    const text = session.life.rendered(child.id).join("\n");
    expect(text).toContain('f="1.0"');
    expect(text).toContain('t="(1, 2)"');
    expect(text).toContain("['a', 'b']");
    expect(text).toContain("Exit(code=0, stdout=Text(");
    await session.life.prompt<string>("str", "finish", { on: child.id });
    expect(rendered.join("\n")).toContain('f="1.0"');
    expect(rendered.join("\n")).toContain("Exit(code=0, stdout=Text(");
  } finally {
    await session.dispose();
  }
});

test("input sent before a held command starts reaches its process after resume", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-fed-"));
  const record = join(cwd, "life.jsonl");
  const first = new World({ cwd, record });
  first.open().wait(60);
  await first.dispose();
  const second = new World({ record });
  try {
    const life = second.open();
    const command = life.bash("cat", { fed: true });
    life.write({ path: `${command.id}/stdin`, content: "before start\n" });
    life.write({ path: `${command.id}/stdin`, content: "" });
    await second.resume();
    const result = await command;
    expect(result.stdout.content).toBe("before start\n");
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
  for (let number = 0; number < 25; number++) life.write({ path: "file", content: String(number) });
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
    expect(second.held.size).toBe(0);
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
          yield ["done", fact[1], { is: "Text", path: `${cwd}/file`, content: "one\ntwo\n" }];
        }
      }
    })(),
  });
  const life = ears.boot();
  try {
    const show = ears.callable((lines: string[]) => lines.map((_, index) => index + 1));
    expect(life.read("file", show).path).toBe("/tmp/file");
    expect(() => life.clock()).toThrow("no number");
  } finally {
    life.dispose();
  }
});

test("an interrupted command is reported on resume without repeating its side effect", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-command-resume-"));
  const record = join(cwd, "life.jsonl");
  const first = new World({ cwd, record });
  const life = first.open();
  const command = life.bash("printf 'once\\n' >> count; printf ready; sleep 5").id;
  const output = () =>
    (first.activity.acts.get(command)?.value as { stdout?: { content: string } }).stdout?.content;
  await until(first, () => output() === "ready");
  await first.dispose();
  const second = new World({ record });
  try {
    const resumed = second.open();
    expect(second.held.has(command)).toBe(true);
    await second.resume();
    await expect(resumed.result(command)).rejects.toThrow("command process ended");
    expect(await readFile(join(cwd, "count"), "utf8")).toBe("once\n");
  } finally {
    await second.dispose();
    await rm(cwd, { recursive: true });
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
    .map((line) => JSON.parse(line)[1][0] as string);
/** One turn of the loop of the host, so that every fact said before it has reached the World. */
const tick = () => new Promise((resolve) => setTimeout(resolve, 0));

test("a command a rung started and that printed nothing is never run again by a later life", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-silent-"));
  const record = join(cwd, "life.jsonl");
  const first = new World({ cwd, record });
  const life = first.open();
  life.rung('await bash("printf once >> count; exec sleep 30")');
  // A second command waits for the first to write, so the first stands silent in a record that holds nothing of it.
  await life.bash("while [ ! -s count ]; do sleep 0.01; done");
  await first.dispose();
  const second = new World({ record });
  try {
    const resumed = second.open();
    expect(second.held.has("bash://operator.2.1")).toBe(true);
    await second.resume();
    await expect(resumed.result("bash://operator.2.1")).rejects.toThrow("command process ended");
    expect(await readFile(join(cwd, "count"), "utf8")).toBe("once");
  } finally {
    await second.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("resume lets go of the hold of the World alone, so a pause of the operator stands", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-own-pause-"));
  const record = join(cwd, "life.jsonl");
  const first = new World({ cwd, record, ...modeled, answer: () => new Promise(() => {}) });
  const life = first.open();
  life.pause(life.root);
  life.prompt("str", "later");
  await first.dispose();
  const second = new World({ record, answer: async () => said('close("x")') });
  try {
    const again = second.open();
    await second.resume();
    expect(second.isPaused(again.root)).toBe(true);
    expect(await kinds(record)).not.toContain("wake");
  } finally {
    await second.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("a reopened World pauses no chain, so new work asks once the held work is gone", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-drained-"));
  const record = join(cwd, "life.jsonl");
  const first = new World({ cwd, record, ...modeled, answer: () => new Promise(() => {}) });
  const prompt = first.open().prompt("str", "first").id;
  await first.dispose();
  const second = new World({ record, answer: async () => said('close("x")') });
  try {
    const again = second.open();
    expect(second.held.has(prompt)).toBe(true);
    again.cancel(prompt);
    await tick();
    expect(second.held.size).toBe(0);
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
  const question = "prompt://operator.2.1";
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
    expect((await inspectRecord(record)).held).toEqual([]);
    second.open();
    expect(second.held.size).toBe(0);
  } finally {
    await second.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("record inspection reports held work when its replay starts or feeds a command", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-inspect-commands-"));
  const record = join(cwd, "life.jsonl");
  const first = new World({ cwd, record });
  const life = first.open();
  life.rung('await bash("exec sleep 30")');
  life.rung('c = bash("head -c 6; exec sleep 30", fed=True)\nwrite(Text(c + "/stdin", "hello\\n"))\nawait c');
  await until(first, () => printed(first, "bash://operator.3.1") === "hello\n");
  await first.dispose();
  try {
    expect((await inspectRecord(record)).held.map(([id]) => id)).toEqual([
      "rung://operator.2",
      "bash://operator.2.1",
      "rung://operator.3",
      "bash://operator.3.1",
    ]);
  } finally {
    await rm(cwd, { recursive: true });
  }
});

test("a later life stands on the models its record was lived on, though the host offers them no more", async () => {
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
  await until(first, () => first.activity.acts.has("wait://operator.3.1.1"));
  await first.dispose();
  const held = ["prompt://operator.3", "rung://operator.3.1", "wait://operator.3.1.1"];
  const second = new World({ record });
  try {
    expect((await inspectRecord(record)).held.map(([id]) => id)).toEqual(held);
    const again = second.open();
    expect(second.roster).toEqual(["claude-cli:sonnet"]);
    expect(again.outcome(answered.id)).toEqual({ done: true, value: 5 });
    expect([...second.held.keys()]).toEqual(held);
  } finally {
    await second.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("a timeout or a wait past the longest timer runs its full time, and a command with no timeout runs to its end", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-long-"));
  const session = boot({ cwd });
  const { life } = session;
  const month = 30 * 86400;
  try {
    const wait = life.wait(month).id;
    const long = life.bash("sleep 1; echo finished", { timeout: month });
    const endless = life.call<string>("bash", ["sleep 1; echo finished"], { timeout: null, on: life.root });
    const exits = await Promise.all([long, life.result<{ code: number | null }>(endless)]);
    expect(exits.map((exit) => exit.code)).toEqual([0, 0]);
    expect(life.outcome(wait).done).toBe(false);
  } finally {
    await session.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("a close of the World whose save fails still ends its life and its commands", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-failed-save-"));
  const directory = join(cwd, "sessions");
  await mkdir(directory);
  const world = new World({ cwd, record: join(directory, "life.jsonl") });
  const life = world.open();
  const command = life.bash("printf $$; exec sleep 30").id;
  await until(world, () => printed(world, command) !== "");
  const pid = printed(world, command);
  await chmod(directory, 0o500);
  try {
    await expect(world.dispose()).rejects.toThrow("EACCES");
    expect(life.disposed).toBe(true);
    const state = Bun.spawnSync(["ps", "-o", "stat=", "-p", pid]).stdout.toString().trim();
    // A process the World killed is gone, or dead and not yet reaped.
    expect(state === "" || state.startsWith("Z")).toBe(true);
  } finally {
    await chmod(directory, 0o700);
    await rm(cwd, { recursive: true });
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
      life.cwd();
    });
    life.cwd();
    await tick();
    expect(calls).toBe(0);
    expect(world.facts.filter(([kind, id]) => kind === "done" && id.startsWith("cwd://"))).toEqual([]);
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
    world.on("change", () => life.cwd());
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
  const session = boot({
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
  const session = boot({ cwd, operator: async () => 2 });
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

test("an operator answer of a list keeps its numbers exact and its maps plain", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-list-answer-"));
  const world = new World({ cwd });
  const life = world.open();
  try {
    const question = life.prompt("list", "Numbers?", { to: "operator" });
    await until(world, () => world.prompts.has(question.id));
    expect(() => world.answer(question.id, "[9007199254740993]")).toThrow("safe integer");
    expect(() => world.answer(question.id, '[{"is": "str"}]')).toThrow("key is");
    world.answer(question.id, "[2.0, 3]");
    await question;
    await life.rung(`x = peek(${JSON.stringify(question.id)})`);
    expect(life.inspect("x").representation).toBe("[2.0, 3]");
  } finally {
    await world.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("a host carries a filter of its own into the life through the ears of its World", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-host-filter-"));
  const world = new World({ cwd });
  const session = boot({ cwd });
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
  const session = boot({ cwd });
  try {
    const text = session.life.read("table.csv");
    expect(text.content.codePointAt(0)).toBe(0xfeff);
    session.life.write({ path: "table.csv", content: text.content.replace("value", "amount") });
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
  const session = boot({
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
    expect(await acknowledgment).toBe("bash://operator.3.1 done");
  } finally {
    await session.dispose();
    await rm(cwd, { recursive: true });
  }
});

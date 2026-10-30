import { expect, test } from "bun:test";
import { appendFile, mkdir, mkdtemp, readdir, readFile, rm, stat, writeFile } from "node:fs/promises";
import { homedir, tmpdir } from "node:os";
import { join, sep } from "node:path";
import { FileChanges } from "../src/changes.ts";
import {
  Activity,
  actorParts,
  boot,
  DamagedJournal,
  type Ear,
  enabled,
  type Fact,
  inspectRecord,
  Life,
  ROOT,
  Session,
  type Turn,
} from "../src/index.ts";
import { compactedRun, pauseSaid } from "../src/life/acts.ts";
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
    const waiting = engine.wait([], { seconds: 60, on }).id;
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
    const here = engine.wait([], { seconds: 60, on: root }).id;
    const there = engine.wait([], { seconds: 60, on: two }).id;
    const paused = () => [root, two, here, there].filter((id) => session.isPaused(id));
    engine.pause(two);
    expect(paused()).toEqual([two, there]);
    const later = engine.wait([], { seconds: 60, on: two }).id;
    expect(session.isPaused(later)).toBe(true);
    engine.pause(here);
    expect(paused()).toEqual([two, here, there]);
    engine.wake(two);
    expect(paused()).toEqual([here]);
    expect(session.isPaused(later)).toBe(false);
    engine.pause(root);
    engine.wake(here);
    expect(paused()).toEqual([root]);
    expect(session.isPaused(engine.wait([], { seconds: 60, on: root }).id)).toBe(true);
    expect(session.isPaused(engine.wait([], { seconds: 60, on: two }).id)).toBe(false);
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
  // A claude command line that the session names is one the catalog offers, and no model is asked.
  const every = new Session({ cwd, claude: join(cwd, "claude") });
  try {
    const [roster, , actor] = every.open().standing() as [[string][], string, string];
    expect([roster.map(([name]) => name).slice(0, 4), roster.at(-1)?.[0], actor]).toEqual([
      ["claude-cli:opus", "claude-cli:sonnet", "claude-cli:haiku", "claude-cli:fable"],
      "operator",
      "claude-cli:opus/high",
    ]);
  } finally {
    await every.dispose();
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
  const { promise: held, resolve: release } = Promise.withResolvers<void>();
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
    // The rung that answers the prompt comes after the ten rungs of the setup of the root, so it is found as the
    // rung that the prompt made.
    const answering = () =>
      [...session.activity.acts.values()].find((act) => act.kind === "rung" && act.by === prompt.id)?.id ??
      "";
    /** A turn of the request, of a role, which holds one text. */
    const turn = (role: string, text: unknown) =>
      expect.objectContaining({ role, content: [{ type: "text", text }] });
    await until(session, () => session.streams.get(answering())?.text === "close(3)");
    expect(answering()).toBe("rung11");
    expect(session.streams.get(answering())).toEqual({
      chain: engine.root,
      text: "close(3)",
      thinking: "counting",
    });
    expect(seen).toEqual([
      {
        actor: "claude-cli:sonnet/high",
        chain: engine.root,
        // The setup of the root as turns of the default actor whose words furb writes: the bare paragraph chain(), the
        // read of the doctrine, the read of the file of each official extension, the stand and the cd, then the life
        // words in one turn, whose user turn holds what they read, then the open of the root and the prompt.
        messages: [
          turn("user", "chain()"),
          // A path of Windows stands with its separators doubled, as python shows a backslash.
          turn("assistant", expect.stringMatching(/^read\('.*[/\\]DOCTRINE\.md'\)$/)),
          turn("user", expect.stringMatching(/^#read1\nread1_path = '.*[/\\]DOCTRINE\.md'\n/)),
          ...["catchup", "memory", "skills", "compaction", "ouroboros", "web", "mcp"].flatMap((name, at) => [
            turn(
              "assistant",
              expect.stringMatching(
                new RegExp(`^read\\('.*[/\\\\]+extensions[/\\\\]+${name}[/\\\\]+${name}\\.py'\\)$`),
              ),
            ),
            turn(
              "user",
              expect.stringMatching(
                new RegExp(`^#read${at + 2}\\nread${at + 2}_path = '.*[/\\\\]${name}\\.py'\\n`),
              ),
            ),
          ]),
          turn("assistant", expect.stringMatching(/^stand\(\)\ncd\('.*'\)$/)),
          turn(
            "user",
            expect.stringMatching(
              /^#stand2\nstand2_roster = \[.*\]\nstand2_actor = 'claude-cli:sonnet\/high'\n\n#cd1$/,
            ),
          ),
          turn("assistant", "catchup()\nawait recall()\nawait listed()\nawait mcp_index()"),
          // The open of the root goes as a block of its own after the setup, which ends the block before it.
          expect.objectContaining({
            role: "user",
            content: [
              {
                type: "text",
                text: expect.stringMatching(
                  /^#read9\nread9_path = 'skills:\/\/'\n(<s:read9_text>\n[\s\S]*<\/s:read9_text>|read9_text = '.*')$/,
                ),
              },
              {
                type: "text",
                text: expect.stringMatching(
                  /^#chain1\nchain1_label = 'root'\nchain1: Act\[Never\] = Act\('chain1'\)\n\n#prompt1\nprompt1_message = 'count'\nprompt1: Act\[int\] = Act\('prompt1'\)\n\n#rung11 advance on prompt1$/,
                ),
              },
            ],
          }),
        ],
        settings: {
          effort: "high",
          max_thinking_tokens: 32768,
          session: expect.stringMatching(/\/chain1$/),
        },
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

test("the act of a rung holds the word that its chain took, as its model or its caller wrote it", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-word-"));
  const session = new Session({
    cwd,
    record: join(cwd, "life.jsonl"),
    ...modeled,
    answer: async () => said("close(7)"),
  });
  try {
    const engine = session.open();
    const asked = engine.prompt("int", { message: "seven", on: engine.root });
    expect(await asked).toBe(7);
    const rung = engine.rung({ word: "x = 1", on: engine.root });
    await engine.result(rung.id);
    const acts = [...session.activity.acts.values()];
    expect(acts.find((act) => act.kind === "rung" && act.by === asked.id)?.word).toBe("close(7)");
    expect(session.activity.acts.get(rung.id)?.word).toBe("x = 1");
  } finally {
    await session.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("what a model writes after its reply is done streams nothing", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-late-"));
  const { promise: held, resolve: release } = Promise.withResolvers<void>();
  let calls = 0;
  const session = new Session({
    cwd,
    record: join(cwd, "life.jsonl"),
    ...modeled,
    answer: async (_, write) => {
      calls++;
      write({ text: `close(${calls})` });
      if (calls === 2) await held;
      return said(`close(${calls})`);
    },
  });
  try {
    const engine = session.open();
    // The door tells what a model wrote on the next turn of the loop of the host, so the session hears the part of
    // the first reply after the done of that reply.
    expect(await engine.prompt("int", { message: "one", on: engine.root })).toBe(1);
    const second = engine.prompt("int", { message: "two", on: engine.root });
    // The parts of the replies come in the order the models wrote them, so the part of the first reply has come
    // once the second streams.
    await until(session, () => [...session.streams.values()].some((stream) => stream.text === "close(2)"));
    expect([...session.streams.values()]).toEqual([{ chain: engine.root, text: "close(2)", thinking: "" }]);
    release();
    expect(await second).toBe(2);
    expect(session.streams.size).toBe(0);
  } finally {
    await session.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("a model call that fails for a cause that passes is asked again, and its stream holds the retry until it writes", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-retry-"));
  let calls = 0;
  const session = new Session({
    cwd,
    record: join(cwd, "life.jsonl"),
    ...modeled,
    answer: async (_, write) => {
      calls++;
      write({ text: "close(" });
      if (calls === 1) throw new Error("read ECONNRESET: connection reset by peer");
      return said("close(5)");
    },
  });
  try {
    const engine = session.open();
    const asked = Date.now();
    const answer = engine.prompt("int", { message: "count", on: engine.root });
    await until(session, () => [...session.streams.values()].some((stream) => stream.retry));
    const [stream] = [...session.streams.values()];
    // What the model wrote before the failure is void.
    expect({ ...stream, retry: { ...stream?.retry, until: 0 } }).toEqual({
      chain: engine.root,
      text: "",
      thinking: "",
      retry: { attempt: 2, most: 10, until: 0, why: "connection reset" },
    });
    expect(stream?.retry?.until).toBeGreaterThanOrEqual(asked + 4000);
    expect(await answer).toBe(5);
    expect(calls).toBe(2);
  } finally {
    await session.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("a session that keeps a record keeps a snapshot of its life, and a later session opens on it", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-snapshot-"));
  const record = join(cwd, "life.jsonl");
  const first = new Session({ cwd, record, extensions: false });
  const engine = first.open();
  engine.prompt("None", { message: "noted", to: "operator", on: engine.root });
  await first.dispose();
  const second = new Session({ cwd, record, extensions: false });
  try {
    expect((await stat(`${record}.snapshot`)).size).toBeGreaterThan(0);
    second.open();
    expect(second.activity.acts.get("prompt1")?.kind).toBe("prompt");
  } finally {
    await second.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("a later session on a record holds no stream of the life before it", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-saved-streams-"));
  const record = join(cwd, "life.jsonl");
  const first = new Session({
    cwd,
    record,
    ...modeled,
    answer: (_, write) => {
      write({ text: "close(" });
      return new Promise(() => {});
    },
  });
  const engine = first.open();
  engine.prompt("int", { message: "count", on: engine.root });
  await until(first, () => first.streams.size === 1);
  await first.dispose();
  const second = new Session({ record });
  try {
    expect(second.streams.size).toBe(0);
  } finally {
    await second.dispose();
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
  const wait = engine.wait([], { seconds: 0.1, on: engine.root }).id;
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

test("work that ends with its life again and again parks until the host retries it", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-parked-"));
  const record = join(cwd, "life.jsonl");
  const never = { ...modeled, answer: () => new Promise(() => {}) };
  const first = new Session({ cwd, record, ...never });
  const engine = first.open();
  const prompt = engine.prompt("str", { message: "parked", on: engine.root }).id;
  await tick();
  await first.dispose();
  for (let life = 1; life < 5; life++) {
    const later = new Session({ record, ...never });
    later.open();
    await later.resume();
    await tick();
    await later.dispose();
  }
  let calls = 0;
  const last = new Session({
    record,
    ...modeled,
    answer: async () => {
      calls++;
      return ["assistant", 'close("retried")', null, null];
    },
  });
  const resumed = last.open();
  try {
    expect([...last.parked.values()]).toEqual(["reply"]);
    const [reply = ""] = last.parked.keys();
    expect(last.pending.size).toBe(0);
    resumed.wake(resumed.root);
    await tick();
    expect(calls).toBe(0);
    last.retry(reply);
    expect(await resumed.result<string>(prompt)).toBe("retried");
    expect(calls).toBe(1);
    expect(last.parked.size).toBe(0);
  } finally {
    await last.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("the session asks the engine for the pending work again only after a fact that can change it", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-pending-"));
  const record = join(cwd, "life.jsonl");
  const first = new Session({ cwd, record, ...modeled, answer: () => new Promise(() => {}) });
  const engine = first.open();
  const gone = engine.wait([], { seconds: 600, on: engine.root }).id;
  const wait = engine.wait([], { seconds: 600, on: engine.root }).id;
  await first.dispose();
  const second = new Session({ record, ...modeled, answer: () => new Promise(() => {}) });
  const resumed = second.open();
  try {
    let asked = 0;
    const pending = resumed.pending.bind(resumed);
    resumed.pending = () => {
      asked++;
      return pending();
    };
    const read = () => [[...second.pending.keys()], asked];
    expect(read()).toEqual([[gone, wait], 1]);
    resumed.clock({ on: resumed.root });
    await tick();
    expect(read()).toEqual([[gone, wait], 1]);
    // The done of an act of the pending work, which a cancel says.
    resumed.cancel(gone);
    await tick();
    expect(read()).toEqual([[wait], 2]);
    resumed.pause(resumed.root);
    await tick();
    expect(read()).toEqual([[], 3]);
    // The wake, and the started of the wait that the World takes again.
    resumed.wake(resumed.root);
    await tick();
    expect(read()).toEqual([[], 4]);
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

test("a chain that its provider paused names the provider and its error as the cause of the pause, until a wake", async () => {
  const broken = boot({
    ...modeled,
    answer: async () => {
      throw new Error("unavailable");
    },
  });
  try {
    const root = broken.engine.root;
    broken.engine.prompt("str", { message: "try", on: root });
    await until(broken, () => broken.facts.some((fact) => fact[0] === "pause"));
    const cause = broken.pauseOf(root);
    expect(cause).toMatchObject({ by: "provider" });
    expect(String(cause?.why)).toContain("unavailable");
    broken.engine.wake(root);
    expect(broken.pauseOf(root)).toBeUndefined();
  } finally {
    await broken.dispose();
  }
});

test("a pause of the operator names the operator and the reason that it gave", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-pause-cause-"));
  const life = boot({ ...modeled, record: join(cwd, "life.jsonl") });
  try {
    const root = life.engine.root;
    life.engine.pause(root, { why: "wait for the owner" });
    expect(life.pauseOf(root)).toEqual({ by: "operator", why: "wait for the owner" });
    const cause = life.pauseOf(root);
    expect(cause && pauseSaid(cause)).toEqual({ text: "the operator: wait for the owner", error: false });
    life.engine.wake(root);
    life.engine.pause(root);
    expect(life.pauseOf(root)).toEqual({ by: "operator" });
  } finally {
    await life.dispose();
    await rm(cwd, { recursive: true, force: true });
  }
});

test("a damaged journal of file changes opens, names the change it cannot read, and is repaired or started anew", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-damaged-"));
  const record = join(cwd, "life.jsonl");
  const journal = `${record}.changes.jsonl`;
  const change = (after: string) => JSON.stringify({ path: "a", before: "", after });
  await writeFile(journal, `${change("one")}\n{ damaged }\n${change("two")}\n`);
  const repaired = new FileChanges(record);
  try {
    expect(repaired.length).toBe(3);
    expect(() => repaired.read()).toThrow(new DamagedJournal(journal, 1));
    expect(repaired.read(0, 1).map((one) => one.after)).toEqual(["one"]);
    // The repair keeps each change in its order, and a change appended after it reads with them.
    expect(repaired.readable().map((one) => one.after)).toEqual(["one", "two"]);
    expect(repaired.repair()).toEqual({ kept: 2, removed: 1 });
    repaired.append({ path: "a", before: "two", after: "three" });
    expect(repaired.read().map((one) => one.after)).toEqual(["one", "two", "three"]);
  } finally {
    repaired.dispose();
  }
  // A repair reads the whole file again: a change that this session appended, and a line that another program
  // appended after the session opened, both stay, and a line that a crash cut is counted as removed.
  await writeFile(journal, `${change("one")}\n{ damaged }\n${change("two")}\n`);
  const open = new FileChanges(record);
  try {
    open.append({ path: "a", before: "two", after: "three" });
    await appendFile(journal, `${change("four")}\nnull\n${change("five")}`);
    expect(open.repair()).toEqual({ kept: 4, removed: 3 });
    expect(open.length).toBe(4);
    expect(open.read().map((one) => one.after)).toEqual(["one", "two", "three", "four"]);
  } finally {
    open.dispose();
  }
  await writeFile(journal, `${change("one")}\nnull\n`);
  const renewed = new FileChanges(record);
  try {
    const aside = renewed.renew();
    expect(renewed.length).toBe(0);
    expect(await readFile(aside, "utf8")).toBe(`${change("one")}\nnull\n`);
    renewed.append({ path: "b", before: "", after: "new" });
    expect(renewed.read()).toEqual([{ path: "b", before: "", after: "new" }]);
    expect((await readdir(cwd)).sort()).toEqual(
      [aside.slice(cwd.length + 1), "life.jsonl.changes.jsonl"].sort(),
    );
  } finally {
    renewed.dispose();
    await rm(cwd, { recursive: true });
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

test("a prompt of None to the operator is closed as the console takes it, and stays a notice until a dismiss", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-notice-"));
  const session = boot({ cwd });
  try {
    const { engine } = session;
    const told = engine.prompt("None", { message: "The build is green.", to: "operator", on: engine.root });
    expect(await told).toBeNull();
    expect(session.console.prompts.size).toBe(0);
    expect([...session.console.notices.values()]).toEqual([{ id: told.id, message: "The build is green." }]);
    session.console.dismiss(told.id);
    expect(session.console.notices.size).toBe(0);
  } finally {
    await session.dispose();
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

test("an operator who takes none of the values of a question answers in his own words, which close it as Refused", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-operator-decline-"));
  const session = new Session({ cwd, record: join(cwd, "life.jsonl") });
  try {
    const engine = session.open();
    const question = engine.prompt("Literal['vitepress', 'mkdocs']", {
      message: "Which runner for the docs?",
      to: "operator",
      on: engine.root,
    }).id;
    await until(session, () => session.console.prompts.has(question));
    session.console.decline(question, "Neither: keep the plain Markdown.");
    await expect(engine.result(question)).rejects.toThrow("Neither: keep the plain Markdown.");
    expect(session.console.prompts.has(question)).toBe(false);
    expect(() => session.console.decline(question, "again")).toThrow("This prompt is no longer open.");
  } finally {
    await session.dispose();
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
  const sleeps = engine.rung({ word: 'await bash("exec sleep 30")', on }).id;
  const feeds = engine.rung({
    word: 'c = bash("head -c 6; exec sleep 30", stdin=None)\nwrite(Text(c + "/stdin", "hello\\n"))\nawait c',
    on,
  }).id;
  await until(first, () => printed(first, "bash2") === "hello\n");
  await first.dispose();
  try {
    // The read of the doctrine, the rungs of the seven official extensions, and the stand and the cd come before the
    // two rungs.
    expect([sleeps, feeds]).toEqual(["rung11", "rung12"]);
    expect((await inspectRecord(record)).pending.map(([id]) => id)).toEqual([
      sleeps,
      "bash1",
      feeds,
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

test("an operator answers a prompt of a list, a dict, a tuple or a union of the shapes that he answers, and a shape he answers not is refused at once", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-shapes-"));
  const session = new Session({ cwd });
  const engine = session.open();
  try {
    const on = engine.root;
    const cases: [string, string, string][] = [
      ["list[str]", '["a", "b"]', "['a', 'b']"],
      ["list[float]", "[1, 2.5]", "[1.0, 2.5]"],
      ["list[Literal['a', 'b']]", '["b"]', "['b']"],
      ["dict[str, list[int]]", '{"a": [1]}', "{'a': [1]}"],
      ["tuple[int, str]", '[1, "a"]', "(1, 'a')"],
      ["int | None", "4", "4"],
      ["int | None", "", "None"],
      ["str | None", "words", "'words'"],
    ];
    for (const [shape, line, value] of cases) {
      const question = engine.rung({
        word: `x = await prompt(${shape}, "Q", to=OPERATOR)\nclose(repr(x))`,
        on,
      });
      await until(session, () => [...session.console.prompts.values()].some((one) => one.shape === shape));
      const ask = [...session.console.prompts.values()].find((one) => one.shape === shape);
      expect(ask).toBeDefined();
      if (/^(list|dict|tuple)/.test(shape))
        expect(() => session.console.answer(ask?.id ?? "", "[")).toThrow();
      session.console.answer(ask?.id ?? "", line);
      expect(await engine.result<string>(question.id)).toBe(value);
    }
    const refused = engine.rung({ word: "close(await prompt(list[Option], 'Q', to=OPERATOR))", on });
    await expect(engine.result(refused.id)).rejects.toThrow();
    expect(session.console.prompts.size).toBe(0);
  } finally {
    await session.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("a session with no model puts to the operator every prompt that names no actor, a notice among them", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-operator-alone-"));
  const { promise: notice, resolve: noticed } = Promise.withResolvers<string>();
  // The first example of the README, whose callback answers for the operator.
  const session = boot({
    cwd,
    record: join(cwd, "work.jsonl"),
    roster: [],
    operator: async ({ shape, message }) => {
      if (shape === "None") noticed(message);
      return shape === "str" ? `You asked: ${message}` : null;
    },
  });
  try {
    const { engine } = session;
    const on = engine.root;
    expect(await engine.prompt("str", { message: "What is this project?", on })).toBe(
      "You asked: What is this project?",
    );
    engine.rung({ word: 'prompt(None, "The notes are ready.")', on });
    expect(await notice).toBe("The notes are ready.");
  } finally {
    await session.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("a wait of a program on a life throws once the life ended, and so does a wait that starts after the end", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-ended-"));
  const life = new Life(boot({ cwd, record: join(cwd, "work.jsonl"), roster: [] }));
  try {
    const waits = life.until(() => false);
    await tick();
    await life.end();
    await expect(waits).rejects.toThrow("The life ended.");
    await expect(life.until(() => false)).rejects.toThrow("The life ended.");
  } finally {
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
    expect(session.changes.read()).toEqual([
      {
        path: join(cwd, "note.txt"),
        before: "",
        after: "one\n",
        act: expect.stringMatching(/^write\d+$/),
        made: true,
      },
    ]);
    expect((await stat(`${record}.changes.jsonl`)).isFile()).toBe(true);
  } finally {
    await session.dispose();
    await rm(cwd, { recursive: true });
  }
});

/** A path as the World writes it for a model: `~` in place of the home of the user when it stands under the home. */
function tilded(path: string): string {
  const home = homedir();
  return path === home ? "~" : path.startsWith(home + sep) ? `~${path.slice(home.length)}` : path;
}

test("a session enables the extensions of the configs, and a later session on its record runs them whatever it says", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-extensions-"));
  const record = join(cwd, "life.jsonl");
  const config = join(cwd, "config");
  await writeFile(join(cwd, "CLAUDE.md"), "Use two spaces.\n");
  /** The names of the extensions that the life of a session runs, which the root says it enabled. */
  const running = (session: Session) =>
    enabled(session.engine?.transcript({ on: ROOT }) ?? []).map(({ name }) => name);
  const first = new Session({ cwd, record, config });
  try {
    first.open();
    expect(running(first)).toEqual(["catchup", "memory", "skills", "compaction", "ouroboros", "web", "mcp"]);
    // The life words of the setup read each memory file, after the reads of the doctrine and of the files of the
    // extensions, and the user turn after them shows what each read gave.
    const read = (first.engine?.transcript({ on: ROOT }) ?? []).find(
      (fact) => fact[0] === "read" && fact[4] === tilded(join(cwd, "CLAUDE.md")),
    )?.[1];
    expect(read).toBe("read9");
    const turns = (first.engine?.turns({ on: "chain1" }) ?? []).map((turn) => String(turn[1]));
    expect(turns.at(-2)).toBe("catchup()\nawait recall()\nawait listed()\nawait mcp_index()");
    expect(turns.at(-1)).toContain(
      `#${read}\n${read}_path = '${tilded(join(cwd, "CLAUDE.md")).replaceAll("\\", "\\\\")}'\n${read}_text = 'Use two spaces.'`,
    );
  } finally {
    await first.dispose();
  }
  const later = new Session({ cwd, record, config, extensions: false });
  const fresh = new Session({ cwd, config, extensions: false });
  try {
    later.open();
    fresh.open();
    expect([running(later), running(fresh)]).toEqual([
      ["catchup", "memory", "skills", "compaction", "ouroboros", "web", "mcp"],
      [],
    ]);
  } finally {
    await later.dispose();
    await fresh.dispose();
    await rm(cwd, { recursive: true, force: true });
  }
});

test("the activity starts the streams of a command again at each started of it, as a World that runs it again does", () => {
  const activity = new Activity();
  const hear = (fact: Fact) => {
    // No control stands, so the activity asks the engine nothing while it hears these facts.
    const heard = activity.hear(fact);
    while (!heard.next().done);
  };
  hear(["bash", "bash1", "operator", ROOT, "run", null, 600]);
  hear(["started", "bash1", "world"]);
  hear(["out", "bash1", "world", "first\n", "stdout"]);
  hear(["started", "bash1", "world"]);
  hear(["out", "bash1", "world", "second\n", "stdout"]);
  hear(["out", "bash1", "world", "bad\n", "stderr"]);
  expect(activity.acts.get("bash1")?.value).toMatchObject({
    stdout: { content: "second\n" },
    stderr: { content: "bad\n" },
  });
});

test("the activity follows no writing of an older record, which holds each writing as a question", () => {
  const activity = new Activity();
  const hear = (fact: Fact) => {
    const heard = activity.hear(fact);
    while (!heard.next().done);
  };
  hear(["rung", "rung1", "prompt1", ROOT, "", "", "claude-cli:opus/high"]);
  hear(["writing", "writing1", "provider", ROOT, "rung1", "x = 1\n", 0]);
  hear(["done", "writing1", "kernel", null]);
  expect(activity.acts.has("writing1")).toBe(false);
  expect(activity.acts.has("rung1")).toBe(true);
});

test("the activity places each act in time by the count of facts, and keeps the answers of each chain", () => {
  const activity = new Activity();
  const hear = (fact: Fact) => {
    const heard = activity.hear(fact);
    while (!heard.next().done);
  };
  hear(["bash", "bash1", "operator", ROOT, "ls", "", 600]);
  hear(["done", "bash1", "world", { is: "Exit", code: 0 }]);
  hear(["reply", "reply1", "rung1", ROOT, "claude-cli:opus/high"]);
  hear(["done", "reply1", "world", ["assistant", "close(1)", [100, 10, 80, 0, 0.5], null]]);
  expect(activity.acts.get("bash1")).toMatchObject({ kind: "bash", began: 1, ended: 2, done: true });
  expect(activity.answers.get(ROOT)).toEqual([
    { at: 4, actor: "claude-cli:opus/high", usage: [100, 10, 80, 0, 0.5] },
  ]);
  expect(activity.cost).toBe(0.5);
});

test("a rung costs the dollars of the answers that wrote its word", () => {
  const activity = new Activity();
  const hear = (fact: Fact) => {
    const heard = activity.hear(fact);
    while (!heard.next().done);
  };
  hear(["rung", "rung1", "prompt1", ROOT, "", "", "claude-cli:opus/high"]);
  expect(activity.acts.get("rung1")?.cost).toBeUndefined();
  hear(["reply", "reply1", "rung1", ROOT, "claude-cli:opus/high"]);
  hear(["done", "reply1", "world", ["assistant", "close(1)", [100, 10, 80, 0, 0.5], null]]);
  hear(["reply", "reply2", "rung1", ROOT, "claude-cli:opus/high"]);
  hear(["done", "reply2", "world", ["assistant", "close(1)", [100, 10, 80, 0, 0.25], null]]);
  expect(activity.acts.get("rung1")?.cost).toBe(0.75);
});

test("the activity keeps each compaction of a chain: a prefix said on it after its birth", () => {
  const activity = new Activity();
  const hear = (fact: Fact) => {
    const heard = activity.hear(fact);
    while (!heard.next().done);
  };
  hear(["chain", "chain2", "rung1", ROOT, "work", ROOT]);
  // A chain with a source says its own prefix at its birth, which cuts nothing.
  hear(["prefix", "chain2", "chain2", [["bash", "bash1", "rung1", ROOT, "ls", "", 600]]]);
  hear(["bash", "bash2", "rung2", "chain2", "ls", "", 600]);
  hear([
    "prefix",
    "chain2",
    "rung3",
    [
      ["bash", "bash2", "rung2", "chain2", "ls", "", 600],
      ["done", "bash2", "world", { is: "Exit", code: 0 }],
    ],
  ]);
  expect(activity.compactions).toEqual([{ chain: "chain2", by: "rung3", at: 4 }]);
});

test("the activity keeps a compaction into a new chain: a chain whose first word from its compaction restores its source, which runs on from that source", () => {
  const activity = new Activity();
  const hear = (fact: Fact) => {
    const heard = activity.hear(fact);
    while (!heard.next().done);
  };
  const word = (source: string) =>
    `restore('${source}')\n<s:summary>\nIt read the plan.</s:summary>\nreground()`;
  hear(["chain", "chain20", "compacted1", ROOT, "compacted", ROOT]);
  hear(["prefix", "chain20", "chain20", []]);
  hear(["rung", "rung2", "extensions", "chain20", "remember()", "", ""]);
  hear(["rung", "rung3", "compacted1", "chain20", word(ROOT), "", ""]);
  // A word that restores another chain than the source, and a later word that restores it again, make none.
  hear(["chain", "chain21", "rung3", ROOT, "aside", ROOT]);
  hear(["rung", "rung4", "rung3", "chain21", word("chain7"), "", ""]);
  hear(["rung", "rung5", "compacted1", "chain20", word(ROOT), "", ""]);
  hear(["chain", "chain22", "compacted1", "chain20", "compacted", "chain20"]);
  hear(["rung", "rung6", "compacted1", "chain22", word("chain20"), "", ""]);
  expect(activity.compactions).toEqual([
    { chain: "chain20", from: ROOT, by: "compacted1", at: 1 },
    { chain: "chain22", from: "chain20", by: "compacted1", at: 8 },
  ]);
  expect(compactedRun("chain20", activity.compactions)).toEqual([ROOT, "chain20", "chain22"]);
  expect(compactedRun("chain21", activity.compactions)).toEqual(["chain21"]);
});

import { afterEach, expect, test } from "bun:test";
import {
  decodeRecord,
  type Ear,
  Ears,
  type Entry,
  type Fact,
  isQuestion,
  type Life,
  WorldAdapter,
  type WorldPart,
  type WorldRequest,
} from "../src/index.ts";
import { display } from "../src/world.ts";
import { bash, verb } from "./verbs.ts";

const lives: Life[] = [];
afterEach(async () => {
  for (const life of lives.splice(0)) await life.dispose();
});

/** The word of an extension of the tests, which the module of the engine runs after the engine. */
const HELLO = "def hello(name: str) -> str:\n  return f'hi {name}'\n";

/** A part of the World that reads and writes files in memory, and does the work of bash, which the test says. */
function memory(files: Map<string, string>): WorldPart {
  return {
    kinds: ["bash"],
    *hears([kind, qid, , , path, content]) {
      if (kind === "read" && isQuestion(kind, qid))
        yield [
          "done",
          qid,
          files.has(String(path))
            ? { path, content: files.get(String(path)) }
            : { is: "Refused", args: ["missing file"] },
        ];
      else if (kind === "write" && isQuestion(kind, qid)) {
        files.set(String(path), String(content));
        yield ["done", qid, { path, content }];
      }
    },
  };
}

async function open(record: unknown[] = [], answer = 'close("hello")', words: string[] = []) {
  const entries: unknown[] = [];
  const facts: unknown[] = [];
  const files = new Map<string, string>([["a", "one\ntwo\n"]]);
  // A wait of no seconds is over at once; any other is over when the test says so, and never by the clock.
  const waits: (() => void)[] = [];
  let asks = 0;
  const world = ({ kind, args }: WorldRequest): unknown => {
    switch (kind) {
      case "Stand":
        return [
          [
            ["model", ["low"], 200000],
            ["operator", [], 200000],
          ],
          "/tmp",
          "model/low",
        ];
      case "Keep":
        entries.push(args[0]);
        return null;
      case "Clock":
        return 123.5;
      case "Chance":
        return 0.25;
      case "Ask":
        asks++;
        return Promise.resolve(["assistant", answer, [20, 8, 0, 0, 0.001], null]);
      case "Wait":
        return Number(args[0]) === 0 ? null : new Promise((resolve) => waits.push(() => resolve(null)));
      case "Prompt":
        return "operator answer";
      default:
        return null;
    }
  };
  const adapter = new WorldAdapter(world, { onFacts: (batch) => facts.push(...batch) });
  adapter.parts = [memory(files)];
  const life = adapter.boot(record as Entry[], { words });
  lives.push(life);
  const release = () => {
    for (const done of waits.splice(0)) done();
  };
  return { life, entries, facts, release, files, asks: () => asks };
}

test("native queries are synchronous and acts await the real engine and async World", async () => {
  const { life, asks } = await open();
  expect(life.root).toBe("chain1");
  expect(life.clock()).toBe(123.5);
  expect(life.chance()).toBe(0.25);
  expect(await life.wait(0)).toBeNull();
  const prompt = life.prompt("str", "Say hello").id;
  expect(await life.result<string>(prompt)).toBe("hello");
  expect(asks()).toBe(1);
  expect((await life.turns()).some((turn) => turn[0] === "assistant")).toBe(true);
  expect(await life.gate("this is not python !!!")).not.toEqual([]);
});

test("a pending result leaves JavaScript and other native operations available", async () => {
  const { life, release } = await open();
  const id = life.wait(60).id;
  const pending = life.result(id);
  expect(await verb<string>(life, "cwd")).toBe("/tmp");
  expect(await life.outcome(id)).toEqual({ done: false, value: null });
  release();
  expect(await pending).toBeNull();
});

test("pause holds a model response until wake and cancel rejects a native await", async () => {
  const { life, asks } = await open();
  await life.pause(life.root);
  const id = life.prompt("str", "Say hello").id;
  // A chain the pause is not over is asked and answered meanwhile, and the paused prompt alone is not asked.
  const free = life.chain("free").id;
  expect(await life.result<string>(life.prompt("str", "Say hello", { on: free }).id)).toBe("hello");
  expect(asks()).toBe(1);
  expect((await life.outcome(id)).done).toBe(false);
  await life.wake(life.root);
  expect(await life.result<string>(id)).toBe("hello");
  expect(asks()).toBe(2);
  const later = life.wait(60).id;
  const result = life.result(later).then(
    () => "resolved",
    (error: Error) => error.message,
  );
  await life.cancel(later);
  expect(await result).toContain("CancelledError");
});

test("text and engine callables cross N-API, and a word reads what a command came to", async () => {
  const { life } = await open();
  const text = verb(life, "read", ["a"]);
  expect(text).toEqual({ is: "Text", path: "a", content: "one\ntwo\n", before: null });
  expect(() => verb(life, "read", ["missing"])).toThrow("missing file");
  const show = life.call<{ is: "made"; id: number }>("span", [1, 1], {});
  expect(await life.made<number[]>(show.id, [["one", "two"]], {})).toEqual([1]);
  await life.forget(show.id);
  const command = bash(life, "fake", { fed: true });
  await life.send("out", command, ["hello\n", "stdout"]);
  await life.send("exited", command, [0]);
  const word = `exit = await Act("${command}")\nif isinstance(exit, Exit):\n  close([exit.code, exit.stdout.content])`;
  expect(await life.result<unknown[]>(life.rung(word).id)).toEqual([0, "hello\n"]);
});

test("the World closes the start of an act whose kind no part does with why", async () => {
  const { life } = await open();
  const id = (await life.rung("done = act('job', acting(), started(ending(idle)))\nclose(done)")) as string;
  const outcome = life.outcome(id);
  expect(outcome.done).toBe(true);
  expect(outcome.value).toEqual({ is: "Refused", args: ["the World does no job"] });
});

test("a life runs the words of its extensions in the module of the engine, and a later life the words it pinned", async () => {
  const first = await open([], 'close(hello("m"))', [HELLO]);
  const program = (life: Life, chain: string) =>
    Object.values(life.call<[unknown, Record<string, string>]>("ask", ["program", chain], {})[1]);
  expect(first.life.system.endsWith(`\n\n${HELLO}`)).toBe(true);
  expect(program(first.life, first.life.root)).toEqual([]);
  expect(await first.life.result<string>(first.life.prompt("str", "Greet").id)).toBe("hi m");
  const fork = first.life.chain("fork").id;
  expect(await first.life.result<string>(first.life.rung('close(hello("fork"))', { on: fork }).id)).toBe(
    "hi fork",
  );
  const second = await open(first.entries, 'close("none")', ["other = 1\n"]);
  expect(second.life.system).toBe(first.life.system);
  expect(second.asks()).toBe(0);
  expect(second.entries).toHaveLength(0);
});

test("a second life replays model answers and durable rung effects without asking again", async () => {
  const first = await open();
  const prompt = first.life.prompt("str", "Say hello").id;
  await first.life.result<string>(prompt);
  const rung = first.life.rung('write(Text("b", "saved"))').id;
  await first.life.result(rung);
  const second = await open(first.entries);
  expect(await second.life.result<string>(prompt)).toBe("hello");
  expect(second.asks()).toBe(0);
  expect(second.files.has("b")).toBe(false);
  expect(second.entries).toHaveLength(0);
  const fork = second.life.chain("branch", second.life.root).id;
  expect(verb<string>(second.life, "cwd", [], { on: fork })).toBe("/tmp");
});

test("dispose rejects pending native results and further operations", async () => {
  const { life } = await open();
  const result = life.result(life.root).then(
    () => "resolved",
    (error: Error) => error.message,
  );
  await life.dispose();
  expect(await result).toContain("disposed");
  expect(() => life.clock()).toThrow("disposed");
});

test("map order and live Python values cross without losing their meaning", async () => {
  const { life } = await open();
  await life.rung('ordered = {"z": 1, "a": 2}\nvalue = 1.0');
  const ordered = life.inspect("ordered");
  expect(Object.keys(ordered.value as object)).toEqual(["z", "a"]);
  expect(life.inspect("value").kind).toBe("float");
});

test("every ear hears a fact whose values have no plain form, and each value crosses as what it is", async () => {
  const { life, facts, files } = await open();
  await life.rung(
    "class P:\n  pass\nd = {1: 'a'}\nn = 2**70\nf = float('-inf')\nb = b'x'\np = P()\nm = {'is': 'name', 'name': 'bash'}\nsend('note', acting(), d, n, f, b, p, m)\ndebug(t'{d}')",
  );
  await Promise.resolve();
  const [note] = facts.filter((fact) => (fact as Fact)[0] === "note") as Fact[];
  expect(note?.slice(3, 7)).toEqual([
    { is: "dict", args: [[[1, "a"]]] },
    { is: "int", args: ["1180591620717411303424"] },
    { is: "float", args: ["-inf"] },
    "b'x'",
  ]);
  expect(note?.[7]).toMatchObject({ is: "instance", class: { is: "class", name: "P" }, value: { is: "P" } });
  // A map of the word that holds the key is crosses as its pairs, and so is read as no mark.
  expect(note?.[8]).toEqual({
    is: "dict",
    args: [
      [
        ["is", "name"],
        ["name", "bash"],
      ],
    ],
  });
  expect(display(note?.[8])).toBe(JSON.stringify({ is: "name", name: "bash" }, null, 2));
  expect(
    life
      .turns()
      .map(([, python]) => python)
      .join("\n"),
  ).toContain("debugged d = {1: 'a'}");
  // The World hears on: it serves a read, a write and a wait after that fact.
  expect(verb<{ content: string }>(life, "read", ["a"]).content).toBe("one\ntwo\n");
  await life.rung('write(Text("c", "after"))');
  expect(files.get("c")).toBe("after");
  expect(await life.wait(0)).toBeNull();
});

test("a record keeps a value with no plain form, so a later life makes the same act again", async () => {
  const first = await open();
  await first.life.rung("note = act('note', '', idle, {1: 'a'}, 2**70, float('inf'))");
  const note = first.life.inspect("note").value as string;
  // The operator speaks of the act, so the record keeps the act with its words.
  first.life.close(5, note);
  expect(JSON.stringify(first.entries)).toContain(
    '"chain1",{"is":"dict","args":[[[1,"a"]]]},{"is":"int","args":["1180591620717411303424"]},{"is":"float","args":["inf"]}]',
  );
  const second = await open(first.entries.map((entry) => decodeRecord(JSON.stringify(entry))));
  expect(second.life.raised).toBeNull();
  expect(second.life.get(note)).toEqual(first.life.get(note));
});

test("a whole JavaScript number is an int, and a BigInt is one exactly", async () => {
  const { life } = await open();
  for (const [value, back] of [
    [5_000_000_000, 5_000_000_000],
    [-5_000_000_000, -5_000_000_000],
    [2n ** 60n, { is: "int", args: ["1152921504606846976"] }],
    [2n ** 63n, { is: "int", args: ["9223372036854775808"] }],
  ] as const) {
    const id = life.prompt("int", "?", { to: "operator" }).id;
    life.close(value, id);
    expect(life.outcome(id)).toEqual({ done: true, value: back });
  }
  const id = life.prompt("float", "?", { to: "operator" }).id;
  life.close(2.5, id);
  expect(life.outcome(id)).toEqual({ done: true, value: 2.5 });
  expect(() => life.close(2 ** 60, life.prompt("int", "?", { to: "operator" }).id)).toThrow("safe integer");
});

test("a life whose replay drifts is kept, with what boot raised", async () => {
  const word = "import random\nawait wait(random.random() + 1)\nclose(1)";
  const first = await open([], word);
  const prompt = first.life.prompt("int", "roll").id;
  const settled = first.life.result<number>(prompt);
  await Bun.sleep(20);
  first.release();
  expect(await settled).toBe(1);
  const second = await open(first.entries, word);
  expect(second.life.raised).toMatchObject({ is: "Drift" });
  expect(String(second.life.raised?.args[0])).toContain("drifts");
  expect(second.life.root).toBe("chain1");
  expect(second.life.clock(second.life.chain("two").id)).toBe(123.5);
});

test("every ear is given the turns of an ask as the python the chain folded", () => {
  const given: [unknown, unknown][] = [];
  const ear = (world: boolean) =>
    (function* (): Ear {
      for (;;) {
        const fact = (yield null) as Fact;
        if (world && fact?.[0] === "stand")
          yield ["done", fact[1], [[["model", ["low"], 200000]], "/tmp", "model/low"]];
      }
    })();
  const ears = new Ears({ world: ear(true), other: ear(false) });
  const callback = ears.callback;
  ears.callback = (request) => {
    const fact = request[2] as Fact | null;
    if (request[0] === "hears" && fact?.[0] === "ask") given.push([request[1], fact[5]]);
    return callback(request);
  };
  const life = ears.boot();
  try {
    life.prompt("str", "hi");
    expect(given.map(([name]) => name)).toEqual(["world", "other"]);
    expect(given[0]?.[1]).toEqual(life.turns());
    expect(given[1]?.[1]).toEqual(life.turns());
    expect(life.turns()[0]?.[1]).toContain("#prompt1 hi");
  } finally {
    life.dispose();
  }
});

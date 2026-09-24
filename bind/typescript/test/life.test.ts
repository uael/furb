import { afterEach, expect, test } from "bun:test";
import {
  decodeRecord,
  type Ear,
  Ears,
  type Entry,
  type Fact,
  type Life,
  WorldAdapter,
  type WorldRequest,
} from "../src/index.ts";
import { display } from "../src/world.ts";

const lives: Life[] = [];
afterEach(async () => {
  for (const life of lives.splice(0)) await life.dispose();
});

async function open(record: unknown[] = [], answer = 'close("hello")', dump?: Uint8Array) {
  const entries: unknown[] = [];
  const facts: unknown[] = [];
  /** The kind of every request of the World that does work, with the act it is for. */
  const work: string[] = [];
  const files = new Map<string, string>([["a", "one\ntwo\n"]]);
  // A wait of no seconds is over at once; any other is over when the test says so, and never by the clock.
  const waits: (() => void)[] = [];
  let asks = 0;
  const world = ({ kind, args }: WorldRequest): unknown => {
    if (["Ask", "Run", "Wait", "Prompt"].includes(kind))
      work.push(
        `${kind} ${kind === "Run" ? (args[0] as { id: string }).id : kind === "Wait" ? args[1] : args[0]}`,
      );
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
      case "Read":
        if (!files.has(String(args[1]))) throw new Error("missing file");
        return { path: args[1], content: files.get(String(args[1])) };
      case "Write":
        files.set(String(args[1]), String(args[2]));
        return { path: args[1], content: args[2] };
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
  const adapter = new WorldAdapter(world, (batch) => facts.push(...batch));
  const life = dump ? adapter.restore(dump, record as Entry[]) : adapter.boot(record as Entry[]);
  lives.push(life);
  const release = () => {
    for (const done of waits.splice(0)) done();
  };
  return { life, entries, facts, release, files, work, asks: () => asks };
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
  expect(await life.cwd()).toBe("/tmp");
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

test("text, command results, and engine callables cross N-API", async () => {
  const { life } = await open();
  expect(await life.read<Record<string, string>>("a")).toEqual({
    is: "Text",
    path: "a",
    content: "one\ntwo\n",
  });
  expect(() => life.read("missing")).toThrow("missing file");
  const show = await life.span(1, 1);
  expect(await life.made<number[]>(show.id, [["one", "two"]], {})).toEqual([1]);
  await life.forget(show.id);
  const command = life.bash("fake", { fed: true }).id;
  await life.send("out", command, ["hello\n", "stdout"]);
  await life.send("exited", command, [0]);
  const exit = await life.result(command);
  expect(exit).toMatchObject({ is: "Exit", code: 0, stdout: { is: "Text", content: "hello\n" } });
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
  expect(await second.life.cwd(fork)).toBe("/tmp");
});

test("dispose rejects pending native results and further operations", async () => {
  const { life } = await open();
  const result = life.result(life.root).then(
    () => "resolved",
    (error: Error) => error.message,
  );
  await life.dispose();
  expect(await result).toContain("disposed");
  expect(() => life.cwd()).toThrow("disposed");
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
  expect(life.read<{ content: string }>("a").content).toBe("one\ntwo\n");
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
  expect(second.life.cwd(second.life.chain("two").id)).toBe("/tmp");
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

test("a life dumped where it stands still is restored on the record and goes on, and its ears hear the next fact", async () => {
  const first = await open();
  await first.life.rung("k = len(read('a').lines)");
  const dump = first.life.dump();
  const record = [...first.entries];
  const second = await open(record, 'close("hello")', dump);
  expect(second.life.root).toBe(first.life.root);
  expect(second.life.inspect("k").value).toBe(2);
  expect(await second.life.rung("close(k + 1)")).toBe(3);
  expect(second.asks()).toBe(0);
  // The World of the restored life hears the first fact said after the restore, which is its first read.
  expect(second.life.read<{ content: string }>("a").content).toBe("one\ntwo\n");
  expect(second.facts.length).toBeGreaterThan(0);
});

test("a dump that does not match its record, its engine or its build is refused with each part that differs", async () => {
  const first = await open();
  await first.life.rung("k = 1");
  const dump = first.life.dump();
  const record = [...first.entries];
  const restamped = (name: string) => {
    const end = dump.indexOf("\n\n");
    const head = dump
      .subarray(0, end)
      .toString("utf8")
      .split("\n")
      .map((line) =>
        line.startsWith(`${name} `) ? `${name} ${"0".repeat(line.length - name.length - 1)}` : line,
      );
    return Buffer.concat([Buffer.from(head.join("\n")), dump.subarray(end)]);
  };
  expect(() => new WorldAdapter(() => null).restore(dump, record.slice(0, -1) as Entry[])).toThrow(
    /^Refused: the dump does not match: it is of a record of \d+ entries that end with /,
  );
  expect(() => new WorldAdapter(() => null).restore(restamped("engine"), record as Entry[])).toThrow(
    "Refused: the dump does not match: it holds another engine",
  );
  expect(() => new WorldAdapter(() => null).restore(restamped("build"), record as Entry[])).toThrow(
    "Refused: the dump does not match: another build of the crate made it",
  );
  expect(() => new WorldAdapter(() => null).restore(Buffer.from("not a dump"), [])).toThrow(
    "Refused: no dump of a life",
  );
});

test("a life whose boot raised is refused its dump, since a restore raises nothing", async () => {
  const word = "import random\nawait wait(random.random() + 1)\nclose(1)";
  const first = await open([], word);
  const settled = first.life.result<number>(first.life.prompt("int", "roll").id);
  await Bun.sleep(20);
  first.release();
  expect(await settled).toBe(1);
  const second = await open(first.entries, word);
  expect(second.life.raised).toMatchObject({ is: "Drift" });
  expect(() => second.life.dump()).toThrow(
    /^Refused: a life is dumped where it stands still, and its boot raised Drift: /,
  );
});

test("a restored life holds its pending work unstarted until a wake, as a booted life does", async () => {
  const first = await open();
  const shown = first.life.prompt("str", "Why?", { to: "operator" }).id;
  const asked = first.life.prompt("str", "Say hello").id;
  // The life ends before its World does the work it started.
  const record = [...first.entries];
  await first.life.dispose();
  const still = await open(record);
  const dump = still.life.dump();
  const lives = [await open(record), await open(record, 'close("hello")', dump)];
  for (const { life, work } of lives) {
    await Promise.resolve();
    expect(work).toEqual([]);
    life.wake(life.root);
    expect(await life.result<string>(shown)).toBe("operator answer");
    expect(await life.result<string>(asked)).toBe("hello");
  }
  const [booted, restored] = lives as [(typeof lives)[number], (typeof lives)[number]];
  expect(booted.work).toEqual(["Ask rung1", `Prompt ${shown}`]);
  expect(restored.work).toEqual(booted.work);
  expect(restored.entries).toEqual(booted.entries);
  expect(await restored.life.turns()).toEqual(await booted.life.turns());
});

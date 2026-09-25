import { afterEach, expect, test } from "bun:test";
import { type Call, decodeRecord, type Ear, Ears, type Entry, type Fact, type Life } from "../src/index.ts";
import { display } from "../src/world.ts";

const lives: Life[] = [];
afterEach(async () => {
  for (const life of lives.splice(0)) await life.dispose();
});

async function open(record: unknown[] = [], answer = 'close("hello")') {
  const entries: unknown[] = [];
  const facts: Fact[] = [];
  const files = new Map<string, string>([["a", "one\ntwo\n"]]);
  // A wait of no seconds is over at once; any other is over when the test says so, and never by the clock.
  const waits: (() => void)[] = [];
  let replies = 0;
  let life: Life | undefined;
  /** What the work of the World says once its hearing is over, said under its name. */
  const speak = (kind: string, id: string, words: unknown[]) => {
    const previous = life?.site("world") ?? "operator";
    try {
      life?.say(kind, id, words);
    } finally {
      life?.site(previous);
    }
  };
  const later = (then: () => void) => queueMicrotask(then);
  const say = (kind: string, id: string, ...words: unknown[]): Call => ({
    verb: "say",
    args: [kind, id, ...words],
  });
  // The World of the test as the one ear of the host: it keeps every fact it hears, answers what is its own at once,
  // and takes a reply, a command, a wait and a prompt to the operator with a started.
  const world = (function* (): Ear {
    for (;;) {
      const fact = (yield null) as Fact | null;
      if (!fact) continue;
      facts.push(fact);
      const [kind, id, , ...words] = fact;
      switch (kind) {
        case "stand":
          yield say("done", id, [
            [
              ["model", ["low"], 200000],
              ["operator", [], 200000],
            ],
            "/tmp",
            "model/low",
          ]);
          break;
        case "keep":
          entries.push(words[0]);
          break;
        case "clock":
          yield say("done", id, 123.5);
          break;
        case "chance":
          yield say("done", id, 0.25);
          break;
        case "read":
          yield say(
            "done",
            id,
            files.has(String(words[1]))
              ? { is: "Text", path: words[1], content: files.get(String(words[1])) }
              : { is: "Refused", args: ["missing file"] },
          );
          break;
        case "write": {
          const { path, content } = words[1] as { path: string; content: string };
          files.set(path, content);
          yield say("done", id, { is: "Text", path, content });
          break;
        }
        case "reply":
          yield say("started", id);
          replies++;
          later(() => speak("done", id, [["assistant", answer, [20, 8, 0, 0, 0.001], null]]));
          break;
        case "wait":
          yield say("started", id);
          if (Number(words[1]) === 0) later(() => speak("done", id, [null]));
          else waits.push(() => speak("done", id, [null]));
          break;
        case "prompt":
          yield say("started", id);
          // The operator answers a prompt of a string the test did not close first, and the World refuses any other.
          later(() => {
            if (life?.outcome(id).done) return;
            const previous = life?.site("world") ?? "operator";
            const shape = String(words[1]);
            life?.close(
              shape === "str" ? "operator answer" : { is: "Refused", args: [`no answer of ${shape}`] },
              id,
            );
            life?.site(previous);
          });
          break;
        case "bash":
          yield say("started", id);
          break;
      }
    }
  })();
  life = new Ears({ world }).boot(record as Entry[]);
  lives.push(life);
  const release = () => {
    for (const done of waits.splice(0)) done();
  };
  return { life, speak, entries, facts, release, files, replies: () => replies };
}

test("native queries are synchronous and acts await the real engine and async World", async () => {
  const { life, replies } = await open();
  expect(life.root).toBe("chain1");
  expect(life.clock()).toBe(123.5);
  expect(life.chance()).toBe(0.25);
  expect(await life.wait(0)).toBeNull();
  const prompt = life.prompt("str", "Say hello").id;
  expect(await life.result<string>(prompt)).toBe("hello");
  expect(replies()).toBe(1);
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
  const { life, replies } = await open();
  await life.pause(life.root);
  const id = life.prompt("str", "Say hello").id;
  // A chain the pause is not over is asked and answered meanwhile, and the paused prompt alone is not asked.
  const free = life.chain("free").id;
  expect(await life.result<string>(life.prompt("str", "Say hello", { on: free }).id)).toBe("hello");
  expect(replies()).toBe(1);
  expect((await life.outcome(id)).done).toBe(false);
  await life.wake(life.root);
  expect(await life.result<string>(id)).toBe("hello");
  expect(replies()).toBe(2);
  const later = life.wait(60).id;
  const result = life.result(later).then(
    () => "resolved",
    (error: Error) => error.message,
  );
  await life.cancel(later);
  expect(await result).toContain("CancelledError");
});

test("text, command results, and engine callables cross N-API", async () => {
  const { life, speak } = await open();
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
  speak("out", command, ["hello\n", "stdout"]);
  const stream = (name: string, content: string) => ({ is: "Text", path: `${command}/${name}`, content });
  speak("done", command, [
    { is: "Exit", code: 0, stdout: stream("stdout", "hello\n"), stderr: stream("stderr", "") },
  ]);
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
  expect(second.replies()).toBe(0);
  expect(second.files.has("b")).toBe(false);
  // Every life stands as it opens, and that stand is all the second life keeps.
  expect(second.entries.map((entry) => (entry as [Fact])[0][0])).toEqual(["stand", "done"]);
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
    "class P:\n  pass\nd = {1: 'a'}\nn = 2**70\nf = float('-inf')\nb = b'x'\np = P()\nm = {'is': 'name', 'name': 'bash'}\nsay('note', acting(), d, n, f, b, p, m)\ndebug(t'{d}')",
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
  await first.life.rung(
    "def takes(id):\n  say('started', id)\n  while True:\n    yield\nnote = act('note', '', takes, {1: 'a'}, 2**70, float('inf'))",
  );
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

test("the World reads the turns of a chain when it takes a reply, which carries none", () => {
  const read: unknown[] = [];
  const world = (function* (): Ear {
    for (;;) {
      const fact = (yield null) as Fact;
      if (fact?.[0] === "stand")
        yield { verb: "say", args: ["done", fact[1], [[["model", ["low"], 200000]], "/tmp", "model/low"]] };
      if (fact?.[0] === "reply") {
        yield { verb: "say", args: ["started", fact[1]] };
        read.push(fact, yield { verb: "turns", kwargs: { on: fact[3] } });
      }
    }
  })();
  const life = new Ears({ world }).boot();
  try {
    life.prompt("str", "hi");
    const [reply, turns] = read;
    expect((reply as Fact).slice(3)).toEqual([life.root, "model/low"]);
    expect(turns).toEqual(life.turns());
    expect(life.turns()[0]?.[1]).toContain("#prompt1 hi");
  } finally {
    life.dispose();
  }
});

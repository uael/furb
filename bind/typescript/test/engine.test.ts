import { afterEach, expect, test } from "bun:test";
import {
  bound,
  decodeRecord,
  display,
  type Ear,
  Engine,
  type Fact,
  files,
  isQuestion,
  paragraphs,
  plain,
  speaking,
} from "../src/index.ts";

// Each test of this file is serial, since this hook disposes the engines of every test that runs.
const engines: Engine[] = [];
afterEach(() => {
  for (const engine of engines.splice(0)) engine.dispose();
});

/** An engine on one ear of the test, which serves every question of the World from memory: a wait of no seconds is
 * over at once, and any other is over when the test says so, never by the clock. */
function open(record: unknown[] = []) {
  const entries: unknown[] = [];
  const facts: Fact[] = [];
  const files = new Map<string, string>([["a", "one\ntwo\n"]]);
  const waits: (() => void)[] = [];
  // Work of the World ends later, under its name, as the work an ear began does.
  const later = (action: () => void) => queueMicrotask(() => speaking(engine, "world", action));
  function* world(): Ear {
    for (;;) {
      const fact = (yield null) as Fact | undefined;
      if (!fact) continue;
      facts.push(fact);
      const [kind, id, , ...words] = fact;
      if (kind === "keep") entries.push(words[0]);
      if (!isQuestion(kind, id)) continue;
      if (kind === "stand")
        yield [
          "done",
          id,
          [
            [
              ["model", ["low"], 200000],
              ["operator", [], 200000],
            ],
            "/tmp",
            "model/low",
          ],
        ];
      else if (kind === "read") {
        const path = String(words[1]);
        const content = files.get(path);
        yield [
          "done",
          id,
          content === undefined ? { is: "Refused", args: ["missing file"] } : { is: "Text", path, content },
        ];
      } else if (kind === "write") {
        const text = words[1] as { path: string; content: string };
        files.set(text.path, text.content);
        yield ["done", id, { is: "Text", ...text }];
      } else if (kind === "wait") {
        yield ["started", id];
        const over = () => later(() => engine.say("done", id, [null]));
        if (Number(words[1]) === 0) over();
        else waits.push(over);
      } else if (kind === "thread") yield ["started", id];
    }
  }
  const engine = Engine.boot(record, [["world", world()]]);
  engines.push(engine);
  const release = () => {
    for (const done of waits.splice(0)) done();
  };
  return { engine, entries, facts, release, files };
}

test.serial("a pending result leaves JavaScript and other native operations available", async () => {
  const { engine, release } = open();
  const on = engine.root;
  const id = engine.wait({ seconds: 60, on }).id;
  const pending = engine.result(id);
  expect(engine.cwd({ on })).toBe("/tmp");
  expect(engine.outcome(id)).toEqual({ done: false, value: null });
  release();
  expect(await pending).toBeNull();
});

test.serial("a cancel rejects a native await", async () => {
  const { engine } = open();
  const later = engine.wait({ seconds: 60, on: engine.root }).id;
  const result = engine.result(later).then(
    () => "resolved",
    (error: Error) => error.message,
  );
  engine.cancel(later);
  expect(await result).toContain("CancelledError");
});

test.serial(
  "a function of JavaScript crosses as a filter, and what it throws is raised where the word called it",
  () => {
    const { engine } = open();
    let calls = 0;
    const counting = (acts: unknown[]) => {
      calls++;
      return acts;
    };
    engine.chain({ label: "fork", source: engine.root, filter: counting });
    expect(calls).toBe(1);
    const refusing = () => {
      throw new Error("no lines");
    };
    expect(() => engine.chain({ label: "fork", source: engine.root, filter: refusing })).toThrow("no lines");
  },
);

test.serial("dispose rejects pending native results and further operations", async () => {
  const { engine } = open();
  const result = engine.result(engine.root).then(
    () => "resolved",
    (error: Error) => error.message,
  );
  engine.dispose();
  expect(await result).toContain("disposed");
  expect(() => engine.cwd({ on: "chain1" })).toThrow("disposed");
});

test.serial("map order and live Python values cross without losing their meaning", async () => {
  const { engine } = open();
  await engine.rung({ word: 'ordered = {"z": 1, "a": 2}\nvalue = 1.0', on: engine.root });
  const ordered = engine.inspect("ordered");
  expect(Object.keys(ordered.value as object)).toEqual(["z", "a"]);
  expect(engine.inspect("value").kind).toBe("float");
});

test.serial(
  "every ear hears a fact whose values have no plain form, and each value crosses as what it is",
  async () => {
    const { engine, facts, files } = open();
    const on = engine.root;
    await engine.rung({
      on,
      word: "class P:\n  pass\nd = {1: 'a'}\nn = 2**70\nf = float('-inf')\nb = b'x'\np = P()\nm = {'is': 'name', 'name': 'bash'}\nsay('note', acting(), d, n, f, b, p, m)",
    });
    const [note] = facts.filter((fact) => fact[0] === "note");
    expect(note?.slice(3, 7)).toEqual([
      { is: "dict", args: [[[1, "a"]]] },
      { is: "int", args: ["1180591620717411303424"] },
      { is: "float", args: ["-inf"] },
      "b'x'",
    ]);
    expect(note?.[7]).toMatchObject({
      is: "instance",
      class: { is: "class", name: "P" },
      value: { is: "P" },
    });
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
    // The World hears on: it serves a read, a write and a wait after that fact.
    expect(engine.read("a", { on }).content).toBe("one\ntwo\n");
    await engine.rung({ word: 'write(Text("c", "after"))', on });
    expect(files.get("c")).toBe("after");
    expect(await engine.wait({ seconds: 0, on })).toBeNull();
  },
);

test.serial("a record keeps a value with no plain form, so a later life makes the same act again", () => {
  const first = open();
  first.engine.rung({
    on: first.engine.root,
    word: "def takes(id):\n  yield 'started', id\n  while True:\n    yield\nnote = act('note', '', takes, {1: 'a'}, 2**70, float('inf'))",
  });
  const note = first.engine.inspect("note").value as string;
  // The operator speaks of the act, so the record keeps the act with its words.
  first.engine.close(5, { id: note });
  expect(JSON.stringify(first.entries)).toContain(
    '"chain1",{"is":"dict","args":[[[1,"a"]]]},{"is":"int","args":["1180591620717411303424"]},{"is":"float","args":["inf"]}]',
  );
  const second = open(first.entries.map((entry) => decodeRecord(JSON.stringify(entry))));
  expect(second.engine.raised).toBeNull();
  expect(second.engine.get(note)).toEqual(first.engine.get(note));
});

test.serial("a whole JavaScript number is an int, and a BigInt is one exactly", () => {
  const { engine } = open();
  const asked = (shape: string) =>
    engine.thread(shape, { markdown: "?", to: "operator", on: engine.root }).id;
  for (const [value, back] of [
    [5_000_000_000, 5_000_000_000],
    [-5_000_000_000, -5_000_000_000],
    [2n ** 60n, { is: "int", args: ["1152921504606846976"] }],
    [2n ** 63n, { is: "int", args: ["9223372036854775808"] }],
  ] as const) {
    const id = asked("int");
    engine.close(value, { id });
    expect(engine.outcome(id)).toEqual({ done: true, value: back });
  }
  const id = asked("float");
  engine.close(2.5, { id });
  expect(engine.outcome(id)).toEqual({ done: true, value: 2.5 });
  expect(() => engine.close(2 ** 60, { id: asked("int") })).toThrow("safe integer");
});

test.serial("a life whose replay drifts is kept, with what boot raised", () => {
  const first = open();
  // No verb makes an act of this kind again, so a later life drifts at its entry.
  first.engine.act("note", first.engine.root, null);
  const second = open(first.entries);
  expect(second.engine.raised).toMatchObject({ is: "Drift" });
  expect(String(second.engine.raised?.args[0])).toContain("drifts");
  expect(second.engine.root).toBe("chain1");
  expect(second.engine.cwd({ on: second.engine.chain({ label: "two" }).id })).toBe("/tmp");
});

test.serial("an ear that throws raises in the life, and an ear of the crate hears in one engine", () => {
  function* world(): Ear {
    for (;;) {
      const fact = (yield null) as Fact | undefined;
      if (fact?.[0] === "stand") yield ["done", fact[1], [[["operator", [], 200000]], "/tmp", "operator"]];
      if (fact?.[0] === "clock") throw { is: "Refused", args: ["no clock here"] };
    }
  }
  const once = files();
  const engine = Engine.boot(
    [],
    [
      ["world", world()],
      ["files", once],
    ],
  );
  engines.push(engine);
  expect(() => engine.clock({ on: engine.root })).toThrow("no clock here");
  expect(() => Engine.boot([], [["files", once]])).toThrow("hears in one engine");
});

test.serial(
  "a function of JavaScript that makes an ear gives the ear that an act brings to life, under the name of the act",
  async () => {
    const { engine } = open();
    const heard: unknown[] = [];
    function* noting(name: string): Ear {
      heard.push(name);
      yield ["started", name];
      for (;;) {
        const fact = (yield null) as Fact | undefined;
        // The owner of the act answers a close of it with what the close gave.
        if (fact?.[0] === "close" && fact[1] === name) yield ["done", name, fact[3]];
      }
    }
    const note = engine.act("note", engine.root, (name: string) => noting(name), ["one"]);
    expect([String(note), heard]).toEqual(["note1", ["note1"]]);
    engine.close(7, { id: String(note) });
    expect(await note).toBe(7);
  },
);

test.serial(
  "the text of a paragraph drops the two marks of each quote, and a binding reads back the string it binds",
  () => {
    const [told] = paragraphs(
      "#thread1\n<s:thread1_message>\ncount\n# them\n</s:thread1_message>\nthread1: Act[int] = Act('thread1')",
    );
    expect(told?.name).toBe("thread1");
    expect(plain(told?.lines ?? [])).toBe("count\n# them\nthread1: Act[int] = Act('thread1')");
    expect(plain(["x = 1", "<s:a>", "b</s:a>", "c</s:a>"])).toBe("x = 1\nb\nc</s:a>");
    expect(bound(told?.lines ?? [], "thread1_message")).toBe("count\n# them\n");
    expect(bound(["<s:a__>", "b</s:a_>", "c</s:a__>"], "a")).toBe("b</s:a_>\nc");
    expect(bound(["x = 1", "a = 'it\\'s\\n\\x07\\u00e9\\\\'", 'b = "it\'s"'], "a")).toBe("it's\n\x07é\\");
    expect(bound(['b = "it\'s"'], "b")).toBe("it's");
    expect(bound(["x = 1"], "x")).toBeUndefined();
  },
);

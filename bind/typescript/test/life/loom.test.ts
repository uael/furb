import { expect, test } from "bun:test";
import type { LiveAct } from "../../src/life/acts.ts";
import { measureOf } from "../../src/life/acts.ts";
import { condense, ladder, makers, thread } from "../../src/life/ladder.ts";
import {
  asOf,
  commandDoing,
  doingOf,
  duration,
  momentsOf,
  nowOf,
  owedAt,
  spans,
  subjectOf,
  weave,
} from "../../src/life/loom.ts";
import { act } from "./acts.ts";

const loomOf = (
  acts: LiveAct[],
  options: { folds?: string[]; prompts?: { id: string; shape: string; message: string }[] } = {},
) =>
  weave({
    acts,
    prompts: options.prompts ?? [],
    measures: {},
    folds: new Set(options.folds ?? []),
    width: 40,
    pending: new Map(),
    label: (id) => (id === "chain1" ? "conduct" : `w ${id}`),
  });

test("the newest moments take a column each, and older moments share a column, so a long life fits its width", () => {
  const moments = Array.from({ length: 300 }, (_, index) => index + 1);
  const times = spans(moments, 40);
  expect(times).toHaveLength(40);
  expect(times.at(-1)).toEqual([300, 300]);
  expect(times.at(-10)).toEqual([291, 291]);
  expect(times[0]?.[0]).toBe(1);
  // Each span begins after the one before it ends, so no moment is lost and none counts twice.
  for (let index = 1; index < times.length; index++)
    expect(times[index]?.[0]).toBe((times[index - 1]?.[1] ?? 0) + 1);
  expect(spans([1, 2, 3], 40)).toEqual([
    [1, 1],
    [2, 2],
    [3, 3],
  ]);
});

test("a chain whose model answers and a chain whose command runs both show work that runs at the present", () => {
  const acts = [
    act("chain1", "chain", "", "operator", 1),
    act("rung1", "rung", "chain1", "operator", 2),
    act("chain2", "chain", "", "rung1", 3),
    act("chain3", "chain", "", "rung1", 4),
    act("chain4", "chain", "", "rung1", 5),
    act("prompt1", "prompt", "chain2", "rung1", 6),
    // The rung of the prompt waits for the turn of its model, whose reply is in flight.
    act("rung2", "rung", "chain2", "prompt1", 7),
    act("prompt2", "prompt", "chain3", "rung1", 8),
    act("rung3", "rung", "chain3", "prompt2", 9),
    act("bash1", "bash", "chain3", "rung3", 10),
    act("prompt3", "prompt", "chain4", "rung1", 11, 14),
    act("rung4", "rung", "chain4", "prompt3", 12, 13),
    act("read1", "read", "chain3", "rung3", 15, 15),
  ];
  const { rows } = loomOf(acts);
  const of = (id: string) => rows.find((thread) => thread.id === id);
  expect(of("chain2")?.state).toBe("working");
  expect(of("chain3")?.state).toBe("working");
  expect(of("chain3")?.waits).toBe("bash1");
  expect(of("chain2")?.waits).toBe("rung2");
  expect(of("chain2")?.cells.at(-1)).toEqual({ mark: "▁", tone: "work" });
  expect(of("chain3")?.cells.at(-1)).toEqual({ mark: "▃", tone: "work" });
  expect(of("chain4")?.state).toBe("done");
  expect(of("chain4")?.cells.at(-1)).toEqual({ mark: " ", tone: "none" });
});

test("the loom stands each worker under the chain whose rung made it, and a fold counts the workers by state", () => {
  const acts = [
    act("chain1", "chain", "", "operator", 1),
    act("rung1", "rung", "chain1", "operator", 2),
    act("chain2", "chain", "", "rung1", 3),
    act("chain3", "chain", "", "rung1", 4),
    act("prompt1", "prompt", "chain2", "rung1", 5),
    act("prompt2", "prompt", "chain3", "rung1", 6, 7),
    act("chain5", "chain", "", "operator", 8),
  ];
  const open = loomOf(acts);
  expect(open.rows.map((thread) => [thread.lines, thread.label])).toEqual([
    ["", "you"],
    ["", "conduct"],
    ["├ ", "w chain2"],
    ["└ ", "w chain3"],
    ["", "w chain5"],
  ]);
  const folded = loomOf(acts, { folds: ["chain1"] });
  expect(folded.rows.map((thread) => thread.id)).toEqual(["operator", "chain1", "chain5"]);
  expect(folded.rows[1]).toMatchObject({ folded: true, workers: 2, tally: { working: 1, done: 1 } });
});

test("the operator owes the prompts that wait for it, the oldest first, and no prompt whose act is done, and its thread and the thread of each asks", () => {
  const acts = [
    act("chain1", "chain", "", "operator", 1),
    act("rung1", "rung", "chain1", "operator", 2),
    act("chain2", "chain", "", "rung1", 3),
    act("prompt3", "prompt", "chain1", "rung1", 4, 5, { words: ["str", "Which name?", "operator"] }),
    act("prompt1", "prompt", "chain2", "rung1", 6, undefined, {
      words: ["str", "Which effort?", "operator"],
    }),
    act("prompt2", "prompt", "chain1", "rung1", 7, undefined, { words: ["bool", "Merge?", "operator"] }),
  ];
  const { owed, rows } = loomOf(acts, {
    prompts: [
      { id: "prompt3", shape: "str", message: "Which name?" },
      { id: "prompt2", shape: "bool", message: "Merge?" },
      { id: "prompt1", shape: "str", message: "Which effort?" },
    ],
  });
  expect(owed.map((prompt) => [prompt.id, prompt.chain])).toEqual([
    ["prompt1", "chain2"],
    ["prompt2", "chain1"],
  ]);
  expect(rows.map((thread) => thread.state)).toEqual(["asks", "asks", "asks"]);
  expect(rows[0]?.cells.at(-1)?.tone).toBe("asks");
});

test("a thread shows each prompt with the rungs of its ladder, the acts of each rung as chips, and what the prompt closed with", () => {
  const acts = [
    act("chain1", "chain", "", "operator", 1),
    act("prompt1", "prompt", "chain1", "operator", 2, 12, { words: ["str", "Explore", ""], value: "Done." }),
    act("rung1", "rung", "chain1", "prompt1", 3, 11),
    act("read1", "read", "chain1", "rung1", 4, 4),
    act("read2", "read", "chain1", "rung1", 5, 5),
    act("read3", "read", "chain1", "rung1", 6, 6),
    act("bash1", "bash", "chain1", "rung1", 7, 9),
    act("rung2", "rung", "chain1", "operator", 13, undefined, { words: ["await wait(60)"] }),
    act("bash2", "bash", "chain1", "rung2", 14, undefined, {
      words: ["make"],
      value: { stdout: { content: "one\ntwo\nthree\nfour\n" }, stderr: { content: "" } },
    }),
  ];
  const words: Record<string, string> = {
    rung1: 'notes = read("README.md")\nclose("Done.")',
    rung2: "await wait(60)",
  };
  const lines = ladder("chain1", acts, (rung) => words[rung.id] ?? "", new Map(), new Set());
  expect(
    lines.map((line) => [
      line.type,
      line.act.id,
      line.depth,
      line.type === "rung"
        ? `${line.word} +${line.more} ${line.acts.map((chip) => chip.label).join(" ")}`
        : "",
      line.type === "tail" || line.type === "value" ? line.text : "",
    ]),
  ).toEqual([
    ["prompt", "prompt1", 0, "", ""],
    ["rung", "rung1", 1, 'notes = read("README.md") +1 read1…3 bash1', ""],
    ["value", "prompt1", 1, "", "Done."],
    ["rung", "rung2", 0, "await wait(60) +0 bash2", ""],
    ["tail", "bash2", 1, "", "two\nthree\nfour"],
  ]);
});

test("the rungs in a row that an ear made on a chain stand on one line, with the acts that they made", () => {
  const acts = [
    act("chain1", "chain", "", "operator", 1),
    act("rung1", "rung", "chain1", "extensions", 2, 3),
    act("remember1", "remember", "chain1", "rung1", 3),
    act("rung2", "rung", "chain1", "extensions", 4, 5),
    act("rung3", "rung", "chain1", "operator", 6, 7, { words: ["x = 1"] }),
  ];
  const lines = ladder("chain1", acts, (rung) => String(rung.words[0] ?? ""), new Map(), new Set());
  expect(
    lines.map((line) => [
      line.type,
      line.act.id,
      line.type === "ear" ? line.rungs.map((rung) => rung.id) : [],
    ]),
  ).toEqual([
    ["ear", "rung1", ["rung1", "rung2"]],
    ["rung", "rung3", []],
  ]);
  expect(lines[0]?.type === "ear" && lines[0].acts.map((chip) => [chip.label, chip.state])).toEqual([
    ["remember1", "stands"],
  ]);
});

test("a ladder that reads the index of a host gives the lines that it gives from every act", () => {
  const acts = [
    act("chain1", "chain", "", "operator", 1),
    act("prompt1", "prompt", "chain1", "operator", 2, 12, { words: ["str", "Explore", ""], value: "Done." }),
    act("rung1", "rung", "chain1", "prompt1", 3, 11),
    act("read1", "read", "chain1", "rung1", 4, 4),
    act("chain2", "chain", "", "rung1", 5),
    act("prompt2", "prompt", "chain2", "rung1", 6, undefined, { words: ["str", "Look", ""] }),
    act("rung2", "rung", "chain2", "prompt2", 7),
    act("rung3", "rung", "chain1", "extensions", 8, 9),
    act("bash1", "bash", "chain1", "rung3", 9, undefined, {
      words: ["make"],
      value: { stdout: { content: "one\n" }, stderr: { content: "" } },
    }),
  ];
  for (const chain of ["chain1", "chain2"]) {
    const index = {
      on: acts.filter((one) => one.on === chain && one.kind !== "chain"),
      made: makers(acts),
      ids: new Map(acts.map((one) => [one.id, one])),
    };
    const word = (rung: LiveAct) => rung.id;
    expect(ladder(chain, acts, word, new Map(), new Set(["prompt2"]), index)).toEqual(
      ladder(chain, acts, word, new Map(), new Set(["prompt2"])),
    );
  }
});

test("a condensed thread read from the index of a host gives the lines that condense gives of its ladder", () => {
  // A life of many steps, made by a fixed sequence of choices: prompts with long ladders, rungs that ask prompts on the
  // chain and run commands, rungs of an ear in a row, and work that runs or asks the operator, old and new.
  let seed = 7;
  const next = (n: number) => {
    seed = (seed * 1103515245 + 12345) % 2147483648;
    return seed % n;
  };
  const acts: LiveAct[] = [act("chain1", "chain", "", "operator", 1)];
  const add = (one: LiveAct) => {
    acts.push(one);
    return one;
  };
  const open = () => (next(9) ? acts.length + 1 : undefined);
  const nest = (prompt: LiveAct, depth: number) => {
    for (let rung = next(12); rung >= 0; rung--) {
      const made = add(act(`rung${acts.length}`, "rung", "chain1", prompt.id, acts.length, open()));
      if (!next(4))
        add(
          act(`bash${acts.length}`, "bash", "chain1", made.id, acts.length, open(), {
            value: { stdout: { content: "one\ntwo\n" }, stderr: { content: "" } },
          }),
        );
      if (!next(3)) add(act(`read${acts.length}`, "read", "chain1", made.id, acts.length, acts.length));
      if (depth < 2 && !next(5))
        nest(
          add(
            act(`prompt${acts.length}`, "prompt", "chain1", made.id, acts.length, open(), {
              words: ["str", "Look", next(4) ? "" : "operator"],
            }),
          ),
          depth + 1,
        );
    }
  };
  for (let top = 0; top < 60; top++) {
    if (!next(6)) {
      for (let ear = next(3); ear >= 0; ear--)
        add(act(`rung${acts.length}`, "rung", "chain1", "extensions", acts.length, acts.length));
      continue;
    }
    const prompt = add(
      act(`prompt${acts.length}`, "prompt", "chain1", "operator", acts.length, open(), {
        words: ["str", "Step", ""],
        value: "Done.",
      }),
    );
    nest(prompt, 0);
  }
  const owed = new Set(
    acts.filter((one) => one.kind === "prompt" && one.words[2] === "operator").map((one) => one.id),
  );
  const index = {
    on: acts.filter((one) => one.on === "chain1" && one.kind !== "chain"),
    made: makers(acts),
    ids: new Map(acts.map((one) => [one.id, one])),
    live: acts.filter((one) => !one.done || one.run?.status === "running"),
  };
  const word = (rung: LiveAct) => `${rung.id}\nmore`;
  const keys = acts.flatMap((one) => [`rungs:${one.id}`, `prompts:${one.id}`]);
  for (const opened of [[], ["prompts:chain1"], keys.filter(() => !next(3))]) {
    const lines = condense(ladder("chain1", acts, word, new Map(), owed, index), new Set(opened));
    if (!opened.length) expect(lines.some((line) => line.type === "more")).toBe(true);
    expect(thread("chain1", word, new Map(), owed, new Set(opened), index)).toEqual(lines);
  }
  // A host that stamps the changes of what each act made has its thread read again only where an act changed.
  const stamps = new Map<string, number>();
  const stamped = { ...index, stamp: (id: string) => stamps.get(id) ?? 0 };
  const read = () => thread("chain1", word, new Map(), owed, new Set(), stamped);
  expect(read()).toEqual(condense(ladder("chain1", acts, word, new Map(), owed, index), new Set()));
  for (const [place, one] of acts.entries())
    if (one.kind === "bash" && !one.done) {
      const done = { ...one, done: true, ended: acts.length + 1 };
      acts[place] = done;
      index.ids.set(done.id, done);
      const made = index.made.get(done.by) ?? [];
      made[made.indexOf(one)] = done;
      stamps.set(done.by, (stamps.get(done.by) ?? 0) + 1);
    }
  index.live = acts.filter((one) => !one.done || one.run?.status === "running");
  expect(read()).toEqual(condense(ladder("chain1", acts, word, new Map(), owed, index), new Set()));
});

test("a loom of no width read from the index of a host gives the rows that it gives from every act", () => {
  const acts: LiveAct[] = [act("chain1", "chain", "", "operator", 1)];
  const states: Partial<LiveAct>[] = [
    {},
    { paused: true },
    { value: { is: "ValueError", args: ["no"] } },
    { value: { is: "CancelledError", args: [] } },
  ];
  for (let chain = 2; chain <= 9; chain++) {
    const maker = `rung${10 * chain}`;
    acts.push(act(maker, "rung", "chain1", "operator", acts.length + 1, acts.length + 1));
    acts.push(act(`chain${chain}`, "chain", "", maker, acts.length + 1));
    acts.push(
      act(
        `prompt${chain}`,
        "prompt",
        `chain${chain}`,
        maker,
        acts.length + 1,
        chain % 3 ? acts.length + 9 : undefined,
        {
          words: ["str", "Work", chain === 4 ? "operator" : ""],
        },
      ),
    );
    for (let step = 0; step < chain; step++) {
      const ended = (chain + step) % 4 ? acts.length + 2 : undefined;
      acts.push(
        act(`bash${10 * chain + step}`, "bash", `chain${chain}`, `prompt${chain}`, acts.length + 1, ended, {
          ...(ended === undefined ? {} : states[(chain * step) % states.length]),
          ...(step === 1 && chain === 6 ? { paused: true } : {}),
        }),
      );
    }
  }
  const prompts = [{ id: "prompt4", shape: "str", message: "Work" }];
  const pending = new Map([["bash73", "chain7"]]);
  const index = {
    ids: new Map(acts.map((one) => [one.id, one])),
    on: Map.groupBy(acts, (one) => one.on),
    chains: acts.filter((one) => one.kind === "chain"),
    live: acts.filter((one) => !one.done),
  };
  const base = { acts, prompts, measures: {}, width: 0, pending, label: (id: string) => id };
  for (const folds of [new Set<string>(), new Set(["chain1"])]) {
    const whole = weave({ ...base, folds });
    if (!folds.size) expect(new Set(whole.rows.map((row) => row.state)).size).toBeGreaterThan(3);
    expect(weave({ ...base, folds, index })).toEqual(whole);
  }
  // A host that stamps the changes of the acts on each chain has the board read again only the chains that changed.
  const stamps = new Map<string, number>();
  const stamped = { ...index, stamp: (chain: string) => stamps.get(chain) ?? 0 };
  expect(weave({ ...base, folds: new Set(), index: stamped })).toEqual(weave({ ...base, folds: new Set() }));
  for (const [place, one] of acts.entries())
    if (one.kind === "bash" && !one.done && one.on !== "chain7") {
      const done = { ...one, done: true, ended: acts.length + 1 };
      acts[place] = done;
      index.ids.set(done.id, done);
      const list = index.on.get(done.on) ?? [];
      list[list.indexOf(one)] = done;
      stamps.set(done.on, (stamps.get(done.on) ?? 0) + 1);
    }
  index.live = acts.filter((one) => !one.done);
  stamped.live = index.live;
  expect(weave({ ...base, folds: new Set(), index: stamped })).toEqual(weave({ ...base, folds: new Set() }));
});

test("the acts as they stood at a moment hold what began by then, and an act that ended later runs, with no value yet", () => {
  const acts = [
    act("chain1", "chain", "", "operator", 1),
    act("prompt1", "prompt", "chain1", "rung1", 2, 9, {
      words: ["str", "Which?", "operator"],
      value: "this",
    }),
    act("rung2", "rung", "chain1", "operator", 3, 5),
    act("bash1", "bash", "chain1", "rung2", 4, 8, {
      value: { stdout: { content: "all\n" }, stderr: { content: "" } },
    }),
    act("read1", "read", "chain1", "rung2", 7, 7),
  ];
  expect(momentsOf(acts)).toEqual([1, 2, 3, 4, 5, 7, 8, 9]);
  const then = asOf(acts, 5);
  expect(then.map((one) => [one.id, one.done])).toEqual([
    ["chain1", false],
    ["prompt1", false],
    ["rung2", true],
    ["bash1", false],
  ]);
  expect(then.find((one) => one.id === "prompt1")?.value).toBeNull();
  expect(then.find((one) => one.id === "bash1")?.value).toMatchObject({
    code: null,
    stdout: { content: "" },
  });
  expect(owedAt(then)).toEqual([{ id: "prompt1", shape: "str", message: "Which?" }]);
  expect(owedAt(asOf(acts, 9))).toEqual([]);
});

test("a chain that asks the operator says its first question now, and counts the others", () => {
  const acts = [
    act("chain1", "chain", "", "operator", 1),
    act("prompt1", "prompt", "chain1", "operator", 2, undefined, {
      words: ["bool", "Apply it?\nIt is safe.", "operator"],
    }),
    act("prompt2", "prompt", "chain1", "operator", 3, undefined, {
      words: ["str", "Which name?", "operator"],
    }),
  ];
  const loom = loomOf(acts, {
    prompts: [
      { id: "prompt1", shape: "bool", message: "Apply it?\nIt is safe." },
      { id: "prompt2", shape: "str", message: "Which name?" },
    ],
  });
  const row = loom.rows.find((one) => one.id === "chain1");
  expect(row && nowOf(row, loom, acts, () => "")).toEqual({
    tone: "asks",
    act: "prompt1",
    more: 1,
    text: "Apply it?",
  });
});

test("a chain at work says the act that it waits on, and a chain at rest says nothing", () => {
  const acts = [
    act("chain1", "chain", "", "operator", 1),
    act("chain2", "chain", "", "operator", 2),
    act("prompt1", "prompt", "chain1", "operator", 3),
    act("rung1", "rung", "chain1", "prompt1", 4),
    act("bash1", "bash", "chain1", "rung1", 5, undefined, { words: ["\nuv run pytest -q\necho done"] }),
  ];
  const loom = loomOf(acts);
  const of = (id: string) => {
    const row = loom.rows.find((one) => one.id === id);
    return row && nowOf(row, loom, acts, () => "");
  };
  expect(of("chain1")).toEqual({ tone: "work", act: "bash1", more: 0, text: "uv run pytest -q" });
  expect(of("chain2")).toEqual({ tone: "none", more: 0, text: "" });
});

test("a chain that failed says why, and a chain whose prompts closed says the first line of its last answer", () => {
  const acts = [
    act("chain1", "chain", "", "operator", 1),
    act("prompt1", "prompt", "chain1", "operator", 2, 5),
    act("rung1", "rung", "chain1", "prompt1", 3, 4, {
      run: { status: "failed", reason: "NameError: BAD\nline 2" },
    }),
    act("chain2", "chain", "", "operator", 6),
    act("prompt2", "prompt", "chain2", "operator", 7, 9, { value: "\n## The **tests** pass.\nAll of them." }),
    act("rung2", "rung", "chain2", "prompt2", 8, 9),
  ];
  const loom = loomOf(acts);
  const of = (id: string) => {
    const row = loom.rows.find((one) => one.id === id);
    return row && nowOf(row, loom, acts, () => "");
  };
  expect(of("chain1")).toEqual({ tone: "failed", act: "rung1", more: 0, text: "NameError: BAD" });
  expect(of("chain2")).toEqual({ tone: "past", act: "prompt2", more: 0, text: "The tests pass." });
});

test("an act says what it is about on one line: the word of a rung, the message of a prompt, the time and the acts of a wait", () => {
  const word = (rung: LiveAct) => (rung.id === "rung1" ? "\nx = 1\ny = 2" : "");
  expect(subjectOf(act("rung1", "rung", "chain1", "operator", 1), word)).toBe("x = 1");
  expect(
    subjectOf(
      act("prompt1", "prompt", "chain1", "operator", 1, undefined, { words: ["str", "Go on.\nNow."] }),
      word,
    ),
  ).toBe("Go on.");
  expect(subjectOf(act("wait1", "wait", "chain1", "operator", 1, undefined, { words: [30] }), word)).toBe(
    "up to 30s",
  );
  expect(
    subjectOf(
      act("wait2", "wait", "chain1", "operator", 1, undefined, { words: [600, "prompt3", "bash2"] }),
      word,
    ),
  ).toBe("up to 10m for prompt3, bash2");
});

test("a long thread folds its earlier steps into one line of more, and keeps a prompt that asks the operator", () => {
  const acts = [act("chain1", "chain", "", "operator", 1)];
  for (let n = 1; n <= 10; n++)
    acts.push(
      act(`prompt${n}`, "prompt", "chain1", "operator", 10 * n, 10 * n + 5, {
        words: ["str", `step ${n}`, ""],
      }),
    );
  // The second prompt asks the operator and waits for an answer.
  acts[2] = act("prompt2", "prompt", "chain1", "operator", 20, undefined, {
    words: ["str", "step 2", "operator"],
  });
  const lines = condense(
    ladder("chain1", acts, () => "", new Map(), new Set(["prompt2"])),
    new Set(),
  );
  expect(lines.map((line) => (line.type === "more" ? `more ${line.count}` : line.act.id))).toEqual([
    "prompt1",
    "prompt2",
    "more 2",
    "prompt5",
    "prompt6",
    "prompt7",
    "prompt8",
    "prompt9",
    "prompt10",
  ]);
  const more = lines.find((line) => line.type === "more");
  expect(more?.type === "more" && [more.key, more.heads]).toEqual(["prompts:chain1", ["prompt"]]);
  expect(
    condense(
      ladder("chain1", acts, () => "", new Map(), new Set(["prompt2"])),
      new Set(["prompts:chain1"]),
    ).some((line) => line.type === "more"),
  ).toBe(false);
});

test("a long ladder folds its earlier rungs into one line of more, keeps a rung that runs, and folds no lone rung", () => {
  const acts = [act("chain1", "chain", "", "operator", 1), act("prompt1", "prompt", "chain1", "operator", 2)];
  for (let n = 1; n <= 9; n++)
    acts.push(act(`rung${n}`, "rung", "chain1", "prompt1", 2 + n, n === 2 ? undefined : 3 + n));
  const ids = (opened: string[]) =>
    condense(
      ladder("chain1", acts, () => "x = 1", new Map(), new Set()),
      new Set(opened),
    ).map((line) => (line.type === "more" ? `more ${line.count}` : line.act.id));
  expect(ids([])).toEqual(["prompt1", "rung1", "rung2", "more 3", "rung6", "rung7", "rung8", "rung9"]);
  expect(ids(["rungs:prompt1"])).toHaveLength(10);
  // Five rungs leave one to fold, and a line of more in place of one rung shows nothing less.
  expect(ids([]).length).toBeLessThan(ids(["rungs:prompt1"]).length);
  const five = acts.slice(0, 7);
  expect(
    condense(
      ladder("chain1", five, () => "", new Map(), new Set()),
      new Set(),
    ).some((line) => line.type === "more"),
  ).toBe(false);
});

test("the measure of a chain counts its answers, sums their dollars, and names the actor of the last", () => {
  expect(
    measureOf([
      { actor: "claude-cli:sonnet/high", usage: [100, 10, 0, 0, 0.25] },
      { actor: "claude-cli:opus/high", usage: [200, 20, 0, 0, 0.5] },
    ]),
  ).toEqual({ dollars: 0.75, spent: 0.75, answers: 2, actor: "claude-cli:opus/high" });
});

test("a count of seconds reads as seconds, minutes or hours", () => {
  expect([duration(45), duration(600), duration(10800)]).toEqual(["45s", "10m", "3h"]);
});

test("a command says what it does by its program, in a few words, and an unknown program says its name", () => {
  expect(commandDoing("gh pr checks 364 --watch --interval 30")).toBe("checks of #364");
  expect(commandDoing("cd ~/src/furb-w && bun run lint 2>&1 | tail -1; bun run test")).toBe(
    "runs bun run lint",
  );
  expect(commandDoing("git push --force-with-lease origin w/feed")).toBe("pushes");
  expect(commandDoing("sh $TMPDIR/build.sh cargo test --workspace")).toBe("runs cargo test");
  expect(commandDoing("gh pr view 364 --json headRefOid,statusCheckRollup")).toBe("reads #364");
  expect(commandDoing("cd /tmp/x && FOO=1 /usr/bin/rg -n theme src")).toBe("runs rg");
  expect(commandDoing("(cd /tmp && PYTHONPATH=. python3 -c 'print(1)')")).toBe("runs python3");
  expect(commandDoing("")).toBe("runs a command");
});

test("an act that runs says what it does: a question to the operator, the chains that it waits for, a command, a file", () => {
  const label = (chain: string) => ({ chain2: "feed", chain3: "qwen" })[chain] ?? chain;
  const asked = act("prompt1", "prompt", "chain2", "rung1", 1, undefined, {
    words: ["str", "Go.", "claude-cli:opus"],
  });
  const other = act("prompt2", "prompt", "chain3", "rung1", 2, undefined, {
    words: ["str", "Go.", "claude-cli:opus"],
  });
  const you = act("prompt3", "prompt", "chain1", "rung1", 3, undefined, {
    words: ["bool", "Merge?", "operator"],
  });
  const acts = new Map([asked, other, you].map((one) => [one.id, one]));
  const find = (id: string) => acts.get(id);
  const wait = (...ids: string[]) =>
    act("wait1", "wait", "chain1", "rung1", 4, undefined, { words: [300, ...ids] });
  expect(doingOf(you, find, label)).toBe("asks you");
  expect(doingOf(asked, find, label)).toBe("waits for feed");
  expect(doingOf(wait("prompt1"), find, label)).toBe("waits for feed");
  expect(doingOf(wait("prompt1", "prompt2"), find, label)).toBe("waits for 2 threads");
  expect(doingOf(wait(), find, label)).toBe("waits 5m");
  const read = act("read1", "read", "chain1", "rung1", 5, undefined, { words: ["tui/src/theme.ts"] });
  expect(doingOf(read, find, label)).toBe("reads theme.ts");
  const bash = act("bash1", "bash", "chain1", "rung1", 6, undefined, {
    words: ["cargo clippy -- -D warnings"],
  });
  expect(doingOf(bash, find, label)).toBe("runs cargo clippy");
  expect(doingOf(wait("bash1"), (id) => (id === "bash1" ? bash : acts.get(id)), label)).toBe(
    "runs cargo clippy",
  );
});

import { expect, test } from "bun:test";
import { Meaning } from "../../index.cjs";
import type { Act } from "../../src/life/acts.ts";
import { isDone } from "../../src/life/acts.ts";
import { thread as drawn } from "../../src/life/ladder.ts";
import { commandDoing, doingOf, duration, subjectOf } from "../../src/life/loom.ts";
import { act, actIndex, changed } from "./acts.ts";

/** A fault of the engine as a record holds it, of a class and with its reason. */
const fault = (reason: string) => {
  const [is = "Exception", ...why] = reason.split(": ");
  return { is: /^\w+$/.test(is) ? is : "Exception", args: [why.join(": ") || reason] };
};

/** The facts of a record that tell these acts, each at the moment it began and at the moment it ended: the fact
 * that asks the act and its done, and for a rung whose word ran, the run of its word between them. */
const factsOf = (acts: readonly Act[]): unknown[][] => {
  const facts: [number, number, unknown[]][] = [];
  for (const [place, one] of acts.entries()) {
    const asked = [one.kind, one.id, one.by, one.kind === "chain" ? "" : one.on, ...one.words];
    facts.push([one.began, 0, asked]);
    const end = one.ended ?? one.began;
    if (one.kind === "rung" && one.run && one.run.run !== "running") {
      const run = `run${10_000 + place}`;
      const reason = "reason" in one.run ? one.run.reason : "";
      const value = one.run.run === "done" ? null : fault(reason);
      facts.push([one.began, 1, ["run", run, one.chain, one.chain, one.id, "", ""]]);
      facts.push([end, 2, ["done", run, one.chain, value]]);
    }
    if (one.phase.phase === "done") facts.push([end, 3, ["done", one.id, one.by, one.phase.value]]);
  }
  return facts.sort((one, other) => one[0] - other[0] || one[1] - other[1]).map(([, , fact]) => fact);
};

/** The thread of a chain from the ladder that the crate folds of the facts of the acts of an index, read through
 * Meaning, the door that a window reads, and drawn as a host draws it. */
const thread = (
  chain: string,
  word: (rung: Act) => string,
  streams: ReadonlyMap<string, { text: string; thinking: string }>,
  owed: ReadonlySet<string>,
  opened: ReadonlySet<string>,
  index: Parameters<typeof drawn>[5],
  shown?: { prompts: number; rungs: number },
) => {
  const meaning = new Meaning();
  for (const fact of factsOf([...index.ids.values()])) meaning.hear(fact);
  return drawn(
    meaning.ladder(chain, undefined, [...opened], [...owed], undefined, shown),
    chain,
    word,
    streams,
    owed,
    index,
  );
};

test("a thread shows each prompt with the rungs of its ladder, the acts of each rung as chips, and what the prompt closed with", () => {
  const acts = [
    act("chain1", "chain", "", "operator", 1),
    act("prompt1", "prompt", "chain1", "operator", 2, 12, { words: ["str", "Explore", ""], value: "Done." }),
    act("rung1", "rung", "chain1", "prompt1", 3, 11),
    act("read1", "read", "chain1", "rung1", 4, 4),
    act("read2", "read", "chain1", "rung1", 5, 5),
    act("read3", "read", "chain1", "rung1", 6, 6),
    act("bash1", "bash", "chain1", "rung1", 7, 9),
    act("rung2", "rung", "chain1", "operator", 13, undefined, { words: ["await wait(60, __name__)"] }),
    act("bash2", "bash", "chain1", "rung2", 14, undefined, {
      words: ["make"],
      value: { stdout: { content: "one\ntwo\nthree\nfour\n" }, stderr: { content: "" } },
    }),
  ];
  const words: Record<string, string> = {
    rung1: 'notes = read("README.md")\nclose("Done.")',
    rung2: "await wait(60, __name__)",
  };
  const lines = thread(
    "chain1",
    (rung) => words[rung.id] ?? "",
    new Map(),
    new Set(),
    new Set(),
    actIndex(acts, "chain1"),
  );
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
    ["rung", "rung2", 0, "await wait(60, __name__) +0 bash2", ""],
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
  const lines = thread(
    "chain1",
    (rung) => String(rung.words[0] ?? ""),
    new Map(),
    new Set(),
    new Set(),
    actIndex(acts, "chain1"),
  );
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
    ["remember1", "delegates"],
  ]);
});

test("a thread that a host stamps reads a step again only where an act under it changed, as a thread that keeps nothing reads it", () => {
  // A life of many steps, made by a fixed sequence of choices: prompts with long ladders, rungs that ask prompts on the
  // chain and run commands, rungs of an ear in a row, and work that runs or asks the operator, old and new.
  let seed = 7;
  const next = (n: number) => {
    seed = (seed * 1103515245 + 12345) % 2147483648;
    return seed % n;
  };
  const acts: Act[] = [act("chain1", "chain", "", "operator", 1)];
  const add = (one: Act) => {
    acts.push(one);
    return one;
  };
  const open = () => (next(9) ? acts.length + 1 : undefined);
  const nest = (prompt: Act, depth: number) => {
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
  const index = actIndex(acts, "chain1");
  const word = (rung: Act) => `${rung.id}\nmore`;
  // A host that stamps the changes of what each act made has its thread read again only where an act changed.
  const stamps = new Map<string, number>();
  const stamped = { ...index, stamp: (id: string) => stamps.get(id) ?? 0 };
  const read = () => thread("chain1", word, new Map(), owed, new Set(), stamped);
  const whole = () =>
    thread("chain1", word, new Map(), owed, new Set(), { ...index, made: new Map(index.made) });
  expect(read().some((line) => line.type === "more")).toBe(true);
  expect(read()).toEqual(whole());
  for (const [place, one] of acts.entries())
    if (one.kind === "bash" && !isDone(one)) {
      const done = changed(one, { done: true, ended: acts.length + 1 });
      acts[place] = done;
      index.ids.set(done.id, done);
      const made = index.made.get(done.by) ?? [];
      made[made.indexOf(one)] = done;
      stamps.set(done.by, (stamps.get(done.by) ?? 0) + 1);
    }
  index.live = new Map(
    acts.filter((one) => !isDone(one) || one.run?.run === "running").map((one) => [one.id, one]),
  );
  expect(read()).toEqual(whole());
});

test("an act says what it is about on one line: the word of a rung, the message of a prompt, the time and the acts of a wait", () => {
  const word = (rung: Act) => (rung.id === "rung1" ? "\nx = 1\ny = 2" : "");
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
  const lines = thread(
    "chain1",
    () => "",
    new Map(),
    new Set(["prompt2"]),
    new Set(),
    actIndex(acts, "chain1"),
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
  const opened = thread(
    "chain1",
    () => "",
    new Map(),
    new Set(["prompt2"]),
    new Set(["prompts:chain1"]),
    actIndex(acts, "chain1"),
  );
  expect(opened.some((line) => line.type === "more")).toBe(false);
});

test("a long ladder folds its earlier rungs into one line of more, keeps a rung that runs, and folds no lone rung", () => {
  const acts = [act("chain1", "chain", "", "operator", 1), act("prompt1", "prompt", "chain1", "operator", 2)];
  for (let n = 1; n <= 9; n++)
    acts.push(act(`rung${n}`, "rung", "chain1", "prompt1", 2 + n, n === 2 ? undefined : 3 + n));
  const ids = (opened: string[]) =>
    thread("chain1", () => "x = 1", new Map(), new Set(), new Set(opened), actIndex(acts, "chain1")).map(
      (line) => (line.type === "more" ? `more ${line.count}` : line.act.id),
    );
  expect(ids([])).toEqual(["prompt1", "rung1", "rung2", "more 3", "rung6", "rung7", "rung8", "rung9"]);
  expect(ids(["rungs:prompt1"])).toHaveLength(10);
  // Five rungs leave one to fold, and a line of more in place of one rung shows nothing less.
  expect(ids([]).length).toBeLessThan(ids(["rungs:prompt1"]).length);
  const five = acts.slice(0, 7);
  expect(
    thread("chain1", () => "", new Map(), new Set(), new Set(), actIndex(five, "chain1")).some(
      (line) => line.type === "more",
    ),
  ).toBe(false);
});

test("a question deep in an early step of a long thread stays shown with each step over it, and the steps beside them fold", () => {
  const acts = [act("chain1", "chain", "", "operator", 1)];
  for (let n = 1; n <= 12; n++)
    acts.push(act(`prompt${n}`, "prompt", "chain1", "operator", 10 * n, n === 2 ? undefined : 10 * n + 9));
  // The second prompt holds eight rungs, and its second rung asks the operator a question that waits.
  for (let n = 1; n <= 8; n++)
    acts.push(act(`rung${n}`, "rung", "chain1", "prompt2", 20 + n, n === 2 ? undefined : 21 + n));
  acts.push(
    act("prompt13", "prompt", "chain1", "rung2", 23, undefined, { words: ["str", "Which?", "operator"] }),
  );
  const lines = thread(
    "chain1",
    () => "x = 1",
    new Map(),
    new Set(["prompt13"]),
    new Set(),
    actIndex(acts, "chain1"),
  );
  expect(lines.map((line) => (line.type === "more" ? `more ${line.count}` : line.act.id))).toEqual([
    "prompt1",
    "prompt2",
    "rung1",
    "rung2",
    "prompt13",
    "more 2",
    "rung5",
    "rung6",
    "rung7",
    "rung8",
    "more 4",
    "prompt7",
    "prompt8",
    "prompt9",
    "prompt10",
    "prompt11",
    "prompt12",
  ]);
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
  const read = act("read1", "read", "chain1", "rung1", 5, undefined, { words: ["src/theme.ts"] });
  expect(doingOf(read, find, label)).toBe("reads theme.ts");
  const bash = act("bash1", "bash", "chain1", "rung1", 6, undefined, {
    words: ["cargo clippy -- -D warnings"],
  });
  expect(doingOf(bash, find, label)).toBe("runs cargo clippy");
  expect(doingOf(wait("bash1"), (id) => (id === "bash1" ? bash : acts.get(id)), label)).toBe(
    "runs cargo clippy",
  );
});

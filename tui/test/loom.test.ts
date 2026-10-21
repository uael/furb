import { expect, test } from "bun:test";
import { ladder } from "../src/ladder.ts";
import { spans, weave } from "../src/loom.ts";
import type { ActRow } from "../src/session.ts";

/** An act of a life, made at a moment, and done at another when it is done. */
function act(
  id: string,
  kind: string,
  on: string,
  by: string,
  began: number,
  ended?: number,
  more: Partial<ActRow> = {},
): ActRow {
  return {
    id,
    kind,
    by,
    on: kind === "chain" ? id : on,
    words: [],
    done: ended !== undefined,
    value: null,
    paused: false,
    began,
    ended,
    ...(kind === "rung" ? { run: { status: ended === undefined ? "running" : "done", reason: "" } } : {}),
    ...more,
  } as ActRow;
}

const loomOf = (
  acts: ActRow[],
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
  const { threads } = loomOf(acts);
  const of = (id: string) => threads.find((thread) => thread.id === id);
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
  expect(open.threads.map((thread) => [thread.lines, thread.label])).toEqual([
    ["", "you"],
    ["", "conduct"],
    ["├ ", "w chain2"],
    ["└ ", "w chain3"],
    ["", "w chain5"],
  ]);
  const folded = loomOf(acts, { folds: ["chain1"] });
  expect(folded.threads.map((thread) => thread.id)).toEqual(["operator", "chain1", "chain5"]);
  expect(folded.threads[1]).toMatchObject({ folded: true, workers: 2, tally: { working: 1, done: 1 } });
});

test("the operator owes the prompts that wait for it, the oldest first, and its thread and the thread of each asks", () => {
  const acts = [
    act("chain1", "chain", "", "operator", 1),
    act("rung1", "rung", "chain1", "operator", 2),
    act("chain2", "chain", "", "rung1", 3),
    act("prompt1", "prompt", "chain2", "rung1", 4, undefined, {
      words: ["str", "Which effort?", "operator"],
    }),
    act("prompt2", "prompt", "chain1", "rung1", 5, undefined, { words: ["bool", "Merge?", "operator"] }),
  ];
  const { owed, threads } = loomOf(acts, {
    prompts: [
      { id: "prompt2", shape: "bool", message: "Merge?" },
      { id: "prompt1", shape: "str", message: "Which effort?" },
    ],
  });
  expect(owed.map((prompt) => [prompt.id, prompt.chain])).toEqual([
    ["prompt1", "chain2"],
    ["prompt2", "chain1"],
  ]);
  expect(threads.map((thread) => thread.state)).toEqual(["asks", "asks", "asks"]);
  expect(threads[0]?.cells.at(-1)?.tone).toBe("asks");
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

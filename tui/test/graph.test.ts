import { expect, test } from "bun:test";
import { messages, places, route, you } from "../src/map.ts";
import { outcome, placed, source } from "../src/source.ts";
import { anchorOf, outline, pullsOf, threads } from "../src/threads.ts";
import { act } from "./acts.ts";

const ask = (
  id: string,
  on: string,
  by: string,
  began: number,
  ended?: number,
  words: unknown[] = ["str", id, ""],
) =>
  act(id, "prompt", on, by, began, ended, {
    words,
    ...(ended === undefined ? {} : { value: `${id} closed` }),
  });

test("the tree of threads stands each prompt under the prompt of the ladder whose rung made it, with its progress and what waits for the operator", () => {
  const acts = [
    act("chain1", "chain", "", "operator", 1),
    ask("prompt1", "chain1", "operator", 2),
    act("rung1", "rung", "chain1", "prompt1", 3),
    act("chain2", "chain", "", "rung1", 4),
    ask("prompt2", "chain2", "rung1", 5, 9),
    act("chain3", "chain", "", "rung1", 6),
    ask("prompt3", "chain3", "rung1", 7),
    act("rung2", "rung", "chain3", "prompt3", 8),
    ask("prompt4", "chain3", "rung2", 10, undefined, ["str", "Which name?", "operator"]),
  ];
  const tree = threads(acts, new Set(["prompt4"]));
  expect(tree.map((one) => one.act.id)).toEqual(["prompt1"]);
  const [top] = tree;
  expect(top?.children.map((one) => one.act.id)).toEqual(["prompt2", "prompt3"]);
  expect([top?.closed, top?.total, top?.owed, top?.working]).toEqual([1, 3, ["prompt4"], 1]);
  expect(outline(tree, new Set(["prompt3"])).map((one) => [one.act.id, one.depth])).toEqual([
    ["prompt1", 0],
    ["prompt2", 1],
    ["prompt3", 1],
  ]);
});

test("an acknowledgment that a chain makes is a message of the thread of the act that it names, and no thread", () => {
  const acts = [
    act("chain1", "chain", "", "operator", 1),
    ask("prompt1", "chain1", "operator", 2),
    act("rung1", "rung", "chain1", "prompt1", 3),
    ask("prompt2", "chain1", "rung1", 4, 5),
    ask("prompt3", "chain1", "chain1", 6, undefined, ["None", "prompt2 done", ""]),
  ];
  const [top] = threads(acts, new Set());
  expect(top?.children.map((one) => one.act.id)).toEqual(["prompt2"]);
  expect(top?.messages.map((one) => one.id)).toEqual(["prompt3"]);
});

test("a thread hangs on the act, the place in a file and the pull request that its message names", () => {
  const acts = [
    act("chain1", "chain", "", "operator", 1),
    act("write4", "write", "chain1", "operator", 2, 3),
    ask("prompt1", "chain1", "operator", 4, undefined, [
      "str",
      "Review write4 of src/search.ts:3-5 in https://github.com/uael/fieldnotes/pull/12.",
      "",
    ]),
  ];
  const rows = new Map(acts.map((one) => [one.id, one]));
  expect(anchorOf(acts[2] as never, rows)).toEqual({
    act: "write4",
    file: "src/search.ts:3-5",
    pull: { number: 12, url: "https://github.com/uael/fieldnotes/pull/12" },
  });
});

test("a thread names the pull requests of the values of its tree and of the output of the commands of its ladder", () => {
  const acts = [
    act("chain1", "chain", "", "operator", 1),
    ask("prompt1", "chain1", "operator", 2),
    act("rung1", "rung", "chain1", "prompt1", 3),
    act("bash1", "bash", "chain1", "rung1", 4, 5, {
      value: {
        is: "Exit",
        code: 0,
        stdout: { content: "https://github.com/o/r/pull/7\n" },
        stderr: { content: "" },
      },
    }),
    ask("prompt2", "chain1", "rung1", 6, 7),
  ];
  acts[4] = { ...acts[4], value: "Opened https://github.com/o/r/pull/8." } as never;
  expect(threads(acts, new Set())[0]?.pulls.map((one) => one.number)).toEqual([7, 8]);
  expect(pullsOf("no pull here")).toEqual([]);
});

test("a chain takes its place at its birth and keeps it: the column of its depth, and the first free row at or under the chain that hired it", () => {
  const born = [
    act("chain1", "chain", "", "operator", 1),
    act("chain2", "chain", "", "rung1", 2),
    act("chain3", "chain", "", "rung1", 3),
  ];
  const parent = (chain: { id: string }) => (chain.id === "chain1" ? undefined : "chain1");
  const first = places(born, parent);
  expect([...first]).toEqual([
    [you, { column: 0, row: 0 }],
    ["chain1", { column: 0, row: 1 }],
    ["chain2", { column: 1, row: 1 }],
    ["chain3", { column: 1, row: 2 }],
  ]);
  const later = places([...born, act("chain4", "chain", "", "rung2", 4)], (chain) =>
    chain.id === "chain4" ? "chain2" : parent(chain),
  );
  for (const [id, place] of first) expect(later.get(id)).toEqual(place);
  expect(later.get("chain4")).toEqual({ column: 2, row: 1 });
});

test("an open prompt flows from the chain of the rung that made it to its chain, a question flows to the operator, and an answer flows back for a moment", () => {
  const acts = [
    act("chain1", "chain", "", "operator", 1),
    act("rung1", "rung", "chain1", "operator", 2),
    act("chain2", "chain", "", "rung1", 3),
    ask("prompt1", "chain2", "rung1", 4),
    ask("prompt2", "chain2", "rung1", 5, 6),
    ask("prompt3", "chain2", "rung1", 7, undefined, ["str", "Which?", "operator"]),
  ];
  expect(messages(acts, (one) => one.id === "prompt2")).toEqual([
    { act: "prompt1", from: "chain1", to: "chain2", kind: "ask" },
    { act: "prompt2", from: "chain2", to: "chain1", kind: "answer" },
    { act: "prompt3", from: "chain2", to: you, kind: "asks you" },
    { act: "prompt3", from: "chain1", to: "chain2", kind: "ask" },
  ]);
  const parents: Record<string, string> = { chain2: "chain1", chain3: "chain1", chain1: you };
  expect(route("chain2", "chain3", (id) => parents[id])).toEqual(["chain2", "chain1", "chain3"]);
});

test("the acts of a word stand on the lines that call their verbs, and a loop puts again on its line the acts that it makes again", () => {
  const made = [
    act("read1", "read", "chain1", "rung1", 2, 3),
    act("bash1", "bash", "chain1", "rung1", 4, 5),
    act("bash2", "bash", "chain1", "rung1", 6, 7),
    act("bash3", "bash", "chain1", "rung1", 8, 9),
  ];
  const word = 'notes = read("README.md")\nfor n in range(3):\n  await bash(f"echo {n}")';
  expect(placed(word, made).map((line) => line.map((one) => one.id))).toEqual([
    ["read1"],
    [],
    ["bash1", "bash2", "bash3"],
  ]);
});

test("the source of a chain heads the words of each ladder with its prompt, marks the line that runs, and ends a ladder with its value", () => {
  const acts = [
    act("chain1", "chain", "", "operator", 1),
    ask("prompt1", "chain1", "operator", 2, 9),
    act("rung1", "rung", "chain1", "prompt1", 3, 8),
    act("bash1", "bash", "chain1", "rung1", 4, 5),
    ask("prompt2", "chain1", "operator", 10),
    act("rung2", "rung", "chain1", "prompt2", 11),
    act("bash2", "bash", "chain1", "rung2", 12),
  ];
  const words: Record<string, string> = {
    rung1: 'r = await bash("ls")\nclose("ok")',
    rung2: 'await bash("sleep 9")',
  };
  const lines = source("chain1", acts, (rung) => words[rung.id] ?? "", new Set());
  expect(
    lines.map((line) =>
      line.type === "code"
        ? [line.rung.id, line.number, line.acts.map((one) => one.id), line.runs]
        : [line.type, line.act.id],
    ),
  ).toEqual([
    ["prompt", "prompt1"],
    ["rung1", 1, ["bash1"], false],
    ["rung1", 2, [], false],
    ["value", "prompt1"],
    ["prompt", "prompt2"],
    ["rung2", 1, ["bash2"], true],
  ]);
  expect(outcome(acts[3] as never, (id) => id)).toBe("");
});

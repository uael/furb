import { expect, test } from "bun:test";
import { program, stack } from "../../src/life/debugger.ts";
import { plain, reason } from "../../src/life/plain.ts";
import { marks } from "../../src/life/time.ts";
import { watch } from "../../src/life/watch.ts";
import { act } from "./acts.ts";

test("a message reads with no name of its exception, no directory of the life, and no period at its end", () => {
  const raw = "Refused('There is no file at /work/notes/test/shortcut.test.ts.')";
  expect(plain(raw, "/work/notes")).toBe("There is no file at test/shortcut.test.ts");
  expect(plain("Refused: There is no file at /work/notes/a.ts.", "/work/notes/")).toBe(
    "There is no file at a.ts",
  );
});

test("a message that names no exception keeps its first word", () => {
  expect(plain("Note: the build is slow")).toBe("Note: the build is slow");
});

test("a failed rung says its reason, and an act that did not fail says nothing", () => {
  const rung = act("rung1", "rung", "chain1", "prompt1", 3, 5, {
    run: { status: "failed", reason: "Refused: There is no file at /p/a.ts." },
  });
  expect(reason(rung, "/p")).toBe("There is no file at a.ts");
  expect(reason(act("read1", "read", "chain1", "rung1", 4, 5))).toBeUndefined();
});

test("a failed act that is no rung says the first argument of its exception", () => {
  const read = act("read1", "read", "chain1", "rung1", 4, 5, {
    value: { is: "Refused", args: ["No file."] },
  });
  expect(reason(read)).toBe("No file");
});

test("the marks of time hold each step, each question to the operator, and each end of a thread", () => {
  const acts = [
    act("prompt1", "prompt", "chain1", "operator", 2, 9, { words: ["str", "Do it", ""], value: "ok" }),
    act("rung1", "rung", "chain1", "prompt1", 3, 8),
    act("prompt2", "prompt", "chain1", "rung1", 4, 6, { words: ["str", "Which?", "operator"], value: "K" }),
  ];
  expect(marks(acts, new Set(["prompt1", "prompt2"]))).toEqual([
    { at: 3, kind: "step", act: "rung1" },
    { at: 4, kind: "asks", act: "prompt2" },
    { at: 6, kind: "done", act: "prompt2" },
    { at: 9, kind: "done", act: "prompt1" },
  ]);
});

test("the watch holds the last value of each name told by its moment, under the act whose name it starts with", () => {
  const told = [
    { name: "bash2", value: "Act[Exit]", at: 3, act: "bash2" },
    { name: "bash2_command", value: "'ls'", at: 3, act: "bash2" },
    { name: "review", value: "'old'", at: 4, act: null },
    { name: "review", value: "'new'", at: 9, act: null },
  ];
  expect(watch(told, 5, 2)).toEqual([
    { items: [{ name: "review", value: "'old'", fresh: true }] },
    { act: "bash2", items: [{ name: "bash2_command", value: "'ls'", fresh: false }] },
  ]);
});

test("the call stack of a chain goes from its rung that runs up through each prompt and the rung that made it", () => {
  const acts = [
    act("prompt1", "prompt", "chain1", "operator", 2),
    act("rung1", "rung", "chain1", "prompt1", 3),
    act("prompt2", "prompt", "chain2", "rung1", 4),
    act("rung2", "rung", "chain2", "prompt2", 5),
    act("bash1", "bash", "chain2", "rung2", 6),
  ];
  expect(stack("chain2", acts)).toEqual([
    { act: "rung2", chain: "chain2", awaits: "bash1" },
    { act: "prompt2", chain: "chain2" },
    { act: "rung1", chain: "chain1", awaits: "prompt2" },
    { act: "prompt1", chain: "chain1" },
  ]);
});

test("the program of a ladder is the words of its rungs, one after the other", () => {
  const acts = [
    act("rung1", "rung", "chain1", "prompt1", 3, 4),
    act("rung2", "rung", "chain1", "prompt1", 5),
  ];
  const words = new Map([
    ["rung1", "a = 1"],
    ["rung2", "close(a)"],
  ]);
  expect(program("prompt1", acts, (rung) => words.get(rung.id) ?? "")).toBe("a = 1\nclose(a)");
});

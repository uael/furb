import { expect, test } from "bun:test";
import { makers } from "../../src/life/ladder.ts";
import { hintOf, threadProgram } from "../../src/life/program.ts";
import { act } from "./acts.ts";

const label = (chain: string) => `label of ${chain}`;

test("the program of a thread holds the words of its steps and its messages in order, with a quote folded and the output of the line that runs", () => {
  const prompt = act("prompt1", "prompt", "chain1", "operator", 1, undefined, {
    words: ["str", "Look at it", "opus"],
  });
  const rung1 = act("rung1", "rung", "chain1", "prompt1", 2, 6);
  const bash1 = act("bash1", "bash", "chain1", "rung1", 3, 4, {
    value: { code: 0, stdout: { path: "", content: "a\nb\nc\n" }, stderr: { path: "", content: "" } },
  });
  const message = act("prompt2", "prompt", "chain1", "operator", 7, undefined, {
    words: ["None", "And this too", "opus"],
    thread: "prompt1",
  });
  const rung2 = act("rung2", "rung", "chain1", "prompt2", 8);
  const bash2 = act("bash2", "bash", "chain1", "rung2", 9, undefined, {
    value: {
      code: null,
      stdout: { path: "", content: "1\n2\n3\n4\n5\n6\n7\n" },
      stderr: { path: "", content: "" },
    },
  });
  const acts = [prompt, rung1, bash1, message, rung2, bash2];
  const words: Record<string, string> = {
    rung1: "<s:cmd>\nls\npwd\n</s:cmd>\ngot = await bash(cmd)\nprint(got)",
    rung2: 'more = await bash("tests")',
  };
  const lines = threadProgram(
    [prompt, message],
    makers(acts),
    (rung) => words[rung.id] ?? "",
    new Set(),
    label,
  );
  expect(lines.map((line) => [line.kind, line.text])).toEqual([
    ["quote", "ls"],
    ["code", "got = await bash(cmd)"],
    ["code", "print(got)"],
    ["note", "# you: And this too"],
    ["code", 'more = await bash("tests")'],
    ["output", "3"],
    ["output", "4"],
    ["output", "5"],
    ["output", "6"],
    ["output", "7"],
  ]);
  const quote = lines[0];
  expect(quote?.kind === "quote" && [quote.name, quote.lines]).toEqual(["cmd", 2]);
  const ran = lines[1];
  expect(ran?.kind === "code" && [ran.hint, ran.runs]).toEqual(["exit 0, 3 lines", false]);
  const runs = lines[4];
  expect(runs?.kind === "code" && [runs.hint, runs.runs]).toEqual(["runs", true]);
});

test("a word that the gate refused marks the line that its reason names", () => {
  const prompt = act("prompt1", "prompt", "chain1", "operator", 1, 5, {
    words: ["str", "Go", "opus"],
    value: "ok",
  });
  const rung = act("rung1", "rung", "chain1", "prompt1", 2, 3, {
    run: { status: "failed", reason: "line 2: Expected a statement" },
  });
  const lines = threadProgram([prompt], makers([prompt, rung]), () => "a = 1\n= 2", new Set(), label);
  const marked = lines.flatMap((line) => (line.kind === "code" && line.fails ? [line.number] : []));
  expect(marked).toEqual([2]);
  expect(lines.at(-1)?.kind).toBe("code");
});

test("a loop on a line says how many times it ran and what its last turn came to", () => {
  const reads = [1, 2, 3].map((n) =>
    act(`read${n}`, "read", "chain1", "rung1", n, n, { value: { path: "f", content: "x\ny\n" } }),
  );
  expect(hintOf(reads, new Set(), label)).toBe("x3  2 lines");
});

test("an act whose host left its text out says the lines that it kept, and a write the lines that its change added and removed", () => {
  const read1 = {
    ...act("read1", "read", "chain1", "rung1", 1, 2, { value: { path: "notes.md" } }),
    count: 4,
  };
  const write1 = {
    ...act("write1", "write", "chain1", "rung1", 3, 4, { words: [{ path: "notes.md" }] }),
    lines: { added: 2, removed: 1 },
  };
  expect(hintOf([read1, write1], new Set(), label)).toBe("4 lines  +2 -1");
});

test("an act of a verb of an extension stands on the line that calls that verb, and not on the line that defines it", () => {
  const prompt = act("prompt1", "prompt", "chain1", "operator", 1, undefined, {
    words: ["str", "Go", "opus"],
  });
  const rung = act("rung1", "rung", "chain1", "prompt1", 2, 5);
  const todo = act("todo1", "todo", "chain1", "rung1", 3, 4);
  const word = 'def todo(item):\n  return act("todo", "", None, item)\n\ntodo("one")';
  const lines = threadProgram([prompt], makers([prompt, rung, todo]), () => word, new Set(), label);
  const on = lines.flatMap((line) => (line.kind === "code" && line.acts.length ? [line.number] : []));
  expect(on).toEqual([4]);
});

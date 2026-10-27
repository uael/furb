import { expect, test } from "bun:test";
import { makers } from "../../src/life/ladder.ts";
import {
  actLines,
  foldStrings,
  hintOf,
  joinedLines,
  literals,
  stepLines,
  threadProgram,
  wrap,
  writing,
} from "../../src/life/program.ts";
import { act } from "./acts.ts";

const label = (chain: string) => `label of ${chain}`;

test("the program of a thread holds the words of its steps and its messages in order, with a quote bound as a string and the output of the line that runs", () => {
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
    ["code", "cmd = 'ls\\npwd\\n'"],
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
  expect(quote?.kind === "code" && [quote.hint, quote.runs, quote.acts.length]).toEqual([
    "2 lines",
    false,
    0,
  ]);
  const ran = lines[1];
  expect(ran?.kind === "code" && [ran.hint, ran.runs]).toEqual(["exit 0, 3 lines", false]);
  const runs = lines[4];
  expect(runs?.kind === "code" && [runs.hint, runs.runs]).toEqual(["runs", true]);
});

test("the numbers of the lines of a thread go on from each step to the next, across the steps that a fold keeps out, to the line that its model writes", () => {
  const prompt = act("prompt1", "prompt", "chain1", "operator", 1, undefined, {
    words: ["str", "Go on", "opus"],
  });
  const rung1 = act("rung1", "rung", "chain1", "prompt1", 2, 3);
  const rung2 = act("rung2", "rung", "chain1", "prompt1", 4, 5);
  const rung3 = act("rung3", "rung", "chain1", "prompt1", 6);
  const words: Record<string, string> = { rung1: "a = 1\nb = 2", rung2: "<s:q>\nx\n</s:q>\nc = 3\nd = 4" };
  const program = (keep?: number) =>
    threadProgram(
      [prompt],
      makers([prompt, rung1, rung2, rung3]),
      (rung) => words[rung.id] ?? "",
      new Set(),
      label,
      undefined,
      keep,
    ).map((line) => [
      line.kind,
      "display" in line ? line.display : undefined,
      "number" in line ? line.number : undefined,
    ]);
  expect(program()).toEqual([
    ["code", 1, 1],
    ["code", 2, 2],
    ["code", 3, 1],
    ["code", 6, 4],
    ["code", 7, 5],
    ["writes", 8, undefined],
  ]);
  expect(program(2)).toEqual([
    ["earlier", undefined, undefined],
    ["code", 3, 1],
    ["code", 6, 4],
    ["code", 7, 5],
    ["writes", 8, undefined],
  ]);
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

test("a line that awaits an act that an earlier line made runs, names that act and its line, and lights that line with the output of its command", () => {
  const prompt = act("prompt1", "prompt", "chain1", "operator", 1, undefined, {
    words: ["str", "Go", "opus"],
  });
  const rung = act("rung1", "rung", "chain1", "prompt1", 2);
  const bash = act("bash1", "bash", "chain1", "rung1", 3, undefined, {
    words: ["make"],
    value: { code: null, stdout: { path: "", content: "compiling\n" }, stderr: { path: "", content: "" } },
  });
  const word = 'build = bash("make")\nx = 1\nb = await build';
  const lines = threadProgram([prompt], makers([prompt, rung, bash]), () => word, new Set(), label);
  expect(lines.map((line) => [line.kind, line.text])).toEqual([
    ["code", 'build = bash("make")'],
    ["output", "compiling"],
    ["code", "x = 1"],
    ["code", "b = await build"],
  ]);
  const made = lines[0];
  const waits = lines[3];
  expect(made?.kind === "code" && [made.runs, made.awaited]).toEqual([false, 3]);
  expect(
    waits?.kind === "code" && [waits.runs, waits.awaits?.act.id, waits.awaits?.display, waits.hint],
  ).toEqual([true, "bash1", 1, "waits on bash1 of line 1"]);
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

test("a program that keeps its last steps makes their lines alone, after the count of the earlier ones", () => {
  const prompt = act("prompt1", "prompt", "chain1", "operator", 1, undefined, {
    words: ["str", "Go", "opus"],
  });
  const steps = [1, 2, 3].map((n) => act(`rung${n}`, "rung", "chain1", "prompt1", n + 1, n + 1));
  const lines = threadProgram(
    [prompt],
    makers([prompt, ...steps]),
    (rung) => `x = ${rung.id}`,
    new Set(),
    label,
    undefined,
    2,
  );
  expect(lines.map((line) => [line.kind, line.text])).toEqual([
    ["earlier", "# 1 earlier step"],
    ["code", "x = rung2"],
    ["code", "x = rung3"],
  ]);
});

test("an act that no word made stands as one line of python, its call bound to its name, lit while it runs with what its command prints under it", () => {
  const bash = act("bash1", "bash", "chain1", "operator", 1, undefined, {
    words: ["printf 'a\\nb\\n'", "", 600],
    value: { code: null, stdout: { path: "", content: "a\nb\n" }, stderr: { path: "", content: "" } },
  });
  expect(actLines(bash, new Set(), label).map((line) => [line.kind, line.text])).toEqual([
    ["code", `bash1 = bash("printf 'a\\\\nb\\\\n'")`],
    ["output", "a"],
    ["output", "b"],
  ]);
  const wait = act("wait1", "wait", "chain1", "operator", 1, 2, { words: [60, "bash1"], value: "bash1" });
  const [line] = actLines(wait, new Set(), label);
  expect(line?.kind === "code" && [line.text, line.runs, line.hint]).toEqual([
    'wait1 = wait(60, "bash1")',
    false,
    "woke: bash1",
  ]);
});

test("a line too wide for its room folds its longest strings first, each to its start and a fold", () => {
  const line = 'b = bash("git fetch -q origin && git log --oneline -5 origin/main", timeout=60)  # "a quote"';
  const { text, folds } = foldStrings(line, 60);
  expect(text).toBe('b = bash("git fetch -q origin...", timeout=60)  # "a quote"');
  expect(folds).toEqual([
    { literal: 0, at: 29, whole: "git fetch -q origin && git log --oneline -5 origin/main" },
  ]);
  expect(foldStrings(line, 50).text).toBe('b = bash("git fetch -q...", timeout=60)  # "a quote"');
  expect(foldStrings(line, 60, new Set([0])).text).toBe(line);
  expect(foldStrings(line, 200).folds).toEqual([]);
});

test("the literals of a line skip a name that ends in a prefix, keep their escapes, stop at a comment, and run a string that does not end to the end of its line", () => {
  expect(literals(`x = f'{a}' + r"\\d" + 'it\\'s' + """t"""  # 'no'`)).toEqual([
    { start: 6, end: 9, raw: false },
    { start: 15, end: 17, raw: true },
    { start: 22, end: 27, raw: false },
    { start: 34, end: 35, raw: false },
  ]);
  expect(literals('brief = """first line')).toEqual([{ start: 11, end: 21, raw: false }]);
});

test("what a model writes splits into the lines of its word that it finished and the line that it still writes", () => {
  const thinking = "plans\nthe first step";
  expect(writing(undefined)).toEqual({ lines: "", tail: "" });
  expect(writing({ text: "", thinking })).toEqual({ lines: "", tail: "the first step" });
  expect(writing({ text: "got = await", thinking })).toEqual({ lines: "", tail: "got = await" });
  expect(writing({ text: "a = 1\nfor x in a:\n  b = await bash(", thinking })).toEqual({
    lines: "a = 1\nfor x in a:",
    tail: "b = await bash(",
  });
  expect(writing({ text: "a = 1\n\n", thinking })).toEqual({ lines: "a = 1", tail: "" });
});

test("a string of three marks that runs over many lines stands in its statement as one string with its escapes, on the line that starts the statement", () => {
  const word = ['text = f"""first', 'second "x" line', 'third""" + "!"', "print(text)"].join("\n");
  const rung = act("rung1", "rung", "chain1", "prompt1", 2, 3);
  const lines = stepLines(rung, makers([rung]), () => word, new Set(), label);
  expect(lines.map((line) => [line.kind, line.text, "number" in line ? line.number : 0])).toEqual([
    ["code", 'text = f"first\\nsecond \\"x\\" line\\nthird" + "!"', 1],
    ["code", "print(text)", 4],
  ]);
  expect(lines.map((line) => ("display" in line ? line.display : 0))).toEqual([1, 4]);
});

test("a raw string loses its prefix and doubles its backslashes, a backslash before a line break joins the lines, and a statement with two such strings takes all their lines", () => {
  expect(joinedLines(['x = r"""a\\d', 'b"""'], 0)).toEqual({ end: 1, text: 'x = "a\\\\d\\nb"' });
  expect(joinedLines(['y = """one \\', 'two"""'], 0)).toEqual({ end: 1, text: 'y = "one two"' });
  expect(joinedLines(['f("""a', 'b""", """c', 'd""")  # done'], 0)).toEqual({
    end: 2,
    text: 'f("a\\nb", "c\\nd")  # done',
  });
});

test("a line with no string that runs past it stands as it is", () => {
  expect(joinedLines(['x = """one line"""', "y = 2"], 0)).toBeUndefined();
  expect(joinedLines(['x = "a # b"  # """', "y = 2"], 0)).toBeUndefined();
  expect(joinedLines(['x = """open', "y = 2"], 0)).toBeUndefined();
});

test("a line too wide wraps between tokens in rows of a width at most, and the rows hold the line whole", () => {
  const line =
    '  result = await prompt(tuple[list[str], str], "Choose what it keeps: each act that holds a decision", to=CHOOSER, on=here)';
  for (const width of [30, 40, 60, 100]) {
    const rows = wrap(line, width);
    expect(rows.length).toBeGreaterThan(line.length > width ? 1 : 0);
    expect(rows[0]?.from).toBe(0);
    expect(rows.at(-1)?.to).toBe(line.length);
    for (const [at, row] of rows.entries()) {
      expect(row.indent + row.to - row.from).toBeLessThanOrEqual(width);
      expect(row.indent).toBe(at ? 4 : 0);
      const next = rows[at + 1];
      if (!next) continue;
      // A row ends between two tokens, and the text that the rows leave out is spaces.
      expect(line[row.to] === " " || "([{,".includes(line[row.to - 1] ?? "")).toBe(true);
      expect(line.slice(row.to, next.from).trim()).toBe("");
    }
  }
  expect(wrap("short", 100)).toEqual([{ from: 0, to: 5, indent: 0 }]);
  // A token wider than a row is the one thing that a row cuts inside.
  const path = wrap(`cd ${"p".repeat(30)}`, 12);
  expect(path.every((row) => row.indent + row.to - row.from <= 12)).toBe(true);
  expect(path.map((row) => row.to - row.from).reduce((a, b) => a + b, 0)).toBeGreaterThanOrEqual(30);
});

test("a fold never cuts an escape of the string that it folds", () => {
  const line = 'x = "aaaaaaaaaaa\\nbbbb cccc dddd eeee"';
  expect(foldStrings(line, 20).text).toBe('x = "aaaaaaaaaaa..."');
  expect(foldStrings('x = "aaaaaaaaaa\\u00e9bbbb cccc dddd eeee"', 20).text).toBe('x = "aaaaaaaaaa..."');
});

test("a quote that ran stands in its word as one binding, and the blank lines that keep the numbers below it stand in its line", () => {
  const word = ["brief = 'a\\nb\\n'", "", "", "", "print(brief)", "", "x = 1"].join("\n");
  const rung = act("rung1", "rung", "chain1", "prompt1", 2, 3);
  const lines = stepLines(rung, makers([rung]), () => word, new Set(), label);
  expect(lines.map((line) => [line.kind, line.text, "number" in line ? line.number : 0])).toEqual([
    ["code", "brief = 'a\\nb\\n'", 1],
    ["code", "print(brief)", 5],
    ["code", "", 6],
    ["code", "x = 1", 7],
  ]);
});

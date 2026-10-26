import { expect, test } from "bun:test";
import { makers } from "../../src/life/ladder.ts";
import {
  actLines,
  foldStrings,
  formatInput,
  formatOutput,
  hintOf,
  literals,
  statementAt,
  statements,
  threadProgram,
} from "../../src/life/program.ts";
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
  expect(
    lines.flatMap((line) =>
      line.kind === "code" || line.kind === "quote" ? [[line.number, line.display]] : [],
    ),
  ).toEqual([
    [1, 1],
    [5, 2],
    [6, 3],
    [1, 4],
  ]);
  const quote = lines[0];
  expect(quote?.kind === "quote" && [quote.value, quote.source]).toEqual([
    "ls\npwd\n",
    "<s:cmd>\nls\npwd\n</s:cmd>",
  ]);
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
  expect(lines.flatMap((line) => (line.kind === "code" ? [[line.number, line.display]] : []))).toEqual([
    [1, 2],
    [1, 3],
  ]);
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
test("multiline and raw strings stay single escaped display lines without changing source coordinates", () => {
  const prompt = act("prompt1", "prompt", "chain1", "operator", 1, undefined, {
    words: ["str", "Go", "opus"],
  });
  const rung = act("rung1", "rung", "chain1", "prompt1", 2, 5);
  const next = act("rung2", "rung", "chain1", "prompt1", 6, 7);
  const word = `text = """one "quoted"\ntwo\\n雪😀"""\nraw = r"""C:\\new\n\\path"""\nprint(text, raw)`;
  const lines = threadProgram(
    [prompt],
    makers([prompt, rung, next]),
    (step) => (step === rung ? word : "print('next')"),
    new Set(),
    label,
  );
  const code = lines.flatMap((line) => (line.kind === "code" ? [line] : []));
  expect(code.map((line) => [line.number, line.display, line.text])).toEqual([
    [1, 1, 'text = "one \\"quoted\\"\\ntwo\\n雪😀"'],
    [3, 2, 'raw = "C:\\\\new\\n\\\\path"'],
    [5, 3, "print(text, raw)"],
    [1, 4, "print('next')"],
  ]);
  expect(code[0]?.source).toBe('text = """one "quoted"\ntwo\\n雪😀"""');
  expect(code[1]?.source).toBe('raw = r"""C:\\new\n\\path"""');
  const incomplete = threadProgram(
    [prompt],
    makers([prompt, rung]),
    () => 'text = """still\narriving',
    new Set(),
    label,
  );
  expect(incomplete.map((line) => line.text)).toEqual(['text = """still', "arriving"]);
});

test("formatter input partitions complete Python statements while leaving quotes alone and guards marker-like comments", () => {
  const source = "# furb:statement\nx=1\n<s:brief>\nquoted\n</s:brief>\ny = [\n  2,\n]";
  const parts = statements(source);
  const input = formatInput(source, parts);
  expect(input).toContain("# furb:statement-1\n# furb:statement\n# furb:statement-1\nx=1");
  expect(input).toContain("# furb:statement-1\ny = [");
  expect(input).not.toContain("quoted");
  expect(statementAt(parts, source, 1)).toBe(0);
  expect(statementAt(parts, source, 5)).toBe(3);
});

test("formatter output maps changes to original statement indices and rejects lost partition marks", () => {
  const source = "x=1\n<s:brief>\nquoted\n</s:brief>\ny = [\n  2,\n]";
  const parts = statements(source);
  const formatted = "# furb:statement\nx = 1\n# furb:statement\ny = [2]";
  expect(formatOutput(source, parts, formatted)).toEqual(
    new Map([
      [0, ["x = 1"]],
      [2, ["y = [2]"]],
    ]),
  );
  expect(formatOutput(source, parts, "x = 1")).toBeUndefined();
});

test("completed formatted statements project as one program while ownership and source remain original", () => {
  const prompt = act("prompt1", "prompt", "chain1", "operator", 1);
  const rung = act("rung1", "rung", "chain1", "prompt1", 2, 5, {
    run: { status: "failed", reason: "line 1: refused" },
  });
  const read = act("read1", "read", "chain1", "rung1", 3, 4);
  const next = act("rung2", "rung", "chain1", "prompt1", 6, 7);
  const source = 'got = read("notes.md")';
  const shaped = new Map([[0, { to: 0, lines: ["got = read(", '    "notes.md"', ")"] }]]);
  const rows = threadProgram(
    [prompt],
    makers([prompt, rung, read, next]),
    (step) => (step === rung ? source : "print(1)"),
    new Set(),
    label,
    undefined,
    undefined,
    (step) => (step === rung ? shaped : undefined),
  );
  const code = rows.filter((line) => line.kind === "code");
  expect(code.map((line) => [line.text, line.display])).toEqual([
    ["got = read(", 1],
    ['    "notes.md"', 2],
    [")", 3],
    ["print(1)", 4],
  ]);
  expect(code[0]?.acts).toEqual([read]);
  expect(code[0]?.fails).toBe("line 1: refused");
  expect(code.slice(0, 3).map((line) => [line.step, line.number, line.source])).toEqual(
    Array.from({ length: 3 }, () => [rung, 1, source]),
  );
  expect(new Set(code.slice(0, 3).map((line) => line.projection)).size).toBe(3);
});

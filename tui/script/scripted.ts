// The long record of the conductor that the readers of the TUI open, made again from scripted words: the shape of a
// life of 23 chains, its acts, its makers, its chains and its moments, with each text of it, the words of each rung,
// each message, each command and its output, each reply and each value, written from the scripted vocabulary below.
// No text of a person stands in it: a text of the record is a token of the engine (a name of an act, a kind, a model,
// a state), one of the scripted messages of the operator, or a line that this script writes, and a second run of the
// script on its own output gives the same file, which tui/test/fixture-words.test.ts holds.
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

/** The messages of the operator that the record keeps, each scripted, which the readers of the TUI name. */
export const SCRIPTED: readonly string[] = [
  "(removed)",
  "You are the conductor of a scripted life, in its third round. The second round stopped at noon, and its record cannot open again, so notes.md at the root hands over what it knew.\n1. Read notes.md whole first. Then read plan.md, board.md, rules.md and the guide at the root, which hold the rules of the scripted work and the way each agent reports.\n2. List the open items of the board, order them by their rank, and hire one agent for each of the first three, with a brief that names its files and its checks.\n3. Watch each agent until its pull request is green, then merge it, and tell the next agent on the list to start.\n4. Report the state of the board to the operator each time that an item ends, in one short line.\n… 9 more lines",
  "Have a worker loading the sample skill and renaming its panel, the labels are unclear.",
  "A second task is a bit more complex: a sample file that you read, and a note to hire a worker on this thread.",
  "tell the sample lane to add colors to the minimal panels, then report the state of the lane to the cond…",
  "the sample flow of how an answer is given to a question is confusing and hard to follow",
  "Report progress.",
];

/** The labels of the chains, in the order that the life makes them: the root, then the lanes that the conductor
 * hires, each of its own, and the chains that a lane hires under it. */
const LABELS = [
  "root",
  "lane/panels",
  "lane/labels",
  "lane/colors",
  "lane/checks",
  "w/fix-board",
  "lane/feed",
  "untitled",
  "w/literal",
  "w/wheels",
  "w/bench-replay",
  "literal/parser",
  "ui-audit",
  "ui-research-tui",
  "ui-research-gui",
  "gui/desktop",
  "gui/web",
  "gui/editor",
  "gui/notebook",
  "tui-rivals/first",
  "tui-rivals/second",
  "tui-rivals/third",
  "tui-rivals/fourth",
];

/** The sentences of a message, a reply, a brief or a value. */
const SAY = [
  "The board holds three open items, and the first one names the panels of the sample skill.",
  "Each lane reads its brief, makes its change on a branch of its own, and runs the checks before it reports.",
  "The checks pass on the branch, and the pull request waits for its review.",
  "The labels of the panel read as the plan names them, and the old ones go.",
  "A lane that finds its item done already says so, and the conductor closes the item.",
  "The sample file holds twelve lines, each a row of the board with its rank and its state.",
  "The minimal panels take the colors of the theme, and the contrast stays readable on both palettes.",
  "The flow of an answer starts at the question, shows the choices, and ends at the value that it gives.",
  "The next lane starts once the merge of the last one is green.",
  "Nothing waits for the operator, and the board moves on.",
  "The review asks for one more test of the empty board, which the lane adds.",
  "The bench of the sample reads the same numbers as before the change.",
];

/** The statements of a word of a rung, as a model of the scripted life writes them. */
const CODE = [
  'board = await read("board.md")',
  'notes = (await read("notes.md")).content',
  'report = await bash("make check", timeout=600)',
  'lane = chain("lane/panels")',
  'brief = "Rename the labels of the sample panel as the plan names them, then run the checks."',
  "items = [line for line in board.lines if line.startswith('- [ ]')]",
  "done = await prompt(str, brief, on=lane)",
  "await wait(300, done)",
  'debug(t"{len(items)=}")',
  'status = await bash("git status --short", timeout=60)',
  'close("The board moved: one item merged, two wait.")',
  "rank = sorted(items, key=len)[:3]",
];

/** The lines that a command prints. */
const PRINTED = [
  "Checked 98 files in 97 ms. No fixes applied.",
  "ok: 58 tests passed in 4.2 s",
  "M  sample/panel.ts",
  "board.md  notes.md  plan.md  rules.md",
  "[lane/panels 3f2a1c0] Rename the labels of the sample panel",
  "Everything up-to-date",
  "12 board.md",
  "the bench reads 41 ms for the first frame of the sample",
];

/** The commands of the scripted life. */
const COMMANDS = [
  "make check",
  "git status --short",
  "git log --oneline -5",
  "wc -l board.md notes.md plan.md",
  "bun run test sample/panel.test.ts",
  "git push -q origin lane/panels",
  "ls sample",
  "bun run bench",
];

/** The tokens of the engine, which the record keeps as they are: names of acts, kinds, models, states and shapes. */
const TOKEN =
  /^(?:|[a-z_]+\d+(?:_[a-z_]+)?|[a-z_]+|operator|journal|extensions|claude-cli:[a-z]+(?:\/[a-z]+)?|[A-Z][A-Za-z]*(?:\[[\w, |[\]]*\])?|str|None|tuple\[[\w, ]+\]|~\/src\/github\.com\/example\/furb|\(removed\))$/;
/** A text of the engine about acts alone: "bash4 done", "prompt3 paused", "#rung35 refused". */
const NOTICE = /^#?[a-z_]+\d+(?: [a-z]+)*$/;
/** The kinds of fact whose fourth place holds a text of the life: a value, a close, a note and a line of output. */
const TEXT = new Set(["done", "close", "tell", "out"]);
/** The mark of the lines that a cut text left out, which the record keeps as the readers count it. */
const MORE = /^… \d+ more lines$/;

/** A number for a key, the same on each run. */
function hashed(key: string): number {
  let hash = 2166136261;
  for (let at = 0; at < key.length; at++) hash = Math.imul(hash ^ key.charCodeAt(at), 16777619) >>> 0;
  return hash;
}

/** A line of a vocabulary of exactly a length, so that the record keeps its weight and a second pass writes the same
 * line again: the words that the key picks, cut at the length. */
function line(words: readonly string[], key: string, length: number): string {
  let out = "";
  for (let more = 0; out.length < length; more++)
    out += `${more ? " " : ""}${words[hashed(`${key}+${more}`) % words.length]}`;
  return out.slice(0, length).replace(/\s+$/, (gap) => ".".repeat(gap.length));
}

/** The lines of a text that the engine writes, which the record keeps as they are: a header of a note, a tag of a text
 * that holds many lines, the mark of a cut, and a bare name of an act. */
const HEADER =
  /^(?:#\S+(?: .*)?|<\/?s:\w+>|… \d+ more lines|[a-z_]+\d+(?: [a-z]+)*|\w+: Act\[[^\]]*\] = Act\('\w+'\))$/;
/** A line that binds a name of an act: the engine writes the name, and the value of it is what the life wrote. */
const BOUND = /^(\s*(?:[a-z_]+\d+_\w+|[a-z_]+\d+) = )(.*)$/;

/** A text written again, line by line, from a vocabulary: the lines that the engine writes stay, the value of a line
 * that binds a name of an act is a string of the vocabulary, and each other line is a line of the vocabulary. */
function written(text: string, key: string, words: readonly string[], code = false): string {
  return text
    .split("\n")
    .map((one, at) => {
      if (!one.trim() || HEADER.test(one.trim())) return one;
      const bound = BOUND.exec(one);
      if (bound) return `${bound[1]}"${line(SAY, `${key}:${at}`, Math.max(1, (bound[2] ?? "").length - 2))}"`;
      const indent = /^\s*/.exec(one)?.[0] ?? "";
      return `${indent}${line(code ? CODE : words, `${key}:${at}`, one.length - indent.length)}`;
    })
    .join("\n");
}

/** A string of a fact written again, by where it stands in the fact. */
function string(text: string, key: string, kind: string, at: number): string {
  if (TOKEN.test(text) || NOTICE.test(text) || SCRIPTED.includes(text)) return text;
  if (/^https?:\/\//.test(text)) return `https://example.com/docs/${hashed(key) % 1000}`;
  if ((kind === "rung" && at === 4) || (kind === "run" && at === 5)) return written(text, key, CODE, true);
  if (kind === "bash" && at === 4)
    return line(COMMANDS, key, Math.min(text.length, 60)).slice(0, text.length);
  if (kind === "out") return written(text, key, PRINTED);
  return written(text, key, SAY);
}

/** A value of a fact written again: each string in it that is no token, and each key of it kept. */
function value(one: unknown, key: string, kind: string, at: number): unknown {
  if (typeof one === "string") return string(one, key, kind, at);
  if (Array.isArray(one)) return one.map((item, index) => value(item, `${key}.${index}`, kind, at));
  if (one && typeof one === "object")
    return Object.fromEntries(
      Object.entries(one).map(([name, item]) => [
        name,
        value(item, `${key}.${name}`, name === "stdout" || name === "stderr" ? "out" : kind, at),
      ]),
    );
  return one;
}

/** The record made again: its first line as it is, each fact with its texts written from the scripted vocabulary. */
export function scripted(record: string): string {
  let chains = 0;
  return record
    .split("\n")
    .map((text, index) => {
      if (!text.startsWith("[")) return text;
      const fact = JSON.parse(text) as unknown[];
      const kind = String(fact[0]);
      if (kind === "chain") fact[4] = LABELS[chains++] ?? `lane/${chains}`;
      return JSON.stringify(
        fact.map((one, at) =>
          at < 3 || (at === 3 && !TEXT.has(kind)) || (kind === "chain" && at === 4)
            ? one
            : value(one, `${index}.${at}`, kind, at),
        ),
      );
    })
    .join("\n");
}

if (import.meta.main) {
  const path = process.argv[2] ?? join(import.meta.dir, "..", "fixtures", "conductor.jsonl");
  writeFileSync(process.argv[3] ?? path, scripted(readFileSync(path, "utf8")));
}

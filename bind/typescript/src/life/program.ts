import { type Exit, failed, type LiveAct } from "./acts.js";
import { actState, tailOf, valueText } from "./ladder.js";
import { reason } from "./plain.js";
import { placed } from "./source.js";

/** One line of the program of a thread, as the feed shows it.
 *
 * - `code`: a line of a word, with the acts that it made, whether it runs now, why it failed, and its hint: what its
 *   acts came to in a few words.
 * - `quote`: a quote of a word, with its name, value and original source for display as a string binding.
 * - `output`: a line that a command prints while its line runs.
 * - `writes`: what the model writes while its word has not come yet.
 * - `note`: a quiet comment: a message that came to the thread.
 * - `earlier`: the steps before those that the program keeps, as their count, which stands first.
 *
 * Each line names its step: the rung of its word, or the prompt of its note. */
export type ProgramLine =
  | {
      kind: "code";
      step: LiveAct;
      number: number;
      /** The number in the thread, separate from the line in the original step. */
      display?: number;
      /** The original source when one display line holds more than one source line. */
      source?: string;
      /** Stable display-row identity; never a source/refusal coordinate. */
      projection?: string;
      text: string;
      acts: LiveAct[];
      runs: boolean;
      fails?: string;
      hint: string;
    }
  | {
      kind: "quote";
      step: LiveAct;
      number: number;
      display?: number;
      text: string;
      name: string;
      lines: number;
      value: string;
      source: string;
    }
  | { kind: "output"; step: LiveAct; act: LiveAct; text: string }
  | { kind: "writes"; step: LiveAct; text: string; display?: number }
  | { kind: "note"; step: LiveAct; text: string }
  | { kind: "earlier"; step: LiveAct; count: number; text: string };

/** The verbs whose acts stand on the line that calls them. */
const followed = new Set(["bash", "read", "write", "prompt", "chain", "wait", "rung", "grant", "cd"]);
/** How many lines that a command prints its line shows while it runs. */
export const OUTPUT_LINES = 5;

const lineCount = (text: unknown) =>
  typeof text === "string" && text ? text.replace(/\n$/, "").split("\n").length : 0;
const first = (text: string) =>
  text
    .split("\n")
    .find((line) => line.trim())
    ?.trim() ?? "";
const lines = (n: number) => `${n} ${n === 1 ? "line" : "lines"}`;

/** What one act came to, in a few words. A notice to the operator says its message. */
function said(act: LiveAct, owed: ReadonlySet<string>, label: (chain: string) => string): string {
  const state = actState(act, owed);
  if (state === "asks") return "asks you";
  if (state === "failed") return first(reason(act) ?? "") || "failed";
  if (state === "cancelled") return "cancelled";
  if (state === "held") return "paused";
  if (act.kind === "bash") {
    if (!act.done) return "runs";
    const exit = act.value as Exit | undefined;
    if (exit?.code === null || exit?.code === undefined) return "timed out";
    const out = lineCount(exit.stdout?.content);
    return `exit ${exit.code}${out ? `, ${lines(out)}` : ""}`;
  }
  if (act.kind === "read" || act.kind === "write") {
    const text = (act.kind === "write" ? act.words[0] : act.value) as
      | { content?: string; lines?: number }
      | undefined;
    // A host that leaves the text out of an act keeps the count of its lines, and for a write what its change added and
    // removed.
    const kept = act as { count?: number; lines?: { added: number; removed: number } };
    if (act.kind === "write" && kept.lines) return `+${kept.lines.added} -${kept.lines.removed}`;
    const n =
      typeof kept.count === "number"
        ? kept.count
        : typeof text?.lines === "number"
          ? text.lines
          : lineCount(text?.content);
    const path = String((text as { path?: string } | undefined)?.path ?? act.words[0] ?? "");
    const name = path.split("/").at(-1) ?? path;
    if (act.kind === "write") return n ? `wrote ${lines(n)}` : `wrote ${name}`;
    return n ? lines(n) : name;
  }
  if (act.kind === "prompt") {
    if (act.words[0] === "None") return first(String(act.words[1] ?? ""));
    if (act.done) return first(valueText(act.value)) || "done";
    return act.words[2] === "operator" ? "waits for you" : `waits for ${label(act.on)}`;
  }
  if (act.kind === "chain") return label(act.id);
  if (act.kind === "wait") return act.done ? `woke: ${valueText(act.value)}` : `waits ${act.words[0] ?? ""}s`;
  if (act.kind === "cd") return valueText(act.value);
  if (act.kind === "grant") return `grant $${act.words[0] ?? ""}`;
  return "";
}

/** What the acts of a line came to, as a quiet hint at its end: one act in a few words, and a loop as its count and
 * what its last turn came to, as "x6  exit 0". */
export function hintOf(
  acts: readonly LiveAct[],
  owed: ReadonlySet<string>,
  label: (chain: string) => string,
): string {
  const last = acts.at(-1);
  if (!last) return "";
  if (acts.length > 1 && acts.every((act) => act.kind === last.kind))
    return `x${acts.length}  ${said(last, owed, label)}`;
  return acts
    .map((act) => said(act, owed, label))
    .filter(Boolean)
    .join("  ");
}

/** The lines of one step: its word, with each quote folded, the acts and the hint of each line, the line that runs
 * with the output that streams under it, and the line that a refusal names. A step whose word has not come shows what
 * its model writes. */
export function stepLines(
  step: LiveAct,
  made: ReadonlyMap<string, readonly LiveAct[]>,
  word: (rung: LiveAct) => string,
  owed: ReadonlySet<string>,
  label: (chain: string) => string,
  streams?: ReadonlyMap<string, { text: string; thinking: string }>,
  shaped?: Shaped,
): ProgramLine[] {
  const out: ProgramLine[] = [];
  const text = word(step);
  const why = reason(step);
  if (!text) {
    if (why) out.push({ kind: "note", step, text: `# ${step.id} failed: ${first(why)}` });
    else if (!step.done) {
      const stream = streams?.get(step.id);
      const tail = (stream?.text || stream?.thinking || "").replace(/\n+$/, "").split("\n").at(-1) ?? "";
      out.push({ kind: "writes", step, text: tail || "writes a step" });
    }
    return out;
  }
  const children = made.get(step.id) ?? [];
  const on = placed(text, children);
  const running = actState(step, owed) === "running";
  const open = running ? children.findLast((act) => !act.done && followed.has(act.kind)) : undefined;
  const source = text.split("\n");
  const compact = compactRows(text);
  const failing = why ? Number(why.match(/\bline (\d+)/)?.[1] ?? source.length) : 0;
  for (let index = 0; index < source.length; index++) {
    const line = source[index] ?? "";
    const quote = line.match(/^<s:(\w+)>(.*)$/);
    const end = quote ? source.findIndex((one, at) => at >= index && one.endsWith(`</s:${quote[1]}>`)) : -1;
    if (quote && end >= 0) {
      const body = [quote[2] ?? "", ...source.slice(index + 1, end + 1)].join("\n").replace(/<\/s:\w+>$/, "");
      out.push({
        kind: "quote",
        step,
        number: index + 1,
        text: first(body),
        name: quote[1] ?? "",
        lines: lineCount(body.replace(/^\n/, "")),
        value: body.replace(/^\n/, ""),
        source: source.slice(index, end + 1).join("\n"),
      });
      index = end;
      continue;
    }
    const row = compact.get(index);
    if (row?.skip) continue;
    const endRow = row?.end ?? index;
    const acts = on.slice(index, endRow + 1).flat();
    const runs = Boolean(open && acts.includes(open));
    out.push({
      kind: "code",
      step,
      number: index + 1,
      text: row?.text ?? line,
      ...(endRow > index ? { source: source.slice(index, endRow + 1).join("\n") } : {}),
      acts,
      runs,
      ...(failing > index && failing <= endRow + 1 && why ? { fails: why } : {}),
      hint: hintOf(acts, owed, label),
    });
    if (runs && open?.kind === "bash")
      for (const printed of tailOf(open, OUTPUT_LINES).split("\n"))
        if (printed) out.push({ kind: "output", step, act: open, text: printed });
  }
  if (failed(step) && !why) out.push({ kind: "note", step, text: `# ${step.id} failed` });
  let display = 0;
  return project(out, text, shaped).map((line) =>
    line.kind === "code" || line.kind === "quote" ? { ...line, display: ++display } : line,
  );
}

/** The program of a thread: the words of its steps, in the order they began, as one program. A thread is its prompt
 * and the messages that came to it, and each message stands as a quiet comment among the steps. */
export function threadProgram(
  prompts: readonly LiveAct[],
  made: ReadonlyMap<string, readonly LiveAct[]>,
  word: (rung: LiveAct) => string,
  owed: ReadonlySet<string>,
  label: (chain: string) => string,
  streams?: ReadonlyMap<string, { text: string; thinking: string }>,
  keep?: number,
  shape?: (step: LiveAct) => Shaped | undefined,
): ProgramLine[] {
  let steps: LiveAct[] = [];
  for (const [at, prompt] of prompts.entries()) {
    if (at > 0) steps.push(prompt);
    for (const rung of made.get(prompt.id) ?? []) if (rung.kind === "rung") steps.push(rung);
  }
  steps.sort((a, b) => a.began - b.began);
  const out: ProgramLine[] = [];
  const offsets = new Map<string, number>();
  const shapes = new Map<string, Shaped | undefined>();
  // The latest words get the first available formatter slots, even in a folded history of many old steps.
  for (const step of [...steps].reverse())
    if (step.kind === "rung" && step.done) shapes.set(step.id, shape?.(step));
  let number = 0;
  for (const step of steps) {
    offsets.set(step.id, number);
    if (step.kind === "rung") {
      const source = word(step);
      const compact = compactRows(source);
      let count = source.split("\n").filter((_, at) => !compact.get(at)?.skip).length;
      for (const [from, part] of shapes.get(step.id) ?? []) {
        for (let at = from; at <= part.to; at++) if (!compact.get(at)?.skip) count--;
        const folded = compactRows(part.lines.join("\n"));
        count += part.lines.filter((_, at) => !folded.get(at)?.skip).length;
      }
      number += Math.max(1, count);
    }
  }
  const positions = new Map<string, number>();
  const numbered = (line: ProgramLine): ProgramLine => {
    if (line.kind !== "code" && line.kind !== "quote" && line.kind !== "writes") return line;
    const next = (positions.get(line.step.id) ?? offsets.get(line.step.id) ?? 0) + 1;
    positions.set(line.step.id, next);
    return { ...line, display: next };
  };
  // A thread of many steps makes the lines of its last steps alone, and the messages that came before them fold with
  // the earlier steps.
  const rungs = steps.filter((step) => step.kind === "rung");
  const head = keep !== undefined && rungs.length > keep ? rungs[rungs.length - keep] : undefined;
  if (head) {
    const count = rungs.length - (keep ?? 0);
    out.push({
      kind: "earlier",
      step: head,
      count,
      text: `# ${count} earlier ${count === 1 ? "step" : "steps"}`,
    });
    steps = steps.slice(steps.indexOf(head));
  }
  // The notices that a chain gives a thread in a row, as the end of each act of the operator, stand on one comment.
  let notices: string[] = [];
  const flush = (step: LiveAct | undefined) => {
    if (step && notices.length) out.push({ kind: "note", step, text: `# ${notices.join("; ")}` });
    notices = [];
  };
  let last: LiveAct | undefined;
  for (const step of steps) {
    const message = first(String(step.words[1] ?? ""));
    if (step.kind === "prompt" && step.by !== "operator") {
      // The end of an act shows on its own line, so a notice that says only that an act is done tells nothing more.
      if (!/^\w+\d+ done$/.test(message)) notices.push(message);
      last = step;
      continue;
    }
    flush(last);
    if (step.kind === "prompt") out.push({ kind: "note", step, text: `# you: ${message}` });
    else out.push(...stepLines(step, made, word, owed, label, streams, shapes.get(step.id)).map(numbered));
  }
  flush(last);
  return out;
}

/** A value of an act as python writes it. */
function python(value: unknown): string {
  if (value === null || value === undefined) return "None";
  if (typeof value === "boolean") return value ? "True" : "False";
  if (typeof value === "number") return String(value);
  return JSON.stringify(typeof value === "string" ? value : String(value));
}

/** An act that no word made, as the operator or an ear makes it, as one line of python: its call, bound to its name,
 * with its hint, lit while it runs, and the last lines that its command prints under it. */
export function actLines(
  act: LiveAct,
  owed: ReadonlySet<string>,
  label: (chain: string) => string,
): ProgramLine[] {
  // A command shows its text alone, since its input and its timeout are those of each command.
  const words = act.kind === "bash" ? act.words.slice(0, 1) : [...act.words];
  while (words.length && (words.at(-1) === "" || words.at(-1) === null || words.at(-1) === undefined))
    words.pop();
  const runs = actState(act, owed) === "running";
  const out: ProgramLine[] = [
    {
      kind: "code",
      step: act,
      number: 1,
      text: `${act.id} = ${act.kind}(${words.map(python).join(", ")})`,
      acts: [act],
      runs,
      hint: hintOf([act], owed, label),
    },
  ];
  if (runs && act.kind === "bash")
    for (const printed of tailOf(act, OUTPUT_LINES).split("\n"))
      if (printed) out.push({ kind: "output", step: act, act, text: printed });
  return out;
}

/** A string literal on one line of python: where its text starts and ends, inside its quotes, and whether it is raw. */
export type Literal = { start: number; end: number; raw: boolean };

/** The string literals of one line of python, in order, each from its first character to its last. A comment ends the
 * line, and a literal that does not end on the line runs to its end. */
export function literals(line: string): Literal[] {
  const found: Literal[] = [];
  for (let at = 0; at < line.length; at++) {
    const char = line[at] ?? "";
    if (char === "#") break;
    if (char !== '"' && char !== "'") continue;
    const prefix = line.slice(0, at).match(/[rRbBuUfFtT]{0,2}$/)?.[0] ?? "";
    if (/[\w]/.test(line[at - prefix.length - 1] ?? "")) continue;
    const mark = line.startsWith(char.repeat(3), at) ? char.repeat(3) : char;
    let end = at + mark.length;
    while (end < line.length && !line.startsWith(mark, end)) end += line[end] === "\\" ? 2 : 1;
    // A string that does not end on the line, as the first line of a long quote of three marks, runs to its end.
    found.push({ start: at + mark.length, end: Math.min(end, line.length), raw: /[rR]/.test(prefix) });
    if (end >= line.length) break;
    at = end + mark.length - 1;
  }
  return found;
}

/** The marks that stand for the end of a string that a line folds. */
export const FOLD = "...";
/** How many characters of a string stay before its fold. */
const KEEP = 12;

/** A string that a line folds: which literal of the line it is, where its fold stands in the text that shows, and its
 * whole text. */
export type Fold = { literal: number; at: number; whole: string };

/** A line of python that fits its room: the longest strings first keep only their start and a fold, until the line
 * fits. A string that the operator opened stays whole, and a line that still does not fit stays longer than its room.
 * The text of a literal is its python, with its escapes. */
export function foldStrings(
  line: string,
  room: number,
  open: ReadonlySet<number> = new Set(),
  width: (text: string) => number = (text) => text.length,
): { text: string; folds: Fold[] } {
  let excess = width(line) - room;
  if (excess <= 0) return { text: line, folds: [] };
  const found = literals(line);
  const keep = new Map<number, number>();
  const order = [...found.keys()]
    .filter((index) => !open.has(index))
    .sort(
      (a, b) =>
        (found[b]?.end ?? 0) - (found[b]?.start ?? 0) - ((found[a]?.end ?? 0) - (found[a]?.start ?? 0)),
    );
  for (const index of order) {
    if (excess <= 0) break;
    const one = found[index];
    if (!one) continue;
    const size = one.end - one.start;
    const kept = Math.max(KEEP, size - excess - FOLD.length);
    if (kept + FOLD.length >= size) continue;
    let boundary = one.start + kept;
    const target = width(line.slice(one.start, one.end)) - excess - width(FOLD);
    while (boundary > one.start + KEEP && width(line.slice(one.start, boundary)) > target) boundary--;
    if (/[\uDC00-\uDFFF]/.test(line[boundary] ?? "")) boundary--;
    while (boundary > one.start && line[boundary - 1] === "\\") boundary--;
    keep.set(index, boundary - one.start);
    excess -= width(line.slice(one.start, one.end)) - width(line.slice(one.start, boundary)) - width(FOLD);
  }
  let text = "";
  let from = 0;
  const folds: Fold[] = [];
  for (const [index, one] of found.entries()) {
    const kept = keep.get(index);
    if (kept === undefined) continue;
    text += line.slice(from, one.start) + line.slice(one.start, one.start + kept).trimEnd();
    folds.push({ literal: index, at: text.length, whole: line.slice(one.start, one.end) });
    text += FOLD;
    from = one.end;
  }
  return { text: text + line.slice(from), folds };
}

/** Display rows by original statement start, with the last source line of each statement. */
export type Shaped = ReadonlyMap<number, { to: number; lines: readonly string[] }>;

/** A top-level statement of a word, as the range of its lines, from 0. A quote of the word is a statement of its own. */
export type Statement = { from: number; to: number; quote: boolean };

/** The top-level statements of a word, in order. A statement holds the lines that its brackets, its strings, its
 * continuations, its decorators and its indented body take, and the blank lines after it. */
export function statements(source: string): Statement[] {
  const lines = source.split("\n");
  const out: Statement[] = [];
  let depth = 0;
  let open = "";
  let continued = false;
  let decorated = false;
  for (let index = 0; index < lines.length; index++) {
    const line = lines[index] ?? "";
    const quote = !depth && !open && !continued ? line.match(/^<s:(\w+)>/) : null;
    const end = quote ? lines.findIndex((one, at) => at >= index && one.endsWith(`</s:${quote[1]}>`)) : -1;
    if (quote && end >= 0) {
      out.push({ from: index, to: end, quote: true });
      index = end;
      continue;
    }
    const last = out.at(-1);
    const top =
      !depth &&
      !open &&
      !continued &&
      /^\S/.test(line) &&
      !/^(else|elif|except|finally)\b/.test(line) &&
      !decorated;
    if (top || !last || last.quote) out.push({ from: index, to: index, quote: false });
    else last.to = index;
    if (!depth && !open && !continued && /^\S/.test(line)) decorated = line.startsWith("@");
    for (let at = 0; at < line.length; at++) {
      const char = line[at] ?? "";
      if (open) {
        if (char === "\\") at++;
        else if (line.startsWith(open, at)) {
          at += open.length - 1;
          open = "";
        }
        continue;
      }
      if (char === "#") break;
      if (char === '"' || char === "'") open = line.startsWith(char.repeat(3), at) ? char.repeat(3) : char;
      if (open.length === 3) at += 2;
      else if ("([{".includes(char)) depth++;
      else if (")]}".includes(char)) depth = Math.max(0, depth - 1);
    }
    // A string of one mark ends with its line, and a backslash at the end of a line goes on to the next.
    if (open.length === 1) open = "";
    continued = !open && line.endsWith("\\");
  }
  return out;
}

/** The mark that parts the statements of a word in the program that a formatter of python reads. */
const PART = "# furb:statement";
const marker = (source: string) => {
  let mark = PART;
  for (let suffix = 1; source.includes(mark); suffix++) mark = `${PART}-${suffix}`;
  return mark;
};

/** The python statements of a word as one program for a formatter, each after a mark, with the statements that the
 * formatter cannot read left out. */
export function formatInput(
  source: string,
  parts: readonly Statement[],
  skip: ReadonlySet<number> = new Set(),
): string {
  const lines = source.split("\n");
  return parts
    .flatMap((part, index) =>
      part.quote || skip.has(index)
        ? []
        : [marker(source), ...lines.slice(part.from, part.to + 1)].join("\n").trimEnd(),
    )
    .join("\n");
}

/** What a formatter gave for the statements of a word, as the lines of each statement that it changed, by the index of
 * its statement. It gives nothing when the formatter lost a mark. */
export function formatOutput(
  source: string,
  parts: readonly Statement[],
  formatted: string,
  skip: ReadonlySet<number> = new Set(),
): Map<number, string[]> | undefined {
  const lines = source.split("\n");
  const shown = parts.flatMap((part, index) => (part.quote || skip.has(index) ? [] : [index]));
  const mark = marker(source);
  const pieces = formatted.split(new RegExp("^" + mark + "\\n?", "m")).slice(1);
  if (pieces.length !== shown.length) return undefined;
  const out = new Map<number, string[]>();
  for (const [at, index] of shown.entries()) {
    const part = parts[index];
    const made = (pieces[at] ?? "").replace(/^\n+|\n+$/g, "").split("\n");
    const was = part
      ? lines
          .slice(part.from, part.to + 1)
          .join("\n")
          .replace(/\n+$/, "")
      : "";
    if (part && made.join("\n") !== was) out.set(index, made);
  }
  return out;
}

/** The statement of a word that holds a line of the program that a formatter read, from 1, as a refusal names it. */
export function statementAt(
  parts: readonly Statement[],
  source: string,
  line: number,
  skip: ReadonlySet<number> = new Set(),
): number {
  const lines = source.split("\n");
  let row = 0;
  for (const [index, part] of parts.entries()) {
    if (part.quote || skip.has(index)) continue;
    const size =
      1 +
      lines
        .slice(part.from, part.to + 1)
        .join("\n")
        .trimEnd()
        .split("\n").length;
    if (line <= row + size) return index;
    row += size;
  }
  return -1;
}

/** Display rows of Python with complete multiline literals on one escaped line. Incomplete literals stay as source,
 * so a stream never invents a closing mark or hides what has not arrived. */
function compactRows(source: string): Map<number, { text?: string; end?: number; skip?: boolean }> {
  const rows = new Map<number, { text?: string; end?: number; skip?: boolean }>();
  let line = 0;
  let start = 0;
  let shown = "";
  let firstLine = 0;
  for (let at = 0; at < source.length; ) {
    const char = source[at] ?? "";
    const quote = at === start ? source.slice(at).match(/^<s:(\w+)>/) : undefined;
    if (quote) {
      const close = new RegExp(`</s:${quote[1]}>$`, "m");
      const end = close.exec(source.slice(at));
      if (!end) break;
      const to = at + end.index + end[0].length;
      const count = source.slice(at, to).split("\n").length - 1;
      for (let k = 1; k <= count; k++) rows.set(line + k, { skip: true });
      line += count;
      firstLine = line;
      at = to;
      continue;
    }
    if (char === "#") {
      const end = source.indexOf("\n", at);
      at = end < 0 ? source.length : end;
      continue;
    }
    if (char === "\n") {
      if (line > firstLine) rows.set(firstLine, { text: shown + source.slice(start, at), end: line });
      line++;
      firstLine = line;
      start = at + 1;
      shown = "";
      at++;
      continue;
    }
    if (char !== "'" && char !== '"') {
      at++;
      continue;
    }
    const triple = source.startsWith(char.repeat(3), at);
    const mark = triple ? char.repeat(3) : char;
    let end = at + mark.length;
    while (end < source.length && !source.startsWith(mark, end)) end += source[end] === "\\" ? 2 : 1;
    if (end >= source.length) break;
    const body = source.slice(at + mark.length, end);
    if (triple && body.includes("\n")) {
      const prefix = source.slice(start, at).match(/(?<![\w])[rRbBuUfFtT]{1,2}$/)?.[0] ?? "";
      const raw = /r/i.test(prefix);
      const literal = raw
        ? JSON.stringify(body)
        : `"${body.replace(/\\[\s\S]|["\n\r]/g, (part) =>
            part === "\n"
              ? "\\n"
              : part === "\r"
                ? "\\r"
                : part === '"'
                  ? '\\"'
                  : part === "\\\n"
                    ? ""
                    : part,
          )}"`;
      shown += source.slice(start, at - prefix.length) + prefix.replace(/[rR]/g, "") + literal;
      const count = body.split("\n").length - 1;
      for (let k = 1; k <= count; k++) rows.set(line + k, { skip: true });
      line += count;
      start = end + mark.length;
    }
    at = end + mark.length;
  }
  if (line > firstLine) rows.set(firstLine, { text: shown + source.slice(start), end: line });
  return rows;
}

/** Replace only the display rows of a statement. Its acts and failure stay on its first row, with the exact
 * original range available for source inspection. No displayed offset is ever an engine coordinate. */
function project(rows: ProgramLine[], source: string, shaped?: Shaped): ProgramLine[] {
  if (!shaped?.size) return rows;
  const out: ProgramLine[] = [];
  const original = source.split("\n");
  for (let at = 0; at < rows.length; at++) {
    const row = rows[at];
    if (!row || row.kind !== "code") {
      if (row) out.push(row);
      continue;
    }
    const part = shaped.get(row.number - 1);
    if (!part) {
      out.push(row);
      continue;
    }
    const owned = [row];
    const outputs: ProgramLine[] = [];
    while (at + 1 < rows.length) {
      const next = rows[at + 1];
      if (next?.kind === "code" && next.number <= part.to + 1) {
        owned.push(next);
        at++;
      } else if (next?.kind === "output") {
        outputs.push(next);
        at++;
      } else break;
    }
    const text = part.lines.join("\n");
    const compact = compactRows(text);
    const acts = owned.flatMap((one) => one.acts);
    const failure = owned.find((one) => one.fails);
    let first = true;
    for (const [index, line] of part.lines.entries()) {
      const folded = compact.get(index);
      if (folded?.skip) continue;
      out.push({
        ...row,
        number: failure?.number ?? row.number,
        projection: `${row.number}:${index}`,
        source: original.slice(row.number - 1, part.to + 1).join("\n"),
        text: folded?.text ?? line,
        acts: first ? acts : [],
        runs: first && owned.some((one) => one.runs),
        fails: first ? failure?.fails : undefined,
        hint: first
          ? owned
              .map((one) => one.hint)
              .filter(Boolean)
              .join("  ")
          : "",
      });
      first = false;
    }
    out.push(...outputs);
  }
  return out;
}

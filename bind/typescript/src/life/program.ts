import { type Exit, failed, type LiveAct } from "./acts.js";
import { actState, type Chip, tailOf, valueText } from "./ladder.js";
import { unmarked } from "./marks.js";
import { reason } from "./plain.js";
import { placed } from "./source.js";

/** One line of the program of a thread, as the feed shows it.
 *
 * - `code`: a line of a word, with the acts that it made, whether it runs now, why it failed, and its hint: what its
 *   acts came to in a few words. A line that runs as it awaits an act that an earlier line made names that act and
 *   the line that made it in `awaits`, and that line holds the line that awaits it in `awaited`.
 * - `quote` is a `code` line too: a quote of a word stands as the binding that it makes, its name bound to its text as
 *   one string with escapes, as python writes it, and its count of lines as its hint.
 * - `output`: a line that a command prints while its line runs.
 * - `writes`: what the model writes while its word has not come yet.
 * - `note`: a quiet comment: a message that came to the thread.
 * - `earlier`: the steps before those that the program keeps, as their count, which stands first.
 *
 * Each line names its step: the rung of its word, or the prompt of its note. A line of code and the line that
 * a model writes hold two numbers: `number`, the line in the word of its step, and `display`, the line in the program
 * of its thread, which goes on across the steps of the thread. */
export type ProgramLine =
  | {
      kind: "code";
      step: LiveAct;
      number: number;
      display: number;
      text: string;
      acts: LiveAct[];
      runs: boolean;
      fails?: string;
      hint: string;
      awaits?: { act: LiveAct; display: number };
      awaited?: number;
    }
  | { kind: "output"; step: LiveAct; act: LiveAct; text: string }
  | { kind: "writes"; step: LiveAct; display: number; text: string }
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
  if (act.kind === "grant") return `grant $${unmarked(act.words[0]) ?? ""}`;
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

/** The acts of a line that does not fail, less the ones that failed while its word went on, as a read of a file that may
 * not stand: the word handled them, so the hint of the line says nothing of them. Every act of the line failed: all
 * stay. */
function handled(acts: readonly LiveAct[], owed: ReadonlySet<string>): readonly LiveAct[] {
  const failed = acts.filter((act) => actState(act, owed) === "failed");
  return failed.length && failed.length < acts.length ? acts.filter((act) => !failed.includes(act)) : acts;
}

/** What a model writes while its word has not come yet: the lines of the word that it finished, which are python,
 * and the line that it still writes. While it writes no word, the tail is the last line that it thinks. */
export function writing(stream?: { text: string; thinking: string }): { lines: string; tail: string } {
  const text = stream?.text ?? "";
  const end = text.lastIndexOf("\n");
  const lines = end < 0 ? "" : text.slice(0, end).replace(/\n+$/, "");
  if (lines) return { lines, tail: text.slice(end + 1).trim() };
  return {
    lines,
    tail: ((text || stream?.thinking || "").replace(/\n+$/, "").split("\n").at(-1) ?? "").trim(),
  };
}

/** The lines of one step: its word, with each quote folded, the acts and the hint of each line, the line that runs
 * with the output that streams under it, and the line that a refusal names. A step whose word has not come shows what
 * its model writes. `start` is the count of the lines of the thread before the step, which its numbers go on from. */
export function stepLines(
  step: LiveAct,
  made: ReadonlyMap<string, readonly LiveAct[]>,
  word: (rung: LiveAct) => string,
  owed: ReadonlySet<string>,
  label: (chain: string) => string,
  streams?: ReadonlyMap<string, { text: string; thinking: string }>,
  start = 0,
): ProgramLine[] {
  const out: ProgramLine[] = [];
  const text = word(step);
  const why = reason(step);
  if (!text) {
    if (why) out.push({ kind: "note", step, text: `# ${step.id} failed: ${first(why)}` });
    else if (!step.done) {
      out.push({
        kind: "writes",
        step,
        display: start + 1,
        text: writing(streams?.get(step.id)).tail || "thinking",
      });
    }
    return out;
  }
  const children = made.get(step.id) ?? [];
  const on = placed(text, children);
  const running = actState(step, owed) === "running";
  const open = running ? children.findLast((act) => !act.done && followed.has(act.kind)) : undefined;
  const source = text.split("\n");
  // A line that awaits the act of the name that an earlier line binds, as `b = await build` after
  // `build = bash(...)`, is where the word stands while that act runs.
  const maker = open ? on.findIndex((acts) => acts.includes(open)) : -1;
  const bound = maker >= 0 ? source[maker]?.match(/^\s*(\w+)\s*=(?!=)(?![^#]*\bawait\b)/)?.[1] : undefined;
  const waits = bound
    ? source.findIndex(
        (line, at) => at > maker && new RegExp(`\\bawait\\s+${bound}\\b`).test(line.split("#")[0] ?? ""),
      )
    : -1;
  const failing = why ? Number(why.match(/\bline (\d+)/)?.[1] ?? source.length) : 0;
  for (let index = 0; index < source.length; index++) {
    const line = source[index] ?? "";
    const quote = line.match(/^<s:(\w+)>(.*)$/);
    const end = quote ? source.findIndex((one, at) => at >= index && one.endsWith(`</s:${quote[1]}>`)) : -1;
    if (quote && end >= 0) {
      // The word binds the text between the marks, less the line break that follows the opening mark.
      const body = [quote[2] ?? "", ...source.slice(index + 1, end + 1)]
        .join("\n")
        .replace(/<\/s:\w+>$/, "")
        .replace(/^\n/, "");
      out.push({
        kind: "code",
        step,
        number: index + 1,
        display: start + index + 1,
        text: `${quote[1]} = ${pythonString(body)}`,
        acts: [],
        runs: false,
        hint: lines(lineCount(body)),
      });
      index = end;
      continue;
    }
    // A string of three marks that runs over many lines stands in its statement as one string with its escapes.
    const joined = /"""|'''/.test(line) ? joinedLines(source, index) : undefined;
    const last = joined?.end ?? index;
    const here = index;
    const acts = last === index ? (on[index] ?? []) : [...new Set(on.slice(index, last + 1).flat())];
    const runs = waits >= 0 ? index === waits : Boolean(open && acts.includes(open));
    const awaits = index === waits && open ? { act: open, display: start + maker + 1 } : undefined;
    const failed = Boolean(why) && failing >= index + 1 && failing <= last + 1;
    const hint = awaits
      ? `waits on ${open?.id} of line ${awaits.display}`
      : hintOf(failed ? acts : handled(acts, owed), owed, label);
    out.push({
      kind: "code",
      step,
      number: index + 1,
      display: start + index + 1,
      text: joined?.text ?? line,
      acts,
      runs,
      ...(why && failing >= index + 1 && failing <= last + 1 ? { fails: why } : {}),
      hint,
      ...(awaits ? { awaits } : {}),
      ...(waits >= 0 && here === maker ? { awaited: start + waits + 1 } : {}),
    });
    // A quote that ran stands in its word as the binding that it made, with blank lines after it that keep the numbers
    // of the lines below. Those blank lines stand in the line of the binding, which shows the lines that it took.
    let pad = 0;
    const bound = joined?.text ?? line;
    const escapes =
      bound.includes("\\n") && /^\w+ = (['"]).*\\n.*\1$/.test(bound) ? (bound.match(/\\n/g) ?? []).length : 0;
    while (pad <= escapes && escapes && source[last + 1 + pad]?.trim() === "") pad++;
    index = last + pad;
    if (open?.kind === "bash" && (waits >= 0 ? here === maker : runs))
      for (const printed of tailOf(open, OUTPUT_LINES).split("\n"))
        if (printed) out.push({ kind: "output", step, act: open, text: printed });
  }
  if (failed(step) && !why) out.push({ kind: "note", step, text: `# ${step.id} failed` });
  return out;
}

/** The program of a thread: the words of its steps, in the order they began, as one program, whose line numbers go on
 * from each step to the next, also across the steps that a fold keeps out. A thread is its prompt and the messages
 * that came to it, and each message stands as a quiet comment among the steps. */
export function threadProgram(
  prompts: readonly LiveAct[],
  made: ReadonlyMap<string, readonly LiveAct[]>,
  word: (rung: LiveAct) => string,
  owed: ReadonlySet<string>,
  label: (chain: string) => string,
  streams?: ReadonlyMap<string, { text: string; thinking: string }>,
  keep?: number,
): ProgramLine[] {
  let steps: LiveAct[] = [];
  for (const [at, prompt] of prompts.entries()) {
    if (at > 0) steps.push(prompt);
    for (const rung of made.get(prompt.id) ?? []) if (rung.kind === "rung") steps.push(rung);
  }
  steps.sort((a, b) => a.began - b.began);
  // The count of the lines of the thread before each step, which the numbers of the step go on from.
  const starts = new Map<LiveAct, number>();
  let count = 0;
  for (const step of steps) {
    if (step.kind !== "rung") continue;
    starts.set(step, count);
    count += lineCount(word(step));
  }
  const out: ProgramLine[] = [];
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
    else out.push(...stepLines(step, made, word, owed, label, streams, starts.get(step)));
  }
  flush(last);
  return out;
}

/** A text as a string of python, as repr writes it: single marks, or double marks for a text that holds a single mark
 * and no double mark. */
function pythonString(text: string): string {
  const mark = text.includes("'") && !text.includes('"') ? '"' : "'";
  let out = "";
  for (const char of text) {
    if (char === "\\") out += "\\\\";
    else if (char === mark) out += `\\${char}`;
    else if (char === "\n") out += "\\n";
    else if (char === "\r") out += "\\r";
    else if (char === "\t") out += "\\t";
    else if (char < " " || char === "\x7f") out += `\\x${char.charCodeAt(0).toString(16).padStart(2, "0")}`;
    else out += char;
  }
  return mark + out + mark;
}

/** A value of an act as python writes it. */
function python(value: unknown): string {
  if (value === null || value === undefined) return "None";
  if (typeof value === "boolean") return value ? "True" : "False";
  if (typeof value === "number") return String(value);
  if (typeof value === "string") return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(python).join(", ")}]`;
  if (typeof value !== "object") return JSON.stringify(String(value));
  // The life marks what JSON cannot hold, as a float with no fraction, with its class in `is`.
  const held = value as Record<string, unknown>;
  const args = Array.isArray(held.args) ? held.args : [];
  if (held.is === "float") {
    const digits = String(args[0]);
    return /^-?\d+$/.test(digits)
      ? `${digits}.0`
      : /inf|nan/i.test(digits)
        ? `float(${JSON.stringify(digits)})`
        : digits;
  }
  if (held.is === "int") return String(args[0]);
  if (held.is === "dict" && Array.isArray(args[0]))
    return `{${(args[0] as [unknown, unknown][]).map(([key, one]) => `${python(key)}: ${python(one)}`).join(", ")}}`;
  if (typeof held.is === "string") {
    const fields = Object.entries(held).filter(([key]) => key !== "is" && key !== "args");
    return `${held.is}(${[...args.map(python), ...fields.map(([key, one]) => `${key}=${python(one)}`)].join(", ")})`;
  }
  return `{${Object.entries(held)
    .map(([key, one]) => `${JSON.stringify(key)}: ${python(one)}`)
    .join(", ")}}`;
}

/** The program of the rungs that one maker wrote with no word of its own, as the words of the extensions that an ear of
 * the outside made or the setup that the root wrote on itself: a comment that names the ear, or setup for a chain, then
 * the calls that end each word, which are the life words of the setup, each on a line, with what the acts of the word
 * came to on the last line. A word that only defines shows no line. The last line runs while an act of the word is at
 * work, and not while an act lives that is no work, as the watcher of an extension does after it said it done. */
export function earProgram(
  ear: LiveAct,
  rungs: readonly LiveAct[],
  chips: readonly Chip[],
  made: (id: string) => readonly LiveAct[],
  subject: (rung: LiveAct) => string,
  owed: ReadonlySet<string>,
  label: (chain: string) => string,
): ProgramLine[] {
  // A chain that wrote rungs on itself, as the root writes its setup, has an id that names no maker for a reader.
  const who = ear.by === ear.on ? "setup" : ear.by;
  const out: ProgramLine[] = [{ kind: "note", step: ear, text: `# ${who}` }];
  for (const rung of rungs) {
    const source = subject(rung).trimEnd().split("\n");
    let first = source.length;
    while (first > 0 && /^(await )?[A-Za-z_][\w.]*\(/.test(source[first - 1] ?? "")) first--;
    if (first === source.length) continue;
    const acts = [...made(rung.id)];
    for (let at = first; at < source.length; at++)
      out.push({
        kind: "code",
        step: rung,
        number: at + 1,
        display: out.length,
        text: source[at] ?? "",
        acts: at === source.length - 1 ? acts : [],
        runs: at === source.length - 1 && acts.some((one) => actState(one, owed) === "running"),
        hint: at === source.length - 1 ? hintOf(acts, owed, label) : "",
      });
  }
  // A word that only defines has no line to show, so the note stands alone, with the rungs and what they made.
  if (out.length === 1) {
    const ids = rungs.map((rung) => rung.id);
    const span = ids.length > 1 ? `${ids[0]}…${(ids.at(-1) ?? "").replace(/^\D+/, "")}` : (ids[0] ?? "");
    const acts = chips.map((chip) => chip.label).join(" ");
    out[0] = { kind: "note", step: ear, text: `# ${who} ${span}  ${acts}`.trimEnd() };
  }
  return out;
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
      display: 1,
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

/** The statement that starts at a line of a word, when a string of three marks opens on it and runs past the end of the
 * line: the last line that the statement takes, and its text on one line, each such string written as one string with
 * its escapes and double marks. A raw string loses its prefix and doubles its backslashes, so it means the same. */
export function joinedLines(
  source: readonly string[],
  index: number,
): { end: number; text: string } | undefined {
  const chunk = source.slice(index).join("\n");
  const endOf = (from: number) => {
    const found = chunk.indexOf("\n", from);
    return found < 0 ? chunk.length : found;
  };
  let lineEnd = endOf(0);
  let out = "";
  let from = 0;
  let at = 0;
  let merged = false;
  while (at < lineEnd) {
    const char = chunk[at] ?? "";
    if (char === "#") break;
    if (char !== '"' && char !== "'") {
      at++;
      continue;
    }
    const prefix = chunk.slice(Math.max(0, at - 2), at).match(/[rRbBuUfFtT]{0,2}$/)?.[0] ?? "";
    if (/\w/.test(chunk[at - prefix.length - 1] ?? "")) {
      at++;
      continue;
    }
    const mark = chunk.startsWith(char.repeat(3), at) ? char.repeat(3) : char;
    let close = at + mark.length;
    while (close < chunk.length && !chunk.startsWith(mark, close)) {
      if (mark.length === 1 && chunk[close] === "\n") break;
      close += chunk[close] === "\\" ? 2 : 1;
    }
    if (close >= chunk.length && mark.length === 3) break;
    if (mark.length === 1) {
      at = Math.min(close + 1, chunk.length);
      if (at > lineEnd) lineEnd = endOf(at);
      continue;
    }
    const body = chunk.slice(at + 3, close);
    if (body.includes("\n")) {
      const raw = /[rR]/.test(prefix);
      let text = "";
      for (let k = 0; k < body.length; k++) {
        const one = body[k] ?? "";
        if (one === "\\" && !raw) {
          // A backslash before a line break joins the lines, and any other escape stays as the word wrote it.
          if (body[k + 1] !== "\n") text += one + (body[k + 1] ?? "");
          k++;
        } else if (one === "\\") text += "\\\\";
        else if (one === "\n") text += "\\n";
        else if (one !== "\r") text += one === '"' ? '\\"' : one;
      }
      out += `${chunk.slice(from, at - prefix.length)}${prefix.replace(/[rR]/g, "")}"${text}"`;
      from = close + 3;
      merged = true;
    }
    at = close + 3;
    if (at > lineEnd) lineEnd = endOf(at);
  }
  if (!merged) return undefined;
  return {
    end: index + chunk.slice(0, lineEnd).split("\n").length - 1,
    text: out + chunk.slice(from, lineEnd),
  };
}

/** A part of a line that wraps: where it starts and ends in the line, and how far its row is indented. */
export type Piece = { from: number; to: number; indent: number };

/** A line of python cut into rows of a width at most, each cut between two tokens: at a space, after a comma or after
 * an opening bracket. A row after the first is indented two columns past the line. Only a token wider than a row, as
 * a path with no space, is cut inside. */
export function wrap(line: string, width: number): Piece[] {
  const lead = line.match(/^ */)?.[0].length ?? 0;
  const indent = Math.min(lead + 2, Math.floor(width / 3));
  const pieces: Piece[] = [];
  let from = 0;
  for (;;) {
    const first = pieces.length === 0;
    const room = width - (first ? 0 : indent);
    if (line.length - from <= room) {
      pieces.push({ from, to: line.length, indent: first ? 0 : indent });
      return pieces;
    }
    let cut = from + room;
    const floor = from + (first ? lead : 0) + 1;
    for (let at = from + room; at > floor; at--) {
      const before = line[at - 1] ?? "";
      const here = line[at] ?? "";
      if ((here === " " && before !== " ") || ("([{,".includes(before) && here !== " ")) {
        cut = at;
        break;
      }
    }
    pieces.push({ from, to: cut, indent: first ? 0 : indent });
    from = cut;
    while (line[from] === " ") from++;
  }
}

/** A row of a line of python that a layout gives: its text; each part of the line that it shows, as where the part
 * starts and ends in the row and where it starts in the line; and the strings that it folds, each where its fold
 * stands in the row. */
export type CodeRow = { text: string; parts: [from: number, to: number, at: number][]; folds: Fold[] };

/** How far the rows inside a pair of brackets stand past the row that opens it, as in the words of furb. */
const STEP = 2;

/** The delimiters at which the body of a pair of brackets splits when it does not fit one row, the loosest first, as
 * ruff format splits it: after a comma, and before an operator of a condition, a boolean, a comparison or a sum. The
 * keywords of a comprehension come before a comma, so a target of two names stays on its row. */
const DELIMITERS = [
  [","],
  [" if ", " else "],
  [" or ", " and "],
  [" not in ", " is not ", " in ", " is ", " == ", " != ", " <= ", " >= ", " < ", " > "],
  [" | ", " ^ ", " & ", " << ", " >> ", " + ", " - ", " * ", " / ", " // ", " % ", " @ "],
];

/** A line of python laid out in rows of a width at most, as ruff format lays it out. The line first folds its longest
 * strings until it fits one row, but each string that open names; with no open, no string folds. A line that still
 * does not fit splits at the last pair of brackets whose opening and closing rows fit: its body stands on the rows
 * between them, one step deeper, on one row when it fits, else at its loosest delimiters, one part on each row, and
 * each part splits in the same way. A part with no brackets that does not fit wraps between its tokens. The rows add
 * and drop no character: together they hold the line, less the spaces at each cut, and each fold. */
export function layout(line: string, width: number, open?: ReadonlySet<number>): CodeRow[] {
  const { text, folds } = open ? foldStrings(line, width, open) : { text: line, folds: [] as Fold[] };
  const found = literals(line);
  // Where each character of the text stands in the line, or -1 for the marks of a fold.
  const source: number[] = [];
  let next = 0;
  for (const fold of folds) {
    while (source.length < fold.at) source.push(next++);
    for (let k = 0; k < FOLD.length; k++) source.push(-1);
    next = found[fold.literal]?.end ?? next;
  }
  while (source.length < text.length) source.push(next++);
  // Whether each character is code, and not a part of a string or of a comment.
  const code = Array.from(text, () => true);
  for (const one of literals(text))
    for (let k = Math.max(0, one.start - 1); k <= Math.min(text.length - 1, one.end); k++) code[k] = false;
  const comment = [...text].findIndex((char, k) => char === "#" && code[k]);
  if (comment >= 0) code.fill(false, comment);
  const trim = (a: number, b: number): [number, number] => {
    while (a < b && text[a] === " ") a++;
    while (b > a && text[b - 1] === " ") b--;
    return [a, b];
  };
  const fits = (a: number, b: number, indent: number) => indent + b - a <= width;
  // The pairs of brackets of a part that no other pair of the part holds.
  const pairs = (a: number, b: number) => {
    const out: [number, number][] = [];
    const opened: number[] = [];
    for (let k = a; k < b; k++) {
      const char = text[k] ?? " ";
      if (!code[k]) continue;
      if ("([{".includes(char)) opened.push(k);
      else if (")]}".includes(char) && opened.length) {
        const start = opened.pop() ?? k;
        if (!opened.length) out.push([start, k]);
      }
    }
    return out;
  };
  // Where a body cuts: after each of its loosest delimiters, or before it, at the depth of the body.
  const cutsOf = (a: number, b: number): number[] => {
    const top = new Set<number>();
    let depth = 0;
    for (let k = a; k < b; k++) {
      const char = text[k] ?? " ";
      if (!code[k]) continue;
      if ("([{".includes(char)) depth++;
      else if (")]}".includes(char)) depth = Math.max(0, depth - 1);
      else if (depth === 0) top.add(k);
    }
    const at = (word: string) =>
      [...top].filter((k) => text.startsWith(word, k) && [...word].every((_, j) => top.has(k + j)));
    const loops = [...at(" for "), ...at(" async for ")].sort((x, y) => x - y);
    if (loops.length) {
      const first = loops[0] ?? 0;
      return [...loops, ...at(" if ").filter((k) => k > first)]
        .filter((k) => !text.startsWith(" for ", k - 6))
        .sort((x, y) => x - y)
        .map((k) => k + 1);
    }
    for (const words of DELIMITERS) {
      const cuts = new Set(
        words.flatMap((word) =>
          at(word)
            .filter((k) => word !== " in " || !text.startsWith(" not", k - 4))
            .filter((k) => word !== " is " || !text.startsWith(" is not ", k))
            .map((k) => k + 1),
        ),
      );
      if (cuts.size) return [...cuts].sort((x, y) => x - y);
    }
    return [];
  };
  const tokens = (a: number, b: number, indent: number): Piece[] =>
    wrap(text.slice(a, b), Math.max(8, width - indent)).map((piece) => ({
      from: a + piece.from,
      to: a + piece.to,
      indent: indent + piece.indent,
    }));
  const lay = (start: number, end: number, indent: number): Piece[] => {
    const [a, b] = trim(start, end);
    if (a >= b) return [];
    if (fits(a, b, indent)) return [{ from: a, to: b, indent }];
    const held = pairs(a, b)
      .filter(([o, c]) => {
        const [x, y] = trim(o + 1, c);
        return x < y;
      })
      .reverse();
    const pair = held.find(([o, c]) => fits(a, o + 1, indent) && fits(...trim(c, b), indent)) ?? held[0];
    if (!pair) return tokens(a, b, indent);
    const [o, c] = pair;
    const [x, y] = trim(o + 1, c);
    const inner = indent + STEP;
    const cuts = fits(x, y, inner) ? [] : cutsOf(x, y);
    const bounds = [x, ...cuts, y];
    const body = cuts.length
      ? bounds.slice(1).flatMap((to, k) => lay(bounds[k] ?? x, to, inner))
      : lay(x, y, inner);
    return [...lay(a, o + 1, indent), ...body, ...lay(c, b, indent)];
  };
  const lead = text.match(/^ */)?.[0].length ?? 0;
  const pieces = lay(0, text.length, lead);
  if (!pieces.length) return [{ text, parts: text ? [[0, text.length, 0]] : [], folds: [] }];
  return pieces.map(({ from, to, indent }) => {
    const parts: CodeRow["parts"] = [];
    for (let k = from; k < to; k++) {
      const at = source[k] ?? -1;
      const last = parts.at(-1);
      if (at < 0) continue;
      if (last && last[1] === indent + k - from && (source[k - 1] ?? -2) === at - 1) last[1]++;
      else parts.push([indent + k - from, indent + k - from + 1, at]);
    }
    return {
      text: " ".repeat(indent) + text.slice(from, to),
      parts,
      folds: folds
        .filter((fold) => fold.at >= from && fold.at < to)
        .map((fold) => ({ ...fold, at: indent + fold.at - from })),
    };
  });
}

/** The start of a string that a fold keeps, less an escape that its end would cut. */
function uncut(part: string): string {
  if ((part.match(/\\*$/)?.[0].length ?? 0) % 2 === 1) return part.slice(0, -1);
  return part
    .replace(/\\(?:u[0-9a-fA-F]{0,3}|U[0-9a-fA-F]{0,7}|x[0-9a-fA-F]?)$/, "")
    .replace(/[\uD800-\uDBFF]$/, "");
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
): { text: string; folds: Fold[] } {
  let excess = line.length - room;
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
    keep.set(index, kept);
    excess -= size - kept - FOLD.length;
  }
  let text = "";
  let from = 0;
  const folds: Fold[] = [];
  for (const [index, one] of found.entries()) {
    const kept = keep.get(index);
    if (kept === undefined) continue;
    text += line.slice(from, one.start) + uncut(line.slice(one.start, one.start + kept)).trimEnd();
    folds.push({ literal: index, at: text.length, whole: line.slice(one.start, one.end) });
    text += FOLD;
    from = one.end;
  }
  return { text: text + line.slice(from), folds };
}

import { type Exit, failed, type LiveAct } from "./acts.js";
import { actState, tailOf, valueText } from "./ladder.js";
import { reason } from "./plain.js";
import { placed } from "./source.js";

/** One line of the program of a thread, as the feed shows it.
 *
 * - `code`: a line of a word, with the acts that it made, whether it runs now, why it failed, and its hint: what its
 *   acts came to in a few words.
 * - `quote`: a quote of a word, folded to its name, its first line and its count of lines.
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
      text: string;
      acts: LiveAct[];
      runs: boolean;
      fails?: string;
      hint: string;
    }
  | { kind: "quote"; step: LiveAct; number: number; text: string; name: string; lines: number }
  | { kind: "output"; step: LiveAct; act: LiveAct; text: string }
  | { kind: "writes"; step: LiveAct; text: string }
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
      });
      index = end;
      continue;
    }
    const acts = on[index] ?? [];
    const runs = Boolean(open && acts.includes(open));
    out.push({
      kind: "code",
      step,
      number: index + 1,
      text: line,
      acts,
      runs,
      ...(failing === index + 1 && why ? { fails: why } : {}),
      hint: hintOf(acts, owed, label),
    });
    if (runs && open?.kind === "bash")
      for (const printed of tailOf(open, OUTPUT_LINES).split("\n"))
        if (printed) out.push({ kind: "output", step, act: open, text: printed });
  }
  if (failed(step) && !why) out.push({ kind: "note", step, text: `# ${step.id} failed` });
  return out;
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
): ProgramLine[] {
  let steps: LiveAct[] = [];
  for (const [at, prompt] of prompts.entries()) {
    if (at > 0) steps.push(prompt);
    for (const rung of made.get(prompt.id) ?? []) if (rung.kind === "rung") steps.push(rung);
  }
  steps.sort((a, b) => a.began - b.began);
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
    else out.push(...stepLines(step, made, word, owed, label, streams));
  }
  flush(last);
  return out;
}

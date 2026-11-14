import { type Act, cancelled, failed, isDone, type Measure, outcomeOf, reasonOf, WORK } from "./acts.js";
import { gist } from "./asks.js";
import { valueText } from "./ladder.js";
import { unmarked } from "./marks.js";
import { busy, type State } from "./states.js";

/** What a row of the loom does now: it asks the operator, it failed, it works, a pause holds it, its prompts are
 * all closed, or it rests. */
/** What a cell of a row shows: work that ran, work of the past, a question to the operator, a failure, or
 * nothing. */
export type Tone = "work" | "past" | "asks" | "failed" | "none";
export interface Cell {
  mark: string;
  tone: Tone;
}
/** One row of the loom: a chain, or the operator at the top. */
export interface Row {
  /** The chain, or the operator. */
  id: string;
  label: string;
  /** The lines of the tree before the label, as ├ and └. */
  lines: string;
  state: State;
  /** The chains under it in the tree, and whether the loom folds them into it. */
  workers: number;
  folded: boolean;
  /** The states of the chains that a folded row holds, by state. */
  tally: Partial<Record<State, number>>;
  /** The act that the chain waits on while it works. */
  waits?: string;
  cells: Cell[];
  measure?: Measure;
}
/** A prompt that the operator owes: its act, the chain it is on, who made it, its shape, its message, and when. */
export interface Owed {
  id: string;
  chain: string;
  by: string;
  shape: string;
  message: string;
  began: number;
}
export interface Loom {
  rows: Row[];
  owed: Owed[];
  /** How many columns of the rows hold more than one moment, which stand at the left of the others. */
  folded: number;
}
export interface Weave {
  acts: readonly Act[];
  /** The prompts that wait for the operator. */
  prompts: readonly { id: string; shape: string; message: string }[];
  measures: Readonly<Record<string, Measure>>;
  /** The rows whose workers the loom folds into them. */
  folds: ReadonlySet<string>;
  /** The columns that the cells of a row take. */
  width: number;
  /** The moments of the life, each start and each end of an act, in order, as the crate gives them: the loom computes
   * none, so a loom given none has no cells. */
  moments?: readonly number[];
  label(id: string): string;
  /** What a host already holds of the acts, which a loom of no width reads in place of a pass over every act: each act
   * by its name, the acts on each chain, the chains, and each act that is not done. */
  index?: {
    ids: ReadonlyMap<string, Act>;
    on: ReadonlyMap<string, readonly Act[]>;
    chains: readonly Act[];
    live: Iterable<Act>;
    /** How many times an act on each chain came or changed, which a host that counts them gives. */
    stamp?: (chain: string) => number;
    /** The chain of each act that came or changed, once each time, in the order of the changes: a list that only
     * grows, which a host that keeps it gives. */
    touched?: readonly string[];
  };
}

/** The heights of a cell, from the work that runs with no moment to many moments. */
const heights = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];

/** The cells of a set of acts over the spans: the height of a cell is how many of the acts began or ended in its span,
 * and a span in which some work of them runs, and none began or ended, is the lowest height. A failure, then a
 * question to the operator, then work that runs, gives the tone. */
export function cells(acts: readonly Act[], times: readonly [number, number][], stretch = 1): Cell[] {
  return times.flatMap(([lo, hi]): Cell[] => {
    const cell = cellOf(acts, lo, hi);
    if (stretch === 1) return [cell];
    // A moment that takes more than a column shows, before its cell, the work that ran up to it.
    const on = acts.some(
      (act) => WORK.includes(act.kind) && act.began < lo && (act.ended == null || act.ended >= lo),
    );
    const gap: Cell = on ? { mark: "▁", tone: "work" } : { mark: " ", tone: "none" };
    return [...Array.from({ length: stretch - 1 }, () => gap), cell];
  });
}
/** The cell of the acts over one span of moments. */
function cellOf(acts: readonly Act[], lo: number, hi: number): Cell {
  {
    let count = 0,
      runs = false,
      fails = false,
      asks = false;
    for (const act of acts) {
      const began = act.began >= lo && act.began <= hi;
      const ended = act.ended != null && act.ended >= lo && act.ended <= hi;
      if (began) count++;
      if (ended) count++;
      if (ended && failed(act) && !cancelled(act)) fails = true;
      if (began && asksOperator(act)) asks = true;
      if (WORK.includes(act.kind) && act.began <= hi && (act.ended == null || act.ended >= lo)) runs = true;
    }
    if (!count && !runs) return { mark: " ", tone: "none" };
    const mark = heights[count ? Math.min(7, 1 + Math.floor(Math.log2(count))) : 0] ?? "▁";
    return { mark, tone: fails ? "failed" : asks ? "asks" : runs ? "work" : "past" };
  }
}

/** Whether an act is a question to the operator: a prompt whose actor is the operator. */
export const asksOperator = (act: Act): boolean => act.kind === "prompt" && act.words[2] === "operator";

/** What a chain does now, in one line: the question that it asks the operator, the act that it waits on while it
 * works, the failure that stopped it, or the answer of its last prompt. The act is the one that the line is about,
 * and more counts the other questions that the chain asks. */
export interface Now {
  tone: Tone;
  text: string;
  act?: string;
  more: number;
}

/** What an act is about, on one line: the first line of the word of a rung, of the message of a prompt, or of the
 * command of a bash, and the seconds of a wait. */
export function subjectOf(act: Act, word: (rung: Act) => string): string {
  const text =
    act.kind === "rung"
      ? word(act)
      : act.kind === "wait"
        ? `up to ${duration(Number(unmarked(act.words[0] ?? 0)))}${act.words.length > 1 ? ` for ${act.words.slice(1).map(String).join(", ")}` : ""}`
        : String(act.kind === "prompt" ? (act.words[1] ?? "") : (act.words[0] ?? ""));
  return (
    text
      .split("\n")
      .find((line) => line.trim())
      ?.trim() ?? ""
  );
}

/** What a command does, by the program that it runs, in a few words: the checks of a pull request, a push, the tests
 * of a tool. A command whose program no rule knows says its program, and no guess. */
const commands: [RegExp, (found: RegExpMatchArray) => string][] = [
  [/\bgh pr checks (\d+)/, (found) => `checks of #${found[1]}`],
  [/\bgh pr merge (\d+)/, (found) => `merges #${found[1]}`],
  [/\bgh pr (?:create|edit)\b/, () => "opens its PR"],
  [/\bgh pr (?:view|diff) (\d+)/, (found) => `reads #${found[1]}`],
  [/\bgh pr list\b/, () => "lists the PRs"],
  [/\bgh run (?:watch|view)\b/, () => "reads a run of CI"],
  [/\bgh issue \w+ (\d+)/, (found) => `reads issue #${found[1]}`],
  [/\bgit push\b/, () => "pushes"],
  [/\bgit rebase\b/, () => "rebases"],
  [/\bgit (?:fetch|pull)\b/, () => "fetches"],
  [/\bgit commit\b/, () => "commits"],
  [/\bcargo (test|build|clippy|fmt|run)\b/, (found) => `runs cargo ${found[1]}`],
  [
    /\b(bun|npm|pnpm|yarn)( run)? (test|build|lint|check|bench|screenshots|animation|format)\b/,
    (found) => `runs ${found[1]}${found[2] ?? ""} ${found[3]}`,
  ],
  [/\b(pytest|ruff|ty check|pre-commit)\b/, (found) => `runs ${found[1]}`],
  [/\bsleep (\d+)/, (found) => `sleeps ${duration(Number(found[1]))}`],
];

/** What a command does, in a few words: the first rule that knows it, or else the program that it runs first, past a
 * change of directory and the variables that it sets. */
export function commandDoing(command: string): string {
  for (const [rule, say] of commands) {
    const found = command.match(rule);
    if (found) return say(found);
  }
  const program = command
    .split(/&&|;|\|\||\n/)
    .map(
      (part) =>
        part
          .trim()
          .replace(/^[({]\s*/, "")
          .split(/\s+/)
          .find((word) => /^[\w./~$]/.test(word) && /^[\w./~$+-]+$/.test(word) && !/^\w+=/.test(word)) ?? "",
    )
    .find((first) => first && first !== "cd");
  return program ? `runs ${program.split("/").at(-1)}` : "runs a command";
}

/** What an act that runs does, in a few words that a person reads at a glance, never a raw command: a question to the
 * operator, the chains that a prompt or a wait waits for, what a command does, and the file that a read or a write
 * takes. */
export function doingOf(
  act: Act,
  find: (id: string) => Act | undefined,
  label: (chain: string) => string,
): string {
  const file = () => {
    const path = String((act.words[0] as { path?: string } | undefined)?.path ?? act.words[0] ?? "");
    return path.split("/").at(-1) ?? path;
  };
  if (act.kind === "prompt") return asksOperator(act) ? "asks you" : `waits for ${label(act.chain)}`;
  if (act.kind === "bash") return commandDoing(String(act.words[0] ?? ""));
  if (act.kind === "read") return `reads ${file()}`;
  if (act.kind === "write") return `writes ${file()}`;
  if (act.kind === "chain") return `starts ${label(act.id)}`;
  if (act.kind === "wait") {
    const awaited = act.words.slice(1).flatMap((id) => find(String(id)) ?? []);
    if (!awaited.length) return `waits ${duration(Number(unmarked(act.words[0] ?? 0)))}`;
    const chains = new Set(awaited.map((one) => (one.kind === "prompt" ? label(one.chain) : "")));
    const [only] = chains;
    if (chains.size === 1 && only) return `waits for ${only}`;
    const [one] = awaited;
    if (one && awaited.length === 1) return doingOf(one, find, label);
    return `waits for ${awaited.length} ${awaited.every((one) => one.kind === "prompt") ? "threads" : "acts"}`;
  }
  return act.kind === "rung" ? "runs a step" : act.kind;
}

/** What the chain of a row does now, from the loom and the acts of the life. */
export function nowOf(
  row: Row,
  loom: Loom,
  acts: readonly Act[],
  word: (rung: Act) => string,
  index?: Pick<NonNullable<Weave["index"]>, "ids" | "on">,
): Now {
  const asks = loom.owed.filter((prompt) => prompt.chain === row.id);
  const [first] = asks;
  if (first)
    return {
      tone: "asks",
      act: first.id,
      more: asks.length - 1,
      text: gist(first.message),
    };
  const waits = row.waits
    ? (index?.ids.get(row.waits) ?? acts.find((act) => act.id === row.waits))
    : undefined;
  if (busy(row.state) && waits) return { tone: "work", act: waits.id, more: 0, text: subjectOf(waits, word) };
  const work = (index?.on.get(row.id) ?? acts).filter(
    (act) => act.chain === row.id && WORK.includes(act.kind),
  );
  if (row.state === "failed") {
    const fault = work.findLast((act) => failed(act) && !cancelled(act));
    const value = (fault ? outcomeOf(fault) : undefined) as
      | { is?: string; args?: unknown[] }
      | null
      | undefined;
    const reason =
      reasonOf(fault?.run) || (value?.is ? `${value.is}: ${(value.args ?? []).map(String).join(", ")}` : "");
    return { tone: "failed", act: fault?.id, more: 0, text: reason.split("\n")[0]?.trim() ?? "" };
  }
  if (row.state === "done") {
    const last = work.findLast((act) => act.kind === "prompt" && isDone(act) && !asksOperator(act));
    const value = last ? valueText(outcomeOf(last)) : "";
    return { tone: "past", act: last?.id, more: 0, text: readable(value) };
  }
  return { tone: "none", more: 0, text: "" };
}

/** The first line of a text that holds words, as a person reads it: with no marks of a heading, of emphasis, or of
 * code of markdown. */
export function readable(text: string): string {
  const line = text.split("\n").find((one) => one.trim()) ?? "";
  return line
    .replace(/^\s*#{1,6}\s+/, "")
    .replace(/(\*\*|__|\*|`)(\S(?:.*?\S)?)\1/g, "$2")
    .trim();
}

/** A count of seconds as a person reads it: seconds under two minutes, minutes under two hours, and hours. */
export function duration(seconds: number): string {
  if (seconds < 120) return `${Math.round(seconds)}s`;
  if (seconds < 7200) return `${Math.round(seconds / 60)}m`;
  return `${Math.round(seconds / 360) / 10}h`;
}

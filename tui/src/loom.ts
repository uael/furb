import { WORK } from "@furb/engine";
import type { Measure } from "./bridge.ts";
import { type ActRow, cancelled, failed } from "./session.ts";

/** What a thread of the loom does now: it asks the operator, it failed, it works, a pause holds it, its prompts are
 * all closed, or it rests. */
export type ThreadState = "asks" | "failed" | "working" | "paused" | "done" | "idle";
/** What a cell of a thread shows: work that ran, work of the past, a question to the operator, a failure, or
 * nothing. */
export type Tone = "work" | "past" | "asks" | "failed" | "none";
export interface Cell {
  mark: string;
  tone: Tone;
}
/** One thread of the loom: a chain, or the operator at the top. */
export interface Thread {
  /** The chain, or the operator. */
  id: string;
  label: string;
  /** The lines of the tree before the label, as ├ and └. */
  lines: string;
  state: ThreadState;
  /** The chains under it in the tree, and whether the loom folds them into it. */
  workers: number;
  folded: boolean;
  /** The states of the chains that a folded thread holds, by state. */
  tally: Partial<Record<ThreadState, number>>;
  /** The act that the thread waits on while it works. */
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
/** A prompt of None to the operator that the operator did not read yet: it asks nothing, so it is owed no answer. */
export type Unread = Omit<Owed, "shape">;
export interface Loom {
  threads: Thread[];
  owed: Owed[];
  /** The notices that the operator did not read, the oldest first. */
  unread: Unread[];
  /** How many columns of the threads hold more than one moment, which stand at the left of the others. */
  folded: number;
}
export interface Weave {
  acts: readonly ActRow[];
  /** The prompts that wait for the operator. */
  prompts: readonly { id: string; shape: string; message: string }[];
  /** The notices that the operator did not dismiss. */
  notices: readonly { id: string; message: string }[];
  measures: Readonly<Record<string, Measure>>;
  /** The threads whose workers the loom folds into them. */
  folds: ReadonlySet<string>;
  /** The columns that the cells of a thread take. */
  width: number;
  /** The acts that an earlier life left, which wait for a wake. */
  pending: ReadonlyMap<string, string>;
  label(id: string): string;
}

/** The heights of a cell, from the work that runs with no moment to many moments. */
const heights = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];

/** The spans of time of the columns, from the oldest to the newest: each span is the first and the last moment that
 * it holds. The newest moments take a column each, and the older moments share a column, twice as many at each step
 * of a quarter of the width back, so that a long life fits and the present has the most room. The oldest column takes
 * all that is left. */
export function spans(moments: readonly number[], width: number): [number, number][] {
  const step = Math.max(4, Math.floor(width / 4));
  const out: [number, number][] = [];
  let end = moments.length;
  for (let column = 0; column < width && end > 0; column++) {
    const size = column === width - 1 ? end : 2 ** Math.floor(column / step);
    const start = Math.max(0, end - size);
    out.unshift([moments[start] ?? 0, moments[end - 1] ?? 0]);
    end = start;
  }
  return out;
}

/** The cells of a set of acts over the spans: the height of a cell is how many of the acts began or ended in its span,
 * and a span in which some work of them runs, and none began or ended, is the lowest height. A failure, then a
 * question to the operator, then work that runs, gives the tone. */
export function cells(acts: readonly ActRow[], times: readonly [number, number][], stretch = 1): Cell[] {
  return times.flatMap(([lo, hi]): Cell[] => {
    const cell = cellOf(acts, lo, hi);
    if (stretch === 1) return [cell];
    // A moment that takes more than a column shows, before its cell, the work that ran up to it.
    const on = acts.some(
      (act) => WORK.includes(act.kind) && act.began < lo && (act.ended === undefined || act.ended >= lo),
    );
    const gap: Cell = on ? { mark: "▁", tone: "work" } : { mark: " ", tone: "none" };
    return [...Array.from({ length: stretch - 1 }, () => gap), cell];
  });
}
/** The cell of the acts over one span of moments. */
function cellOf(acts: readonly ActRow[], lo: number, hi: number): Cell {
  {
    let count = 0,
      runs = false,
      fails = false,
      asks = false;
    for (const act of acts) {
      const began = act.began >= lo && act.began <= hi;
      const ended = act.ended !== undefined && act.ended >= lo && act.ended <= hi;
      if (began) count++;
      if (ended) count++;
      if (ended && failed(act) && !cancelled(act)) fails = true;
      if (began && asksOperator(act)) asks = true;
      if (WORK.includes(act.kind) && act.began <= hi && (act.ended === undefined || act.ended >= lo))
        runs = true;
    }
    if (!count && !runs) return { mark: " ", tone: "none" };
    const mark = heights[count ? Math.min(7, 1 + Math.floor(Math.log2(count))) : 0] ?? "▁";
    return { mark, tone: fails ? "failed" : asks ? "asks" : runs ? "work" : "past" };
  }
}

/** Whether an act is a prompt to the operator: a question, or a notice when its shape is None. */
export const toOperator = (act: ActRow): boolean => act.kind === "prompt" && act.words[2] === "operator";
/** Whether an act is a notice to the operator: a prompt of None, which asks nothing. */
export const isNotice = (act: ActRow): boolean => toOperator(act) && act.words[0] === "None";
/** Whether an act is a question to the operator: a prompt to the operator that is no notice. */
export const asksOperator = (act: ActRow): boolean => toOperator(act) && !isNotice(act);

/** The chain that a chain stands under in the tree: the chain of the rung that made it, and nothing for a chain that
 * the operator or an ear made. */
export function parentOf(chain: ActRow, acts: ReadonlyMap<string, ActRow>): string | undefined {
  const maker = acts.get(chain.by);
  return maker && maker.on !== chain.id ? maker.on : undefined;
}

/** The state of the acts of a chain, with the prompts that the operator owes on it. */
export function threadState(
  work: readonly ActRow[],
  owes: boolean,
  pending: ReadonlyMap<string, string>,
): ThreadState {
  if (owes) return "asks";
  const live = work.filter((act) => WORK.includes(act.kind));
  if (live.some((act) => pending.has(act.id) || (act.paused && !act.done))) return "paused";
  if (live.some((act) => !act.done)) return "working";
  const latest = live.at(-1);
  if (latest && cancelled(latest)) return "idle";
  if (latest && failed(latest)) return "failed";
  const prompts = live.filter((act) => act.kind === "prompt");
  return prompts.length && prompts.every((act) => act.done) ? "done" : "idle";
}

/** The act that a chain waits on while it works: its newest command or wait that runs, then the newest prompt that one
 * of its rungs made on another chain, then its newest rung that runs. */
export function waitsOn(
  chain: string,
  acts: readonly ActRow[],
  rows: ReadonlyMap<string, ActRow>,
): string | undefined {
  const open = acts.filter((act) => !act.done);
  const local = open.filter((act) => act.on === chain);
  return (
    local.findLast((act) => act.kind === "bash" || act.kind === "wait")?.id ??
    open.findLast((act) => act.kind === "prompt" && act.on !== chain && rows.get(act.by)?.on === chain)?.id ??
    local.findLast((act) => act.kind === "rung")?.id
  );
}

/** The loom of a life: the operator, then each chain in the order of the tree, with its cells over the moments of the
 * life, its state, what it waits on and its measure. A moment is the start or the end of an act. */
export function weave({ acts, prompts, notices, measures, folds, width, pending, label }: Weave): Loom {
  const rows = new Map(acts.map((act) => [act.id, act]));
  const owed: Owed[] = prompts
    .flatMap((prompt) => {
      const act = rows.get(prompt.id);
      // A prompt is owed until its act is done, though the console can hold it a moment longer.
      return act && !act.done ? [{ ...prompt, chain: act.on, by: act.by, began: act.began }] : [];
    })
    .sort((one, other) => one.began - other.began);
  const unread: Unread[] = notices
    .flatMap((notice) => {
      const act = rows.get(notice.id);
      return act ? [{ ...notice, chain: act.on, by: act.by, began: act.began }] : [];
    })
    .sort((one, other) => one.began - other.began);
  const owing = new Set(owed.map((prompt) => prompt.chain));
  const moments = [
    ...new Set(acts.flatMap((act) => [act.began, ...(act.ended === undefined ? [] : [act.ended])])),
  ]
    .filter((moment) => moment > 0)
    .sort((one, other) => one - other);
  // A short life stretches each moment over as many as three columns, so that its threads fill the loom.
  const stretch = Math.max(1, Math.min(3, Math.floor(width / Math.max(1, moments.length))));
  const times = spans(moments, Math.floor(width / stretch));
  const folded = times.filter(([lo, hi]) => hi > lo).length * stretch;
  const byChain = new Map<string, ActRow[]>();
  for (const act of acts) {
    const list = byChain.get(act.on) ?? [];
    list.push(act);
    byChain.set(act.on, list);
  }
  const chains = acts.filter((act) => act.kind === "chain").sort((one, other) => one.began - other.began);
  const children = new Map<string | undefined, ActRow[]>();
  for (const chain of chains) {
    const parent = parentOf(chain, rows);
    const list = children.get(parent) ?? [];
    list.push(chain);
    children.set(parent, list);
  }
  const under = (id: string): string[] =>
    (children.get(id) ?? []).flatMap((child) => [child.id, ...under(child.id)]);
  const stateOf = (id: string) =>
    threadState(
      (byChain.get(id) ?? []).filter((act) => act.kind !== "chain"),
      owing.has(id),
      pending,
    );
  const operatorActs = acts.filter(toOperator);
  const threads: Thread[] = [
    {
      id: "operator",
      label: "you",
      lines: "",
      state: owed.length ? "asks" : "idle",
      workers: 0,
      folded: false,
      tally: {},
      cells: cells(operatorActs, times, stretch),
    },
  ];
  const visit = (chain: ActRow, lines: string, last: boolean, depth: number): void => {
    const below = under(chain.id);
    const fold = folds.has(chain.id) && below.length > 0;
    const tally: Partial<Record<ThreadState, number>> = {};
    if (fold) for (const id of below) tally[stateOf(id)] = (tally[stateOf(id)] ?? 0) + 1;
    const own = (fold ? [chain.id, ...below] : [chain.id]).flatMap((id) =>
      (byChain.get(id) ?? []).filter((act) => act.kind !== "chain" || act.id !== id),
    );
    const state = stateOf(chain.id);
    threads.push({
      id: chain.id,
      label: label(chain.id),
      lines: depth ? `${lines}${last ? "└ " : "├ "}` : "",
      state,
      workers: below.length,
      folded: fold,
      tally,
      waits: state === "working" ? waitsOn(chain.id, acts, rows) : undefined,
      cells: cells(own, times, stretch),
      measure: measures[chain.id],
    });
    if (fold) return;
    const next = children.get(chain.id) ?? [];
    for (const [index, child] of next.entries())
      visit(child, depth ? `${lines}${last ? "  " : "│ "}` : "", index === next.length - 1, depth + 1);
  };
  for (const top of children.get(undefined) ?? []) visit(top, "", true, 0);
  return { threads, owed, unread, folded };
}

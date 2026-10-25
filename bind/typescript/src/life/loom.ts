import { cancelled, failed, type LiveAct, type Measure, WORK } from "./acts.js";
import { gist } from "./asks.js";
import { valueText } from "./ladder.js";

/** What a row of the loom does now: it asks the operator, it failed, it works, a pause holds it, its prompts are
 * all closed, or it rests. */
export type RowState = "asks" | "failed" | "working" | "paused" | "done" | "idle";
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
  state: RowState;
  /** The chains under it in the tree, and whether the loom folds them into it. */
  workers: number;
  folded: boolean;
  /** The states of the chains that a folded row holds, by state. */
  tally: Partial<Record<RowState, number>>;
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
  acts: readonly LiveAct[];
  /** The prompts that wait for the operator. */
  prompts: readonly { id: string; shape: string; message: string }[];
  measures: Readonly<Record<string, Measure>>;
  /** The rows whose workers the loom folds into them. */
  folds: ReadonlySet<string>;
  /** The columns that the cells of a row take. */
  width: number;
  /** The acts that an earlier life left, which wait for a wake. */
  pending: ReadonlyMap<string, string>;
  label(id: string): string;
  /** What a host already holds of the acts, which a loom of no width reads in place of a pass over every act: each act
   * by its name, the acts on each chain, the chains, and each act that is not done. */
  index?: {
    ids: ReadonlyMap<string, LiveAct>;
    on: ReadonlyMap<string, readonly LiveAct[]>;
    chains: readonly LiveAct[];
    live: Iterable<LiveAct>;
    /** How many times an act on each chain came or changed, which a host that counts them gives. */
    stamp?: (chain: string) => number;
    /** The chain of each act that came or changed, once each time, in the order of the changes: a list that only
     * grows, which a host that keeps it gives. */
    touched?: readonly string[];
  };
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
export function cells(acts: readonly LiveAct[], times: readonly [number, number][], stretch = 1): Cell[] {
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
function cellOf(acts: readonly LiveAct[], lo: number, hi: number): Cell {
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

/** Whether an act is a question to the operator: a prompt whose actor is the operator. */
export const asksOperator = (act: LiveAct): boolean => act.kind === "prompt" && act.words[2] === "operator";

/** The chain that a chain stands under in the tree: the chain of the rung that made it, and nothing for a chain that
 * the operator or an ear made. */
export function parentOf(chain: LiveAct, acts: ReadonlyMap<string, LiveAct>): string | undefined {
  const maker = acts.get(chain.by);
  return maker && maker.on !== chain.id ? maker.on : undefined;
}

/** The state of the acts of a chain, with the prompts that the operator owes on it. */
export function rowState(
  work: readonly LiveAct[],
  owes: boolean,
  pending: ReadonlyMap<string, string>,
): RowState {
  if (owes) return "asks";
  const live = work.filter((act) => WORK.includes(act.kind) && !act.unfinished);
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
  acts: readonly LiveAct[],
  rows: ReadonlyMap<string, LiveAct>,
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
export function weave({ acts, prompts, measures, folds, width, pending, label, index }: Weave): Loom {
  // A loom of no width shows no cell, so it reads the index of a host when it has one, in the time of the chains and of
  // the work that is not done, and not of every act.
  if (index && width <= 0) return board({ prompts, measures, folds, pending, label }, index);
  const byId = new Map(acts.map((act) => [act.id, act]));
  const owed: Owed[] = prompts
    .flatMap((prompt) => {
      const act = byId.get(prompt.id);
      // A prompt is owed until its act is done, though the console can hold it a moment longer.
      return act && !act.done ? [{ ...prompt, chain: act.on, by: act.by, began: act.began }] : [];
    })
    .sort((one, other) => one.began - other.began);
  const owing = new Set(owed.map((prompt) => prompt.chain));
  // A loom of no width, as the board weaves it, has no cells, and reads no moment.
  const moments = width > 0 ? momentsOf(acts) : [];
  // A short life stretches each moment over as many as three columns, so that its rows fill the loom.
  const stretch = Math.max(1, Math.min(3, Math.floor(width / Math.max(1, moments.length))));
  const times = spans(moments, Math.floor(width / stretch));
  const folded = times.filter(([lo, hi]) => hi > lo).length * stretch;
  const byChain = new Map<string, LiveAct[]>();
  for (const act of acts) {
    const list = byChain.get(act.on) ?? [];
    list.push(act);
    byChain.set(act.on, list);
  }
  const chains = acts.filter((act) => act.kind === "chain").sort((one, other) => one.began - other.began);
  const children = new Map<string | undefined, LiveAct[]>();
  for (const chain of chains) {
    const parent = parentOf(chain, byId);
    const list = children.get(parent) ?? [];
    list.push(chain);
    children.set(parent, list);
  }
  const under = (id: string): string[] =>
    (children.get(id) ?? []).flatMap((child) => [child.id, ...under(child.id)]);
  const stateOf = (id: string) =>
    rowState(
      (byChain.get(id) ?? []).filter((act) => act.kind !== "chain"),
      owing.has(id),
      pending,
    );
  const operatorActs = acts.filter(asksOperator);
  // The acts that run, which each row that works reads for what it waits on.
  const open = acts.filter((act) => !act.done);
  const rows: Row[] = [
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
  const visit = (chain: LiveAct, lines: string, last: boolean, depth: number): void => {
    const below = under(chain.id);
    const fold = folds.has(chain.id) && below.length > 0;
    const tally: Partial<Record<RowState, number>> = {};
    if (fold)
      for (const id of below) {
        const one = stateOf(id);
        tally[one] = (tally[one] ?? 0) + 1;
      }
    const own = (fold ? [chain.id, ...below] : [chain.id]).flatMap((id) =>
      (byChain.get(id) ?? []).filter((act) => act.kind !== "chain" || act.id !== id),
    );
    const state = stateOf(chain.id);
    rows.push({
      id: chain.id,
      label: label(chain.id),
      lines: depth ? `${lines}${last ? "└ " : "├ "}` : "",
      state,
      workers: below.length,
      folded: fold,
      tally,
      waits: state === "working" ? waitsOn(chain.id, open, byId) : undefined,
      cells: cells(own, times, stretch),
      measure: measures[chain.id],
    });
    if (fold) return;
    const next = children.get(chain.id) ?? [];
    for (const [index, child] of next.entries())
      visit(child, depth ? `${lines}${last ? "  " : "│ "}` : "", index === next.length - 1, depth + 1);
  };
  for (const top of children.get(undefined) ?? []) visit(top, "", true, 0);
  return { rows, owed, folded };
}

/** The state of a chain on the board, and what it was read of: the stamp of its acts, whether the operator owes a
 * question on it, its held work, and the last prompt that it asks on another chain. */
/** What the loom of no width of the index of a host read: the tree of its chains, as many as it read, and the state of
 * each chain with what it was read of. A host that names each chain that an act came or changed on has the board
 * read again only those chains, and the chains whose question of the operator, held work or measure changed: the
 * board keeps its rows, the place of each chain among them, how many names of chains it read, the chains that held a
 * question, held work or a prompt on another chain, and the order of the rows by state. */
interface Boarded {
  chains: number;
  tree: ReturnType<typeof treeOf>;
  states: Map<string, Boarding>;
  kept?: {
    rows: Row[];
    place: Map<string, number>;
    seen: number;
    marked: Set<string>;
    measures: Readonly<Record<string, Measure>>;
    order: Ordering;
  };
}
/** The state of a chain on the board, and what it was read of: the stamp of its acts, whether the operator owes a
 * question on it, its held work, and the last prompt that it asks on another chain. */
interface Boarding {
  stamp: number;
  owes: boolean;
  held: string;
  asks: string;
  state: RowState;
  waits?: string;
}
const boarded = new WeakMap<object, Boarded>();

/** The rows of the chains of a board in the orders that the board reads them: every row, the rows that ask, then
 * those that need a look, each in the order of the tree, the first rows of the others, the place of each row, and how
 * many rows hold each state. */
export interface BoardOrder {
  rows: readonly Row[];
  asking: Row[];
  looking: Row[];
  resting(count: number): Row[];
  place: ReadonlyMap<string, number>;
  counts: ReadonlyMap<RowState, number>;
}
/** The chains that ask and that need a look, and how many rows hold each state, which a board keeps as rows change. */
interface Ordering {
  asking: Set<string>;
  looking: Set<string>;
  counts: Map<RowState, number>;
}
/** The states of a row that needs a look. */
const looks: readonly RowState[] = ["failed", "paused", "working"];
const orders = new WeakMap<Loom, BoardOrder>();

/** A row of a chain that holds a new state, or no state when it is gone, in an ordering. */
function reorder(order: Ordering, id: string, was: RowState | undefined, now: RowState | undefined): void {
  if (was === now) return;
  if (was !== undefined) order.counts.set(was, (order.counts.get(was) ?? 1) - 1);
  if (now !== undefined) order.counts.set(now, (order.counts.get(now) ?? 0) + 1);
  if (now === "asks") order.asking.add(id);
  else order.asking.delete(id);
  if (now !== undefined && now !== "asks" && looks.includes(now)) order.looking.add(id);
  else order.looking.delete(id);
}

/** The order of the rows of the chains of a loom, as a board reads them. A loom that a board read from the index of a
 * host gives the order that the board kept, and any other loom gives the order of its rows. */
export function orderOf(loom: Loom): BoardOrder {
  const known = orders.get(loom);
  if (known) return known;
  const rows = loom.rows.filter((row) => row.id !== "operator");
  const order: Ordering = { asking: new Set(), looking: new Set(), counts: new Map() };
  for (const row of rows) reorder(order, row.id, undefined, row.state);
  const made = ordered(rows, new Map(rows.map((row, at) => [row.id, at])), order);
  orders.set(loom, made);
  return made;
}

/** The order of rows from what an ordering holds. */
function ordered(rows: readonly Row[], place: ReadonlyMap<string, number>, order: Ordering): BoardOrder {
  const of = (ids: Set<string>) =>
    [...ids]
      .map((id) => place.get(id) ?? -1)
      .filter((at) => at >= 0)
      .sort((one, other) => one - other)
      .map((at) => rows[at] as Row);
  return {
    rows,
    asking: of(order.asking),
    looking: of(order.looking),
    resting(count) {
      const out: Row[] = [];
      for (const row of rows) {
        if (out.length >= count) break;
        if (!order.asking.has(row.id) && !order.looking.has(row.id)) out.push(row);
      }
      return out;
    },
    place,
    counts: new Map(order.counts),
  };
}

/** The loom of no width of a life, read from the index of a host: its rows, with no cells. A host that stamps each
 * change of the acts on each chain has the state of a chain read again only once an act on it, a question of the
 * operator on it, or its held work changed. */
function board(
  { prompts, measures, folds, pending, label }: Omit<Weave, "acts" | "width" | "index">,
  { ids, on, chains: listed, live, stamp, touched }: NonNullable<Weave["index"]>,
): Loom {
  const owed: Owed[] = prompts
    .flatMap((prompt) => {
      const act = ids.get(prompt.id);
      return act && !act.done ? [{ ...prompt, chain: act.on, by: act.by, began: act.began }] : [];
    })
    .sort((one, other) => one.began - other.began);
  const owing = new Set(owed.map((prompt) => prompt.chain));
  // The work that is not done on each chain, and the prompts that are not done on another chain that the rungs of
  // each chain made, in the order of the acts, which tell what each chain waits on as waitsOn tells it.
  const openOn = new Map<string, LiveAct[]>();
  const asked = new Map<string, LiveAct[]>();
  const put = (map: Map<string, LiveAct[]>, key: string, act: LiveAct) => {
    const list = map.get(key);
    if (list) list.push(act);
    else map.set(key, [act]);
  };
  for (const act of live) {
    if (act.done) continue;
    put(openOn, act.on, act);
    const maker = act.kind === "prompt" ? ids.get(act.by)?.on : undefined;
    if (maker !== undefined && maker !== act.on) put(asked, maker, act);
  }
  const held = new Map<string, string[]>();
  for (const id of pending.keys()) {
    const act = ids.get(id);
    if (act) held.set(act.on, [...(held.get(act.on) ?? []), id]);
  }
  // The tree of the chains holds while no chain comes, and a fold of the operator lays it out again.
  const read = boarded.get(ids);
  const same = read?.chains === listed.length;
  const unfolded = same && read ? read.tree : treeOf(listed, ids, new Set());
  const tree = folds.size ? treeOf(listed, ids, folds) : unfolded;
  const states = read?.states ?? new Map<string, Boarding>();
  const kept = same && !folds.size && touched ? read?.kept : undefined;
  const entry: Boarded = { chains: listed.length, tree: unfolded, states };
  boarded.set(ids, entry);
  // The state of a chain, as rowState gives it of all its work, from the work that waits or is not done, its last
  // work, and whether it holds a prompt, since each prompt of a chain with no work that is not done is done.
  const stateOf = (id: string): Boarding => {
    const owes = owing.has(id);
    const pend = held.get(id)?.join(" ") ?? "";
    const asks = asked.get(id)?.at(-1)?.id ?? "";
    const changed = stamp?.(id) ?? -1;
    const kept = states.get(id);
    if (
      kept &&
      changed >= 0 &&
      kept.stamp === changed &&
      kept.owes === owes &&
      kept.held === pend &&
      kept.asks === asks
    )
      return kept;
    const work = (act?: LiveAct): act is LiveAct =>
      act !== undefined && act.on === id && WORK.includes(act.kind) && !act.unfinished;
    const waiting = [...(openOn.get(id) ?? []), ...(held.get(id) ?? []).map((one) => ids.get(one))].filter(
      work,
    );
    const acts = on.get(id) ?? [];
    const latest = acts.findLast(work);
    const state: RowState = owes
      ? "asks"
      : waiting.some((act) => pending.has(act.id) || (act.paused && !act.done))
        ? "paused"
        : waiting.some((act) => !act.done)
          ? "working"
          : latest && cancelled(latest)
            ? "idle"
            : latest && failed(latest)
              ? "failed"
              : acts.some((act) => act.kind === "prompt")
                ? "done"
                : "idle";
    const local = openOn.get(id) ?? [];
    const waits =
      state === "working"
        ? (local.findLast((act) => act.kind === "bash" || act.kind === "wait")?.id ??
          asked.get(id)?.at(-1)?.id ??
          local.findLast((act) => act.kind === "rung")?.id)
        : undefined;
    const made = { stamp: changed, owes, held: pend, asks, state, waits };
    states.set(id, made);
    return made;
  };
  const rowOf = ({ chain, lines, below }: (typeof tree)[number]): Row => {
    const fold = folds.has(chain.id) && below.length > 0;
    const tally: Partial<Record<RowState, number>> = {};
    if (fold)
      for (const id of below) {
        const one = stateOf(id).state;
        tally[one] = (tally[one] ?? 0) + 1;
      }
    const { state, waits } = stateOf(chain.id);
    return {
      id: chain.id,
      label: label(chain.id),
      lines,
      state,
      workers: below.length,
      folded: fold,
      tally,
      waits,
      cells: [],
      measure: measures[chain.id],
    };
  };
  const operator: Row = {
    id: "operator",
    label: "you",
    lines: "",
    state: owed.length ? "asks" : "idle",
    workers: 0,
    folded: false,
    tally: {},
    cells: [],
  };
  // The chains that hold a question of the operator, held work or a prompt on another chain now.
  const marked = new Set([...owing, ...held.keys(), ...asked.keys()]);
  let rows: Row[];
  let place: Map<string, number>;
  let order: Ordering;
  if (kept && touched) {
    ({ rows, place, order } = kept);
    // Only a chain that an act came or changed on, or whose question, held work, prompt on another chain or measure
    // changed, is read again.
    const again = new Set([...touched.slice(kept.seen), ...marked, ...kept.marked]);
    if (measures !== kept.measures)
      for (const id of Object.keys(measures))
        if (measures[id] !== rows[place.get(id) ?? -1]?.measure) again.add(id);
    for (const id of again) {
      const at = place.get(id) ?? -1;
      const was = rows[at];
      const one = tree[at];
      if (!was || !one) continue;
      const before = states.get(id);
      const now = stateOf(id);
      if (now === before && was.measure === measures[id]) continue;
      const row = rowOf(one);
      rows[at] = row;
      reorder(order, id, was.state, row.state);
    }
  } else {
    rows = tree.map(rowOf);
    place = new Map(rows.map((row, at) => [row.id, at]));
    order = { asking: new Set(), looking: new Set(), counts: new Map() };
    for (const row of rows) reorder(order, row.id, undefined, row.state);
  }
  entry.kept = { rows, place, seen: touched?.length ?? 0, marked, measures, order };
  const loom: Loom = { rows: [operator, ...rows], owed, folded: 0 };
  if (!folds.size) orders.set(loom, ordered([...rows], place, order));
  return loom;
}

/** The rows of the tree of the chains of a life, in its order: each chain under the chain whose rung made it, with
 * the lines of the tree before its label and the chains under it, and none under a chain that a fold holds. */
function treeOf(listed: readonly LiveAct[], ids: ReadonlyMap<string, LiveAct>, folds: ReadonlySet<string>) {
  const chains = [...listed].sort((one, other) => one.began - other.began);
  const children = new Map<string | undefined, LiveAct[]>();
  for (const chain of chains) {
    const parent = parentOf(chain, ids);
    const list = children.get(parent);
    if (list) list.push(chain);
    else children.set(parent, [chain]);
  }
  const under = (id: string): string[] =>
    (children.get(id) ?? []).flatMap((child) => [child.id, ...under(child.id)]);
  const out: { chain: LiveAct; lines: string; depth: number; below: string[] }[] = [];
  const visit = (chain: LiveAct, lines: string, last: boolean, depth: number): void => {
    const below = under(chain.id);
    out.push({ chain, lines: depth ? `${lines}${last ? "└ " : "├ "}` : "", depth, below });
    if (folds.has(chain.id) && below.length > 0) return;
    const next = children.get(chain.id) ?? [];
    for (const [place, child] of next.entries())
      visit(child, depth ? `${lines}${last ? "  " : "│ "}` : "", place === next.length - 1, depth + 1);
  };
  for (const top of children.get(undefined) ?? []) visit(top, "", true, 0);
  return out;
}

/** The acts of a life as they stood at a moment: each act that began by then, and an act that ended after it as one
 * that runs, with no value yet. */
export function asOf(acts: readonly LiveAct[], at: number): LiveAct[] {
  return acts
    .filter((act) => act.began <= at)
    .map((act): LiveAct => {
      if (act.ended === undefined || act.ended <= at) return act;
      const value =
        act.kind === "bash"
          ? {
              is: "Exit",
              code: null,
              stdout: { is: "Text", content: "" },
              stderr: { is: "Text", content: "" },
            }
          : null;
      return {
        ...act,
        done: false,
        ended: undefined,
        value,
        paused: false,
        ...(act.run ? { run: { status: "running", reason: "" } } : {}),
      };
    });
}

/** The prompts that the operator owed at a moment, from the acts as they stood then. */
export function owedAt(acts: readonly LiveAct[]): { id: string; shape: string; message: string }[] {
  return acts
    .filter((act) => asksOperator(act) && !act.done)
    .map((act) => ({ id: act.id, shape: String(act.words[0] ?? ""), message: String(act.words[1] ?? "") }));
}

/** The moments of a life, in order: each start and each end of an act. */
export function momentsOf(acts: readonly LiveAct[]): number[] {
  return [...new Set(acts.flatMap((act) => [act.began, ...(act.ended === undefined ? [] : [act.ended])]))]
    .filter((moment) => moment > 0)
    .sort((one, other) => one - other);
}

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
export function subjectOf(act: LiveAct, word: (rung: LiveAct) => string): string {
  const text =
    act.kind === "rung"
      ? word(act)
      : act.kind === "wait"
        ? `up to ${duration(Number(act.words[0] ?? 0))}${act.words.length > 1 ? ` for ${act.words.slice(1).map(String).join(", ")}` : ""}`
        : String(act.kind === "prompt" ? (act.words[1] ?? "") : (act.words[0] ?? ""));
  return (
    text
      .split("\n")
      .find((line) => line.trim())
      ?.trim() ?? ""
  );
}

/** What the chain of a row does now, from the loom and the acts of the life. */
export function nowOf(
  row: Row,
  loom: Loom,
  acts: readonly LiveAct[],
  word: (rung: LiveAct) => string,
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
  if (row.state === "working" && waits)
    return { tone: "work", act: waits.id, more: 0, text: subjectOf(waits, word) };
  const work = (index?.on.get(row.id) ?? acts).filter((act) => act.on === row.id && WORK.includes(act.kind));
  if (row.state === "failed") {
    const fault = work.findLast((act) => failed(act) && !cancelled(act));
    const value = fault?.value as { is?: string; args?: unknown[] } | null | undefined;
    const reason =
      fault?.run?.reason || (value?.is ? `${value.is}: ${(value.args ?? []).map(String).join(", ")}` : "");
    return { tone: "failed", act: fault?.id, more: 0, text: reason.split("\n")[0]?.trim() ?? "" };
  }
  if (row.state === "done") {
    const last = work.findLast((act) => act.kind === "prompt" && act.done && !asksOperator(act));
    const value = last ? valueText(last.value) : "";
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

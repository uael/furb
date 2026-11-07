import type { Rung } from "../protocol.js";
import { type Act, cancelled, type Exit, exitOf, failed, isDone, outcomeOf } from "./acts.js";
import type { State } from "./states.js";

/** One line of the thread of a chain.
 *
 * - `prompt`: a prompt on the chain, with its shape, its message and its actor.
 * - `rung`: a rung of a ladder, or a word that its caller wrote, with the first line of its word, the acts that it
 *   made, and what refused or ended it.
 * - `act`: an act on the chain that no rung of the chain made, such as a command of the operator.
 * - `ear`: the rungs in a row that an ear of the outside made on the chain, as the words of the extensions, with the
 *   acts that they made.
 * - `tail`: the last lines that a command that runs printed, or that a model writes while it answers.
 * - `value`: what a prompt closed with.
 * - `more`: the earlier prompts of a long thread, or the earlier rungs of a long ladder, which a fold holds as one
 *   line until it opens, with how many it holds.
 */
export type Line =
  | { type: "prompt"; act: Act; depth: number; state: State }
  | { type: "rung"; act: Act; depth: number; state: State; word: string; more: number; acts: Chip[] }
  | { type: "act"; act: Act; depth: number; state: State }
  | { type: "ear"; act: Act; depth: number; state: State; rungs: Act[]; acts: Chip[] }
  | { type: "tail"; act: Act; depth: number; text: string; model: boolean }
  | { type: "value"; act: Act; depth: number; text: string }
  | {
      type: "more";
      act: Act;
      depth: number;
      count: number;
      of: "prompts" | "rungs";
      key: string;
      /** The kinds of the acts that head the lines the fold holds, as prompt for a message and rung for a word. */
      heads: string[];
    };
/** The acts of a rung, by name: one act, or a run of acts of one kind and one state, as read3…7. */
export interface Chip {
  ids: string[];
  label: string;
  state: State;
}

/** The acts of a rung as chips: a run of acts of one kind and one state stands as one chip, by its first and its last
 * number. */
export function chips(acts: readonly Act[]): Chip[] {
  const out: Chip[] = [];
  for (const act of acts) {
    const state = act.state;
    const last = out.at(-1);
    if (
      last &&
      state !== "thinks" &&
      state !== "runs" &&
      last.state === state &&
      kindOf(last.ids[0] ?? "") === act.kind
    )
      last.ids.push(act.id);
    else out.push({ ids: [act.id], label: act.id, state });
  }
  for (const chip of out) {
    const first = chip.ids[0] ?? "",
      end = chip.ids.at(-1) ?? "";
    chip.label = chip.ids.length > 1 ? `${first}…${end.slice(kindOf(end).length)}` : first;
  }
  return out;
}
const kindOf = (id: string) => id.replace(/\d+$/, "");

/** The value of a prompt as a line shows it: the first line of a text, or the value as JSON. */
export function valueText(value: unknown): string {
  if (value === null || value === undefined) return "";
  if (typeof value === "string") return value;
  return JSON.stringify(value) ?? "";
}

/** The last lines that a command printed, at most a count of them, with no empty line at the end. */
export function tailOf(act: Act & { printed?: Exit }, count = 3): string {
  const exit = exitOf(act);
  const lines = `${exit?.stdout?.content ?? ""}${exit?.stderr?.content ?? ""}`
    .replace(/\n+$/, "")
    .split("\n");
  return lines.slice(-count).join("\n");
}

/** What a host already holds of the acts of a life, which the ladder reads in place of a pass over every act: the acts
 * on the chain but its chains, the acts that each act made, and each act by its name, each in the order of the acts. */
export type ActIndex = {
  on: readonly Act[];
  made: ReadonlyMap<string, readonly Act[]>;
  ids: ReadonlyMap<string, Act>;
  /** Each act that is not done, and each rung whose run runs, by its name, which the steps that stay shown hold. */
  live: ReadonlyMap<string, Act>;
  /** How many times an act that each act made came or changed, which a host that counts them gives: a thread keeps what
   * it read of each step until that count changes. */
  stamp?: (id: string) => number;
};

/** How a thread reads its acts: the chain, the word of a rung, and what each rung streams. */
interface Reading {
  chain: string;
  made: ReadonlyMap<string, readonly Act[]>;
  word: (rung: Act) => string;
  streams: ReadonlyMap<string, { text: string; thinking: string }>;
}

/** A step of a thread: its line, the lines that stand first under it, the acts whose steps stand under it, and the
 * lines that end it. A prompt holds the steps of the rungs of its ladder and ends with its value. A rung holds the
 * tails of what streams and the steps of the prompts that it made on the chain, and the chips of every other act that
 * it made. A command that runs holds its tail. */
function step(act: Act, depth: number, { chain, made, word, streams }: Reading) {
  const state = act.state;
  const lead: Line[] = [];
  const trail: Line[] = [];
  if (act.kind === "prompt") {
    const text = isDone(act) && !failed(act) && !cancelled(act) ? valueText(outcomeOf(act)) : "";
    if (text) trail.push({ type: "value", act, depth: depth + 1, text });
    const line: Line = { type: "prompt", act, depth, state };
    return { line, lead, steps: made.get(act.id) ?? [], trail };
  }
  if (act.kind === "rung") {
    const children = made.get(act.id) ?? [];
    const [first = "", ...rest] = word(act).split("\n");
    const asked = children.filter((child) => child.kind === "prompt" && child.chain === chain);
    const asks = new Set(asked);
    const line: Line = {
      type: "rung",
      act,
      depth,
      state,
      word: first,
      more: rest.length,
      acts: chips(children.filter((child) => !asks.has(child))),
    };
    const stream = streams.get(act.id);
    if (stream && !isDone(act))
      lead.push({
        type: "tail",
        act,
        depth: depth + 1,
        text: (stream.text || stream.thinking).replace(/\n+$/, "").split("\n").slice(-3).join("\n"),
        model: true,
      });
    for (const child of children)
      if (child.kind === "bash" && !isDone(child) && tailOf(child))
        lead.push({ type: "tail", act: child, depth: depth + 1, text: tailOf(child), model: false });
    return { line, lead, steps: asked, trail };
  }
  if (act.kind === "bash" && !isDone(act) && tailOf(act))
    lead.push({ type: "tail", act, depth: depth + 1, text: tailOf(act), model: false });
  return { line: { type: "act", act, depth, state } as Line, lead, steps: [] as Act[], trail };
}

/** The line of the rungs in a row that one maker wrote on the chain with no word of its own, as the words of the
 * extensions and the setup of a chain. */
function earLine(rungs: Act[], { made }: Reading): Line {
  const states = rungs.map((rung) => rung.state);
  return {
    type: "ear",
    act: rungs[0] as Act,
    depth: 0,
    state: states.find((state) => state !== "done") ?? "done",
    rungs,
    acts: chips(rungs.flatMap((rung) => made.get(rung.id) ?? [])),
  };
}

/** The parts of a step that a thread read, and what they were read of. */
interface Kept {
  act: Act;
  depth: number;
  stamp: number;
  word: string;
  stream?: { text: string; thinking: string };
  owed: string;
  parts: ReturnType<typeof step>;
}
/** The parts of each step that a thread read, by the index of the host that it read them from. */
const steps = new WeakMap<object, Map<string, Kept>>();

/** The thread of a chain as a person reads it at a glance, from the ladder that the crate folds of it: the lines of
 * each step that it shows, with the lines that head the steps under it and those that end them, each line of the rungs
 * of an ear, and a line of more in place of each fold, which a key of the opened set opens. It makes the lines of the
 * steps that the crate shows, so that it takes the time of what it shows and not the time of the life. */
export function thread(
  rungs: readonly Rung[],
  chain: string,
  word: (rung: Act) => string,
  streams: ReadonlyMap<string, { text: string; thinking: string }>,
  owed: ReadonlySet<string>,
  index: ActIndex,
): Line[] {
  const { ids } = index;
  const reading: Reading = { chain, made: index.made, word, streams };
  // A host that stamps each change of the acts that an act made keeps the parts of each step that it read, until the
  // step, its word, what it streams, or the prompts that the operator owes change.
  const kept = index.stamp && (steps.get(index.made) ?? new Map<string, Kept>());
  if (kept) steps.set(index.made, kept);
  const asks = [...owed].join(" ");
  const parts = (act: Act, depth: number) => {
    const stamp = index.stamp?.(act.id) ?? 0;
    const text = act.kind === "rung" ? word(act) : "";
    const stream = streams.get(act.id);
    const held = kept?.get(act.id);
    if (
      held &&
      held.act === act &&
      held.depth === depth &&
      held.stamp === stamp &&
      held.word === text &&
      held.stream === stream &&
      held.owed === asks
    )
      return held.parts;
    const made = step(act, depth, reading);
    kept?.set(act.id, { act, depth, stamp, word: text, stream, owed: asks, parts: made });
    return made;
  };
  const out: Line[] = [];
  // The lines that end a step stand after the steps under it, so each waits until a line of its depth or above comes.
  const trails: { depth: number; lines: Line[] }[] = [];
  const close = (depth: number) => {
    for (let last = trails.at(-1); last && last.depth >= depth; last = trails.at(-1)) {
      trails.pop();
      out.push(...last.lines);
    }
  };
  for (const rung of rungs) {
    close(rung.depth);
    if (rung.type === "ear") {
      const acts = rung.rungs.flatMap((id) => ids.get(id) ?? []);
      if (acts.length) out.push(earLine(acts, reading));
      continue;
    }
    const act = ids.get(rung.type === "step" ? rung.id : rung.head);
    if (!act) continue;
    if (rung.type === "more") {
      const { depth, count, key, heads } = rung;
      out.push({
        type: "more",
        act,
        depth,
        count,
        of: rung.of === "rungs" ? "rungs" : "prompts",
        key,
        heads,
      });
      continue;
    }
    const read = parts(act, rung.depth);
    out.push(read.line, ...read.lead);
    trails.push({ depth: rung.depth, lines: read.trail });
  }
  close(0);
  return out;
}

/** How many of the last prompts of a thread, and of the last rungs of each ladder, a condensed thread shows. */
export const SHOWN = { prompts: 6, rungs: 4 };

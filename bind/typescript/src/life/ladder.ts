import { cancelled, type Exit, failed, type LiveAct, WORK } from "./acts.js";

/** What an act of a thread is: it runs, it ended well, it failed, a cancel ended it, a pause holds it, or it asks the
 * operator. */
export type ActState = "running" | "stands" | "done" | "failed" | "cancelled" | "held" | "asks";
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
  | { type: "prompt"; act: LiveAct; depth: number; state: ActState }
  | { type: "rung"; act: LiveAct; depth: number; state: ActState; word: string; more: number; acts: Chip[] }
  | { type: "act"; act: LiveAct; depth: number; state: ActState }
  | { type: "ear"; act: LiveAct; depth: number; state: ActState; rungs: LiveAct[]; acts: Chip[] }
  | { type: "tail"; act: LiveAct; depth: number; text: string; model: boolean }
  | { type: "value"; act: LiveAct; depth: number; text: string }
  | {
      type: "more";
      act: LiveAct;
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
  state: ActState;
}

/** The state of an act, with the prompts that the operator owes. */
export function actState(act: LiveAct, owed: ReadonlySet<string>): ActState {
  if (owed.has(act.id)) return "asks";
  if ((act.paused || act.unfinished) && !act.done) return "held";
  if (cancelled(act)) return "cancelled";
  if (failed(act)) return "failed";
  if (!act.done && !WORK.includes(act.kind)) return "stands";
  return act.done && (act.kind !== "rung" || act.run?.status !== "running") ? "done" : "running";
}

/** The acts of a rung as chips: a run of acts of one kind and one state stands as one chip, by its first and its last
 * number. */
export function chips(acts: readonly LiveAct[], owed: ReadonlySet<string>): Chip[] {
  const out: Chip[] = [];
  for (const act of acts) {
    const state = actState(act, owed);
    const last = out.at(-1);
    if (last && state !== "running" && last.state === state && kindOf(last.ids[0] ?? "") === act.kind)
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
export function tailOf(act: LiveAct, count = 3): string {
  const exit = act.value as Exit | undefined;
  const lines = `${exit?.stdout?.content ?? ""}${exit?.stderr?.content ?? ""}`
    .replace(/\n+$/, "")
    .split("\n");
  return lines.slice(-count).join("\n");
}

/** What a host already holds of the acts of a life, which the ladder reads in place of a pass over every act: the acts
 * on the chain but its chains, the acts that each act made, and each act by its name, each in the order of the acts. */
export type ActIndex = {
  on: readonly LiveAct[];
  made: ReadonlyMap<string, readonly LiveAct[]>;
  ids: ReadonlyMap<string, LiveAct>;
};

/** The acts that each act made, in the order of the acts. */
export function makers(acts: readonly LiveAct[]): Map<string, LiveAct[]> {
  const made = new Map<string, LiveAct[]>();
  for (const act of acts) {
    const list = made.get(act.by);
    if (list) list.push(act);
    else made.set(act.by, [act]);
  }
  return made;
}

/** How a thread reads its acts: the chain, the word of a rung, what each rung streams, and the prompts that the
 * operator owes. */
interface Reading {
  chain: string;
  made: ReadonlyMap<string, readonly LiveAct[]>;
  word: (rung: LiveAct) => string;
  streams: ReadonlyMap<string, { text: string; thinking: string }>;
  owed: ReadonlySet<string>;
}

/** A step of a thread: its line, the lines that stand first under it, the acts whose steps stand under it, and the
 * lines that end it. A prompt holds the steps of the rungs of its ladder and ends with its value. A rung holds the
 * tails of what streams and the steps of the prompts that it made on the chain, and the chips of every other act that
 * it made. A command that runs holds its tail. */
function step(act: LiveAct, depth: number, { chain, made, word, streams, owed }: Reading) {
  const state = actState(act, owed);
  const lead: Line[] = [];
  const trail: Line[] = [];
  if (act.kind === "prompt") {
    const text = act.done && !failed(act) && !cancelled(act) ? valueText(act.value) : "";
    if (text) trail.push({ type: "value", act, depth: depth + 1, text });
    const line: Line = { type: "prompt", act, depth, state };
    return { line, lead, steps: made.get(act.id) ?? [], trail };
  }
  if (act.kind === "rung") {
    const children = made.get(act.id) ?? [];
    const [first = "", ...rest] = word(act).split("\n");
    const asked = children.filter((child) => child.kind === "prompt" && child.on === chain);
    const asks = new Set(asked);
    const line: Line = {
      type: "rung",
      act,
      depth,
      state,
      word: first,
      more: rest.length,
      acts: chips(
        children.filter((child) => !asks.has(child)),
        owed,
      ),
    };
    const stream = streams.get(act.id);
    if (stream && !act.done)
      lead.push({
        type: "tail",
        act,
        depth: depth + 1,
        text: (stream.text || stream.thinking).replace(/\n+$/, "").split("\n").slice(-3).join("\n"),
        model: true,
      });
    for (const child of children)
      if (child.kind === "bash" && !child.done && tailOf(child))
        lead.push({ type: "tail", act: child, depth: depth + 1, text: tailOf(child), model: false });
    return { line, lead, steps: asked, trail };
  }
  if (act.kind === "bash" && !act.done && tailOf(act))
    lead.push({ type: "tail", act, depth: depth + 1, text: tailOf(act), model: false });
  return { line: { type: "act", act, depth, state } as Line, lead, steps: [] as LiveAct[], trail };
}

/** A rung that no act wrote: an ear of the outside wrote it, or the chain wrote it on itself and it retells no word of
 * another rung, as the setup of the root and each stand and cd. The words of a chain that retell its source, whose
 * donor is a rung, are no such rung. */
function written(act: LiveAct, known: (id: string) => boolean): boolean {
  return (
    act.kind === "rung" && act.by !== "operator" && (!known(act.by) || (act.by === act.on && !act.words[1]))
  );
}

/** The line of the rungs in a row that one maker wrote on the chain with no word of its own, as the words of the
 * extensions and the setup of a chain. */
function earLine(rungs: LiveAct[], { made, owed }: Reading): Line {
  const states = rungs.map((rung) => actState(rung, owed));
  return {
    type: "ear",
    act: rungs[0] as LiveAct,
    depth: 0,
    state: states.find((state) => state !== "done") ?? "done",
    rungs,
    acts: chips(
      rungs.flatMap((rung) => made.get(rung.id) ?? []),
      owed,
    ),
  };
}

/** The steps at the top of the thread of a chain: each act on it that no act on it made, in the order it began, and
 * the rungs in a row that one maker wrote with no word of its own, as one step. */
function tops(on: readonly LiveAct[], local: (id: string) => boolean, rows: ReadonlyMap<string, LiveAct>) {
  const out: { act: LiveAct; ear?: LiveAct[] }[] = [];
  for (const act of on) {
    if (local(act.by)) continue;
    if (written(act, (id) => rows.has(id))) {
      const last = out.at(-1);
      if (last?.ear && last.act.by === act.by) last.ear.push(act);
      else out.push({ act, ear: [act] });
    } else out.push({ act });
  }
  return out;
}

/** The thread of a chain: each step at its top, and under each step the steps and the lines that it holds. The index
 * of a host that holds one spares a pass over every act, so that a long life draws its thread as fast as a short
 * one. */
export function ladder(
  chain: string,
  acts: readonly LiveAct[],
  word: (rung: LiveAct) => string,
  streams: ReadonlyMap<string, { text: string; thinking: string }>,
  owed: ReadonlySet<string>,
  index?: ActIndex,
): Line[] {
  const on = index?.on ?? acts.filter((act) => act.on === chain && act.kind !== "chain");
  const local = new Set(on.map((act) => act.id));
  const reading: Reading = { chain, made: index?.made ?? makers(acts), word, streams, owed };
  const lines: Line[] = [];
  const emit = (act: LiveAct, depth: number): void => {
    const { line, lead, steps, trail } = step(act, depth, reading);
    lines.push(line, ...lead);
    for (const one of steps) emit(one, depth + 1);
    lines.push(...trail);
  };
  const rows = index?.ids ?? new Map(acts.map((act) => [act.id, act]));
  for (const top of tops(on, (id) => local.has(id), rows))
    if (top.ear) lines.push(earLine(top.ear, reading));
    else emit(top.act, 0);
  return lines;
}

/** The parts of a step that a thread read, and what they were read of. */
interface Kept {
  act: LiveAct;
  depth: number;
  stamp: number;
  word: string;
  stream?: { text: string; thinking: string };
  owed: string;
  parts: ReturnType<typeof step>;
}
/** The parts of each step that a thread read, by the index of the host that it read them from. */
const steps = new WeakMap<object, Map<string, Kept>>();

/** The steps of one level of a thread, which a list of acts that only grows gives: each by its name and its kind, the
 * rungs of an ear that stand as one step, the place of each step by the name of its act, and the places of the steps
 * of each kind, so that a level finds what a fold holds with no pass over the steps. */
interface Listed {
  seen: number;
  steps: { id: string; kind: string; by: string; ear?: string[] }[];
  place: Map<string, number>;
  kinds: Map<string, number[]>;
}
/** The steps of each level that a thread read, by the index of the host, and by the list that gives them. */
const levels = new WeakMap<object, Map<string, Listed>>();

/** The steps that a list of acts gives, read again from where the list was read last. A list that holds fewer acts
 * than were read is a new list, which is read whole. */
function listed(
  kept: Map<string, Listed>,
  key: string,
  acts: readonly LiveAct[],
  take: (act: LiveAct) => "step" | "ear" | "",
): Listed {
  let held = kept.get(key);
  if (!held || acts.length < held.seen) {
    held = { seen: 0, steps: [], place: new Map(), kinds: new Map() };
    kept.set(key, held);
  }
  for (; held.seen < acts.length; held.seen++) {
    const act = acts[held.seen] as LiveAct;
    const taken = take(act);
    if (!taken) continue;
    const last = held.steps.at(-1);
    // The rungs in a row that one maker wrote with no word of its own stand as one step.
    if (taken === "ear" && last?.ear && last.by === act.by) {
      last.ear.push(act.id);
      continue;
    }
    const place = held.steps.length;
    held.steps.push({
      id: act.id,
      kind: act.kind,
      by: act.by,
      ...(taken === "ear" ? { ear: [act.id] } : {}),
    });
    held.place.set(act.id, place);
    const kinds = held.kinds.get(act.kind);
    if (kinds) kinds.push(place);
    else held.kinds.set(act.kind, [place]);
  }
  return held;
}

/** The thread of a chain condensed, as condense gives the ladder of the chain, read from the index of a host: it
 * makes the lines of the steps that it shows, and only counts the steps that a fold holds, so that it takes the time
 * of what it shows and not the time of the life. The live acts of the index are each act that is not done, and each
 * rung whose run runs, which the steps that stay shown hold. */
export function thread(
  chain: string,
  word: (rung: LiveAct) => string,
  streams: ReadonlyMap<string, { text: string; thinking: string }>,
  owed: ReadonlySet<string>,
  opened: ReadonlySet<string>,
  index: ActIndex & { live: Iterable<LiveAct>; stamp?: (id: string) => number },
  shown: { prompts: number; rungs: number } = SHOWN,
): Line[] {
  const { ids } = index;
  const reading: Reading = { chain, made: index.made, word, streams, owed };
  // A host that stamps each change of the acts that an act made keeps the parts of each step that it read, until the
  // step, its word, what it streams, or the prompts that the operator owes change.
  const kept = index.stamp && (steps.get(index.made) ?? new Map<string, Kept>());
  if (kept) steps.set(index.made, kept);
  const asks = [...owed].join(" ");
  const parts = (act: LiveAct, depth: number) => {
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
  const local = (id: string) => {
    const act = ids.get(id);
    return act !== undefined && act.on === chain && act.kind !== "chain";
  };
  const ear = (act: LiveAct) => written(act, (id) => ids.has(id));
  // A step stays shown when a prompt or a rung under it asks the operator or runs: each such act marks each step from
  // it up to the top, through the lines that the thread makes of it, in the level of the step above it.
  const alive = new Map<string, string[]>();
  for (const act of [...index.live, ...[...owed].flatMap((id) => ids.get(id) ?? [])]) {
    if ((act.kind !== "prompt" && act.kind !== "rung") || !["asks", "running"].includes(actState(act, owed)))
      continue;
    const path: string[] = [];
    for (let at: LiveAct | undefined = act; at && path.length <= ids.size; ) {
      path.push(at.id);
      if (at.on === chain && at.kind !== "chain" && !local(at.by)) {
        if (!ear(at))
          for (const [place, id] of path.entries()) {
            const above = path[place + 1] ?? "";
            const marked = alive.get(above) ?? [];
            if (!marked.includes(id)) marked.push(id);
            alive.set(above, marked);
          }
        break;
      }
      const maker = ids.get(at.by);
      const under: boolean =
        maker?.kind === "prompt" || (maker?.kind === "rung" && at.kind === "prompt" && at.on === chain);
      at = under ? maker : undefined;
    }
  }
  const cache = levels.get(index.made) ?? new Map<string, Listed>();
  levels.set(index.made, cache);
  /** The steps under a step: each act that a prompt made, and each prompt that a rung made on the chain. */
  const under = (act: LiveAct) =>
    act.kind === "prompt" || act.kind === "rung"
      ? listed(cache, `${act.kind}:${act.id}`, index.made.get(act.id) ?? [], (child) =>
          act.kind === "prompt" || (child.kind === "prompt" && child.on === chain) ? "step" : "",
        )
      : empty;
  const out: Line[] = [];
  /** One level of the thread, as level condenses it: a group of the lines that stand first, which stays, then a group
   * for each step, and the lines that end the level in the last group. It reads only the steps that it shows, and
   * the places of the kinds of the steps that a fold holds. */
  const level = (
    lead: Line[],
    list: Listed,
    trail: Line[],
    depth: number,
    key: string,
    above: string,
  ): void => {
    const rungs = key.startsWith("rungs:");
    const keep = rungs ? shown.rungs : shown.prompts;
    const count = list.steps.length;
    if (lead.length || (!count && trail.length)) out.push(...lead, ...(count ? [] : trail));
    // The steps that show: each step once the operator opened the fold of the level, else the last steps and each
    // step under which a prompt or a rung asks the operator or runs.
    const end = opened.has(key) ? 0 : Math.max(0, count - keep);
    const marked = (alive.get(above) ?? []).flatMap((id) => {
      const place = list.place.get(id);
      return place !== undefined && place < end ? [place] : [];
    });
    const shows = [...new Set(marked)].sort((one, other) => one - other);
    for (let place = end; place < count; place++) shows.push(place);
    const show = (place: number) => {
      const one = list.steps[place];
      const act = one && ids.get(one.id);
      if (!one || !act) return;
      if (one.ear)
        out.push(
          earLine(
            one.ear.flatMap((id) => ids.get(id) ?? []),
            reading,
          ),
        );
      else {
        const read = parts(act, depth);
        out.push(read.line);
        level(
          read.lead,
          under(act),
          read.trail,
          depth + 1,
          `${act.kind === "prompt" ? "rungs" : "prompts"}:${act.id}`,
          act.id,
        );
      }
      if (place === count - 1) out.push(...trail);
    };
    let before = -1;
    for (const place of [...shows, count]) {
      // A fold holds two steps in a row at least, and a lone step that it would hold shows.
      const from = before + 1;
      if (place - from >= 2) {
        const heads = [...list.kinds]
          .flatMap(([kind, places]) => {
            const at = places[lowest(places, from)];
            return at !== undefined && at < place ? [[at, kind] as const] : [];
          })
          .sort(([one], [other]) => one - other)
          .map(([, kind]) => kind);
        const head = ids.get(list.steps[from]?.id ?? "");
        if (head)
          out.push({
            type: "more",
            act: head,
            depth,
            count: place - from,
            of: rungs ? "rungs" : "prompts",
            key,
            heads,
          });
      } else if (place - from === 1) show(from);
      if (place < count) show(place);
      before = place;
    }
  };
  const top = listed(cache, `on:${chain}`, index.on, (act) =>
    local(act.by) ? "" : ear(act) ? "ear" : "step",
  );
  level([], top, [], 0, `prompts:${ids.get(top.steps[0]?.id ?? "")?.on ?? ""}`, "");
  return out;
}

/** The first place in a sorted list of places that is not less than a place. */
function lowest(places: readonly number[], place: number): number {
  let low = 0;
  for (let high = places.length; low < high; ) {
    const middle = (low + high) >> 1;
    if ((places[middle] ?? 0) < place) low = middle + 1;
    else high = middle;
  }
  return low;
}

/** The steps of a list that gives none. */
const empty: Listed = { seen: 0, steps: [], place: new Map(), kinds: new Map() };

/** How many of the last prompts of a thread, and of the last rungs of each ladder, a condensed thread shows. */
export const SHOWN = { prompts: 6, rungs: 4 };

/** A long thread as a person reads it at a glance: the last steps at each level of the thread, and a line of more in
 * place of each run of earlier ones, which a key of the opened set opens. At the top and under a rung, a step is a
 * prompt, or another line that heads lines of its own, and the last prompts show; under a prompt, a step is a rung,
 * and the last rungs show. A step that asks the operator or runs stays shown, and a fold holds two steps in a row at
 * least. The key of a fold is prompts: or rungs: and the act that the steps stand under, or the chain at the top. */
export function condense(
  lines: readonly Line[],
  opened: ReadonlySet<string>,
  shown: { prompts: number; rungs: number } = SHOWN,
): Line[] {
  return level(lines, 0, `prompts:${lines[0]?.act.on ?? ""}`, opened, shown);
}

/** The lines of one level of a thread, condensed, with each level under them condensed in turn. */
function level(
  lines: readonly Line[],
  depth: number,
  key: string,
  opened: ReadonlySet<string>,
  shown: { prompts: number; rungs: number },
): Line[] {
  const heads = (line: Line) => line.depth === depth && ["prompt", "rung", "act", "ear"].includes(line.type);
  const groups: Line[][] = [];
  for (const line of lines) {
    if (heads(line) || !groups.length) groups.push([line]);
    else groups.at(-1)?.push(line);
  }
  const stays = (group: Line[]) =>
    !heads(group[0] as Line) ||
    group.some(
      (line) => (line.type === "prompt" || line.type === "rung") && ["asks", "running"].includes(line.state),
    );
  const rungs = key.startsWith("rungs:");
  const keep = rungs ? shown.rungs : shown.prompts;
  const folds = opened.has(key)
    ? new Set<number>()
    : new Set(
        groups.flatMap((group, index) => (index < groups.length - keep && !stays(group) ? [index] : [])),
      );
  for (const index of [...folds]) if (!folds.has(index - 1) && !folds.has(index + 1)) folds.delete(index);
  const out: Line[] = [];
  let hidden: Line[][] = [];
  const flush = () => {
    const [head] = hidden[0] ?? [];
    if (head)
      out.push({
        type: "more",
        act: head.act,
        depth,
        count: hidden.length,
        of: rungs ? "rungs" : "prompts",
        key,
        heads: [...new Set(hidden.map((group) => group[0]?.act.kind ?? ""))],
      });
    hidden = [];
  };
  for (const [index, group] of groups.entries()) {
    if (folds.has(index)) {
      hidden.push(group);
      continue;
    }
    flush();
    const [head, ...under] = group as [Line, ...Line[]];
    out.push(head);
    if (!heads(head)) {
      out.push(...under);
      continue;
    }
    // A value or a tail at the level of the head stays with it; the lines under it form the next level.
    const next = under.filter((line) => line.depth > depth);
    const after = under.filter((line) => line.depth <= depth);
    out.push(
      ...level(
        next,
        depth + 1,
        `${head.type === "prompt" ? "rungs" : "prompts"}:${head.act.id}`,
        opened,
        shown,
      ),
    );
    out.push(...after);
  }
  flush();
  return out;
}

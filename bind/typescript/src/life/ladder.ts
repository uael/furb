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
  if (act.paused && !act.done) return "held";
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

/** The thread of a chain: each act on it that no act on it made, in the order it began, and under a prompt each rung
 * of its ladder, under a rung each prompt that it made on the chain, and the chips of every other act that it made.
 * A command that runs shows its tail, and a model that answers shows what it writes. */
export function ladder(
  chain: string,
  acts: readonly LiveAct[],
  word: (rung: LiveAct) => string,
  streams: ReadonlyMap<string, { text: string; thinking: string }>,
  owed: ReadonlySet<string>,
): Line[] {
  const on = acts.filter((act) => act.on === chain && act.kind !== "chain");
  const local = new Set(on.map((act) => act.id));
  const made = new Map<string, LiveAct[]>();
  for (const act of acts) {
    if (act.id === chain) continue;
    const list = made.get(act.by) ?? [];
    list.push(act);
    made.set(act.by, list);
  }
  const lines: Line[] = [];
  const emit = (act: LiveAct, depth: number): void => {
    const state = actState(act, owed);
    if (act.kind === "prompt") {
      lines.push({ type: "prompt", act, depth, state });
      for (const rung of made.get(act.id) ?? []) emit(rung, depth + 1);
      const text = act.done && !failed(act) && !cancelled(act) ? valueText(act.value) : "";
      if (text) lines.push({ type: "value", act, depth: depth + 1, text });
      return;
    }
    if (act.kind === "rung") {
      const children = made.get(act.id) ?? [];
      const text = word(act);
      const [first = "", ...rest] = text.split("\n");
      const asked = children.filter((child) => child.kind === "prompt" && child.on === chain);
      lines.push({
        type: "rung",
        act,
        depth,
        state,
        word: first,
        more: rest.length,
        acts: chips(
          children.filter((child) => !asked.includes(child)),
          owed,
        ),
      });
      const stream = streams.get(act.id);
      if (stream && !act.done)
        lines.push({
          type: "tail",
          act,
          depth: depth + 1,
          text: (stream.text || stream.thinking).replace(/\n+$/, "").split("\n").slice(-3).join("\n"),
          model: true,
        });
      for (const child of children)
        if (child.kind === "bash" && !child.done && tailOf(child))
          lines.push({ type: "tail", act: child, depth: depth + 1, text: tailOf(child), model: false });
      for (const prompt of asked) emit(prompt, depth + 1);
      return;
    }
    lines.push({ type: "act", act, depth, state });
    if (act.kind === "bash" && !act.done && tailOf(act))
      lines.push({ type: "tail", act, depth: depth + 1, text: tailOf(act), model: false });
  };
  const rows = new Map(acts.map((act) => [act.id, act]));
  for (const act of on) {
    if (local.has(act.by)) continue;
    // The rungs that an ear made, as the words of the extensions, stand together on one line.
    const last = lines.at(-1);
    if (act.kind === "rung" && act.by !== "operator" && !rows.has(act.by)) {
      if (last?.type === "ear" && last.act.by === act.by) last.rungs.push(act);
      else lines.push({ type: "ear", act, depth: 0, state: "done", rungs: [act], acts: [] });
      continue;
    }
    emit(act, 0);
  }
  for (const line of lines)
    if (line.type === "ear") {
      const states = line.rungs.map((rung) => actState(rung, owed));
      line.state = states.find((state) => state !== "done") ?? "done";
      line.acts = chips(
        line.rungs.flatMap((rung) => made.get(rung.id) ?? []),
        owed,
      );
    }
  return lines;
}

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

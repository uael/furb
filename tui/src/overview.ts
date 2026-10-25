import type { RGBA } from "@opentui/core";
import type { Measure, Owed, Row, RowState, Tone } from "furb/life";
import { asksOperator, gist, type Loom } from "furb/life";
import { clip } from "./format.ts";
import type { ActRow } from "./session.ts";
import { theme as c, spin } from "./theme.ts";
import { bold, mix, type Part } from "./ui.ts";

/** The views of a life that the TUI holds side by side, which a key, a command and a setting switch at run time: the
 * main, the feed beside the threads, the loom of the chains in time over the feed, and the graph of the programs that run. */
export const lifeViews = ["main", "loom", "graph"] as const;
export type LifeView = (typeof lifeViews)[number];
export const lifeViewLabels: Record<LifeView, string> = { main: "Main", loom: "Loom", graph: "Graph" };
export const lifeViewDetails: Record<LifeView, string> = {
  main: "The feed of the selected chain beside the threads",
  loom: "Each chain as a thread in time over the feed, with a cursor in time",
  graph: "The threads and the map of the programs, with a zoom into a program, a word and an act",
};
/** Whether a text names a view of the life. */
export const isLifeView = (text: unknown): text is LifeView => lifeViews.includes(text as LifeView);

/** The actor of a chain: the actor of its last answer, else the actor that its last rung of the actor set, else the
 * actor that its last prompt asked, else the actor of the life. */
export function actorOf(
  chain: string,
  acts: readonly ActRow[],
  measures: Readonly<Record<string, Measure>>,
  fallback: string,
): string {
  const answered = measures[chain]?.actor;
  if (answered) return answered;
  for (let index = acts.length - 1; index >= 0; index--) {
    const act = acts[index];
    if (!act || act.on !== chain) continue;
    const set = act.kind !== "rung" ? null : /^actor = ['"]([^'"]+)['"]\s*$/.exec(String(act.words[0] ?? ""));
    if (set?.[1]) return set[1];
    const asked = act.kind === "prompt" && !asksOperator(act) ? String(act.words[2] ?? "") : "";
    if (asked) return asked;
  }
  return fallback;
}

/** The question that a chain waits on: the first line of the oldest prompt that the operator owes on it, with the
 * count of the others. */
export function questionOf(chain: string, owed: readonly Owed[]): string {
  const mine = owed.filter((prompt) => prompt.chain === chain);
  const [first] = mine;
  if (!first) return "";
  return `${gist(first.message)}${mine.length > 1 ? `  +${mine.length - 1}` : ""}`;
}

/** The columns of what a row of the loom waits on. */
const waitsWidth = 22;
/** An actor with no provider, as opus/high for claude-cli:opus/high. */
export const shortActor = (actor: string): string => actor.replace(/^[\w.-]+:/, "");
/** The mark and the color of each state of a row. */
export const rowMarks: Record<RowState, [string, () => RGBA]> = {
  asks: ["◆", () => c.warning],
  failed: ["✗", () => c.danger],
  working: ["◉", () => c.accent],
  paused: ["◌", () => c.warning],
  done: ["✓", () => c.success],
  idle: ["○", () => c.faint],
};
const toneColor = (tone: Tone): RGBA =>
  ({ work: c.accent, past: c.muted, asks: c.warning, failed: c.danger, none: c.faint })[tone];

/** What the loom reads to draw itself: the loom of the life as it stands at the time cursor, the actor of each chain,
 * the chain that the session selects, and whether the cursor stands in the past, where nothing moves. */
export interface LoomInput {
  loom: Loom;
  width: number;
  height: number;
  selected: string;
  frozen: boolean;
  /** Whether the keys move on the loom. */
  steering: boolean;
  actor(chain: string): string;
}

/** A bar of eight steps to a cell, for a share from 0 to 1. */
export function gauge(share: number, cells: number): string {
  const steps = Math.round(Math.max(0, Math.min(1, share)) * cells * 8);
  const blocks = ["", "▏", "▎", "▍", "▌", "▋", "▊", "▉"];
  return ("█".repeat(Math.floor(steps / 8)) + (blocks[steps % 8] ?? "")).padEnd(cells, " ");
}

/** Parts cut or filled to a width, on a ground. */
export function pad(parts: readonly Part[], width: number, ground?: RGBA): Part[] {
  const out: Part[] = [];
  let used = 0;
  for (const [text, fg, attributes, bg] of parts) {
    if (used >= width) break;
    const room = width - used;
    const cut = Bun.stringWidth(text) > room ? clip(text, room) : text;
    used += Bun.stringWidth(cut);
    out.push([cut, fg, attributes, bg ?? ground]);
  }
  if (used < width) out.push([" ".repeat(width - used), undefined, 0, ground]);
  return out;
}

/** The loom of the chains in time: one thread for each chain in the order of the tree, under the operator, with its
 * state, its cells over the moments of the life, the question that it waits on or the act that it waits for, its
 * actor, its dollars against the ceiling of its grant, and the share of the window that its last answer read. The past
 * fades and folds, and the right edge is now, or the moment of the time cursor. */
export class LoomPane {
  /** The rows whose workers the loom folds into them. */
  readonly folds = new Set<string>();
  /** The moment that the time cursor stands at, and nothing at the present. */
  cursor?: number;
  /** The first row that shows, which follows the selection. */
  private top = 0;
  /** The chain of each row of the last draw, by its row on the screen, for a click. */
  targets: string[] = [];

  /** The columns of the label of a row. */
  static labelWidth(width: number): number {
    return width >= 100 ? 22 : 16;
  }
  /** Whether a width holds the actor and the measure of each row. */
  static wide(width: number): boolean {
    return width >= 110;
  }
  /** The columns of a row that are not its cells. */
  static fixed(width: number): number {
    return 4 + LoomPane.labelWidth(width) + 1 + 2 + waitsWidth + (LoomPane.wide(width) ? 2 + 14 + 20 : 0);
  }
  static cellWidth(width: number): number {
    return Math.max(8, width - LoomPane.fixed(width));
  }

  /** The ruler and the rows of the loom at a size, the selected chain in view. */
  draw(input: LoomInput): Part[][] {
    const { loom, height } = input;
    const rows = loom.rows;
    const room = Math.max(1, height - 1);
    const at = Math.max(
      0,
      rows.findIndex((row) => row.id === input.selected),
    );
    if (at < this.top) this.top = at;
    if (at >= this.top + room) this.top = at - room + 1;
    this.top = Math.max(0, Math.min(this.top, rows.length - room));
    const shown = rows.slice(this.top, this.top + room);
    this.targets = ["", ...shown.map((row) => row.id)];
    return [this.ruler(input), ...shown.map((row) => this.row(row, input))];
  }

  private ruler({ loom, width, frozen }: LoomInput): Part[] {
    const cellWidth = LoomPane.cellWidth(width);
    const span = loom.rows[0]?.cells.length ?? 0;
    const edge: Part = frozen ? ["then", c.secondary, bold] : [" now", c.faint];
    const start = 4 + LoomPane.labelWidth(width) + 1;
    const parts: Part[] = [[" ".repeat(start), c.faint]];
    const blank = cellWidth - span;
    if (loom.folded && span) {
      const older = blank + loom.folded;
      parts.push([" ".repeat(Math.max(0, older - 7)), c.faint], ["older ┊", c.faint]);
      parts.push([" ".repeat(Math.max(0, cellWidth - Math.max(older, 7) - 4)), c.faint], edge);
    } else parts.push([" ".repeat(Math.max(0, cellWidth - 4)), c.faint], edge);
    parts.push(["  "], ["waits on".padEnd(waitsWidth), c.faint]);
    if (LoomPane.wide(width))
      parts.push(
        ["  "],
        ["actor".padEnd(14), c.faint],
        ["dollars".padStart(9), c.faint],
        ["  window", c.faint],
      );
    return pad(parts, width);
  }

  private row(row: Row, input: LoomInput): Part[] {
    const { loom, width, frozen } = input;
    const selected = row.id === input.selected;
    const [mark, color] = rowMarks[row.state];
    const live = row.state === "working" && !frozen ? spin() : mark;
    const ground = selected ? (input.steering ? c.selected : c.panel) : undefined;
    const tally = row.folded
      ? ` ${Object.entries(row.tally)
          .map(([state, n]) => `${n}${rowMarks[state as RowState][0]}`)
          .join(" ")}`
      : "";
    const fold = row.workers ? (row.folded ? "▸ " : "▾ ") : "";
    const labelWidth = LoomPane.labelWidth(width);
    const label = clip(`${row.lines}${fold}${row.label}${tally}`, labelWidth).padEnd(labelWidth);
    const cellWidth = LoomPane.cellWidth(width);
    const count = row.cells.length;
    const cells: Part[] = row.cells.map((cell, index) =>
      index === count - 1 && row.state === "working" && !frozen
        ? [spin(), c.accent]
        : [cell.mark, mix(c.background, toneColor(cell.tone), 0.45 + (0.55 * (index + 1)) / count)],
    );
    const owed = row.id === "operator" ? loom.owed.length : 0;
    const question = row.id === "operator" ? "" : questionOf(row.id, loom.owed);
    const waits =
      row.id === "operator"
        ? owed
          ? `${owed} owed`
          : ""
        : question
          ? `◆ ${question}`
          : row.state === "paused"
            ? "paused"
            : (row.waits ?? "");
    const parts: Part[] = [
      [selected ? "▎" : " ", c.accent],
      [" "],
      [live, color()],
      [" "],
      [label, selected ? c.text : row.state === "idle" ? c.muted : c.text, selected ? bold : 0],
      [" "],
      [" ".repeat(Math.max(0, cellWidth - cells.length)), c.faint],
      ...cells,
      ["  "],
      [clip(waits, waitsWidth).padEnd(waitsWidth), question || row.state === "paused" ? c.warning : c.muted],
    ];
    if (LoomPane.wide(width)) {
      const actor = row.id === "operator" ? "" : input.actor(row.id);
      parts.push(["  "], [clip(shortActor(actor), 13).padEnd(14), c.muted], ...measureParts(row.measure));
    }
    return pad(parts, width, ground);
  }

  /** The chain of the row that a click on a row of the pane reaches. */
  click(row: number): string | undefined {
    return this.targets[row] || undefined;
  }
}

/** The measure of a row: its dollars against the ceiling of its grant, and the share of the window that its last
 * answer read, as a bar of eight steps to a cell. */
export function measureParts(measure: Measure | undefined): Part[] {
  if (!measure) return [[" ".repeat(20), c.faint]];
  // A ceiling shows beside the dollars that the chain spent, each at two places, so that a row stays short.
  const cents = (value: number) => (value > 0 && value < 0.01 ? "<$0.01" : `$${value.toFixed(2)}`);
  const money = measure.ceiling
    ? `${cents(measure.spent)}/${cents(measure.ceiling).slice(1)}`
    : measure.answers
      ? cents(measure.dollars)
      : "";
  const full = measure.ceiling !== undefined && measure.spent >= measure.ceiling;
  const share = measure.share ?? 0;
  return [
    [money.padStart(9), full ? c.warning : c.muted],
    [" ▕", c.faint],
    [gauge(share, 4), share > 0.5 ? c.warning : c.accent],
    ["▏", c.faint],
    [
      measure.share === undefined
        ? "    "
        : (share > 0 && share < 0.01 ? "<1%" : `${Math.round(share * 100)}%`).padStart(4),
      c.faint,
    ],
  ];
}

/** The rows of parts as the text of one node. */
export function joined(rows: readonly Part[][]): Part[] {
  return rows.flatMap((row, index) => (index ? [["\n"] as Part, ...row] : row));
}

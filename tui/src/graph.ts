import type { RGBA } from "@opentui/core";
import {
  type ActState,
  actState,
  type Loom,
  messages,
  outcome,
  outline,
  type Place,
  places,
  type RowState,
  route,
  type SourceLine,
  source,
  type Thread,
  threads,
  valueText,
  you,
} from "furb/life";
import { clock } from "./clock.ts";
import { clip, dollars } from "./format.ts";
import { questionOf, shortActor } from "./overview.ts";
import type { ActRow } from "./session.ts";
import { theme as c, spin } from "./theme.ts";
import { bold, mix, type Part, plain } from "./ui.ts";

/** How near the graph stands to the life: the tree of threads, the map of the programs, the source of one program,
 * one word of it, or one act. */
export type Level = "threads" | "map" | "program" | "word" | "act";
export const levels: readonly Level[] = ["threads", "map", "program", "word", "act"];

/** What the graph reads of the App that holds it: the life as it stands at the time cursor, and the controls that it
 * asks the App for. */
export interface Stage {
  readonly acts: readonly ActRow[];
  readonly loom: Loom;
  readonly owed: ReadonlySet<string>;
  /** Whether the time cursor stands in the past, where nothing moves. */
  readonly frozen: boolean;
  /** The chain that the session selects, whose words the source reads. */
  readonly selected: string;
  words(): Map<string, string>;
  label(chain: string): string;
  /** The actor of a chain, as its answers, its rungs or its prompts name it. */
  actor(chain: string): string;
  streams(): ReadonlyMap<string, { text: string; thinking: string }>;
  /** The session selects a chain. */
  select(chain: string): Promise<void>;
  /** The lens shows an act whole, or nothing. */
  lens(act?: ActRow): void;
}

/** The size of a program on the map, and the room between two of them. */
const columnWidth = 30,
  rowHeight = 3,
  indent = 3,
  sidebarWidth = 38;
/** The glyph of the lines that meet in a cell, by the sides they reach: up 1, right 2, down 4, left 8. */
const joins = ["", "│", "─", "└", "│", "│", "┌", "├", "─", "┘", "─", "┴", "┐", "┤", "┬", "┼"];
const stateMarks: Record<RowState, [string, () => RGBA]> = {
  asks: ["◆", () => c.warning],
  failed: ["✗", () => c.danger],
  working: ["◉", () => c.accent],
  paused: ["◌", () => c.warning],
  done: ["✓", () => c.success],
  idle: ["○", () => c.faint],
};
const actMarks: Record<ActState, [string, () => RGBA]> = {
  running: ["", () => c.accent],
  stands: ["◇", () => c.muted],
  done: ["✓", () => c.success],
  failed: ["✗", () => c.danger],
  cancelled: ["⊘", () => c.faint],
  held: ["◌", () => c.warning],
  asks: ["◆", () => c.warning],
};
const markOf = (state: ActState): Part => {
  const [glyph, color] = actMarks[state];
  return [glyph || spin(), color()];
};

/** A grid of cells that the map draws on, and gives as rows of parts. */
class Canvas {
  readonly chars: string[];
  readonly fg: (RGBA | undefined)[];
  readonly bg: (RGBA | undefined)[];
  readonly weight: number[];
  readonly sides: number[];
  constructor(
    readonly width: number,
    readonly height: number,
  ) {
    const size = width * height;
    this.chars = Array.from({ length: size }, () => " ");
    this.fg = Array.from({ length: size }, () => undefined);
    this.bg = Array.from({ length: size }, () => undefined);
    this.weight = Array.from({ length: size }, () => 0);
    this.sides = Array.from({ length: size }, () => 0);
  }
  private at(x: number, y: number): number {
    return x >= 0 && y >= 0 && x < this.width && y < this.height ? y * this.width + x : -1;
  }
  put(x: number, y: number, text: string, fg?: RGBA, weight = 0, bg?: RGBA): void {
    let column = x;
    for (const glyph of text) {
      const index = this.at(column++, y);
      if (index < 0) continue;
      this.chars[index] = glyph;
      this.fg[index] = fg;
      this.weight[index] = weight;
      this.sides[index] = 0;
      if (bg) this.bg[index] = bg;
    }
  }
  ground(x: number, y: number, width: number, bg: RGBA): void {
    for (let column = x; column < x + width; column++) {
      const index = this.at(column, y);
      if (index >= 0) this.bg[index] = bg;
    }
  }
  /** A line that reaches the sides of a cell, which joins the lines that reach it already. */
  join(x: number, y: number, sides: number, fg: RGBA): void {
    const index = this.at(x, y);
    if (index < 0 || (this.chars[index] !== " " && !this.sides[index])) return;
    this.sides[index] = (this.sides[index] ?? 0) | sides;
    this.chars[index] = joins[this.sides[index] ?? 0] ?? " ";
    this.fg[index] = fg;
  }
  /** The color of a cell that a line holds, and a glyph in place of the line where one is given. */
  tint(x: number, y: number, fg: RGBA, glyph?: string): void {
    const index = this.at(x, y);
    if (index < 0 || !this.sides[index]) return;
    this.fg[index] = fg;
    if (glyph) this.chars[index] = glyph;
  }
  rows(): Part[][] {
    const out: Part[][] = [];
    for (let y = 0; y < this.height; y++) {
      const row: Part[] = [];
      let text = "",
        fg: RGBA | undefined,
        bg: RGBA | undefined,
        weight = 0;
      for (let x = 0; x < this.width; x++) {
        const index = y * this.width + x;
        const next = [this.fg[index], this.bg[index], this.weight[index] ?? 0] as const;
        if (text && (next[0] !== fg || next[1] !== bg || next[2] !== weight)) {
          row.push([text, fg, weight, bg]);
          text = "";
        }
        [fg, bg, weight] = next;
        text += this.chars[index] ?? " ";
      }
      if (text) row.push([text, fg, weight, bg]);
      out.push(row);
    }
    return out;
  }
}

/** The cells of a line from one place to another: across, then down or up, then across, as the map draws an edge. */
function path(from: [number, number], trunk: number, to: [number, number]): [number, number][] {
  const out: [number, number][] = [];
  const [x0, y0] = from,
    [x1, y1] = to;
  for (let x = x0; x !== trunk; x += Math.sign(trunk - x0)) out.push([x, y0]);
  for (let y = y0; y !== y1; y += Math.sign(y1 - y0)) out.push([trunk, y]);
  for (let x = trunk; x !== x1 + Math.sign(x1 - trunk); x += Math.sign(x1 - trunk) || 1) {
    out.push([x, y1]);
    if (x === x1) break;
  }
  return out;
}

/** The sides by which a cell of a path reaches the cell before it and the cell after it. */
function sidesOf(cells: readonly [number, number][], index: number): number {
  const [x, y] = cells[index] ?? [0, 0];
  let sides = 0;
  for (const other of [cells[index - 1], cells[index + 1]]) {
    if (!other) continue;
    const [ox, oy] = other;
    sides |= oy < y ? 1 : ox > x ? 2 : oy > y ? 4 : ox < x ? 8 : 0;
  }
  return sides || (cells.length > 1 ? 0 : 10);
}

/** The view of the debugger of a living program: the tree of threads, the map of the programs with the messages
 * between them, the source of one program with what each line made, one word, and one act, each a level of zoom over
 * the same life. */
export class Graph {
  level: Level = "threads";
  /** The threads whose sub-threads the tree folds into them. */
  readonly folds = new Set<string>();
  /** The thread that the tree selects, the program that the map selects, the line that the source and the word
   * select, and the act of that line that the word selects. */
  thread = 0;
  node = "";
  line = 0;
  chip = 0;
  /** When the graph saw each prompt end, so that its answer flows back for a moment. */
  private readonly seen = new Map<string, number>();
  private top = 0;
  private camera: [number, number] = [0, 0];
  private tree: Thread[] = [];
  private shown: Thread[] = [];
  private placesOf = new Map<string, Place>();
  /** The lines of the source that the levels of one program show, as the last render read them. */
  lines: SourceLine[] = [];
  /** What each row of the stage selects, for a click. */
  private targets: (() => void)[] = [];

  constructor(private readonly stage: Stage) {}

  /** The act or the chain that the verbs act on at this level. */
  get selection(): string {
    if (this.level === "threads") return this.shown[this.thread]?.act.id ?? this.stage.selected;
    if (this.level === "map") return this.node || this.stage.selected;
    const line = this.lines[this.line];
    if (!line) return this.stage.selected;
    if (line.type !== "code") return line.act.id;
    if (this.level === "word" || this.level === "act") return line.acts[this.chip]?.id ?? line.rung.id;
    return line.rung.id;
  }
  /** The rows of the stage at a size. On a wide terminal the threads stand in a sidebar beside the map, and the
   * levels of one program take the whole width. */
  rows(width: number, height: number): Part[][] {
    const { acts, owed } = this.stage;
    const now = clock();
    for (const act of acts)
      if (act.kind === "prompt" && act.done && !this.seen.has(act.id)) this.seen.set(act.id, now);
    this.tree = threads(acts, owed);
    this.shown = outline(this.tree, this.folds);
    this.thread = Math.max(0, Math.min(this.thread, this.shown.length - 1));
    this.targets = [];
    const side = width >= 110 && (this.level === "threads" || this.level === "map") ? sidebarWidth : 0;
    if (!side) {
      if (this.level === "threads") return this.threadRows(width, height, false);
      if (this.level === "map") return this.mapRows(width, height);
      return this.sourceRows(width, height);
    }
    const left = this.threadRows(side, height, true);
    const targets = this.targets;
    this.targets = [];
    const right = this.mapRows(width - side - 3, height);
    const rights = this.targets;
    this.targets = Array.from({ length: height }, (_, row) => targets[row] ?? rights[row] ?? (() => {}));
    return Array.from({ length: height }, (_, row) => [
      ...(left[row] ?? pad([], side, c.panel)),
      ["   "],
      ...(right[row] ?? []),
    ]);
  }

  /** A click on a row of the stage selects what it shows, and a second click zooms in. */
  click(row: number): boolean {
    const target = this.targets[row];
    if (!target) return false;
    target();
    return true;
  }

  // The threads.

  private threadRows(width: number, height: number, side: boolean): Part[][] {
    const shown = this.shown;
    const focused = this.level === "threads";
    const ground = side ? c.panel : undefined;
    const rows: Part[][] = [pad([[" Threads", c.muted, bold]], width, ground), pad([], width, ground)];
    this.targets.push(
      () => {},
      () => {},
    );
    const room = Math.max(1, height - 2);
    if (this.thread < this.top) this.top = this.thread;
    if (this.thread >= this.top + room) this.top = this.thread - room + 1;
    this.top = Math.max(0, Math.min(this.top, shown.length - room));
    for (const [index, thread] of shown.slice(this.top, this.top + room).entries()) {
      const at = this.top + index;
      rows.push(this.threadRow(thread, at === this.thread, focused, width, side));
      this.targets.push(() => {
        if (this.thread === at && this.level === "threads") void this.zoom(1);
        else {
          this.thread = at;
          this.level = "threads";
        }
      });
    }
    if (!shown.length)
      rows.push(
        pad(
          [[side ? " No thread yet." : "  No thread yet: the input asks the model of the root.", c.faint]],
          width,
          ground,
        ),
      );
    while (side && rows.length < height) rows.push(pad([], width, ground));
    return rows;
  }

  private threadRow(
    thread: Thread,
    selected: boolean,
    focused: boolean,
    width: number,
    side: boolean,
  ): Part[] {
    const ground = selected ? (focused ? c.selected : c.raised) : side ? c.panel : undefined;
    const indent = "  ".repeat(Math.min(thread.depth, side ? 3 : 8));
    const fold = thread.children.length && this.folds.has(thread.act.id) ? "▸" : " ";
    const asks = thread.state === "asks";
    const title = thread.title || `${thread.shape} for ${thread.to || "the model"}`;
    const counts = thread.total ? `${thread.closed}/${thread.total}` : "";
    const owed = thread.owed.length && !asks ? `◆${thread.owed.length}` : "";
    const tail: Part[] = [
      [owed ? ` ${owed}` : "", c.warning],
      [counts ? ` ${counts}` : "", c.faint],
    ];
    const tailWidth = Bun.stringWidth(plain(tail));
    if (side) {
      const room = width - indent.length - 4 - tailWidth;
      return pad(
        [
          [" "],
          [indent],
          markOf(thread.state),
          [fold, c.faint],
          [clip(title, room), asks ? c.warning : thread.act.done ? c.muted : c.text, selected ? bold : 0],
          [" ".repeat(Math.max(0, room - Bun.stringWidth(clip(title, room))))],
          ...tail,
        ],
        width,
        ground,
      );
    }
    const anchor = thread.anchor
      ? [thread.anchor.act, thread.anchor.file, thread.anchor.pull ? `#${thread.anchor.pull.number}` : ""]
          .filter(Boolean)
          .join(" ")
      : "";
    const who = thread.to === "operator" ? "" : this.stage.label(thread.chain);
    const pulls = thread.pulls.map((pull) => `#${pull.number}`).join(" ");
    const right: Part[] = [
      [clip(who, 16).padEnd(18), c.faint],
      [(thread.total ? progress(thread.closed, thread.total, 6) : "").padEnd(8), c.accent],
      [clip(pulls, 14).padEnd(16), c.success],
      ...tail,
    ];
    const rightWidth = Bun.stringWidth(plain(right));
    const room = width - indent.length - 6 - rightWidth;
    const titled = clip(title, Math.max(8, room - (anchor ? Bun.stringWidth(anchor) + 4 : 0)));
    const left: Part[] = [
      [selected ? " ▎" : "  ", c.accent],
      [indent],
      markOf(thread.state),
      [fold, c.faint],
      [titled, asks ? c.warning : thread.act.done ? c.muted : c.text, selected ? bold : 0],
      [anchor ? `  ↳ ${anchor}` : "", c.secondary],
    ];
    const used = Bun.stringWidth(plain(left));
    return pad([...left, [" ".repeat(Math.max(1, width - rightWidth - used))], ...right], width, ground);
  }

  // The map.

  /** The chain over each chain in the tree of hires, and the operator over each chain at the top, which each render
   * reads once. */
  private parents = new Map<string, string>();
  /** The cells of the edge to each chain, and of the way of each message between two programs, which no later chain
   * moves. */
  private readonly edgeCache = new Map<string, [number, number][]>();
  private readonly routeCache = new Map<string, [number, number][]>();
  private parentOf(id: string): string | undefined {
    return id === you ? undefined : (this.parents.get(id) ?? you);
  }

  /** The chains that the selected thread holds: the chain of each thread of its tree. */
  private held(): Set<string> {
    const thread = this.shown[this.thread];
    const out = new Set<string>();
    const visit = (one: Thread) => {
      out.add(one.chain);
      for (const child of one.children) visit(child);
    };
    if (thread) visit(thread);
    return out;
  }

  private mapRows(width: number, height: number): Part[][] {
    const { acts, loom } = this.stage;
    const chains = acts.filter((act) => act.kind === "chain");
    const rows = new Map(acts.map((act) => [act.id, act]));
    const parent = (chain: ActRow) => {
      const maker = rows.get(chain.by);
      return maker && maker.on !== chain.id ? maker.on : undefined;
    };
    this.parents = new Map();
    for (const chain of chains) {
      const up = parent(chain);
      if (up) this.parents.set(chain.id, up);
    }
    this.placesOf = places(chains, parent);
    if (!this.node || !this.placesOf.has(this.node)) this.node = this.stage.selected;
    const held = this.held();
    const thread = this.shown[this.thread];
    const focused = this.level === "map";
    const title: Part[] = [
      [thread ? clip(thread.title, width - 20) : "The programs of the life", c.text, bold],
      [`   ${held.size} of ${chains.length}`, c.faint],
    ];
    const canvas = new Canvas(width, Math.max(4, height - 2));
    const corner = (place: Place): [number, number] =>
      place.column === 0 && place.row === 0 && this.placesOf.get(you) === place
        ? [0, 0]
        : [indent + place.column * columnWidth, place.row * rowHeight];
    // The camera follows the selected program.
    const chosen = this.placesOf.get(this.node);
    if (chosen) {
      const [x, y] = corner(chosen);
      let [cx, cy] = this.camera;
      if (x < cx) cx = Math.max(0, x - indent);
      if (x + columnWidth > cx + width) cx = x + columnWidth - width;
      if (y < cy) cy = Math.max(0, y - 1);
      if (y + rowHeight > cy + canvas.height) cy = y + rowHeight - canvas.height;
      // A map that fits the stage stands at its corner.
      const places = [...this.placesOf.values()];
      const wide = indent + (Math.max(0, ...places.map((one) => one.column)) + 1) * columnWidth;
      const tall = (Math.max(0, ...places.map((one) => one.row)) + 1) * rowHeight;
      this.camera = [wide <= width ? 0 : Math.max(0, cx), tall <= canvas.height ? 0 : Math.max(0, cy)];
    }
    const [cx, cy] = this.camera;
    const view = (point: [number, number]): [number, number] => [point[0] - cx, point[1] - cy];
    const byId = new Map(loom.rows.map((row) => [row.id, row]));
    const labelOf = (id: string) => clip(id === you ? "you" : this.stage.label(id), columnWidth - 8);
    // The edges of the tree of hires: from the end of the name of a program to the program that it hired, and from
    // the operator down the left side to each program at the top.
    const edges = new Map<string, [number, number][]>();
    const faintBorder = mix(c.background, c.border, 0.55);
    for (const chain of chains) {
      const place = this.placesOf.get(chain.id);
      if (!place) continue;
      const [x, y] = corner(place);
      const up = parent(chain);
      // A place never moves, so the edge to a chain is found once, the first time that the map holds the chain.
      let cells = this.edgeCache.get(chain.id);
      if (!cells) {
        const above = up ? this.placesOf.get(up) : undefined;
        if (up && !above) continue;
        if (above && up) {
          const [px, py] = corner(above);
          cells = path([px + 3 + Bun.stringWidth(labelOf(up)), py], px + columnWidth - 3, [x - 2, y]);
        } else {
          cells = [];
          for (let row = 1; row <= y; row++) cells.push([0, row]);
          cells.push([1, y]);
        }
        this.edgeCache.set(chain.id, cells);
      }
      edges.set(chain.id, cells);
      const tone = held.has(chain.id) ? c.border : faintBorder;
      // Only the cells in sight are drawn, so that a life of many chains costs no more than the stage it fills.
      for (let index = 0; index < cells.length; index++) {
        const [px, py] = cells[index] ?? [0, 0];
        if (px < cx || py < cy || px >= cx + width || py >= cy + canvas.height) continue;
        canvas.join(px - cx, py - cy, sidesOf(cells, index), tone);
      }
    }
    // The messages flow along the edges, from the chain of the rung that made a prompt to the chain that serves it,
    // and an answer flows back for a moment once it closes.
    const recent = (act: ActRow) => !this.stage.frozen && clock() - (this.seen.get(act.id) ?? 0) < 4000;
    const tick = Math.floor(clock() / 120);
    const trails = new Map<RGBA, RGBA>();
    for (const message of messages(acts, recent)) {
      const key = `${message.from} ${message.to}`;
      const cells = this.routeCache.get(key) ?? [];
      if (!this.routeCache.has(key)) {
        const way = route(message.from, message.to, (id) => this.parentOf(id));
        for (let index = 0; index + 1 < way.length; index++) {
          const a = way[index] ?? "",
            b = way[index + 1] ?? "";
          const down = edges.get(b);
          const up = edges.get(a);
          if (down && this.parentOf(b) === a) cells.push(...down);
          else if (up && this.parentOf(a) === b) cells.push(...[...up].reverse());
        }
        this.routeCache.set(key, cells);
      }
      if (!cells.length) continue;
      const color = message.kind === "ask" ? c.accent : message.kind === "answer" ? c.success : c.warning;
      const trail = trails.get(color) ?? mix(c.background, color, 0.5);
      trails.set(color, trail);
      for (const [px, py] of cells)
        if (px >= cx && py >= cy && px < cx + width && py < cy + canvas.height)
          canvas.tint(px - cx, py - cy, trail);
      const dot = cells[tick % cells.length];
      if (dot && !this.stage.frozen) canvas.tint(...view(dot), color, "•");
    }
    // Each program: its mark and its name, and under them, faint, what it waits on, its actor and its dollars.
    for (const [id, place] of this.placesOf) {
      const [x, y] = view(corner(place));
      if (x + columnWidth < 0 || y + 2 < 0 || x >= width || y >= canvas.height) continue;
      const row = byId.get(id);
      const lit = id === you || held.has(id);
      const tone = (one: RGBA) => (lit ? one : mix(c.background, one, 0.4));
      const state: RowState = id === you ? (loom.owed.length ? "asks" : "idle") : (row?.state ?? "idle");
      const [mark, color] = stateMarks[state];
      const selected = id === this.node;
      const label = labelOf(id);
      canvas.put(x, y, state === "working" && !this.stage.frozen ? spin() : mark, tone(color()));
      canvas.put(
        x + 2,
        y,
        label,
        tone(selected ? c.text : state === "idle" || state === "done" ? c.prose : c.text),
        selected ? bold : 0,
        selected ? (focused ? c.selected : c.raised) : undefined,
      );
      const measure = row?.measure;
      const meta =
        id === you
          ? loom.owed.length
            ? `${loom.owed.length} owed`
            : ""
          : [
              state === "paused" ? "paused" : state === "asks" ? "" : (row?.waits ?? ""),
              shortActor(this.stage.actor(id)).replace(/\/.*/, ""),
              measure
                ? measure.ceiling
                  ? `${dollars(measure.spent)} of ${dollars(measure.ceiling)}`
                  : dollars(measure.dollars)
                : "",
              // A program that asks says its question, after its actor and its cost, which stay in view.
              state === "asks" ? `◆ ${questionOf(id, loom.owed) || "asks you"}` : "",
            ]
              .filter(Boolean)
              .join("  ");
      canvas.put(
        x + 2,
        y + 1,
        clip(meta, columnWidth - 4),
        tone(state === "asks" || state === "paused" ? c.warning : c.faint),
      );
    }
    this.targets.push(
      () => {},
      () => {},
    );
    return [pad(title, width), [], ...canvas.rows()];
  }

  // The source, a word, and an act.

  private sourceRows(width: number, height: number): Part[][] {
    const { acts, owed } = this.stage;
    const chain = this.node || this.stage.selected;
    const words = this.stage.words();
    this.lines = source(chain, acts, (rung) => words.get(rung.id) ?? "", owed);
    this.line = Math.max(0, Math.min(this.line, this.lines.length - 1));
    const side = width >= 120 ? 38 : 0;
    const main = width - side - (side ? 2 : 0);
    const selected = this.lines[this.line];
    const rung = selected?.type === "code" ? selected.rung : undefined;
    const shown =
      this.level === "program"
        ? this.lines.map((line, index) => [line, index] as const)
        : this.lines
            .map((line, index) => [line, index] as const)
            .filter(([line]) => line.type === "code" && line.rung === rung);
    const title: Part[] =
      this.level === "program"
        ? [
            [this.stage.label(chain), c.text, bold],
            [`  ${chain}  the source of its ladders`, c.faint],
          ]
        : [
            [rung?.id ?? "", c.text, bold],
            [`  a word on ${this.stage.label(chain)}, with what each line made`, c.faint],
          ];
    const room = Math.max(1, height - 2);
    const body: Part[][] = [];
    const owners: number[] = [];
    for (const [line, index] of shown) {
      const parts = this.sourceLine(line, index === this.line, main);
      for (const row of parts) {
        body.push(row);
        owners.push(index);
      }
      if (this.level === "word" && line.type === "code")
        for (const [n, act] of line.acts.entries()) {
          const chosen = index === this.line && n === this.chip;
          const text = full(act).split("\n").slice(0, 6);
          for (const [k, piece] of text.entries()) {
            body.push(
              pad(
                [
                  [k ? "        " : `     ${chosen ? "▸" : " "}  `, c.accent],
                  ...(k ? [] : ([[`${act.id} `, chosen ? c.text : c.muted, chosen ? bold : 0]] as Part[])),
                  [clip(piece, main - 20), c.prose],
                ],
                main,
              ),
            );
            owners.push(index);
          }
        }
    }
    const at = owners.indexOf(this.line);
    if (at < this.top) this.top = at;
    const last = owners.lastIndexOf(this.line);
    if (last >= this.top + room) this.top = last - room + 1;
    this.top = Math.max(0, Math.min(this.top, body.length - room));
    const window = body.slice(this.top, this.top + room);
    const panel = side ? this.sidePanel(side, room) : [];
    const out: Part[][] = [pad(title, width), []];
    this.targets.push(
      () => {},
      () => {},
    );
    for (let row = 0; row < room; row++) {
      const left = window[row] ?? pad([], main);
      out.push(side ? [...left, ["  "], ...(panel[row] ?? [])] : left);
      const owner = owners[this.top + row];
      this.targets.push(() => {
        if (owner === undefined) return;
        if (owner === this.line) void this.zoom(1);
        else this.line = owner;
      });
    }
    return out;
  }

  private sourceLine(line: SourceLine, selected: boolean, width: number): Part[][] {
    const ground = selected ? c.selected : undefined;
    const bar: Part = [selected ? "▎" : " ", c.accent];
    if (line.type === "prompt") {
      const [shape, message, to] = line.act.words.map(String);
      const who = to === "operator" ? "  asks you" : to ? `  to ${to}` : "";
      return [
        pad(
          [
            bar,
            markOf(line.state),
            [" "],
            [line.act.id, c.muted],
            ["  "],
            [
              clip((message ?? "").split("\n")[0] ?? "", width - 30),
              line.state === "asks" ? c.warning : c.text,
              bold,
            ],
            [`  ${shape}${who}`, c.faint],
          ],
          width,
          ground,
        ),
      ];
    }
    if (line.type === "value")
      return [
        pad(
          [bar, ["   → ", c.success], [clip(line.text.split("\n")[0] ?? "", width - 8), c.prose]],
          width,
          ground,
        ),
      ];
    const margin = line.acts.length
      ? line.acts.slice(0, 2).flatMap((act): Part[] => {
          const state = actState(act, this.stage.owed);
          return [
            ["  "],
            [act.id, c.muted],
            [" "],
            markOf(state),
            [" "],
            [
              clip(
                outcome(act, (id) => this.stage.label(id)),
                18,
              ),
              c.faint,
            ],
          ];
        })
      : [];
    const extra = line.acts.length > 2 ? `  +${line.acts.length - 2}` : "";
    const marginWidth = Math.min(46, Bun.stringWidth(plain(margin)) + extra.length);
    const runs = line.runs && !this.stage.frozen;
    const number = String(line.number).padStart(4);
    const text = line.text || (line.state === "running" ? "the model writes its word" : "");
    const code = highlight(clip(text, width - marginWidth - 8), !line.text);
    return [
      pad(
        [
          bar,
          [number, runs ? c.accent : c.faint],
          [runs ? "▶ " : "  ", c.accent],
          ...code,
          [" ".repeat(Math.max(1, width - 7 - Bun.stringWidth(plain(code)) - marginWidth))],
          ...clipParts([...margin, [extra, c.faint]], marginWidth),
        ],
        width,
        ground,
      ),
    ];
  }

  /** The stack and the watch: the chain of awaits from the operator to what the chain waits on, and the acts of the
   * selected line with their values. */
  private sidePanel(width: number, height: number): Part[][] {
    const { acts } = this.stage;
    const rows = new Map(acts.map((act) => [act.id, act]));
    const out: Part[][] = [[["stack", c.text, bold]]];
    const selected = this.lines[this.line];
    const start = selected?.type === "code" ? selected.rung : selected?.act;
    const stack: ActRow[] = [];
    for (let at = start; at && stack.length < 12; at = rows.get(at.by))
      if (at.kind === "prompt" || at.kind === "rung") stack.unshift(at);
    for (const act of stack)
      out.push([
        [act.kind === "prompt" ? "  " : "    ", c.faint],
        [act.id.padEnd(10), c.muted],
        [
          clip(
            act.kind === "prompt"
              ? this.stage.label(act.on)
              : ((this.stage.words().get(act.id) ?? "").split("\n")[0] ?? ""),
            width - 14,
          ),
          c.prose,
        ],
      ]);
    const row = this.stage.loom.rows.find((one) => one.id === (this.node || this.stage.selected));
    if (row?.waits)
      out.push([
        ["  ▶ waits on ", c.accent],
        [row.waits, c.text],
      ]);
    out.push([], [["watch", c.text, bold]]);
    const watched = selected?.type === "code" ? selected.acts : selected ? [selected.act] : [];
    for (const act of watched.slice(0, Math.max(0, height - out.length - 1)))
      out.push([
        ["  "],
        [act.id.padEnd(10), c.muted],
        [
          clip(
            outcome(act, (id) => this.stage.label(id)) || valueText(act.value).split("\n")[0] || "",
            width - 14,
          ),
          c.prose,
        ],
      ]);
    if (!watched.length) out.push([["  the line made no act", c.faint]]);
    return out.map((parts) => pad(clipParts(parts, width), width, c.panel));
  }

  // The keys.

  /** A key at this level: true when the graph took it. */
  async key(name: string): Promise<boolean> {
    const moves: Record<string, [number, number]> = {
      up: [0, -1],
      k: [0, -1],
      down: [0, 1],
      j: [0, 1],
      left: [-1, 0],
      h: [-1, 0],
      right: [1, 0],
      l: [1, 0],
    };
    if (name === "return" || name === "enter") return this.zoom(1).then(() => true);
    if (name === "escape") return this.zoom(-1);
    if (name === "pageup" || name === "pagedown")
      return this.move([0, name === "pageup" ? -10 : 10]).then(() => true);
    const move = moves[name];
    if (move) return this.move(move).then(() => true);
    return false;
  }

  /** The selection moves in a direction: a thread of the tree, a program of the map by its place, or a line. */
  async move([dx, dy]: [number, number]): Promise<void> {
    if (this.level === "threads") {
      if (dx) {
        const thread = this.shown[this.thread];
        if (thread?.children.length) {
          if (dx < 0) this.folds.add(thread.act.id);
          else this.folds.delete(thread.act.id);
        }
        return;
      }
      this.thread = Math.max(0, Math.min(this.shown.length - 1, this.thread + dy));
      // The session reads the chain of the selection while the key returns at once.
      const thread = this.shown[this.thread];
      if (thread && thread.chain !== this.stage.selected) void this.stage.select(thread.chain);
      return;
    }
    if (this.level === "map") {
      const from = this.placesOf.get(this.node);
      if (!from) return;
      let best: [string, number] | undefined;
      for (const [id, place] of this.placesOf) {
        if (id === this.node) continue;
        const ax = place.column - from.column,
          ay = place.row - from.row;
        const along = ax * dx + ay * dy;
        if (along <= 0) continue;
        const score = along + 3 * Math.abs(dx ? ay : ax);
        if (!best || score < best[1]) best = [id, score];
      }
      if (best) {
        this.node = best[0];
        if (this.node !== you && this.node !== this.stage.selected) void this.stage.select(this.node);
      }
      return;
    }
    if (this.level === "word" && dx) {
      const line = this.lines[this.line];
      const count = line?.type === "code" ? line.acts.length : 0;
      this.chip = Math.max(0, Math.min(count - 1, this.chip + dx));
      return;
    }
    if (dx) return;
    const rung = this.lines[this.line];
    let next = this.line + dy;
    // A word holds the lines of its rung alone.
    if (this.level === "word")
      while (next >= 0 && next < this.lines.length) {
        const line = this.lines[next];
        if (line?.type === "code" && rung?.type === "code" && line.rung === rung.rung) break;
        next += Math.sign(dy);
      }
    if (next >= 0 && next < this.lines.length) {
      this.line = next;
      this.chip = 0;
    }
  }

  /** One level in, or one level out, which is false at the top. */
  async zoom(by: number): Promise<boolean> {
    const at = levels.indexOf(this.level);
    if (by < 0) {
      if (at === 0) return false;
      if (this.level === "act") this.stage.lens(undefined);
      this.level = levels[at - 1] ?? "threads";
      return true;
    }
    if (this.level === "threads") {
      const thread = this.shown[this.thread];
      if (!thread) return true;
      this.node = thread.chain;
      await this.stage.select(thread.chain);
      this.level = "map";
      return true;
    }
    if (this.level === "map") {
      if (this.node === you) return true;
      if (this.node !== this.stage.selected) await this.stage.select(this.node);
      this.level = "program";
      this.line = Number.MAX_SAFE_INTEGER;
      return true;
    }
    const line = this.lines[this.line];
    if (this.level === "program") {
      if (line?.type === "code") this.level = "word";
      else if (line) {
        this.level = "act";
        this.stage.lens(line.act);
      }
      return true;
    }
    if (this.level === "word" && line?.type === "code") {
      const act = line.acts[this.chip] ?? line.rung;
      this.level = "act";
      this.stage.lens(act);
    }
    return true;
  }
}

/** What an act came to, whole, as the word shows it under its line. */
function full(act: ActRow): string {
  if (act.kind === "bash") {
    const exit = act.value as
      | { stdout?: { content?: string }; stderr?: { content?: string }; code?: number }
      | undefined;
    const out = `${exit?.stdout?.content ?? ""}${exit?.stderr?.content ?? ""}`.trimEnd();
    return [
      `${String(act.words[0] ?? "").split("\n")[0]}${act.done && exit?.code !== undefined ? `  exited ${exit.code}` : ""}`,
      ...out.split("\n").slice(-5),
    ].join("\n");
  }
  if (act.kind === "prompt") {
    const [shape, message] = act.words.map(String);
    return `${(message ?? "").split("\n")[0]}  ${shape}${act.done ? `\n→ ${valueText(act.value)}` : ""}`;
  }
  const value = act.value as { is?: string; content?: string } | null;
  if (value && typeof value === "object" && typeof value.content === "string")
    return `${outcome(act, (id) => id)}\n${value.content.split("\n").slice(0, 4).join("\n")}`;
  return outcome(act, (id) => id) || valueText(act.value);
}

/** A bar of progress of a count of a whole, in cells. */
export function progress(done: number, whole: number, cells: number): string {
  const full = whole ? Math.round((done / whole) * cells) : 0;
  return `${"▰".repeat(full)}${"▱".repeat(Math.max(0, cells - full))}`;
}

const keywords = new Set(
  "def async await for in if elif else while return import from with as try except raise not and or is None True False lambda class pass break continue".split(
    " ",
  ),
);
const verbs = new Set([
  "bash",
  "read",
  "write",
  "prompt",
  "chain",
  "close",
  "wait",
  "rung",
  "grant",
  "cd",
  "pause",
  "wake",
  "cancel",
  "debug",
]);
/** A line of python in the colors of its parts: a comment, a string, a keyword, and a verb of the engine that the line
 * calls. */
export function highlight(text: string, faint = false): Part[] {
  if (faint) return [[text, c.faint]];
  const out: Part[] = [];
  const pattern = /(#.*$)|("(?:[^"\\]|\\.)*"?|'(?:[^'\\]|\\.)*'?)|(\b\w+\b)|(\s+|[^\w\s"'#]+)/g;
  for (const match of text.matchAll(pattern)) {
    const [whole, comment, string, word] = match;
    const after = text.slice((match.index ?? 0) + whole.length).trimStart();
    if (comment) out.push([whole, c.secondary]);
    else if (string) out.push([whole, c.success]);
    else if (word && verbs.has(word) && after.startsWith("(")) out.push([whole, c.accent, bold]);
    else if (word && keywords.has(word)) out.push([whole, c.warning]);
    else out.push([whole, c.text]);
  }
  return out;
}

/** The parts cut to a width of the terminal. */
function clipParts(parts: readonly Part[], width: number): Part[] {
  const out: Part[] = [];
  let used = 0;
  for (const [text, ...rest] of parts) {
    const room = width - used;
    if (room <= 0) break;
    const cut = Bun.stringWidth(text) > room ? clip(text, room) : text;
    out.push([cut, ...rest] as Part);
    used += Bun.stringWidth(cut);
  }
  return out;
}

/** The parts of a row, filled with spaces to a width, all on a ground when it has one. */
function pad(parts: readonly Part[], width: number, ground?: RGBA): Part[] {
  const used = Bun.stringWidth(plain(parts));
  const all: Part[] = [...parts, [" ".repeat(Math.max(0, width - used))]];
  return ground ? all.map(([text, fg, attributes, bg]) => [text, fg, attributes, bg ?? ground]) : all;
}

import type { RGBA } from "@opentui/core";
import { isDone, type Loom, messages, type Place, places, route, type State, you } from "furb/life";
import { clock } from "./clock.ts";
import { clip, dollars } from "./format.ts";
import { questionOf, shortActor } from "./overview.ts";
import type { ActRow } from "./session.ts";
import { theme as c, rowMark } from "./theme.ts";
import { bold, mix, type Part, plain } from "./ui.ts";

/** What the graph reads of the App that holds it: the life as it stands at the time cursor, and the controls that it
 * asks the App for. */
export interface Stage {
  readonly acts: readonly ActRow[];
  readonly loom: Loom;
  /** Whether the time cursor stands in the past, where nothing moves. */
  readonly frozen: boolean;
  /** The chain that the session selects, on which the selection of the map starts. */
  readonly selected: string;
  /** The thread that the sidebar shows, as it stood at the time cursor: its title, and the chain of it and of each of
   * its agents, which the map lights. */
  readonly thread: { title: string; chains: ReadonlySet<string> } | undefined;
  label(chain: string): string;
  /** The actor of a chain, as its answers, its rungs or its prompts name it. */
  actor(chain: string): string;
  /** The session selects a chain. */
  select(chain: string): Promise<void>;
}

/** The size of a program on the map, and the room between two of them. */
const columnWidth = 30,
  rowHeight = 3,
  indent = 3;
/** The glyph of the lines that meet in a cell, by the sides they reach: up 1, right 2, down 4, left 8. */
const joins = ["", "│", "─", "└", "│", "│", "┌", "├", "─", "┘", "─", "┴", "┐", "┤", "┬", "┼"];
/** The direction in which each key moves the selection of the map. */
const moves: Record<string, [number, number]> = {
  up: [0, -1],
  down: [0, 1],
  left: [-1, 0],
  right: [1, 0],
  pageup: [0, -10],
  pagedown: [0, 10],
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

/** The map of the programs of a life: each program at the place of its birth, with the lines of the tree of hires, a
 * dot where a message flows between two programs, and the chains of the thread that the sidebar shows lit. */
export class Graph {
  /** The program that the map selects. */
  node = "";
  /** When the graph saw each prompt end, so that its answer flows back for a moment. */
  private readonly seen = new Map<string, number>();
  private camera: [number, number] = [0, 0];
  private placesOf = new Map<string, Place>();

  constructor(private readonly stage: Stage) {}

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

  /** The rows of the map at a size: its title, and under it the programs, the selected one in view. */
  rows(width: number, height: number): Part[][] {
    const { acts, loom } = this.stage;
    const now = clock();
    for (const act of acts)
      if (act.kind === "prompt" && isDone(act) && !this.seen.has(act.id)) this.seen.set(act.id, now);
    const chains = acts.filter((act) => act.kind === "chain");
    const rows = new Map(acts.map((act) => [act.id, act]));
    const parent = (chain: ActRow) => {
      const maker = rows.get(chain.by);
      return maker && maker.chain !== chain.id ? maker.chain : undefined;
    };
    this.parents = new Map();
    for (const chain of chains) {
      const up = parent(chain);
      if (up) this.parents.set(chain.id, up);
    }
    this.placesOf = places(chains, parent);
    if (!this.node || !this.placesOf.has(this.node)) this.node = this.stage.selected;
    const thread = this.stage.thread;
    const held = thread?.chains ?? new Set<string>();
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
      const state: State = id === you ? (loom.owed.length ? "asks" : "idle") : (row?.state ?? "idle");
      const [mark, color] = rowMark(state);
      const selected = id === this.node;
      const label = labelOf(id);
      canvas.put(x, y, mark, tone(color ?? c.faint));
      canvas.put(
        x + 2,
        y,
        label,
        tone(selected ? c.text : state === "idle" || state === "done" ? c.prose : c.text),
        selected ? bold : 0,
        selected ? c.selected : undefined,
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
    return [pad(title, width), [], ...canvas.rows()];
  }

  /** A key moves the selection to the program nearest in its direction, by the places of the programs, and the session
   * selects the chain of that program while the key returns at once. */
  move(key: string): void {
    const [dx, dy] = moves[key] ?? [0, 0];
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
    if (!best) return;
    this.node = best[0];
    if (this.node !== you && this.node !== this.stage.selected) void this.stage.select(this.node);
  }
}

/** The parts of a row, filled with spaces to a width, all on a ground when it has one. */
function pad(parts: readonly Part[], width: number, ground?: RGBA): Part[] {
  const used = Bun.stringWidth(plain(parts));
  const all: Part[] = [...parts, [" ".repeat(Math.max(0, width - used))]];
  return ground ? all.map(([text, fg, attributes, bg]) => [text, fg, attributes, bg ?? ground]) : all;
}

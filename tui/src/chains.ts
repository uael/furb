import type { RGBA } from "@opentui/core";
import type { Loom, Row, Tone } from "furb/life";
import { clip } from "./format.ts";
import { measureParts, pad, questionOf, rowMarks, shortActor } from "./overview.ts";
import { theme as c, spin } from "./theme.ts";
import { bold, mix, type Part } from "./ui.ts";

/** What the panel of the chains reads to draw itself. */
export interface ChainsInput {
  loom: Loom;
  width: number;
  height: number;
  /** The chain that the pointer stands on, or `rest:` and the chain whose finished chains the pointer stands on. */
  pointer: string;
  /** The chains whose branches the panel folds into them. */
  folds: ReadonlySet<string>;
  /** The chains whose finished chains the panel shows one by one, and not as one row. */
  rested: ReadonlySet<string>;
  /** The first line of the body that shows. */
  top: number;
  actor(chain: string): string;
}

/** A row that the pointer can stand on. */
export interface ChainEntry {
  id: string;
  parent?: string;
  /** Whether it holds branches, and whether they are folded. */
  foldable: boolean;
  folded: boolean;
}

export interface ChainsDrawn {
  /** The title bar. */
  header: Part[];
  lines: Part[][];
  /** The id of the entry of each line, for a click. */
  targets: (string | undefined)[];
  /** The column of the fold arrow of the entry of each line. */
  arrows: (number | undefined)[];
  top: number;
  entries: ChainEntry[];
}

interface Node {
  row: Row;
  parent?: Node;
  kids: Node[];
  /** The first moment of its cells that holds work or a question, and -1 when none does. */
  birth: number;
}

interface Shown {
  node: Node;
  lines: string;
  folded: boolean;
  /** For a row that stands for the finished chains of a chain: how many, and whether they show. */
  rest?: { count: number; open: boolean };
}

interface Line {
  parts: Part[];
  target?: string;
  arrow?: number;
}

const ink: Record<Tone, () => RGBA> = {
  work: () => c.accent,
  past: () => c.muted,
  asks: () => c.warning,
  failed: () => c.danger,
  none: () => c.faint,
};
const settled = (row: Row) => row.state === "done" || row.state === "idle";
const labelWidth = (width: number) => Math.max(22, Math.min(32, Math.floor(width * 0.28)));
/** The columns of what a row waits on. */
const waitsWidth = 20;

/** How many cells of time the panel weaves at a width. */
export function weaveWidth(width: number): number {
  return Math.max(8, width - 4 - labelWidth(width) - 1 - waitsWidth - 2);
}

/** The chains of a loom as a tree, with the operator left out. */
function nodesOf(loom: Loom): Node[] {
  const out: Node[] = [];
  const stack: Node[] = [];
  for (const row of loom.rows) {
    if (row.id === "operator") continue;
    const depth = Math.floor(row.lines.length / 3);
    stack.length = Math.min(depth, stack.length);
    const parent = stack[depth - 1];
    const node: Node = { row, parent, kids: [], birth: row.cells.findIndex((cell) => cell.tone !== "none") };
    parent?.kids.push(node);
    stack[depth] = node;
    out.push(node);
  }
  return out;
}

/** The chain of the pointer and the chains above it. */
function lineageOf(nodes: Node[], pointer: string): Set<string> {
  const path = new Set<string>();
  const id = pointer.startsWith("rest:") ? pointer.slice(5) : pointer;
  for (let node = nodes.find((one) => one.row.id === id); node; node = node.parent) path.add(node.row.id);
  return path;
}

const allSettled = (node: Node): boolean => settled(node.row) && node.kids.every(allSettled);

/** The chains that a fresh panel folds: the branches that are all finished, but those on the way to the pointer. */
export function quietFolds(loom: Loom, pointer: string): Set<string> {
  const nodes = nodesOf(loom);
  const path = lineageOf(nodes, pointer);
  return new Set(
    nodes
      .filter((n) => n.kids.length > 0 && n.kids.every(allSettled) && !path.has(n.row.id))
      .map((n) => n.row.id),
  );
}

/** The rows in the order of the tree, with their elbows. The finished chains of a chain, with no branch of their own
 * and away from the pointer, join one row when they are more than one. */
function layout(nodes: Node[], input: ChainsInput, path: ReadonlySet<string>): Shown[] {
  const out: Shown[] = [];
  const pointed = input.pointer.startsWith("rest:") ? input.pointer.slice(5) : input.pointer;
  const quiet = (kid: Node) => !kid.kids.length && settled(kid.row) && !path.has(kid.row.id);
  const visit = (node: Node, cont: string, last: boolean, depth: number) => {
    const folded = input.folds.has(node.row.id) && (node.row.id === pointed || !path.has(node.row.id));
    out.push({ node, folded, lines: depth ? cont + (last ? "╰─ " : "├─ ") : "" });
    if (folded || !node.kids.length) return;
    const open = input.rested.has(node.row.id);
    const count = node.kids.filter(quiet).length;
    const squash = count > 1;
    const shown = node.kids.filter((kid) => !squash || open || !quiet(kid));
    const inner = depth ? cont + (last ? "   " : "│  ") : "";
    for (const [index, kid] of shown.entries())
      visit(kid, inner, index === shown.length - 1 && !squash, depth + 1);
    if (squash)
      out.push({
        node: { ...node, kids: [], row: { ...node.row, id: `rest:${node.row.id}`, cells: [] } },
        folded: !open,
        lines: `${inner}╰─ `,
        rest: { count, open },
      });
  };
  for (const root of nodes.filter((n) => !n.parent)) visit(root, "", true, 0);
  return out;
}

function merged(parts: Part[]): Part[] {
  const out: Part[] = [];
  for (const part of parts) {
    const last = out.at(-1);
    if (last && last[1] === part[1] && (last[2] ?? 0) === (part[2] ?? 0) && last[3] === part[3])
      out[out.length - 1] = [last[0] + part[0], last[1], last[2], last[3]];
    else out.push(part);
  }
  return out;
}

function stateWord(row: Row, input: ChainsInput): string {
  const ask = questionOf(row.id, input.loom.owed);
  if (ask) return ask;
  if (row.state === "working") return row.waits ?? "working";
  return row.state === "asks" ? "asks you" : row.state;
}

function stateInk(row: Row, input: ChainsInput): RGBA {
  if (questionOf(row.id, input.loom.owed) || row.state === "paused") return c.warning;
  return row.state === "failed" ? c.danger : c.muted;
}

function markOf(shown: Shown): Part {
  if (shown.rest) return ["✓", c.faint];
  const [mark, color] = rowMarks[shown.node.row.state];
  return [shown.node.row.state === "working" ? spin() : mark, color()];
}

/** The label of a row: its elbows, its fold arrow, and its name. */
function labelParts(
  shown: Shown,
  path: ReadonlySet<string>,
  width: number,
): { parts: Part[]; arrow?: number } {
  const { node } = shown;
  const row = node.row;
  const hot = path.has(row.id);
  const holds = shown.rest ? true : node.kids.length > 0;
  const arrow = holds ? (shown.folded ? "▸ " : "▾ ") : "  ";
  const tally = !shown.rest && shown.folded ? ` ${node.kids.length}↳` : "";
  const name = shown.rest ? `${shown.rest.count} finished` : row.label;
  const room = Math.max(4, width - Bun.stringWidth(shown.lines + arrow));
  return {
    parts: [
      [shown.lines, hot ? c.accent : c.faint],
      [arrow, c.faint],
      [
        clip(name + tally, room),
        hot ? c.text : shown.rest || settled(row) ? c.muted : c.text,
        hot ? bold : 0,
      ],
    ],
    arrow: holds ? Bun.stringWidth(shown.lines) : undefined,
  };
}

/** The lines that stand under the body: the chain of the pointer, its state and its keys. */
function footer(input: ChainsInput, shown: Shown[]): Line[] {
  const { width } = input;
  const at = shown.find((one) => one.node.row.id === input.pointer);
  const rule: Line = { parts: [["─".repeat(width), c.faint]] };
  const blank: Line = { parts: pad([], width) };
  const keys: Line = {
    parts: pad(
      [
        [" ↑↓ ", c.text],
        ["move   ", c.faint],
        ["←→ ", c.text],
        ["fold   ", c.faint],
        ["Enter ", c.text],
        ["open   ", c.faint],
        ["Tab ", c.text],
        ["next question   ", c.faint],
        ["Space ", c.text],
        ["pause   ", c.faint],
        ["x ", c.text],
        ["cancel   ", c.faint],
        ["Esc ", c.text],
        ["close", c.faint],
      ],
      width,
    ),
  };
  if (!at) return [rule, blank, blank, keys];
  const row = at.node.row;
  if (at.rest)
    return [
      rule,
      { parts: pad([[` ${at.rest.count} finished chains of ${at.node.row.label}`, c.muted]], width) },
      {
        parts: pad(
          [[at.rest.open ? " Enter folds them into one row" : " Enter shows each of them", c.faint]],
          width,
        ),
      },
      keys,
    ];
  const crumbs: string[] = [];
  for (let node: Node | undefined = at.node; node; node = node.parent) crumbs.unshift(node.row.label);
  const [mark, color] = rowMarks[row.state];
  const ask = questionOf(row.id, input.loom.owed);
  const doing = row.state === "working" ? `working${row.waits ? `, waits on ${row.waits}` : ""}` : row.state;
  return [
    rule,
    {
      parts: [
        ...pad(
          [
            [` ${crumbs.slice(0, -1).join(" › ")}${crumbs.length > 1 ? " › " : ""}`, c.faint],
            [crumbs.at(-1) ?? "", c.text, bold],
            [`   ${shortActor(input.actor(row.id))}`, c.muted],
          ],
          width - 20,
        ),
        ...measureParts(row.measure),
      ],
    },
    {
      parts: pad(
        [
          [` ${mark} `, color()],
          [ask || doing, ask || row.state === "paused" ? c.warning : c.muted],
        ],
        width,
      ),
    },
    keys,
  ];
}

/** The ribbon of a row over the cells of time, with the right edge at now. */
function ribbon(row: Row, width: number): Part[] {
  const cells = row.cells.slice(-width);
  const offset = width - cells.length;
  const out: Part[] = Array.from({ length: width }, () => [" "] as Part);
  for (const [index, cell] of cells.entries()) {
    const glyph = { work: "━", past: "─", asks: "◆", failed: "✗", none: " " }[cell.tone];
    const fade = 0.45 + (0.55 * (index + 1)) / cells.length;
    out[offset + index] =
      index === cells.length - 1 && row.state === "working"
        ? [spin(), c.accent]
        : [glyph, mix(c.background, ink[cell.tone](), fade)];
  }
  return out;
}

/** Time runs to the right and its right edge is now. Each chain is a thread, in the order of the tree, and a branch
 * drops from the thread of its chain at the moment it was made. */
function weave(input: ChainsInput, shown: Shown[], path: ReadonlySet<string>) {
  const { width } = input;
  const label = labelWidth(width);
  const span = weaveWidth(width);
  const grids = shown.map((one) => ribbon(one.node.row, span));
  const index = new Map(shown.map((one, at) => [one.node.row.id, at]));
  for (const [at, one] of shown.entries()) {
    const parent = one.node.parent && index.get(one.node.parent.row.id);
    if (parent === undefined || one.node.birth < 0 || one.rest) continue;
    const col = Math.max(0, span - Math.min(span, one.node.row.cells.length) + one.node.birth - 1);
    for (let between = parent + 1; between < at; between++) {
      const row = grids[between];
      if (row && row[col]?.[0] === " ") row[col] = ["│", c.faint];
    }
    const mine = grids[at];
    if (mine && mine[col]?.[0] === " ") mine[col] = ["╰", c.faint];
    const top = grids[parent];
    const over = top?.[col];
    if (top && over) top[col] = [over[0] === "━" ? "┰" : over[0] === "─" ? "┬" : over[0], over[1]];
  }
  const rows = shown.map((one, at): Line => {
    const row = one.node.row;
    const active = row.id === input.pointer;
    const named = labelParts(one, path, label);
    const waits = one.rest ? "" : stateWord(row, input);
    return {
      target: row.id,
      arrow: named.arrow === undefined ? undefined : 4 + named.arrow,
      parts: pad(
        [
          [active ? "▎" : " ", c.accent],
          [" "],
          markOf(one),
          [" "],
          ...pad(named.parts, label),
          [" "],
          ...merged(grids[at] ?? []),
          ["  "],
          [clip(waits, waitsWidth), one.rest ? c.faint : stateInk(row, input)],
        ],
        width,
        active ? c.selected : undefined,
      ),
    };
  });
  const ruler: Line = {
    parts: pad(
      [
        [" ".repeat(4 + label + 1), c.faint],
        ["older".padEnd(Math.max(0, span - 3)), c.faint],
        ["now", c.faint],
        ["  waits on", c.faint],
      ],
      width,
    ),
  };
  return { rows, focus: shown.findIndex((one) => one.node.row.id === input.pointer), fixed: [ruler] };
}

/** The panel of the chains, drawn at a size. */
export function drawChains(input: ChainsInput): ChainsDrawn {
  const nodes = nodesOf(input.loom);
  const path = lineageOf(nodes, input.pointer);
  const shown = layout(nodes, input, path);
  const { rows, focus, fixed } = weave(input, shown, path);
  const foot = footer(input, shown);
  const room = Math.max(3, input.height - fixed.length - foot.length);
  let top = Math.max(0, Math.min(input.top, rows.length - room));
  if (focus >= 0 && focus < top) top = focus;
  if (focus >= top + room) top = focus - room + 1;
  const body = rows.slice(top, top + room);
  while (body.length < room) body.push({ parts: pad([], input.width) });
  const all = [...fixed, ...body, ...foot];
  const asking = new Set(input.loom.owed.map((prompt) => prompt.chain)).size;
  const header: Part[] = [
    ["Chains", c.text, bold],
    [`  ${shown.filter((one) => !one.rest).length} shown`, c.faint],
    ...(asking
      ? ([[`   ◆ ${asking} ${asking === 1 ? "chain asks" : "chains ask"} you`, c.warning]] as Part[])
      : []),
  ];
  return {
    header,
    lines: all.map((line) => line.parts),
    targets: all.map((line) => line.target),
    arrows: all.map((line) => line.arrow),
    top,
    entries: shown.map((one) => ({
      id: one.node.row.id,
      parent: one.rest ? one.node.row.id.slice(5) : one.node.parent?.row.id,
      foldable: one.rest ? true : one.node.kids.length > 0,
      folded: one.folded,
    })),
  };
}

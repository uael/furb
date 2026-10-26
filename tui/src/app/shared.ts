import type { BoxRenderable, CliRenderer, KeyEvent, Renderable, RGBA, TextOptions } from "@opentui/core";
import type { Keymap } from "@opentui/keymap";
import { createDefaultOpenTuiKeymap } from "@opentui/keymap/opentui";
import type { ChainThread, Chip, RowState } from "furb/life";
import { ago, clip, dollars, kibibytes } from "../format.ts";
import type { ActRow, SessionStatus, View } from "../session.ts";
import { statusLabels } from "../session.ts";
import { theme as c, glyph, terminalThemes, themeLabels } from "../theme.ts";
import type { Part } from "../ui.ts";
import type { SessionEntry } from "../workspaces.ts";

export const exitNotice = "Press ⌃D again to exit.";

export const rewindNotice = "Press Escape again to rewind.";

/** The time within which a second Escape rewinds. */
export const twice = 800;

/** The name of each view as the toggle and the commands show it. */
export const viewLabels: Record<View, string> = {
  feed: "Feed",
  transcript: "Transcript",
  changes: "Changes",
};

/** A text with its first letter in upper case. */
export const title = (text: string) => text.charAt(0).toUpperCase() + text.slice(1);

/** What an act is doing: the word that says it, the glyph that shows it, and their color. */
export type State = { word: string; mark: string; color: RGBA };

/** The state of an act that failed, that a cancel ended, that is done, or that a pause holds. */
export const stateOf = (state: "failed" | "cancelled" | "done" | "held"): State =>
  ({
    failed: { word: "failed", mark: glyph.failed, color: c.danger },
    cancelled: { word: "cancelled", mark: glyph.cancelled, color: c.faint },
    done: { word: "", mark: glyph.done, color: c.success },
    held: { word: "waits for a wake", mark: glyph.held, color: c.warning },
  })[state];

/** The mark of a state in its color, one shape for each: at work, waiting on the operator, paused, failed, finished
 * and not yet seen, and at rest. */
export const statusMark = (status: SessionStatus): Part =>
  (
    ({
      working: [`${glyph.running} `, c.accent],
      opening: [`${glyph.running} `, c.accent],
      blocked: [`${glyph.asks} `, c.warning],
      paused: [`${glyph.held} `, c.warning],
      error: [`${glyph.failed} `, c.danger],
      reading: [`${glyph.ring} `, c.warning],
      done: [`${glyph.dot} `, c.success],
      idle: [`${glyph.ring} `, c.faint],
      saved: [`${glyph.ring} `, c.faint],
    }) satisfies Record<SessionStatus, Part>
  )[status];

/** The rows that a key moves the pointer of a list by, in the rewind tree and in a dialog. */
/** The mark and the color of the state of a chain, as every view marks it. */
export const chainMarks: Record<RowState, [() => string, () => RGBA]> = {
  asks: [() => glyph.asks, () => c.warning],
  failed: [() => glyph.failed, () => c.danger],
  working: [() => glyph.running, () => c.accent],
  paused: [() => glyph.held, () => c.warning],
  done: [() => glyph.done, () => c.success],
  idle: [() => glyph.ring, () => c.faint],
};

/** The mark of a thread in the list of the threads of a chain, and its color: it asks the operator, a pause holds it,
 * it failed, it is resolved, or it runs. */
/** The threads of the chain shown: in order, each by its id, and the thread of each message, by the id of the message. */
export type ThreadList = {
  acts: unknown;
  owed: string;
  list: ChainThread[];
  by: Map<string, ChainThread>;
  of: Map<string, ChainThread>;
};

export const threadMarks = (thread: ChainThread): [string, RGBA] =>
  thread.state === "asks"
    ? [glyph.asks, c.warning]
    : thread.state === "held"
      ? [glyph.held, c.warning]
      : thread.state === "failed"
        ? [glyph.failed, c.danger]
        : thread.state === "cancelled"
          ? [glyph.cancelled, c.faint]
          : thread.resolved
            ? [glyph.done, c.faint]
            : [glyph.running, c.accent];

/** How many of the last steps of a thread its program shows until a click shows the earlier ones. */
export const PROGRAM_STEPS = 6;

/** A time in a few characters: milliseconds under a second, tenths under ten seconds, then seconds and minutes. */
export const spell = (ms: number) =>
  ms < 1000
    ? `${Math.round(ms)}ms`
    : ms < 10000
      ? `${(ms / 1000).toFixed(1)}s`
      : ms < 60000
        ? `${Math.round(ms / 1000)}s`
        : `${Math.floor(ms / 60000)}m ${Math.round((ms % 60000) / 1000)}s`;

/** How many rows of the feed a long message takes until a click opens it. */
export const MESSAGE_ROWS = 3;

/** The lines of a text cut at its words to a width, at most a count of them, the last one cut where the text goes
 * on. */
export function wrapped(text: string, width: number, most: number): string[] {
  const lines: string[] = [];
  let line = "";
  for (const word of text.split(/\s+/).filter(Boolean)) {
    if (line && Bun.stringWidth(`${line} ${word}`) > width) {
      lines.push(line);
      line = word;
    } else line = line ? `${line} ${word}` : word;
  }
  if (line) lines.push(line);
  const kept = lines.slice(0, most);
  if (lines.length > most) kept[most - 1] = clip(`${kept[most - 1]} …`, width);
  return kept.map((one) => clip(one, width));
}

/** The color of the state of an act on a chip. */
export const chipColors: Record<Chip["state"], () => RGBA> = {
  running: () => c.accent,
  stands: () => c.muted,
  done: () => c.faint,
  failed: () => c.danger,
  cancelled: () => c.faint,
  held: () => c.warning,
  asks: () => c.warning,
};

export const steps: Record<string, number> = {
  up: -1,
  down: 1,
  pageup: -8,
  pagedown: 8,
  home: -1e9,
  end: 1e9,
};

/** What the footer says of a key: its chord and what it does, each read when the footer is drawn. */
export type Hint = readonly [chord: string | (() => string), action: string | (() => string)];

/** A command of a layer of keys: the presses that run it, what it does with the key, when it acts, and what the footer
 * says of it. */
export type KeyCommand = {
  on: string | readonly string[];
  /** What the command does with its key, which gives false when it does not act, and the key goes on. */
  run: (key?: KeyEvent) => unknown;
  when?: () => boolean;
  hint?: Hint;
};

/** The keymap of each renderer, which lives as long as the renderer: each App on it adds its layers, and takes them
 * away when it ends. A binding may carry a hint, which the footer reads. */
export const keymaps = new WeakMap<CliRenderer, Keymap<Renderable, KeyEvent>>();

export function keymapOf(renderer: CliRenderer): Keymap<Renderable, KeyEvent> {
  let keymap = keymaps.get(renderer);
  if (!keymap) {
    keymap = createDefaultOpenTuiKeymap(renderer);
    keymap.registerBindingFields({ hint: (value, field) => field.attr("hint", value) });
    keymaps.set(renderer, keymap);
  }
  return keymap;
}

/** A span of seconds as a wait says it: 1 second, 0.2 seconds. */
export const seconds = (value: number) => `${value} ${value === 1 ? "second" : "seconds"}`;

/** The options of a text, and the action that a click on it runs. */
export type TextShape = TextOptions & { run?: () => void };

/** A border that draws nothing, which a part of a border replaces. */
export const noBorder = {
  topLeft: " ",
  topRight: " ",
  bottomLeft: " ",
  bottomRight: " ",
  horizontal: " ",
  vertical: " ",
  topT: " ",
  bottomT: " ",
  leftT: " ",
  rightT: " ",
  cross: " ",
};

/** The colors of an input on a ground: of its text, its placeholder, and its cursor. */
export const inputColors = (ground: RGBA) => ({
  backgroundColor: ground,
  focusedBackgroundColor: ground,
  textColor: c.text,
  focusedTextColor: c.text,
  placeholderColor: c.faint,
  cursorColor: c.accent,
});

/** The suggestions of an empty feed: what each says, and the prompt it puts in the composer. */
/** The card that plays a life to watch, which the welcome offers first. */
export const watching = ["Watch a life", "Play a short life on a small project. No model, no cost."] as const;

export const starters = [
  ["Explore a codebase", "Read the README and explain how this project works."],
  ["Make something better", "Find one useful improvement in this project and implement it."],
  ["Start with a plan", "Read the project and propose a small, testable plan."],
] as const;

export interface BlockOptions {
  compact?: boolean;
  collapsible?: boolean;
  heading?: boolean;
  group?: string;
  separate?: boolean;
  preview?: (box: BoxRenderable) => void;
  act?: ActRow;
  /** The label again, which the tick reads while the label moves with time. */
  title?: () => Part[];
  /** The columns the card stands in from the edge of the feed, to show that the act above made it. */
  indent?: number;
}

export interface Choice {
  label: string;
  detail: string;
  run(): void | Promise<void>;
  /** The color of the label, for a choice whose label is a state. */
  color?: RGBA;
  /** The colors that a choice of a theme shows before its detail. */
  swatch?: string[];
  /** A glyph before the label, in its own color, that shows the state of what the choice names. */
  mark?: Part;
  /** The slash command that does what the choice does, with its arguments, which the list shows under the label. */
  command?: string;
  /** The keys that do what the choice does. */
  keys?: string;
  /** The state of a session or a chain that the choice names, which its mark shows. */
  status?: SessionStatus;
  /** The title of the part of the list that the choice opens, which the list shows above it. */
  heading?: string;
}

/** A value that the first argument of a command may take: what it means, whether it is the current one, whether the
 * command takes more after it, and the label and the heading that a picker shows it under. */
export interface Value {
  value: string;
  detail: string;
  current?: boolean;
  more?: boolean;
  label?: string;
  heading?: string;
}

/** One suggestion for the token before the cursor: the text that Tab puts in its place, and what Enter does when it
 * does more than that. */
export interface Suggestion {
  label: string;
  detail: string;
  text: string;
  submit?: () => void;
}

/** A row of the rewind tree: the chain or the act it shows, the lines of the tree before it, its label, the tag at
 * its right, what it does when it is chosen, and what the footer says it does. */
export interface TreeRow {
  id: string;
  lines: string;
  parts: Part[];
  tag: string;
  hint: string;
  run(): void | Promise<void>;
  /** Whether the row has rows under it, and whether they are folded. */
  parent: boolean;
  folded: boolean;
  /** The row above it in the tree, which Left goes to. */
  up?: string;
}

/** What the choice of the theme of the terminal says in a picker. */
export const terminalDetail = `The colors of the terminal, or ${themeLabels[terminalThemes.dark][0]} on a dark terminal and ${themeLabels[terminalThemes.light][0]} on a light one when it does not say them`;

/** The commands whose value is a path of the project, which the suggestions wait for. */
export const pathCommands = new Set(["read", "image", "cd"]);

/** What each effort of a model does, which the picker of the effort says beside it. */
export const efforts: Record<string, string> = {
  off: "Answer with no thought first",
  minimal: "Think as little as the model can",
  low: "Quick answers to simple work",
  medium: "A balance of speed and depth",
  high: "More thought for hard work",
  xhigh: "Deep thought for the hardest work",
  max: "All the thought the model allows",
};

/** What the picker of the workspaces says of a session: its state, whether it is the current one, the time since
 * its record was saved, its cost, and the size of its record. */
export function sessionDetail(entry: SessionEntry, current: boolean): string {
  return [
    statusLabels[entry.status],
    current ? "current" : "",
    entry.modified ? ago(entry.modified) : "",
    dollars(entry.cost ?? 0),
    entry.size === undefined ? "" : kibibytes(entry.size),
  ]
    .filter(Boolean)
    .join("   ");
}

import type { BoxRenderable, CliRenderer, KeyEvent, Renderable, RGBA, TextOptions } from "@opentui/core";
import type { Keymap } from "@opentui/keymap";
import { createDefaultOpenTuiKeymap } from "@opentui/keymap/opentui";
import { busy, type ChainThread, type Chip } from "furb/life";
import { ago, clip, dollars, kibibytes } from "../format.ts";
import type { ActRow, Status, View } from "../session.ts";
import { statusLabels } from "../session.ts";
import { theme as c, markOf, spacing as space, terminalThemes, themeLabels } from "../theme.ts";
import { bold, type Part } from "../ui.ts";
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
export const stateOf = (state: "failed" | "cancelled" | "done" | "paused"): State => {
  const [mark, color] = markOf(state);
  return {
    word: { failed: "failed", cancelled: "cancelled", done: "", paused: "waits for a wake" }[state],
    mark,
    color,
  };
};

/** The mark of the status of a session in its color, with the space after it: at work, waiting on the operator,
 * paused, failed, finished and not yet seen, and at rest. */
export const statusMark = (status: Status): Part => {
  // A state of the window shows as the state of work that it stands for: a session that opens runs, and the others rest.
  const [mark, color, attributes] = markOf(
    status === "opening" ? "runs" : status === "reading" || status === "saved" ? "idle" : status,
  );
  return [`${mark} `, color, attributes];
};

/** The threads of one chain, with what the keys of the sidebar read of them while the list stands. */
export type ChainRead = {
  list: ChainThread[];
  key?: { minute: number; text: string };
  /** Whether each thread of the list holds work that the operator did not read, at a count of the reads. */
  unread?: { reads: number; text: string };
};

/** The mark of a thread in the list of the threads of a chain: it asks the operator, a pause holds it, it failed, a
 * cancel ended it, it is resolved with work that the operator did not read, it is resolved and seen, or it runs. Work
 * that the operator did not read concerns him only on a thread that he started. */
export const threadMarks = (
  thread: ChainThread,
  state: ChainThread["state"] = thread.state,
  unread = false,
): Part =>
  markOf(
    state === "asks" || state === "paused" || state === "failed" || state === "cancelled"
      ? state
      : thread.resolved
        ? unread && thread.yours
          ? "unread"
          : "seen"
        : busy(state)
          ? state
          : "runs",
  );

/** How many of the last steps of a thread its program shows until a click shows the earlier ones. */
export const PROGRAM_STEPS = 6;

/** The width of the numbers of a program, with the space after them: room for two digits at least, so the code of
 * each thread of under a hundred lines starts at one column. */
export const gutterWidth = (digits: number) => Math.max(2, digits) + space.inset;

/** The cells of the column of the states, at the right of the numbers of a block of code, so the numbers stand at the
 * left edge of the block. They show the loader while the model writes, and the mark of the line that failed. A code
 * block of markdown keeps the column blank. So the numbers and the code of every block have the same columns, which
 * the one gutter of furb, `Gutter` in `gutter.ts`, draws for both. */
export const STATE = 1;

/** The label that heads a block of code: the model that wrote a word, or the language of a code block of Markdown. A
 * word draws it with the group word.label of the syntax style, which holds this color and this weight. */
export const blockLabel = (text: string): Part => [text, c.muted, bold];

/** A row that a model writes as the tick compares it: its number, and its text with the color of each part. */
export const saidOf = (number: number, parts: readonly Part[]) =>
  `${number}:${parts.map(([text, fg]) => `${text}\u0000${fg ? `${fg.r},${fg.g},${fg.b}` : ""}`).join("\u0001")}`;

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
  asks: () => c.warning,
  unread: () => c.accent,
  failed: () => c.danger,
  paused: () => c.warning,
  thinks: () => c.accent,
  runs: () => c.accent,
  delegates: () => c.muted,
  done: () => c.faint,
  cancelled: () => c.faint,
  idle: () => c.faint,
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
  /** A card that no fold of the feed closes, since its body folds itself, as the panel of a message does. */
  open?: boolean;
  /** A card that is a part of the box of a thread: the cards of one thread share a panel, and the first and the last
   * of them draw its edges. */
  join?: "only" | "first" | "middle" | "last";
  /** The color of the line of a message of the operator that stands first or last in the box of a thread, whose cap
   * the edge of the box holds. */
  edge?: RGBA;
  /** The thread whose box the card stands in, which a fold from any line of the box folds, and whose last card ends it. */
  thread?: string;
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
  status?: Status;
  /** The title of the part of the list that the choice opens, which the list shows above it. */
  heading?: string;
  /** The lines that show under the choice while it is selected, in a list that is not rich. */
  lines?: Part[][];
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

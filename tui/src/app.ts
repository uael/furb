import { isAbsolute, join, relative } from "node:path";
import {
  type BoxOptions,
  BoxRenderable,
  type CliRenderer,
  CodeRenderable,
  DiffRenderable,
  getTreeSitterClient,
  InputRenderable,
  InputRenderableEvents,
  infoStringToFiletype,
  type KeyEvent,
  type LineNumberOptions,
  LineNumberRenderable,
  MarkdownRenderable,
  type MouseEvent,
  type Renderable,
  type RenderNodeContext,
  RGBA,
  type ScrollBoxOptions,
  ScrollBoxRenderable,
  type SimpleHighlight,
  StyledText,
  TextareaRenderable,
  type TextChunk,
  type TextOptions,
  TextRenderable,
} from "@opentui/core";
import type { Keymap } from "@opentui/keymap";
import { createDefaultOpenTuiKeymap } from "@opentui/keymap/opentui";
import {
  actorParts,
  display,
  imageContent,
  imagePath,
  imageReferences,
  safeText,
  shapes,
  TIMEOUT,
  type Turn,
  WORK,
} from "furb";
import {
  type Ask,
  actLines,
  asksOperator,
  beside,
  type ChainThread,
  type Chip,
  cancelled,
  chainThreads,
  condense,
  doingOf,
  type Exit,
  FOLD,
  type Fold,
  failed,
  foldStrings,
  gist,
  type Loom,
  ladder,
  literals,
  makers,
  momentsOf,
  nowOf,
  owedAt,
  type ProgramLine,
  type RowState,
  stepLines,
  thread,
  threadProgram,
  wants,
  weave,
  writing,
} from "furb/life";
import type { Token, Tokens } from "marked";
import { clipboardImage } from "./clipboard.ts";
import { commands, slashes } from "./commands.ts";
import { conversation } from "./conversation.ts";
import { demoSession, tour } from "./demo.ts";
import { externalEditor, openFile } from "./editor.ts";
import { emojiNamed, emojis } from "./emoji.ts";
import { fromDirectory, shortenHome, shortenHomes } from "./files.ts";
import {
  ago,
  clip,
  count,
  dollars,
  elapsed,
  graphemes,
  kibibytes,
  modelName,
  plainFailure,
  retryLine,
  share,
} from "./format.ts";
import { Graph, type Stage } from "./graph.ts";
import { type Action, bindings, chords, keys, shown } from "./keys.ts";
import {
  actorOf,
  isLifeView,
  joined,
  type LifeView,
  LoomPane,
  lifeViewDetails,
  lifeViewLabels,
  lifeViews,
} from "./overview.ts";
import { loadParsers } from "./parsers.ts";
import {
  type Line as PartLine,
  Parts,
  type Tone as PartTone,
  plainError,
  type Row,
  type Seen,
} from "./parts.ts";
import { commentStart, type Hunk, hunkPatch, lines, type Scope, scopeLabels, scopes } from "./review.ts";
import {
  type ActRow,
  type Cancel,
  type Scroll,
  type Session,
  type SessionStatus,
  statusLabels,
  type View,
  views,
  working,
} from "./session.ts";
import { type Check, checks } from "./setup.ts";
import { publishShare } from "./share.ts";
import {
  theme as c,
  colorless,
  defaultTheme,
  frame,
  glyph,
  motion,
  paletteOf,
  palettes,
  setTheme,
  spacing as space,
  spin,
  syntax,
  type ThemeChoice,
  type ThemeName,
  terminalThemes,
  themeLabels,
  threadTone,
} from "./theme.ts";
import { bold, italic, logo, mix, type Part, plain, styled, underline } from "./ui.ts";
import type { SessionEntry, Workspace, Workspaces } from "./workspaces.ts";

const exitNotice = "Press ⌃D again to exit.";
const rewindNotice = "Press Escape again to rewind.";
/** The time within which a second Escape rewinds. */
const twice = 800;
/** The name of each view as the toggle and the commands show it. */
const viewLabels: Record<View, string> = { feed: "Feed", transcript: "Transcript", changes: "Changes" };
/** A text with its first letter in upper case. */
const title = (text: string) => text.charAt(0).toUpperCase() + text.slice(1);
/** What an act is doing: the word that says it, the glyph that shows it, and their color. */
type State = { word: string; mark: string; color: RGBA };
/** The state of an act that failed, that a cancel ended, that is done, or that a pause holds. */
const stateOf = (state: "failed" | "cancelled" | "done" | "held"): State =>
  ({
    failed: { word: "failed", mark: glyph.failed, color: c.danger },
    cancelled: { word: "cancelled", mark: glyph.cancelled, color: c.faint },
    done: { word: "", mark: glyph.done, color: c.success },
    held: { word: "waits for a wake", mark: glyph.held, color: c.warning },
  })[state];
/** The mark of a state in its color, one shape for each: at work, waiting on the operator, paused, failed, finished
 * and not yet seen, and at rest. */
const statusMark = (status: SessionStatus): Part =>
  (
    ({
      working: [`${glyph.running} `, c.accent],
      opening: [`${glyph.running} `, c.accent],
      blocked: [`${glyph.asks} `, c.warning],
      paused: [`${glyph.held} `, c.warning],
      error: [`${glyph.failed} `, c.danger],
      done: [`${glyph.dot} `, c.success],
      idle: [`${glyph.ring} `, c.faint],
      saved: [`${glyph.ring} `, c.faint],
    }) satisfies Record<SessionStatus, Part>
  )[status];
/** The rows that a key moves the pointer of a list by, in the rewind tree and in a dialog. */
/** The mark and the color of the state of a chain, as every view marks it. */
const chainMarks: Record<RowState, [() => string, () => RGBA]> = {
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
type ThreadList = {
  acts: unknown;
  owed: string;
  list: ChainThread[];
  by: Map<string, ChainThread>;
  of: Map<string, ChainThread>;
};
const threadMarks = (thread: ChainThread): [string, RGBA] =>
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
const PROGRAM_STEPS = 6;
/** A time in a few characters: milliseconds under a second, tenths under ten seconds, then seconds and minutes. */
const spell = (ms: number) =>
  ms < 1000
    ? `${Math.round(ms)}ms`
    : ms < 10000
      ? `${(ms / 1000).toFixed(1)}s`
      : ms < 60000
        ? `${Math.round(ms / 1000)}s`
        : `${Math.floor(ms / 60000)}m ${Math.round((ms % 60000) / 1000)}s`;
/** How many rows of the feed a long message takes until a click opens it. */
const MESSAGE_ROWS = 3;
/** The lines of a text cut at its words to a width, at most a count of them, the last one cut where the text goes
 * on. */
function wrapped(text: string, width: number, most: number): string[] {
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
const chipColors: Record<Chip["state"], () => RGBA> = {
  running: () => c.accent,
  stands: () => c.muted,
  done: () => c.faint,
  failed: () => c.danger,
  cancelled: () => c.faint,
  held: () => c.warning,
  asks: () => c.warning,
};
const steps: Record<string, number> = { up: -1, down: 1, pageup: -8, pagedown: 8, home: -1e9, end: 1e9 };
/** What the footer says of a key: its chord and what it does, each read when the footer is drawn. */
type Hint = readonly [chord: string | (() => string), action: string | (() => string)];
/** A command of a layer of keys: the presses that run it, what it does with the key, when it acts, and what the footer
 * says of it. */
type KeyCommand = {
  on: string | readonly string[];
  /** What the command does with its key, which gives false when it does not act, and the key goes on. */
  run: (key?: KeyEvent) => unknown;
  when?: () => boolean;
  hint?: Hint;
};
/** The keymap of each renderer, which lives as long as the renderer: each App on it adds its layers, and takes them
 * away when it ends. A binding may carry a hint, which the footer reads. */
const keymaps = new WeakMap<CliRenderer, Keymap<Renderable, KeyEvent>>();
function keymapOf(renderer: CliRenderer): Keymap<Renderable, KeyEvent> {
  let keymap = keymaps.get(renderer);
  if (!keymap) {
    keymap = createDefaultOpenTuiKeymap(renderer);
    keymap.registerBindingFields({ hint: (value, field) => field.attr("hint", value) });
    keymaps.set(renderer, keymap);
  }
  return keymap;
}
/** A span of seconds as a wait says it: 1 second, 0.2 seconds. */
const seconds = (value: number) => `${value} ${value === 1 ? "second" : "seconds"}`;
/** The options of a text, and the action that a click on it runs. */
type TextShape = TextOptions & { run?: () => void };
/** A border that draws nothing, which a part of a border replaces. */
const noBorder = {
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
const inputColors = (ground: RGBA) => ({
  backgroundColor: ground,
  focusedBackgroundColor: ground,
  textColor: c.text,
  focusedTextColor: c.text,
  placeholderColor: c.faint,
  cursorColor: c.accent,
});
/** The suggestions of an empty feed: what each says, and the prompt it puts in the composer. */
/** The card that plays a life to watch, which the welcome offers first. */
const watching = ["Watch a life", "Play a short life on a small project. No model, no cost."] as const;
const starters = [
  ["Explore a codebase", "Read the README and explain how this project works."],
  ["Make something better", "Find one useful improvement in this project and implement it."],
  ["Start with a plan", "Read the project and propose a small, testable plan."],
] as const;
interface BlockOptions {
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
interface Choice {
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
interface Value {
  value: string;
  detail: string;
  current?: boolean;
  more?: boolean;
  label?: string;
  heading?: string;
}
/** One suggestion for the token before the cursor: the text that Tab puts in its place, and what Enter does when it
 * does more than that. */
interface Suggestion {
  label: string;
  detail: string;
  text: string;
  submit?: () => void;
}
/** A row of the rewind tree: the chain or the act it shows, the lines of the tree before it, its label, the tag at
 * its right, what it does when it is chosen, and what the footer says it does. */
interface TreeRow {
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

export interface AppOptions {
  /** Close the window: each life detaches, and a life in a host of its own goes on, or each life ends when end is set. */
  quit(end?: boolean): void | Promise<void>;
  workspaces: Workspaces;
  /** Rings the bell of the terminal, which calls the operator to a question when the terminal can show no
   * notification. By default it writes BEL to the standard output. */
  bell?(): void;
}

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

/** An App on the session that the library selects, which a new App replaces for each session that the library
 * selects later. It gives the App that shows now. */
export function follow(renderer: CliRenderer, options: AppOptions): () => App {
  const session = options.workspaces.current?.session;
  if (!session) throw new Error("The library has no session selected.");
  let app = new App(renderer, session, options);
  const swap = (next: Session) => {
    app.dispose();
    app = new App(renderer, next, options);
  };
  options.workspaces.on("select", (next: Session) => {
    if (app.session !== next) swap(next);
  });
  // The mode of a screen reader shapes each part of the view, so a change of it draws a new view of the session.
  options.workspaces.on("access", () => swap(app.session));
  return () => app;
}

/** What the choice of the theme of the terminal says in a picker. */
const terminalDetail = `The colors of the terminal, or ${themeLabels[terminalThemes.dark][0]} on a dark terminal and ${themeLabels[terminalThemes.light][0]} on a light one when it does not say them`;
/** The commands whose value is a path of the project, which the suggestions wait for. */
const pathCommands = new Set(["read", "image", "cd"]);
/** What each effort of a model does, which the picker of the effort says beside it. */
const efforts: Record<string, string> = {
  off: "Answer with no thought first",
  minimal: "Think as little as the model can",
  low: "Quick answers to simple work",
  medium: "A balance of speed and depth",
  high: "More thought for hard work",
  xhigh: "Deep thought for the hardest work",
  max: "All the thought the model allows",
};
export class App {
  readonly root: BoxRenderable;
  readonly composer: TextareaRenderable;
  readonly scroll: ScrollBoxRenderable;
  private style: ReturnType<typeof syntax>;
  private theme: ThemeName = defaultTheme;
  private draftKey = "";
  private hover?: BoxRenderable;
  private questionDocument?: ScrollBoxRenderable;
  private hoverTimer?: ReturnType<typeof setTimeout>;
  private editorVersion = 0;
  private closed = false;
  /** The keys of the App, as layers of the keymap of its renderer, and what takes each layer away again. */
  private readonly keymap: Keymap<Renderable, KeyEvent>;
  private readonly layersOff: (() => void)[];
  /** The sidebar at the right: the session, its chains and its usage, then the workspaces, which scroll. */
  private readonly rail: BoxRenderable;
  private readonly railSession: BoxRenderable;
  private readonly railHeading: BoxRenderable;
  private readonly railSpaces: ScrollBoxRenderable;
  private readonly railUsage: BoxRenderable;
  /** What the sidebar of the session drew last, which a frame that would draw the same leaves as it is. */
  private railKey = "";
  /** The program of each thread and each word of the operator, as it stood at a version of the acts and of the words
   * that its models write. */
  private programs = new Map<string, { version: number; live: string; lines: ProgramLine[] }>();
  /** The chain whose threads the sidebar lists in place of the tree of the chains, once a click or Enter chose it. */
  private railChain?: string;
  /** Whether the keys move in the sidebar, after ⌥T, and the row that they stand on: a chain in the tree, or a thread in
   * the list of a chain. */
  private railKeys = false;
  private railAt?: string;
  /** The ids of the chains of the tree, in the order that the sidebar shows them, which the keys move through. */
  private railRows: string[] = [];
  /** The first row of the list of the threads that the sidebar shows, which the wheel and the keys move. */
  private railFrom = 0;
  /** The threads of the chain shown, read again only when the acts or the questions that wait change. */
  private threadList: ThreadList = { acts: undefined, owed: "", list: [], by: new Map(), of: new Map() };
  /** The lists of the sidebar that the operator unfolded: the finished chains, under the key finished, and the
   * archived sessions of a workspace, under its folder. */
  private readonly unfolded = new Set<string>();
  /** The session or the workspace that the operator renames in its row, and the name typed so far. */
  private renaming?: {
    item: SessionEntry | Workspace;
    value: string;
    cursor?: number;
    input?: InputRenderable;
  };
  /** The session or the workspace whose menu is open, whose row stays lit under it. */
  private menuItem?: SessionEntry | Workspace;
  /** The switch of the mode of the input, which the layout places. */
  private readonly modeBox: BoxRenderable;
  /** Where the operator is, at the left of the top line, and the toggle of the views at its right. */
  private readonly headline: BoxRenderable;
  private readonly toggle: BoxRenderable;
  private readonly status: TextRenderable;
  /** The keys that the footer offers for what the operator can do now, each a button. */
  private readonly hints: BoxRenderable;
  /** The mode, the model, the effort, and the shape of the next message, under the input, each a button. */
  private readonly meta: BoxRenderable;
  /** The rows of half blocks above and below the composer, and its bar, which the mode colors. */
  private readonly composeEdges: BoxRenderable[];
  private readonly composeBox: BoxRenderable;
  /** The values of a question of a choice, which the box of the prompt offers in place of its input. */
  private readonly pickBox: BoxRenderable;
  /** The question that the list offers the values of, the row of its pointer, and whether Other takes the input. */
  private choice = { id: "", at: 0, other: false };
  private pickKey = "";
  private readonly queueBox: BoxRenderable;
  private readonly imageBox: BoxRenderable;
  /** The suggestions for a `/command` or an `@path` typed in the input, drawn above it while the input keeps focus. */
  private readonly suggestionBox: BoxRenderable;
  private suggestions: Suggestion[] = [];
  private suggestionIndex = 0;
  /** The token whose suggestions Escape hid, which a change of the token shows again. */
  private dismissed = "";
  private lastToken = "";
  private readonly search: InputRenderable;
  private readonly searchRow: BoxRenderable;
  /** The bar above the feed while the rewind tree is open, which says what a choice does. */
  private readonly treeBar: BoxRenderable;
  /** The view of the life that shows over the feed: the loom or the graph, drawn as rows of text. */
  private readonly stage: TextRenderable;
  /** The loom and the graph, each with its own selection, folds and level, which a switch of the view keeps. */
  readonly loom = new LoomPane();
  readonly graph: Graph;
  /** Whether the keys move on the loom or the graph, in place of the input. */
  steering = false;
  /** The moment that the loom, the graph and the feed show, which is the moment of the session, and nothing at the
   * present: one cursor in time for every view. */
  get cursor(): number | undefined {
    return this.session.moment;
  }
  set cursor(at: number | undefined) {
    this.session.moment = at;
  }
  /** The chains that a step lets run for one rung, with the count of their rungs that had ended when it began. */
  private readonly stepping = new Map<string, number>();
  /** What waits for the operator, over the input, and the key of what it shows. */
  private readonly deskBox: BoxRenderable;
  private deskKey = "";
  /** The parts of the extensions for the TUI, which add panels, rows, keys and commands. */
  readonly parts: Parts;
  /** The panels of the parts over the input, the key of what it shows, and what takes the keys of the parts away. */
  private readonly dock: BoxRenderable;
  private dockKey = "";
  private partKeysOff?: () => void;
  /** Whether a part drew the rows of a kind at the last load. */
  private partRows = false;
  /** The ask that the answer dialog shows, which Tab moves from. */
  private answering?: string;
  /** The rewind tree, which the feed shows in place of its cards while it is open: its rows, and the row that the
   * pointer stands on. */
  private tree?: { rows: TreeRow[]; selected: string; title: string };
  private readonly treeFolds = new Map<string, boolean>();
  /** When the last Escape that had nothing to close was pressed. */
  private escapedAt = 0;
  private readonly paneKeys = new WeakMap<Renderable, string>();
  /** The node that the last press of a button reached. */
  private pressed: Renderable | null = null;
  /** Where the last press was, so that a click still counts when the view drew its node again between the press and
   * the release. */
  private pressedAt?: { x: number; y: number };
  private readonly cards = new Map<
    string,
    {
      node: BoxRenderable;
      heading: TextRenderable;
      /** The line of the heading, set again only when it changes. */
      label: (string | number | undefined)[];
      title?: () => Part[];
      marker: Part[];
      key: string;
      compact: boolean;
      collapsible: boolean;
      state: string;
      closed: boolean;
      /** The act that the card shows. */
      act?: string;
      /** What the card of a done act is made of, which a later render compares to make it again only on a change. */
      same?: readonly unknown[];
    }
  >();
  /** Where the view scrolls to, or the card it brings into view, once the scroll box has laid out its cards. */
  private scrollTarget?: Scroll | { card: string };
  private overlay?: BoxRenderable;
  /** The veil behind the dialog that the overlay holds. */
  private backdrop?: BoxRenderable;
  private paletteInput?: InputRenderable;
  /** The row of the filter of the palette, with its prompt. */
  private paletteInputRow?: BoxRenderable;
  private paletteWidth = 80;
  /** The rows of the note under the title of the palette. */
  private paletteNote = 0;
  private paletteList?: BoxRenderable;
  private filtered: Choice[] = [];
  private selection = 0;
  /** The first choice that the list shows, and whether each choice takes rows for its command and its detail. */
  private paletteStart = 0;
  private rich = false;
  private redraw?: ReturnType<typeof setTimeout>;
  private lastView = "";
  /** Whether this run is the first one, whose welcome shows the check of the setup. */
  private readonly firstRun: boolean;
  private submitting = false;
  private historyIndex = -1;
  private historyDraft = "";
  private diagnosticsKey = "";
  /** What the footer shows, set again only when it changes. */
  private statusKey = "";
  /** Whether the footer shows a spinner, which the tick turns. */
  private statusMoves = false;
  /** The whole text of the footer, which a notice cut to its room shows in a tip. */
  private statusWhole = "";
  private readonly tick: ReturnType<typeof setInterval>;
  /** Whether the view serves a screen reader, as the preferences said when the view was made. */
  private readonly reader: boolean;
  /** Whether the terminal has the focus, as it said last, or null while it said nothing. */
  private focused: boolean | null = null;
  /** The questions to the operator that the view saw, so that it calls him once for each new one. */
  private readonly called = new Set<string>();
  private readonly navigation: {
    chain: string;
    view: View;
    search: string;
    place: Scroll;
    mode: "prompt" | "python";
  }[] = [];

  constructor(
    readonly renderer: CliRenderer,
    readonly session: Session,
    readonly options: AppOptions,
  ) {
    // The theme that follows the terminal reads the mode that the terminal said, and each later change of it.
    session.preferences.terminalMode = renderer.themeMode ?? session.preferences.terminalMode;
    this.reader = session.preferences.screenReader;
    motion.still = session.preferences.still;
    setTheme(session.theme);
    this.theme = session.theme;
    this.firstRun = !session.preferences.setupSeen;
    this.style = syntax();
    this.root = this.box({
      id: "furb",
      width: "100%",
      height: "100%",
      flexDirection: "row",
      backgroundColor: c.background,
      // Every press reaches the root, which keeps what it pressed for the release that makes it a click.
      onMouseDown: (event) => {
        this.pressed = event.target;
        this.pressedAt = { x: event.x, y: event.y };
      },
    });
    renderer.root.add(this.root);
    const center = this.box({
      flexGrow: 1,
      flexShrink: 1,
      minHeight: 0,
      minWidth: 0,
      paddingX: space.gutter,
      gap: space.stack,
    });
    this.root.add(center);
    // The top line says where the operator is at its left, and holds the toggle of the views at its right.
    const top = this.row({ gap: space.between });
    this.headline = this.row({ flexGrow: 1, flexShrink: 1, minWidth: 0, overflow: "hidden" });
    top.add(this.headline);
    this.toggle = this.row({ id: "views" });
    top.add(this.toggle);
    center.add(top);
    // The filter of the view is a bar of the panel, with its title, the prompt of every filter, and the key that
    // leaves it. The bar of the rewind tree stands in the same place.
    const bar = (options: BoxOptions = {}) =>
      this.row({
        visible: false,
        marginTop: space.section,
        paddingX: space.inset,
        backgroundColor: c.panel,
        ...options,
      });
    this.searchRow = bar();
    this.searchRow.add(
      this.text(
        [
          ["Filter  ", c.text, bold],
          [`${glyph.prompt} `, c.accent],
        ],
        c.text,
      ),
    );
    this.search = new InputRenderable(renderer, {
      id: "search",
      flexGrow: 1,
      placeholder: "Filter this view",
      ...inputColors(c.panel),
    });
    this.search.on(InputRenderableEvents.INPUT, (value: string) => {
      session.search = value;
      this.renderContent();
    });
    this.searchRow.add(this.search);
    this.searchRow.add(this.text("Esc", c.faint, { run: () => this.closeSearch() }));
    center.add(this.searchRow);
    this.treeBar = bar({ id: "rewind-bar" });
    center.add(this.treeBar);
    this.stage = new TextRenderable(renderer, {
      id: "life-stage",
      content: "",
      fg: c.text,
      wrapMode: "none",
      flexShrink: 0,
      visible: false,
      marginTop: space.section,
      onMouseUp: (event) => this.clickStage(event.y - this.stage.y),
    });
    center.add(this.stage);
    this.graph = new Graph(this.stageOf());
    this.scroll = new ScrollBoxRenderable(renderer, {
      id: "timeline",
      flexGrow: 1,
      flexShrink: 1,
      minHeight: 0,
      scrollX: false,
      stickyScroll: true,
      stickyStart: "bottom",
      onSizeChange: this.schedule,
      // The feed keeps a row of space under the top line, however far it scrolls.
      marginTop: space.section,
      contentOptions: { gap: space.stack, paddingBottom: space.section },
    });
    // The ScrollBar constructor resets manual visibility; set it after construction.
    this.scroll.verticalScrollBar.visible = false;
    this.scroll.horizontalScrollBar.visible = false;
    center.add(this.scroll);
    this.dock = this.box({
      id: "parts",
      visible: false,
      marginTop: space.section,
      paddingX: space.inset,
      paddingY: space.inset,
      gap: space.stack,
      backgroundColor: c.panel,
    });
    center.add(this.dock);
    this.deskBox = this.box({
      id: "desk",
      visible: false,
      marginTop: space.section,
      paddingX: space.inset,
      paddingY: space.inset,
      backgroundColor: c.panel,
    });
    center.add(this.deskBox);
    this.queueBox = this.row({
      id: "queued-follow-ups",
      visible: false,
      onMouseUp: this.click(() => this.queuePicker()),
    });
    center.add(this.queueBox);
    this.imageBox = this.row({ id: "attached-images", gap: space.between, visible: false });
    center.add(this.imageBox);
    this.suggestionBox = this.box({ id: "suggestions", visible: false, backgroundColor: c.raised });
    center.add(this.suggestionBox);
    this.modeBox = this.row();
    // The composer is a panel, whose three rows the mode colors.
    this.composeBox = this.panel(center, c.accent, { id: "composer-box" });
    this.composeEdges = this.reader ? [] : (center.getChildren().slice(-3) as BoxRenderable[]);
    this.composer = new TextareaRenderable(renderer, {
      id: "composer",
      flexGrow: 1,
      minHeight: 1,
      placeholder: "What would you like to build?",
      ...inputColors(c.panel),
      keyBindings: [
        { name: "return", action: "submit" },
        { name: "return", shift: true, action: "newline" },
        { name: "j", ctrl: true, action: "newline" },
      ],
      syntaxStyle: this.style,
      onContentChange: () => {
        this.schedule();
        void this.highlightEditor();
        this.suggest();
      },
      onCursorChange: () => {
        void this.highlightEditor();
        this.suggest();
      },
      onSubmit: () => {
        void this.submit();
      },
    });
    this.pickBox = this.box({ id: "choices", visible: false });
    this.composeBox.add(this.pickBox);
    this.composeBox.add(this.composer);
    this.meta = this.row({ overflow: "hidden", flexGrow: 1 });
    // The line under the text holds the switch of the mode and where the input goes, a row of space under the text.
    const metaLine = this.row({ marginTop: space.section });
    metaLine.add(this.modeBox);
    metaLine.add(this.text("   ", c.faint));
    metaLine.add(this.meta);
    this.composeBox.add(metaLine);
    const footer = this.row({ gap: space.between });
    this.status = this.whole(
      this.line("", c.muted, { truncate: true, flexGrow: 1, flexShrink: 1 }),
      () => this.statusWhole,
    );
    footer.add(this.status);
    this.hints = this.row();
    footer.add(this.hints);
    center.add(footer);
    this.rail = this.box({
      id: "sidebar",
      width: session.preferences.sidebarWidth,
      backgroundColor: c.panel,
      overflow: "hidden",
    });
    this.railSession = this.box({ paddingX: space.between, gap: space.stack });
    this.rail.add(this.railSession);
    this.railHeading = this.row({ marginTop: space.section, paddingX: space.between });
    this.rail.add(this.railHeading);
    this.railSpaces = new ScrollBoxRenderable(renderer, {
      id: "workspaces",
      flexGrow: 1,
      flexShrink: 1,
      minHeight: 0,
      scrollX: false,
      backgroundColor: c.panel,
      contentOptions: { paddingBottom: space.section },
    });
    this.railSpaces.verticalScrollBar.visible = false;
    this.railSpaces.horizontalScrollBar.visible = false;
    this.rail.add(this.railSpaces);
    // The usage of the chain stands at the foot of the sidebar, under a rule, where the list above it does not move it.
    this.railUsage = this.box({
      paddingX: space.between,
      paddingBottom: space.inset,
      border: ["top"],
      borderColor: c.border,
      customBorderChars: { ...noBorder, horizontal: glyph.rule },
      visible: false,
    });
    this.rail.add(this.railUsage);
    this.root.add(this.rail);
    session.on("change", this.schedule);
    options.workspaces.on("change", this.schedule);
    session.on("compose", this.compose);
    session.on("resume", this.resume);
    session.on("shared", this.shared);
    session.on("confirm", this.confirm);
    session.on("authentication", this.authentication);
    session.on("codexModels", this.codexModels);
    this.keymap = keymapOf(renderer);
    this.layersOff = this.layers();
    this.parts = new Parts(session.host.directory, this.partsChanged);
    void this.parts.load();
    renderer.on("resize", this.render);
    renderer.on("selection", this.copySelection);
    renderer.on("theme_mode", this.modeChanged);
    renderer.setFrameCallback(this.drawDue);
    renderer.on("focus", this.focusTaken);
    renderer.on("blur", this.focusLost);
    // A renderer that ends on its own, as on a signal, ends the App before it frees the buffers of the view.
    renderer.on("destroy", this.dispose);
    // A label that moves with time is read again, and no other part of the view is drawn again.
    this.tick = setInterval(() => {
      for (const card of this.cards.values())
        if (card.title) this.label(card, [...card.marker, ...card.title()]);
      if (this.statusMoves) this.renderStatus();
      if (this.stage.visible && this.cursor === undefined && this.moves()) this.renderStage();
    }, frame);
    this.render();
    this.composer.focus();
    void loadParsers()
      .then(() => {
        if (this.closed) return;
        // Code drawn before the parsers came is highlighted again where it stands, and no card is made again.
        const highlight = (node: Renderable) => {
          if (node instanceof CodeRenderable && node.filetype) {
            const filetype = node.filetype;
            node.filetype = undefined;
            node.filetype = filetype;
          }
          for (const child of node.getChildren()) highlight(child);
        };
        highlight(this.root);
        void this.highlightEditor();
      })
      .catch(session.fail);
    if (session.host.pending.size) this.resume();
    if (session.host.reading && !session.readingTold) this.readOrRenew();
  }

  /** The dialog of a record that a life refused for a drift, as a record that a build of another version made, which
   * opens to read: it names both versions, or both engines for a record of a build before the versions, and offers
   * to read the record, or to start a new life in its workspace. */
  private readOrRenew(): void {
    const reading = this.session.host.reading;
    if (!reading) return;
    this.session.readingTold = true;
    const ours = reading.engine.slice(0, 12);
    const same =
      reading.version === null ? reading.made === reading.engine : reading.version === reading.current;
    const theirs =
      reading.version !== null
        ? `a build of version ${reading.version}`
        : reading.made
          ? `the engine ${reading.made.slice(0, 12)}`
          : "an earlier engine, which the record does not name";
    const build = reading.version !== null ? `is of version ${reading.current}` : `runs the engine ${ours}`;
    const note = same
      ? `This build ${build}, which made it, and its replay drifts: ${clip(reading.why, 240)}`
      : `It was made by ${theirs}, and this build ${build}. A replay would drift, so it opens to read.`;
    const library = this.options.workspaces;
    this.openPalette(
      same ? "This record does not replay" : "This record was made by another build",
      [
        { label: "Read it", detail: "See what it kept. No word runs again.", run: () => {} },
        {
          label: "Start a new life from it",
          detail: "A new session in its workspace",
          run: () => library.create().then(() => {}, this.session.fail),
        },
      ],
      { note },
    );
  }

  private box(options: BoxOptions = {}): BoxRenderable {
    // The mode of a screen reader draws no border, since a reader reads each glyph of a border aloud. A box with no
    // border takes no color and no glyph of a border, since either of them draws a whole border.
    const { border, borderColor, borderStyle, customBorderChars, ...plain } = options;
    return new BoxRenderable(this.renderer, {
      flexDirection: "column",
      flexShrink: 0,
      ...(this.reader ? plain : { border, borderColor, borderStyle, customBorderChars, ...plain }),
    });
  }
  /** A box that lays out what it holds in a row one bar high. */
  private row(options: BoxOptions = {}): BoxRenderable {
    return this.box({ flexDirection: "row", height: space.bar, ...options });
  }
  /** A handler of the release of the left button that runs only for a click: a press and a release on one node that
   * selected no text between them. A drag over a button selects its text and does nothing else, and the release of
   * a press that opened a dialog does not close it. A node that the view drew again between the press and the
   * release is the same node when the pointer did not move. */
  private click(run: (event: MouseEvent) => void): (event: MouseEvent) => void {
    return (event) => {
      const redrawn =
        Boolean(this.pressed?.isDestroyed) && event.x === this.pressedAt?.x && event.y === this.pressedAt?.y;
      if (
        event.button !== 0 ||
        (event.target !== this.pressed && !redrawn) ||
        this.renderer.getSelection()?.getSelectedText()
      )
        return;
      run(event);
    };
  }
  /** A node whose background takes a color while the pointer is on it, and gives it back when the pointer leaves. */
  private hoverable<T extends BoxRenderable | TextRenderable>(node: T, color = c.raised): T {
    // A box has a background, and a text the color behind its cells.
    const property = node instanceof BoxRenderable ? "backgroundColor" : "bg";
    const rest: unknown = Reflect.get(node, property);
    node.onMouseOver = () => {
      Reflect.set(node, property, color);
    };
    node.onMouseOut = () => {
      Reflect.set(node, property, rest);
    };
    return node;
  }
  /** The mode of the terminal changed, so the theme that follows it changes too. */
  private modeChanged = (mode: "dark" | "light"): void => {
    this.session.preferences.terminalMode = mode;
    this.render();
  };
  /** The terminal took the focus, as it says. */
  private focusTaken = (): void => {
    this.focused = true;
  };
  /** The terminal lost the focus, as it says. */
  private focusLost = (): void => {
    this.focused = false;
  };
  /** Each new question to the operator calls him while the terminal has no focus: a notification of the terminal, as
   * OSC 9 shows it, or the bell where the terminal shows none. A question that waited when the view was made calls
   * nobody. */
  private call(asks: readonly { id: string; chain: string; label: string; message: string }[]): void {
    const fresh = asks.filter((ask) => !this.called.has(ask.id));
    for (const ask of fresh) this.called.add(ask.id);
    const first = fresh[0];
    if (!first || this.focused !== false || this.closed) return;
    const text = `${first.label || first.chain} asks: ${first.message}`.replace(/\s+/g, " ").slice(0, 200);
    if (this.renderer.triggerNotification(text, "furb")) return;
    if (this.options.bell) this.options.bell();
    else process.stdout.write("\x07");
  }
  /** The mode of a screen reader or reduced motion, turned on or off, which every session keeps. A change of the mode
   * of a screen reader draws a new view of the session. */
  private toggleAccess(which: "reader" | "motion"): void {
    const preferences = this.session.preferences;
    if (which === "reader") preferences.screenReader = !preferences.screenReader;
    else preferences.reducedMotion = !preferences.reducedMotion;
    preferences.save();
    const on = which === "reader" ? preferences.screenReader : preferences.reducedMotion;
    this.session.notice = `${which === "reader" ? "Screen reader mode" : "Reduced motion"} is ${on ? "on" : "off"}.`;
    if (which === "reader") this.options.workspaces.emit("access");
    else this.render();
  }
  /** The choices of access: the mode of a screen reader, and reduced motion. */
  private accessPicker = (): void => {
    const preferences = this.session.preferences;
    this.openPalette(
      "Access",
      [
        {
          label: "Screen reader",
          detail: "One column of text, with no box, no spinner, and nothing that moves",
          mark: this.current(preferences.screenReader),
          command: "/access reader",
          run: () => this.toggleAccess("reader"),
        },
        {
          label: "Reduced motion",
          detail: "No spinner turns, and no part of the view moves with time",
          mark: this.current(preferences.reducedMotion),
          command: "/access motion",
          run: () => this.toggleAccess("motion"),
        },
      ],
      {
        note: colorless.on
          ? "NO_COLOR is set, so the view shows the colors of the terminal alone."
          : "Every session shares these choices. NO_COLOR turns every color off.",
      },
    );
  };
  /** The text that a drag selected goes to the clipboard as the drag ends. */
  private copySelection = (selection: { getSelectedText(): string } | null): void => {
    const text = selection?.getSelectedText() ?? "";
    if (this.closed || !text.trim()) return;
    this.renderer.copyToClipboardOSC52(text);
    const length = [...graphemes.segment(text)].length;
    this.session.notice = `Copied ${length} ${length === 1 ? "character" : "characters"}.`;
  };
  /** A text, which runs an action at a click when it has one. */
  private text(content: string | Part[], fg = c.text, { run, ...options }: TextShape = {}): TextRenderable {
    const node = new TextRenderable(this.renderer, {
      content: typeof content === "string" ? safeText(content) : styled(content),
      fg,
      // A text truncates only on a line it does not wrap, so a text that truncates keeps one line with an ellipsis.
      wrapMode: options.truncate ? "none" : "word",
      flexShrink: 0,
      onMouseUp: run && this.click(run),
      ...options,
    });
    return options.truncate && !options.onMouseOver ? this.whole(node) : node;
  }
  /** A text one bar high. */
  private line(content: string | Part[], fg = c.text, options: TextShape = {}): TextRenderable {
    return this.text(content, fg, { height: space.bar, ...options });
  }
  /** A text cut to its room shows the whole of it in a tip while the pointer is over it: the text that the terminal
   * cut, or the whole that a caller gives for a text that it cut itself. The tip takes the hover handlers of the text,
   * since OpenTUI keeps one of each and gives none back. */
  private whole(node: TextRenderable, full?: () => string): TextRenderable {
    node.onMouseOver = (event) => {
      // A drag selects text, and no tip comes between the drag and what it selects.
      if (event.isDragging) return;
      const text = full ? full() : node.plainText;
      const cut = full ? text !== node.plainText : Bun.stringWidth(text) > node.width;
      if (cut && text) this.tip([[text, c.text]], event.x, event.y);
    };
    node.onMouseOut = () => {
      this.unhover();
    };
    return node;
  }
  /** The columns of the feed: the screen but the sidebar that shows and the gutters. It is known before the feed is
   * laid out. */
  private get feedWidth(): number {
    return (
      this.renderer.width - (this.rail.visible ? this.session.preferences.sidebarWidth : 0) - space.gutter * 2
    );
  }
  /** A node that stands a number of columns in from the left, since a text leaves its own padding out when it draws. */
  private inset(left: number, node: Renderable): BoxRenderable {
    const box = this.box({ paddingLeft: left, flexDirection: "row" });
    box.add(node);
    return box;
  }
  private paneChanged(node: Renderable, state: unknown): boolean {
    const key = JSON.stringify(state);
    if (this.paneKeys.get(node) === key) return false;
    this.paneKeys.set(node, key);
    return true;
  }
  private clear(node: Renderable): void {
    for (const child of [...node.getChildren()]) child.destroyRecursively();
  }
  /** The text of the composer kept as the draft it shows. An empty draft is none, so that a program to edit
   * opens from its door. */
  private keepDraft(): void {
    const text = this.composer.plainText;
    if (text) this.session.drafts[this.draftKey] = text;
    else delete this.session.drafts[this.draftKey];
  }
  /** The composer shows the draft of a key, and the draft it showed stays under its own key. */
  private showDraft(key: string, text = this.session.drafts[key] ?? ""): void {
    if (this.draftKey) this.keepDraft();
    this.draftKey = key;
    this.composer.setText(text);
    this.historyIndex = -1;
  }
  private compose = (text: string) => {
    this.closeTree();
    this.showDraft(this.session.draftKey, text);
    this.composer.focus();
  };
  /** A change of the session draws the view again once the changes of 35 ms are in, at the start of the next frame
   * of the renderer, so that the work of the view and the frame that shows it share the time of one frame. */
  private schedule = () => {
    if (this.closed) return;
    if (!this.redraw)
      this.redraw = setTimeout(() => {
        this.redraw = undefined;
        this.due = true;
        this.renderer.requestRender();
      }, 35);
  };
  /** Whether a change waits for the next frame to draw the view again. */
  private due = false;
  private drawDue = async (): Promise<void> => {
    if (!this.due || this.closed) return;
    this.due = false;
    this.render();
  };
  /** What the input sent on this chain in this mode. A chain that holds messages the input did not send, as a record
   * that opens with no saved input, walks those messages instead. */
  private history(): string[] {
    const w = this.session;
    const kept = w.histories[w.draftKey] ?? [];
    if (kept.length || w.mode !== "prompt" || w.editing) return kept;
    return w.activity
      .filter((act) => w.isUserPrompt(act))
      .map((act) => String(act.words[1] ?? ""))
      .filter(Boolean);
  }
  async submit(): Promise<void> {
    if (this.submitting) return;
    this.submitting = true;
    const key = this.draftKey;
    const content = this.composer.plainText;
    const python = this.session.mode === "python" && !this.session.editing && !content.startsWith("/");
    // The text leaves its draft as it is sent, so what is typed while it is sent stays in the composer.
    this.composer.setText("");
    try {
      const declined = this.picking();
      if (declined && this.choice.other && !content.startsWith("/")) {
        if (content.trim()) await this.session.decline(declined, content);
      } else if (!(await this.globalCommand(content.trim()))) {
        const woke = !content.startsWith("/") && (await this.session.wakeForInput());
        await this.session.submit(python ? `/run ${content}` : content);
        if (woke) this.session.notice = "This message woke the paused chain.";
      }
      const history = this.session.histories[key] ?? [];
      this.session.histories[key] = history;
      if (content && history.at(-1) !== content) history.push(content);
      if (history.length > 200) history.shift();
      this.historyIndex = -1;
    } catch (error) {
      // A text that was not sent goes back to its draft, unless that draft holds new text by now.
      if (this.draftKey === key && !this.composer.plainText) this.composer.setText(content);
      else if (this.draftKey !== key && !this.session.drafts[key]) this.session.drafts[key] = content;
      this.report(error);
    } finally {
      this.submitting = false;
      this.render();
    }
  }

  render = (): void => {
    if (this.closed) return;
    const w = this.session;
    this.parts.heard(w.written);
    motion.still = w.preferences.still;
    if (w.theme !== this.theme) this.applyTheme(w.theme);
    this.call(w.desk.asks);
    if (this.draftKey !== w.draftKey) this.showDraft(w.draftKey);
    // The mode of a screen reader shows one column of text, and no sidebar beside it.
    this.rail.visible = w.preferences.sidebar && !this.reader && this.renderer.width >= 100;
    this.rail.width = w.preferences.sidebarWidth;
    this.renderTop();
    const pending = w.operatorPrompt;
    const images = w.images[w.selected] ?? [];
    this.imageBox.visible = images.length > 0;
    if (this.paneChanged(this.imageBox, [images, this.theme])) {
      this.clear(this.imageBox);
      this.imageBox.add(
        this.line(
          [
            [`${glyph.chip} `, c.accent],
            [`${images.length} ${images.length === 1 ? "image" : "images"} attached  `, c.text],
            [
              images
                .map((image) => image.name)
                .join("  ")
                .replace(/\s+/g, " "),
              c.muted,
            ],
          ],
          c.muted,
          {
            truncate: true,
            flexShrink: 1,
            run: () =>
              this.openPalette(
                "Image attachments",
                images.map((image) => ({
                  label: image.name,
                  detail: `${image.mimeType}  ${kibibytes(image.size)}`,
                  run: () => this.imageActions(image.uri),
                })),
              ),
          },
        ),
      );
    }
    const queued = w.queued.filter((entry) => entry.chain === w.selected);
    this.queueBox.visible = w.queued.length > 0;
    if (this.paneChanged(this.queueBox, [w.queued, w.queueHeld, w.selected, this.theme])) {
      this.clear(this.queueBox);
      const first = (queued.at(-1) ?? w.queued.at(-1))?.text.split("\n")[0] ?? "";
      this.queueBox.add(
        this.line(
          [
            [`${w.queueHeld ? glyph.held : glyph.ring} `, w.queueHeld ? c.warning : c.faint],
            [
              `${w.queueHeld ? "Queue held" : "Queued"} ${w.queued.length}  `,
              w.queueHeld ? c.warning : c.text,
            ],
            [first, c.muted],
          ],
          c.muted,
          { truncate: true, flexGrow: 1, flexShrink: 1 },
        ),
      );
      this.queueBox.add(
        this.line(
          queued.length
            ? [
                ["↑", c.muted],
                [" takes the last back", c.faint],
              ]
            : [],
          c.faint,
        ),
      );
    }
    const choosing = this.picking();
    const [mode, modeColor, placeholder] = w.editing
      ? ["Edit program", c.warning, "Edit this prompt's Python program"]
      : pending
        ? [
            "Answer",
            c.warning,
            choosing
              ? `Your own words in place of ${wants(pending)}. Esc goes back to the values.`
              : `Your answer to ${pending.id}, as ${pending.shape}`,
          ]
        : w.mode === "python"
          ? ["Python", c.secondary, "Write Python. The gate reads it before it runs."]
          : ["Prompt", c.accent, "Ask anything, or type / for commands"];
    for (const edge of this.composeEdges) if (edge.borderColor !== modeColor) edge.borderColor = modeColor;
    this.composer.cursorColor = modeColor;
    this.composer.placeholder = placeholder;
    this.renderPick(choosing);
    // The box holds the values of a choice, or the lines of the text, up to six, then a row of space and the line
    // under the text. While Other takes the words of the operator, it holds both.
    const typed = Math.min(
      6,
      Math.max(space.bar, this.composer.lineCount, this.composer.lineInfo.lineSources.length),
    );
    const listed = choosing ? choosing.choices.length + 1 : 0;
    this.composeBox.height =
      (choosing && !this.choice.other ? listed : listed + typed) + space.section + space.bar;
    this.renderMeta(mode, modeColor);
    this.renderDock();
    this.renderDesk();
    this.renderStatus();
    this.renderContent();
    this.renderRail();
    this.suggest();
    const diagnostics = JSON.stringify([w.rejectedWord, w.findings]);
    if (diagnostics !== this.diagnosticsKey) {
      this.diagnosticsKey = diagnostics;
      void this.highlightEditor();
    }
  };

  /** A switch between a few choices, which the views and the mode of the input share. A track in a color of its own
   * holds a segment for each choice, two columns of space at each side of its label. The chosen segment is filled
   * with the color of its choice from edge to edge, the segment under the pointer lights in place, and a click chooses
   * a segment. The chord that moves the switch stands after the track. */
  private switcher(
    box: BoxRenderable,
    choices: { label: string; badge?: string; color: RGBA; run?: () => void }[],
    chosen: number,
    chord: string,
    track: RGBA,
  ): void {
    for (const [index, choice] of choices.entries()) {
      const active = index === chosen;
      const fill = active ? choice.color : track;
      const parts = (lit: boolean): Part[] => [
        [`  ${choice.label}`, active ? c.background : lit ? c.text : c.muted, active ? bold : 0, fill],
        [choice.badge ? ` ${choice.badge}` : "", active ? c.background : lit ? c.text : c.faint, 0, fill],
        ["  ", c.text, 0, fill],
      ];
      // The pointer recolors the segment that it is over, and builds no node, so that a press and its release land on
      // the same segment.
      const segment: TextRenderable = this.line(parts(false), c.text, {
        run: choice.run,
        ...(choice.run && !active
          ? {
              onMouseOver: () => {
                segment.content = styled(parts(true));
              },
              onMouseOut: () => {
                segment.content = styled(parts(false));
              },
            }
          : {}),
      });
      box.add(segment);
    }
    if (chord) box.add(this.line(`  ${chord}`, c.faint));
  }
  /** The top line: the session and the chain at its left, each a button, and the toggle of the views at its right. */
  private renderTop(): void {
    const w = this.session;
    const changes = w.host.changes;
    const shown = this.tree ? "feed" : w.view;
    const directory = shortenHome(w.workingDirectory);
    // The top line spends its room in this order: the session and the chain, the switch of the views, the key of the
    // switch, then the directory. A part that finds no room is left out, and a session name that is still too long is
    // cut at its end.
    const chord = `${this.kitty ? "⌃" : "⌥"}1-3`;
    const toggle = views.reduce(
      (sum, view) =>
        sum + viewLabels[view].length + 4 + (view === "changes" && changes ? String(changes).length + 1 : 0),
      0,
    );
    const head = Bun.stringWidth(w.sessionName) + 3 + Bun.stringWidth(w.label);
    const room = this.feedWidth - space.between;
    const hint = head + toggle + chord.length + 2 <= room;
    const folder = head + 3 + Bun.stringWidth(directory) + toggle + (hint ? chord.length + 2 : 0) <= room;
    const name = clip(w.sessionName, Math.max(8, room - toggle - 3 - Bun.stringWidth(w.label)));
    if (this.paneChanged(this.toggle, [shown, changes, this.kitty, hint, this.theme])) {
      this.clear(this.toggle);
      this.switcher(
        this.toggle,
        views.map((view) => ({
          label: viewLabels[view],
          badge: view === "changes" && changes ? String(changes) : "",
          color: c.accent,
          run: () => this.showView(view),
        })),
        views.indexOf(shown),
        // F1 lists the chord where the top line has no room for it.
        hint ? chord : "",
        c.panel,
      );
    }
    if (this.paneChanged(this.headline, [name, w.label, folder && directory, this.theme])) {
      this.clear(this.headline);
      this.headline.add(
        this.line([[name, c.text, bold]], c.text, { run: () => void this.workspacePicker() }),
      );
      this.headline.add(this.line(` ${glyph.crumb} `, c.faint));
      this.headline.add(this.line(w.label, c.muted, { run: () => this.chains() }));
      if (folder)
        this.headline.add(
          this.line(`   ${directory}`, c.faint, {
            truncate: true,
            flexShrink: 1,
            run: () => this.showValue("Directory", w.workingDirectory),
          }),
        );
    }
  }
  /** Where the input goes, under it: its mode, its chain, and for a prompt the model, the effort, and the type of its
   * answer. Each part is a button that changes it. */
  private renderMeta(mode: string, modeColor: RGBA): void {
    const w = this.session;
    const pending = w.operatorPrompt;
    const { model, effort } = w.actorChoice;
    const { provider, id: name } = modelName(model);
    const stash = w.stashes[w.draftKey];
    const prompt = w.mode === "prompt" && !w.editing && !pending;
    // The line spans the input but its bar and its padding.
    const room = this.feedWidth - 3;
    if (
      !this.paneChanged(this.meta, [
        mode,
        w.label,
        name,
        provider,
        effort,
        w.shape,
        pending?.id,
        pending?.shape,
        stash,
        prompt,
        room,
        this.theme,
      ])
    )
      return;
    this.clear(this.meta);
    // A narrow input leaves out, in this order, the provider, the type of the answer, the effort, and the chord of the
    // switch, and the stash shows less of what waits in it.
    const edited = Boolean(w.editing || pending);
    const shown = { chord: !edited, provider: prompt && Boolean(provider), shape: prompt, effort: prompt };
    const gaps = 3;
    const width = () =>
      (edited ? Bun.stringWidth(mode) + 4 : "Prompt".length + "Python".length + 8) +
      gaps +
      (shown.chord ? 4 : 0) +
      Bun.stringWidth(`on ${w.label}`) +
      (pending ? 2 * gaps + Bun.stringWidth(`answers ${pending.id}returns ${pending.shape}`) : 0) +
      (prompt ? gaps + Bun.stringWidth(name) : 0) +
      (shown.provider ? provider.length + 1 : 0) +
      (shown.effort ? gaps + Bun.stringWidth(`${effort} effort`) : 0) +
      (shown.shape ? gaps + Bun.stringWidth(`returns ${w.shape}`) : 0) +
      // The stash takes a gap, its word, its quotes and their space, and its key, and its preview takes the rest.
      (stash ? gaps + "stashed “”  ⌃S".length : 0);
    for (const part of ["provider", "shape", "effort", "chord"] as const)
      if (width() > room) shown[part] = false;
    const gap = () => this.meta.add(this.text("   ", c.faint));
    const button = (parts: Part[], run?: () => void) => this.meta.add(this.line(parts, c.muted, { run }));
    // The mode of the input is a switch between a prompt and Python, and an answer or a program that the input edits
    // is a switch of one segment, which only its key leaves.
    this.clear(this.modeBox);
    if (edited) this.switcher(this.modeBox, [{ label: mode, color: modeColor }], 0, "", c.raised);
    else
      this.switcher(
        this.modeBox,
        [
          { label: "Prompt", color: c.accent, run: () => w.mode === "prompt" || this.toggleMode() },
          { label: "Python", color: c.secondary, run: () => w.mode === "python" || this.toggleMode() },
        ],
        w.mode === "python" ? 1 : 0,
        shown.chord ? "⌃R" : "",
        c.raised,
      );
    button(
      [
        ["on ", c.faint],
        [w.label, c.muted],
      ],
      () => this.chains(),
    );
    if (pending) {
      gap();
      button(
        [
          ["answers ", c.faint],
          [pending.id, c.muted],
        ],
        () => this.question(pending),
      );
      gap();
      button([
        ["returns ", c.faint],
        [pending.shape, c.muted],
      ]);
    } else if (prompt) {
      gap();
      button(
        [
          [name, c.text],
          [shown.provider ? ` ${provider}` : "", c.faint],
        ],
        () => this.models(),
      );
      if (shown.effort) {
        gap();
        button(
          [
            [effort, c.muted],
            [" effort", c.faint],
          ],
          () => this.effortPicker(),
        );
      }
      if (shown.shape) {
        gap();
        button(
          [
            ["returns ", c.faint],
            [w.shape, c.muted],
          ],
          this.shapes,
        );
      }
    }
    if (stash) {
      this.meta.add(this.box({ flexGrow: 1 }));
      // The stash shows the start of what waits in it, as much as the line has room for, and the key that brings it
      // back.
      const first = stash.split("\n")[0] ?? "";
      const preview = Math.min(28, room - width());
      const more = first.length < stash.length && Bun.stringWidth(first) <= preview ? " …" : "";
      button(
        [
          ["stashed ", c.faint],
          [preview >= 6 ? `“${clip(first, preview)}${more}”  ` : "", c.muted],
          ["⌃S", c.faint],
        ],
        () => this.stash(),
      );
    }
  }

  /** The state of the session and the keys that act on it, in the footer. A notice stands after the state until it
   * ends, and each key is a button that does what it says. */
  /** The tokens of the chain shown, which the tip of the footer says whole: the whole prompt of its last answer, and
   * what every answer read past the cache, wrote, and read and wrote in the cache, each token counted once. */
  private tokens(): string {
    const { spend, context } = this.session;
    return [
      context === undefined ? "" : `last prompt ${count(context)}`,
      spend.input ? `fresh input ${count(spend.input)}` : "",
      spend.output ? `output ${count(spend.output)}` : "",
      spend.cacheRead ? `cache read ${count(spend.cacheRead)}` : "",
      spend.cacheWrite ? `cache write ${count(spend.cacheWrite)}` : "",
    ]
      .filter(Boolean)
      .join(", ");
  }
  private renderStatus(): void {
    const w = this.session;
    // A refresh follows each fact, so the footer says Loading only while the chain has nothing to show yet.
    const loading = w.loading && !w.turns.length;
    const status = loading ? "opening" : w.status(w.selected);
    // A call of a model that waits to be made again counts its wait down in the footer, on the selected chain first.
    const retries = [...w.host.streams.values()].filter((stream) => stream.retry);
    const retry = loading
      ? undefined
      : (retries.find((stream) => stream.chain === w.selected) ?? retries[0])?.retry;
    const moving = status === "working" || status === "opening" || Boolean(retry);
    this.statusMoves = moving;
    const [mark, color] = retry ? [`${glyph.held} `, c.warning] : statusMark(status);
    const label = loading
      ? "Loading"
      : retry
        ? retryLine(retry)
        : status === "error"
          ? w.damagedJournal
            ? "The journal of file changes is damaged"
            : w.error
              ? "The view could not load"
              : "The last act failed"
          : statusLabels[status];
    // The footer offers the keys of the top layer that offers any, each of which does what its key does, and under
    // the layer of the table, the wake of a paused chain.
    type Offer = readonly [chord: string, action: string, run: () => void];
    const text = (value: string | (() => string)) => (typeof value === "function" ? value() : value);
    const offered = this.keymap
      .getActiveKeys({ includeMetadata: true })
      .flatMap(({ bindingAttrs, command }) => {
        const hint = bindingAttrs?.hint as readonly [...Hint, group: string] | undefined;
        return hint && typeof command === "string" ? [{ hint, command }] : [];
      });
    const group = offered[0]?.hint[2];
    const keys: readonly Offer[] = [
      ...(group === "" && w.paused
        ? ([["/wake", "wake the chain", () => this.action("/wake")]] as const)
        : []),
      ...offered
        .filter(({ hint }) => hint[2] === group)
        .map(
          ({ hint: [chord, action], command }): Offer => [
            text(chord),
            text(action),
            () => void this.keymap.dispatchCommand(command),
          ],
        ),
    ];
    const buttons = keys.map(([chord, action], index): Part[] => [
      [index ? "   " : "", c.faint],
      [chord, c.muted],
      [` ${action}`, c.faint],
    ]);
    const hints = buttons.flat();
    const notice = w.error ? "" : w.notice;
    // The state stays in the footer, and a notice stands after it, cut at its end where the footer has no room for it
    // beside the keys.
    const state: Part[] = [
      [moving && !retry ? `${spin()} ` : mark, color],
      [label, moving ? c.text : c.muted],
    ];
    const room =
      this.feedWidth - Bun.stringWidth(plain(hints)) - Bun.stringWidth(plain(state)) - space.between * 2;
    const tokens = this.tokens();
    this.statusWhole = [plain(state), notice, tokens && `Tokens: ${tokens}.`].filter(Boolean).join("   ");
    if (notice && room >= 8)
      state.push(
        ["   ", c.text],
        [clip(notice, room - 3), [exitNotice, rewindNotice].includes(notice) ? c.warning : c.text],
      );
    const key = JSON.stringify([plain(state), plain(hints), color, this.theme]);
    if (key === this.statusKey) return;
    this.statusKey = key;
    this.status.content = styled(state);
    this.clear(this.hints);
    for (const [index, [, , run]] of keys.entries())
      this.hints.add(this.line(buttons[index] ?? [], c.faint, { run }));
  }

  /** The heading of a card set to a line, only when the line changed. A frame reads each card, so the line is held
   * as a flat list of its texts, the numbers of its colors and its attributes, which it compares with no copy. */
  private label(
    card: { heading: TextRenderable; label: (string | number | undefined)[] },
    parts: Part[],
  ): void {
    const held = card.label;
    let at = 0;
    const same = (value: string | number | undefined) => held[at++] === value;
    const color = (one?: RGBA) =>
      one ? same(one.r) && same(one.g) && same(one.b) && same(one.a) : same(undefined);
    if (
      held.length === parts.reduce((count, [, fg, , bg]) => count + 2 + (fg ? 4 : 1) + (bg ? 4 : 1), 0) &&
      parts.every(([text, fg, attributes, bg]) => same(text) && color(fg) && same(attributes) && color(bg))
    )
      return;
    card.label = parts.flatMap(([text, fg, attributes, bg]) => [
      text,
      ...(fg ? [fg.r, fg.g, fg.b, fg.a] : [undefined]),
      attributes,
      ...(bg ? [bg.r, bg.g, bg.b, bg.a] : [undefined]),
    ]);
    card.heading.content = styled(parts);
  }
  private card(
    id: string,
    key: string,
    label: Part[],
    body: (box: BoxRenderable) => void,
    index: number,
    options: BlockOptions = {},
  ): void {
    const rung = options.act?.kind === "rung" ? options.act : undefined;
    const state = rung?.id ?? id;
    const closed = this.folded(state, rung, Boolean(options.compact));
    key =
      options.compact && closed && !options.preview
        ? "closed"
        : `${key}:${closed}:${options.preview && closed ? this.feedWidth : ""}`;
    const marker: Part[] =
      rung || options.compact || options.collapsible
        ? [[`${closed ? glyph.closed : glyph.open} `, c.faint]]
        : [];
    const heading = [...marker, ...label];
    const visible = Boolean(plain(label)) && (options.compact || options.heading !== false || closed);
    const margin = options.separate ? space.section : space.stack;
    const prior = this.cards.get(id);
    if (prior?.key === key) {
      this.label(prior, heading);
      Object.assign(prior, { marker, title: options.title, closed });
      if (prior.heading.visible !== visible) prior.heading.visible = visible;
      if (prior.node.marginTop !== margin) prior.node.marginTop = margin;
      if (this.feedChild(index) !== prior.node) this.placeCard(prior.node, index);
      return;
    }
    prior?.node.destroyRecursively();
    const box = this.box({ id, gap: space.stack, marginTop: margin, paddingLeft: options.indent ?? 0 });
    const labelNode = this.line(heading, c.muted, {
      truncate: true,
      visible,
      onMouseDown: (event) => {
        if (event.button === 2 && options.act)
          this.actActions(this.session.acts.find((act) => act.id === options.act?.id) ?? options.act);
      },
      run: () => {
        this.session.folds[state] = !closed;
        this.renderContent();
      },
    });
    box.add(labelNode);
    if (!closed) body(box);
    else if (!rung) options.preview?.(box);
    this.placeCard(box, index);
    const card = {
      key,
      node: box,
      heading: labelNode,
      label: [],
      marker,
      title: options.title,
      compact: options.compact ?? false,
      collapsible: Boolean(rung || options.collapsible),
      state,
      closed,
      act: options.act?.id,
    };
    this.label(card, heading);
    this.cards.set(id, card);
  }
  /** The cards of the feed in their order, read once while no card moves, since each read of them copies them. */
  private feedOrder?: Renderable[];
  /** The card at a place of the feed. */
  private feedChild(index: number): Renderable | undefined {
    this.feedOrder ??= this.scroll.getChildren();
    return this.feedOrder[index];
  }
  /** A card moved to a place of the feed, or added there. */
  private placeCard(node: Renderable, index: number): void {
    this.scroll.add(node, index);
    this.feedOrder = undefined;
  }
  /** A panel in a box: a bar of a color at its left, and half a row of the panel above and below what it holds. */
  private panel(box: BoxRenderable, color: RGBA, options: BoxOptions = {}): BoxRenderable {
    const edge = (side: "top" | "bottom") => {
      const bar = this.box({
        height: space.bar,
        border: ["left"],
        borderColor: color,
        customBorderChars: { ...noBorder, vertical: side === "top" ? glyph.barTop : glyph.barBottom },
      });
      bar.add(
        this.box({
          height: space.bar,
          flexGrow: 1,
          border: [side === "top" ? "bottom" : "top"],
          borderColor: c.panel,
          customBorderChars: { ...noBorder, horizontal: side === "top" ? glyph.halfTop : glyph.halfBottom },
        }),
      );
      return bar;
    };
    const inner = this.box({
      paddingLeft: space.between - space.inset,
      paddingRight: space.inset,
      border: ["left"],
      borderColor: color,
      customBorderChars: { ...noBorder, vertical: glyph.bar },
      backgroundColor: c.panel,
      ...options,
    });
    // The mode of a screen reader draws the panel with no bar and no edge, as text alone.
    if (this.reader) {
      box.add(inner);
      return inner;
    }
    box.add(edge("top"));
    box.add(inner);
    box.add(edge("bottom"));
    return inner;
  }
  /** Python, highlighted as it shows, or by colors, which give the highlights of the whole content in their place. */
  private code(
    content: string,
    marks?: SimpleHighlight[],
    hover?: (event: MouseEvent) => boolean,
    colors?: () => Promise<SimpleHighlight[]>,
  ): CodeRenderable {
    const code = new CodeRenderable(this.renderer, {
      content: safeText(content),
      filetype: "python",
      syntaxStyle: this.style,
      wrapMode: "word",
      drawUnstyledText: true,
      ...(colors
        ? { onHighlight: () => colors() }
        : marks?.length
          ? { onHighlight: (highlights: SimpleHighlight[]) => [...highlights, ...marks] }
          : {}),
    });
    return this.pointable(code, content, hover);
  }
  /** The group of each character of a program, as the parser of python highlights it, kept for the last programs. */
  private groupsOf(source: string): Promise<(string | undefined)[]> {
    const kept = this.sourceGroups.get(source);
    if (kept) return kept;
    const made = getTreeSitterClient()
      .highlightOnce(source, "python")
      .then(
        (result) => {
          const groups: (string | undefined)[] = new Array(source.length);
          for (const [start, end, group] of result?.highlights ?? [])
            for (let at = start; at < end; at++) groups[at] = group;
          return groups;
        },
        () => [],
      );
    const oldest = this.sourceGroups.keys().next();
    if (this.sourceGroups.size >= 64 && !oldest.done) this.sourceGroups.delete(oldest.value);
    this.sourceGroups.set(source, made);
    return made;
  }
  private sourceGroups = new Map<string, Promise<(string | undefined)[]>>();
  /** Python under the pointer. A name shows what it holds in a card once the pointer rests on it, and a click with ⌃
   * or ⌥ opens it in the inspector. A header that names an act and an image attachment are references, which show
   * what they name in the card and open it at a click. */
  private pointable<T extends CodeRenderable | TextRenderable>(
    node: T,
    content: string,
    hover?: (event: MouseEvent) => boolean,
  ): T {
    const target = (x: number, y: number) => {
      // A row of the node is a line of the content, or a part of a line that the node wraps.
      const row = y - node.y;
      const info = node.lineInfo;
      const line = content.split("\n")[node.getLineSources(row, 1)[0] ?? row] ?? "";
      const first = row - (info.lineWraps[row] ?? 0);
      const column = x - node.x + (info.lineStartCols[row] ?? 0) - (info.lineStartCols[first] ?? 0);
      const under = (pattern: RegExp) =>
        [...line.matchAll(pattern)].filter(
          (match) =>
            column >= Bun.stringWidth(line.slice(0, match.index)) &&
            column < Bun.stringWidth(line.slice(0, match.index + match[0].length)),
        );
      const reference = under(/^#([\w@.]+)|furb-image:\/\/[\w.]+/g).find(
        (match) => match[1] === undefined || this.session.actOf(match[1]),
      );
      if (reference) return { reference: true, value: reference[1] ?? reference[0] };
      const name = under(/[\p{L}_][\p{L}\p{N}_]*/gu)[0];
      return name && { reference: false, value: name[0] };
    };
    node.onMouseMove = (event) => {
      clearTimeout(this.hoverTimer);
      if (hover?.(event)) return;
      const value = target(event.x, event.y);
      this.unhover();
      if (value)
        this.hoverTimer = setTimeout(() => {
          if (value.reference) void this.referenceHover(value.value, event.x, event.y);
          else void this.showHover(value.value, event.x, event.y);
        }, 220);
    };
    node.onMouseOut = () => {
      clearTimeout(this.hoverTimer);
      this.unhover();
    };
    node.onMouseDown = (event) => {
      const value = target(event.x, event.y);
      if (value?.reference) void this.follow(value.value).catch(this.report);
      else if (value && (event.modifiers.ctrl || event.modifiers.alt)) this.inspect(value.value);
    };
    return node;
  }
  /** Markdown on a surface, whose code blocks stand out from it on the surface of a code block, or on the surface
   * that a code block takes in a panel. */
  private markdown(content: string, fg = c.prose, surface = c.panel): MarkdownRenderable {
    return new MarkdownRenderable(this.renderer, {
      content: safeText(content),
      syntaxStyle: this.style,
      fg,
      // OpenTUI draws a code block as bare lines in the color of the prose, so the TUI draws each one as a block. A
      // renderer that OpenTUI does not mark as one of code blocks alone parts the rest into blocks with no space
      // between them, so the mark keeps the prose as OpenTUI draws it.
      renderNode: Object.assign(
        (token: Token, context: RenderNodeContext) =>
          token.type === "code"
            ? this.codeBlock(
                token as Tokens.Code,
                context,
                surface,
                !content.trimEnd().endsWith(token.raw.trimEnd()),
              )
            : undefined,
        { codeBlockOnly: true },
      ),
    });
  }
  /** A code block: a surface of its own with an inset, its language as a label, the colors of its grammar, and its
   * lines as they are. A line too long for the block does not wrap, so that it keeps the indentation of each line. */
  private codeBlock(
    { text, lang }: Tokens.Code,
    context: RenderNodeContext,
    surface: RGBA,
    followed: boolean,
  ): BoxRenderable {
    const language = lang?.trim().split(/\s+/)[0] ?? "";
    const block = this.box({
      backgroundColor: surface,
      paddingX: space.inset,
      marginBottom: followed ? space.section : 0,
    });
    if (language) block.add(this.text(language, c.faint));
    block.add(
      new CodeRenderable(this.renderer, {
        content: text,
        filetype: infoStringToFiletype(language),
        syntaxStyle: context.syntaxStyle,
        treeSitterClient: context.treeSitterClient,
        fg: c.text,
        wrapMode: "none",
        drawUnstyledText: true,
        width: "100%",
      }),
    );
    return block;
  }

  /** Whether a card is folded: by the operator's click, or else as its kind starts. A rung that runs, or that failed
   * with no rung that took its place, stands open, and any other rung starts folded when the preference says so. */
  private folded(state: string, rung?: ActRow, compact = false): boolean {
    return (
      this.session.folds[state] ??
      (rung
        ? this.session.preferences.foldThreads &&
          !working(rung) &&
          (!failed(rung) || Boolean(this.retry(rung)))
        : compact)
    );
  }
  renderContent(): void {
    const w = this.session;
    const view = this.tree ? "tree" : `${w.selected}:${w.view}`;
    this.treeBar.visible = Boolean(this.tree);
    if (this.lastView !== view) {
      if (this.lastView && this.lastView !== "tree") w.scrolls[this.lastView] = this.place;
      this.clear(this.scroll);
      this.cards.clear();
      this.paneKeys.delete(this.scroll);
      this.lastView = view;
      this.scroll.stickyScroll = w.view === "feed" && !this.tree;
      // A view opens where it was left, and a feed that was never shown opens at its end, once it is laid out with the
      // rows around the input that it shares the screen with.
      if (!this.tree) {
        const place = w.scrolls[view] ?? (this.scroll.stickyScroll ? "end" : 0);
        this.scrollNow(place);
        this.scrollAfterLayout(place);
      }
    }
    this.renderStage();
    if (this.tree) {
      this.renderTree();
      return;
    }
    const existing = new Set(this.cards.keys());
    this.feedOrder = undefined;
    const used = new Map<string, number>();
    const matches = (text: string) => !w.search || text.toLowerCase().includes(w.search.toLowerCase());
    let order = 0,
      items = 0,
      group = "";
    // A card that shows an item of the view says the text that the filter reads, and counts as an item once it shows.
    // A card of a done act names in same what its card is made of, and it is made again only when one of these, or
    // its place, changed: a change of the life makes again only the cards of the acts that changed, so that a frame
    // takes the same time in a long life as in a short one.
    const add = (
      id: string,
      key: string,
      label: Part[] | (() => Part[]),
      body: (box: BoxRenderable) => void,
      options: BlockOptions & { shown?: string; same?: readonly unknown[] } = {},
    ) => {
      if (options.shown !== undefined) {
        if (!matches(options.shown)) return;
        items++;
      }
      // Two cards never share a name: a second card of one name takes the count of its name.
      const taken = used.get(id) ?? 0;
      used.set(id, taken + 1);
      if (taken) id = `${id}~${taken + 1}`;
      existing.delete(id);
      const heading = !options.group || group !== options.group;
      group = options.group ?? "";
      const shape = {
        ...options,
        heading: options.heading ?? heading,
        separate: order > 0 && (options.separate ?? heading),
      };
      const same = options.same && [
        ...options.same,
        shape.heading,
        shape.separate,
        this.theme,
        this.feedWidth,
        this.partsLoaded,
      ];
      const prior = this.cards.get(id);
      if (
        same &&
        prior?.same?.length === same.length &&
        same.every((one, at) => prior.same?.[at] === one) &&
        this.feedChild(order) === prior.node
      ) {
        // The label of an act that works moves with time, which a render reads again as the tick does.
        if (options.title) this.label(prior, [...prior.marker, ...options.title()]);
        order++;
        return;
      }
      this.card(id, key, typeof label === "function" ? label() : label, body, order, shape);
      const made = this.cards.get(id);
      if (made) made.same = same;
      order++;
    };
    if (w.preferences.notice)
      add("preferences-notice", w.preferences.notice, [], (box) => {
        const panel = this.panel(box, c.warning);
        panel.add(
          this.text(w.preferences.notice, c.text, {
            run: () => {
              w.preferences.notice = "";
              this.renderContent();
            },
          }),
        );
      });
    if (w.error && w.view === "changes" && w.damagedJournal)
      // A damaged journal of file changes says what broke, and offers to repair it or to start a new one.
      add("view-error", `${w.view}:${w.damagedJournal}`, [], (box) =>
        this.banner(
          box,
          c.danger,
          [
            [`${glyph.failed} `, c.danger],
            ["The journal of file changes is damaged", c.text, bold],
          ],
          this.text(w.damagedJournal, c.muted),
          [
            "Repair the journal",
            `keeps each change that it can read, ${this.chordOf("repair")}`,
            () => void w.repairJournal().catch(w.fail),
          ],
          [
            "Start a new journal",
            `keeps the damaged one beside it, ${this.chordOf("renew")}`,
            () => void w.renewJournal().catch(w.fail),
          ],
        ),
      );
    else if (w.error)
      add("view-error", `${w.view}:${w.error}`, [], (box) =>
        this.banner(
          box,
          c.danger,
          [
            [`${glyph.failed} `, c.danger],
            [`The ${viewLabels[w.view].toLowerCase()} view could not load`, c.text, bold],
          ],
          this.text(plainFailure(w.error), c.muted),
          [
            "Refresh view",
            "reads the view again",
            () => {
              w.error = "";
              void w.refresh().catch(w.fail);
            },
          ],
        ),
      );
    if (w.view === "feed") {
      // The feed shows the acts as they stood at the moment that the operator moved to, or as they stand now.
      const shown = w.at(w.acts);
      // The acts of now are found by the index of the session, and those of a moment of the past by a map of their own.
      const then = shown === w.acts ? undefined : new Map(shown.map((one) => [one.id, one]));
      const rows = { get: (id: string) => (then ? then.get(id) : w.actOf(id)) };
      // The acts of now are read from the index of the session, which spares a pass over every act in each frame, and
      // those of a moment of the past from an index of their own.
      const index = then ? undefined : w.ladderIndex;
      // The acts that each act made, in the order of the acts, which each open rung reads once the first one asks.
      let made: ReadonlyMap<string, readonly ActRow[]> | undefined = index?.made;
      const madeBy = (id: string): readonly ActRow[] => {
        made ??= makers(shown);
        return made?.get(id) ?? [];
      };
      if (w.watching) {
        add("watch", `watch:${w.moment}:${w.cost}:${this.theme}`, [], (box) => this.watchBanner(box));
      }
      // The feed of a chain is its ladder, which the model of a life gives every host: each prompt with the rungs of
      // its ladder, the acts of each rung, and what the prompt closed with.
      // A fold that the operator opened shows the earlier prompts of the thread, or the earlier rungs of a ladder.
      const opened = new Set(
        Object.entries(w.folds).flatMap(([key, closed]) =>
          closed === false && /^(prompts|rungs):/.test(key) ? [key] : [],
        ),
      );
      // The thread of now is read from the index, in the time of what it shows, and that of a moment of the past whole.
      const owed = new Set(w.host.prompts.keys());
      const word = (rung: ActRow) => this.subject(rung);
      const lines = index
        ? thread(w.selected, word, w.host.streams, owed, opened, index)
        : condense(ladder(w.selected, shown, word, w.host.streams, owed), opened);
      let retold = false;
      // The feed is one program of threads: each thread is its header, then the words of its steps as python, with
      // what each line came to as a quiet comment beside it. A thread that the operator collapsed shows its header
      // line alone.
      const threadsOwn = this.chainThreadsBy(w.selected);
      const waiting = new Set(w.desk.asks.map((ask) => ask.id));
      const named = (chain: string) => w.labelOf(chain);
      const madeMap = () => {
        made ??= makers(shown);
        return made;
      };
      // A program is made again only when what it shows changed: a step, its word, its state, or an act of it, as its
      // state and what its command printed. A new act on another thread leaves it as it was.
      const stepOf = (rung: ActRow) => {
        const acts = madeBy(rung.id);
        let done = 0;
        let printed = 0;
        for (const one of acts) {
          if (one.done) done++;
          printed += one.output ?? 0;
        }
        // The word of a step that runs can still come, and the word of a step that ended stays as it is.
        const said = rung.done ? "" : (w.program[rung.id]?.length ?? 0);
        return `${rung.id}:${rung.done}:${rung.run?.status ?? ""}:${said}:${acts.length}:${done}:${printed}`;
      };
      const liveOf = (ids: readonly string[]) =>
        ids
          .map((id) =>
            [
              id,
              ...madeBy(id)
                .filter((one) => one.kind === "rung")
                .map(stepOf),
            ].join(" "),
          )
          .join("|");
      const programFor = (key: string, live: string, make: () => ProgramLine[]): ProgramLine[] => {
        const known = this.programs.get(key);
        if (!then && known?.version !== -1 && known?.live === live) return known.lines;
        const made = make();
        if (this.programs.size > 2000) this.programs.clear();
        this.programs.set(key, { version: then ? -1 : w.version, live, lines: made });
        return made;
      };
      const programCard = (id: string, given: readonly ProgramLine[], indent: number) => {
        // What a model writes changes at each token, so it stands in a card of its own under the program, and the
        // program stays as it was drawn.
        const all = given.filter((line) => line.kind !== "writes");
        const writers = given.filter((line) => line.kind === "writes");
        const tails = () => {
          for (const line of writers) {
            const stream = w.host.streams.get(line.step.id);
            // Each line that the model finished shows as python, highlighted as it comes, and the line that it
            // still writes stays a comment under it, so the card is made again at each line and not at each token.
            const { lines, tail: open } = writing(stream);
            if (lines)
              add(
                `wrote-${line.step.id}`,
                lines,
                [],
                (box) => box.add(this.inset(space.between, this.code(lines))),
                { indent, same: [lines] },
              );
            const tail = open || (lines ? "writes" : line.text);
            add(
              `writes-${line.step.id}`,
              `${tail}:${this.feedWidth}:${this.theme}`,
              [],
              (box) =>
                box.add(
                  this.inset(
                    space.between,
                    this.text([
                      [`${glyph.running} `, c.accent],
                      [`# ${clip(tail, this.feedWidth - indent - 12)}`, c.syntaxComment],
                    ]),
                  ),
                ),
              { indent },
            );
          }
        };
        if (!all.length) return tails();
        // A thread whose program made its last steps alone says how many came before them.
        const earlier = all[0]?.kind === "earlier" ? all[0].count : 0;
        const steps = [...new Set(all.filter((line) => line.kind !== "note").map((line) => line.step))];
        const hide = earlier
          ? earlier
          : w.folds[`steps:${id}`] !== false && steps.length > PROGRAM_STEPS
            ? steps.length - PROGRAM_STEPS
            : 0;
        // The notes that came before the first step shown fold with the earlier steps.
        const from = earlier ? 1 : hide ? all.findIndex((line) => line.step === steps[hide]) : 0;
        const program = from ? all.slice(from) : all;
        const took = (step: ActRow) =>
          step.kind !== "rung" || !step.done
            ? ""
            : [
                w.took[step.id] === undefined ? "" : spell(w.took[step.id] ?? 0),
                step.cost ? dollars(step.cost) : "",
              ]
                .filter(Boolean)
                .join(" ");
        const since = (act?: ActRow) => {
          const began = act && w.started[act.id];
          const ms = began ? Date.now() - began : 0;
          return ms >= 1000 ? spell(Math.floor(ms / 1000) * 1000) : "";
        };
        const room = this.feedWidth - indent - 8;
        // A click on a line opens it in place: its whole text, and under it what each act of the line gave. The
        // program parts at each open line, so that the cards of its acts stand under it.
        const segments: ProgramLine[][] = [[]];
        for (const line of program) {
          segments.at(-1)?.push(line);
          if (this.lineOpen(line)) segments.push([]);
        }
        // The time and the cost of a step stand on its first line alone, in whichever part of the program it falls.
        const heads = new Set<ProgramLine>();
        let before: ActRow | undefined;
        for (const line of program)
          if (line.kind === "code" && line.step !== before) {
            heads.add(line);
            before = line.step;
          }
        for (const [at, segment] of segments.entries()) {
          if (!segment.length) continue;
          const key = [
            at ? 0 : hide,
            room,
            ...segment.map((line) =>
              line.kind === "code"
                ? `${line.text}|${this.lineHint(line)}|${line.runs ? since(line.acts.at(-1)) : ""}|${line.fails ?? ""}|${took(line.step)}|${this.lineOpen(line)}|${[...this.openStrings(line)]}`
                : `${line.kind}|${line.text}`,
            ),
          ].join("\n");
          add(
            at ? `program-${id}-${at}` : `program-${id}`,
            key,
            [],
            (box) => {
              if (hide && !at)
                box.add(
                  this.link(
                    [
                      [`${glyph.closed} `, c.faint],
                      [`# ${hide} earlier ${hide === 1 ? "step" : "steps"}`, c.syntaxComment],
                    ],
                    () => {
                      w.folds[`steps:${id}`] = false;
                      w.save();
                      this.renderContent();
                    },
                  ),
                );
              box.add(this.programView(segment, room, took, since, heads));
            },
            { indent, same: [key] },
          );
          const open = segment.at(-1);
          if (open?.kind === "code" && this.lineOpen(open) && open.acts.length)
            add(
              `opened-${id}-${at}`,
              open.acts.map((one) => `${one.id}:${one.done}:${(one as ActRow).output ?? ""}`).join(" ") +
                this.theme,
              [],
              (box) => {
                const inner = this.box({ paddingLeft: space.between * 2, gap: space.stack });
                for (const one of open.acts) this.actDetails(inner, one as ActRow);
                box.add(inner);
              },
              { indent },
            );
        }
        tails();
      };
      let collapsed = -1;
      for (const line of lines) {
        if (collapsed >= 0 && line.depth > collapsed) continue;
        collapsed = -1;
        // The steps of a thread stand in its program, and its messages as comments there. What a thread closed with
        // and a question that waits for the operator stand on their own.
        if (
          line.depth > 0 &&
          !(line.type === "value" && threadsOwn.by.has(line.act.id)) &&
          !(line.type === "prompt" && waiting.has(line.act.id))
        )
          continue;
        if (line.type === "prompt" && threadsOwn.of.has(line.act.id)) continue;
        const depth = line.depth;
        const act = line.act as ActRow;
        const indent = line.depth * space.between;
        if (line.type === "rung") {
          const code = this.subject(act);
          // The words that a chain wrote to retell its source stand as one line, which names the source.
          if (rows.get(act.by)?.kind === "chain") {
            if (retold) continue;
            retold = true;
            const source = String(rows.get(act.by)?.words[1] ?? "");
            if (source)
              add(`retold-${act.by}`, `${source}:${this.theme}`, [], (box) =>
                box.add(
                  this.link(
                    [
                      ["↳ ", c.secondary],
                      ["Branched from ", c.muted],
                      [w.labelOf(source), c.text, bold],
                    ],
                    () => void w.select(source).catch(w.fail),
                  ),
                ),
              );
            continue;
          }
          // A word of the operator that only made a chain, as a branch or a new chain does, reads as the chain that it
          // made, and a click on it opens that chain.
          const made =
            act.by === "operator" && /^\s*chain\(/.test(code)
              ? w.chains.find((chain) => chain.by === act.id)
              : undefined;
          if (made) {
            const name = w.labelOf(made.id);
            const here = made.id === w.selected;
            add(
              act.id,
              `made:${made.id}:${name}:${here}:${this.theme}`,
              [],
              (box) =>
                box.add(
                  here
                    ? this.text([
                        ["↳ ", c.secondary],
                        ["This branch starts here", c.muted],
                      ])
                    : this.link(
                        [
                          ["↳ ", c.secondary],
                          [made.words[1] ? "Branched to " : "Started the chain ", c.muted],
                          [name, c.text, bold],
                        ],
                        () => void w.select(made.id).catch(w.fail),
                      ),
                ),
              { shown: code },
            );
            continue;
          }
          // A rung folds to one row: its state, the first line of its word, and the acts that it made. Open, it shows
          // its whole word, and each act that it made stands under it.
          // A rung that a cancel ended before its model wrote a word shows nothing, since the message says the cancel.
          if (!code.trim() && cancelled(act)) continue;
          // A word of the operator stands as its program under a quiet header.
          add(
            act.id,
            `you:${this.theme}`,
            [
              [glyph.mark, threadTone(act.id)],
              ["You ran", c.muted],
            ],
            () => {},
            { indent, heading: true, shown: code },
          );
          programCard(
            act.id,
            programFor(act.id, stepOf(act), () =>
              stepLines(act, madeMap(), word, owed, named, w.host.streams),
            ),
            indent,
          );
          // A word of the operator that the gate refused says what to do under it.
          const hint = this.gateHint(act);
          if (hint)
            add(`hint-${act.id}`, `${hint}:${this.theme}`, [], (box) =>
              box.add(this.inset(space.between, this.text(hint, c.muted))),
            );
        } else if (line.type === "tail") {
          const maker = rows.get(act.by);
          if (line.model) {
            // A model shows the last lines that it writes, or of its thinking until it writes, with a cursor.
            const thinking = !w.host.streams.get(act.id)?.text;
            add(`stream-${act.id}`, line.text, [], (box) => {
              // A column holds the lines, so that a long line wraps at the width of the feed.
              const tail = this.box({ paddingLeft: indent + space.between });
              tail.add(
                this.text(
                  [
                    [line.text, thinking ? c.faint : c.muted, thinking ? italic : 0],
                    [glyph.mark, c.accent],
                  ],
                  c.muted,
                ),
              );
              box.add(tail);
            });
          } else if (maker?.kind === "rung" && this.folded(maker.id, maker))
            // A command that runs shows its last lines under the folded rung that made it.
            add(`tail-${act.id}`, line.text, [], (box) => this.excerpt(box, line.text, true), { indent });
        } else if (line.type === "value") {
          // A notice closes with nothing, which its quiet line says already.
          if (act.words[0] === "None") continue;
          const value = typeof act.value === "boolean" ? (act.value ? "yes" : "no") : display(act.value);
          const answered = asksOperator(act);
          const short = !value.includes("\n") && Bun.stringWidth(value) < this.feedWidth - 20 - indent;
          add(
            `value-${act.id}`,
            `${value}\n${short}\n${answered}`,
            answered
              ? [
                  [`${glyph.done} `, c.success],
                  ["You answered", c.text, bold],
                  [short ? `  ${value}` : "", c.muted],
                ]
              : this.answerLabel(act, false),
            (box) => {
              if (answered && short) return;
              // A column, and not a row, holds the answer, so that its paragraphs wrap at the width of the feed.
              const answer = this.box({ paddingLeft: space.between });
              answer.add(this.markdown(value));
              box.add(answer);
            },
            {
              group: answered ? "question" : "assistant",
              act,
              heading: true,
              indent: indent - space.between,
              shown: value,
            },
          );
        } else if (line.type === "ear") {
          // The rungs that an ear wrote, as the words of the extensions, stand on one quiet row.
          const ids = line.rungs.map((rung) => rung.id);
          const span =
            ids.length > 1 ? `${ids[0]}…${(ids.at(-1) ?? "").replace(/^\D+/, "")}` : (ids[0] ?? "");
          add(
            `ear-${act.id}`,
            `${span}:${line.state}:${line.acts.map((chip) => chip.label + chip.state).join(" ")}:${this.theme}`,
            [
              [
                `${line.state === "done" ? glyph.ring : glyph.dot} `,
                line.state === "failed" ? c.danger : c.faint,
              ],
              [act.by, c.muted],
              [`  ${span}`, c.faint],
              ...this.chipParts(line.acts, this.feedWidth - indent - Bun.stringWidth(act.by + span) - 8),
            ],
            () => {},
            { heading: true, indent, shown: `${act.by} ${span}` },
          );
        } else if (line.type === "more") {
          // The earlier prompts of a long thread, or the earlier rungs of a long ladder, stand as one line until a click
          // opens them.
          const { key, count, heads } = line;
          // The line names what it holds: messages, words, or steps of both.
          const noun =
            heads.length === 1 && heads[0] === "rung"
              ? count === 1
                ? "rung"
                : "rungs"
              : heads.length === 1 && heads[0] === "prompt"
                ? count === 1
                  ? "message"
                  : "messages"
                : count === 1
                  ? "step"
                  : "steps";
          add(
            `more-${key}`,
            `${key}:${count}:${this.theme}`,
            [],
            (box) =>
              box.add(
                this.link(
                  [
                    [`${glyph.closed} `, c.faint],
                    [`${count} earlier ${noun}`, c.muted],
                  ],
                  () => {
                    w.folds[key] = false;
                    w.save();
                    this.renderContent();
                  },
                ),
              ),
            { indent },
          );
        } else if (line.type === "act") {
          // The ceiling of a grant stands in the sidebar, so the feed shows no grant. An act that no word made stands as
          // a line of python under a quiet header, as a word that the operator runs.
          if (act.kind === "grant") continue;
          add(
            act.id,
            `ran:${act.by}:${this.theme}`,
            [
              [glyph.mark, threadTone(act.id)],
              [act.by === "operator" ? "You ran" : `${act.by} ran`, c.muted],
            ],
            () => {},
            { indent, heading: true, shown: act.id },
          );
          programCard(
            act.id,
            programFor(`act:${act.id}`, `${act.done}:${act.paused}:${act.output ?? 0}`, () =>
              actLines(act, owed, named),
            ),
            indent,
          );
        } else if (asksOperator(act) && act.words[0] === "None") {
          // A prompt of None asks nothing: the console closed it as it took it, and it settles as a quiet line, its
          // first line, which opens to the whole message. No key dismisses it.
          const message = String(act.words[1] ?? "");
          const from = act.by === "operator" ? "for you" : `from ${act.by}`;
          const lines = message.split("\n").filter((one) => one.trim()).length;
          const more = lines > 1 ? `  +${lines - 1} ${lines === 2 ? "line" : "lines"}` : "";
          const room = this.feedWidth - indent - 12 - Bun.stringWidth(from + more);
          add(
            act.id,
            `${message}\n${this.feedWidth}\n${this.theme}`,
            [
              [`${glyph.notice} `, c.faint],
              ["Notice", c.muted],
              [`  ${from}`, c.faint],
              [`  ${clip(shortenHomes(gist(message)), room)}`, c.muted],
              [more, c.faint],
            ],
            (box) => {
              const body = this.box({ paddingLeft: space.between });
              body.add(this.markdown(message, c.muted));
              box.add(body);
            },
            { compact: true, group: "notice", act, indent, shown: message },
          );
        } else if (asksOperator(act)) {
          const message = String(act.words[1] ?? "");
          const ask = w.desk.asks.find((one) => one.id === act.id);
          const waiting = Boolean(ask);
          const aimed = waiting && w.aimed?.id === act.id;
          add(
            act.id,
            `${message}\n${waiting}\n${aimed}`,
            [
              [`${glyph.asks} `, waiting ? c.warning : c.faint],
              ["Question", c.text, bold],
              [`  ${act.by === "operator" ? "for you" : `from ${act.by}`}`, c.faint],
            ],
            (box) => {
              this.panel(box, waiting ? c.warning : c.border).add(
                this.markdown(message, c.text, c.background),
              );
              // The question that the input answers says so, and a click on another one points the input at it.
              if (ask)
                box.add(
                  this.inset(
                    space.between,
                    this.text(
                      aimed
                        ? [
                            ["The input below answers this question", c.muted],
                            [ask.choices.length ? ` with ${wants(ask)}` : `, as ${ask.shape}`, c.muted],
                            [", or press ", c.faint],
                            ["⌃A", c.muted],
                          ]
                        : [["Click here to answer this question in the input below", c.muted]],
                      c.muted,
                      { run: () => (aimed ? this.question(ask) : void w.point(ask.id).catch(this.report)) },
                    ),
                  ),
                );
            },
            { group: "question", act, shown: message },
          );
        } else {
          const message = String(act.words[1] ?? "");
          const user = w.isUserPrompt(act);
          // Each thread carries its color on its header and on the mark at its left.
          // A message of a thread takes the color of its thread, and hides with it.
          const own = this.chainThreadsBy(w.selected);
          const thread = own.by.get(act.id);
          const host = thread ?? own.of.get(act.id);
          const tone = threadTone(host?.act.id ?? act.id);
          if (host && this.threadFolded(host.act.id)) {
            collapsed = depth;
            if (!thread) continue;
            const [mark, color] = thread ? threadMarks(thread) : [glyph.ring, c.faint];
            const steps = (madeBy(act.id).length || 0) + (thread?.messages.length ?? 0);
            const room = this.feedWidth - indent - 16;
            add(
              act.id,
              `folded\n${message}\n${mark}\n${steps}\n${this.feedWidth}\n${this.theme}`,
              [],
              (box) =>
                box.add(
                  this.link(
                    [
                      [`${glyph.mark}`, tone],
                      [`${mark} `, color],
                      [`${glyph.closed} `, c.faint],
                      [clip(shortenHomes(gist(message)), room), c.muted],
                      [steps ? `  ${steps} ${steps === 1 ? "step" : "steps"}` : "", c.faint],
                      // A folded thread that a cancel or a failure ended says so in words, beside its mark.
                      [
                        thread.state === "cancelled" || thread.state === "failed" ? `  ${thread.state}` : "",
                        thread.state === "failed" ? c.danger : c.faint,
                      ],
                    ],
                    () => this.foldThread(act.id, false),
                  ),
                ),
              { act, indent, shown: message },
            );
            continue;
          }
          // A message that the chain or a rung sent is its heading when it is one short line, and stands under its
          // heading otherwise. The heading says who sent it, and to which model.
          const sent = user ? "" : this.sender(act);
          const line =
            !message.includes("\n") && Bun.stringWidth(message) + Bun.stringWidth(sent) < this.feedWidth - 8;
          const state = user ? this.promptState(act) : [];
          // A long message shows its first rows until a click opens it, so that a brief of a page leaves the feed
          // readable at a glance.
          const [shortened, rest] =
            w.folds[`message:${act.id}`] === false ? [message, 0] : this.firstRows(message, MESSAGE_ROWS);
          add(
            act.id,
            `${message}\n${line}\n${sent}\n${plain(state)}\n${rest}\n${thread?.resolved}\n${this.theme}`,
            user
              ? [["You", tone]]
              : [
                  [`${glyph.mark}`, tone],
                  [`${glyph.ring} `, c.faint],
                  [line ? message : sent, c.muted, line ? bold : 0],
                  [line ? `  ${sent}` : "", c.faint],
                ],
            (box) => {
              // A message shows as the operator typed it, with each image it attached named by a mark that opens it.
              // Its first line says at its right the type of the answer that it asks for, and where it stands.
              if (user) {
                // A right click on a message opens its menu, as on the heading of any other act.
                const row = this.box({
                  flexDirection: "row",
                  onMouseDown: (event) => {
                    if (event.button === 2)
                      this.actActions(this.session.acts.find((one) => one.id === act.id) ?? act);
                  },
                });
                const text = this.message(shortened);
                text.flexGrow = 1;
                text.flexShrink = 1;
                row.add(text);
                row.add(this.text(state, c.faint, { marginLeft: space.between }));
                const controls = this.threadControls(act);
                if (controls) row.add(controls);
                this.panel(box, tone).add(row);
              } else if (!line) {
                const note = this.box({ paddingLeft: space.between });
                note.add(this.markdown(shortened, c.muted));
                box.add(note);
              }
              if (rest)
                box.add(
                  this.inset(
                    space.between,
                    this.link(
                      [
                        [`${glyph.closed} `, c.faint],
                        [`${rest} more ${rest === 1 ? "line" : "lines"}`, c.muted],
                      ],
                      () => {
                        w.folds[`message:${act.id}`] = false;
                        w.save();
                        this.renderContent();
                      },
                    ),
                  ),
                );
              const controls = user ? undefined : this.threadControls(act);
              if (controls) box.add(controls);
            },
            { group: user ? "user" : "sent", act, heading: user ? false : undefined, indent, shown: message },
          );
          const parts = [act, ...(thread?.messages ?? [])];
          programCard(
            act.id,
            programFor(act.id, `${w.folds[`steps:${act.id}`]}|${liveOf(parts.map((one) => one.id))}`, () =>
              threadProgram(
                parts,
                madeMap(),
                word,
                owed,
                named,
                undefined,
                w.folds[`steps:${act.id}`] === false ? undefined : PROGRAM_STEPS,
              ),
            ),
            indent,
          );
        }
      }
      // A paused chain says so at the end of its feed, with the reason that the last failure gave, and a button that
      // wakes it, since nothing new runs on it until then.
      if (w.paused && !w.search) {
        const failure = w.activity.findLast((act) => act.kind === "rung" && act.run?.status === "failed")?.run
          ?.reason;
        add("paused", `paused:${failure ?? ""}:${this.theme}`, [], (box) =>
          this.banner(
            box,
            c.warning,
            [
              [`${glyph.held} `, c.warning],
              ["This chain is paused", c.text, bold],
              ["  New work waits for a wake.", c.muted],
            ],
            failure
              ? this.whole(
                  this.text(clip(shortenHomes(failure).split("\n")[0] ?? "", this.feedWidth - 8), c.danger),
                  () => shortenHomes(failure),
                )
              : undefined,
            ["Wake", "runs what waits, once the cause is fixed", () => this.action("/wake")],
          ),
        );
        items++;
      }
      if (!items && !w.search && !w.loading && !w.error) {
        const { width, height } = this.scroll.viewport;
        const welcomeKey = JSON.stringify([
          this.reader,
          w.catalogRead,
          w.catalog.length,
          w.setup,
          w.actorChoice.model,
          w.workingDirectory,
          w.watching,
        ]);
        add("welcome", `welcome:${width}:${height}:${this.theme}:${welcomeKey}`, [], (box) =>
          this.welcome(box, height),
        );
        items++;
      }
    } else if (w.view === "transcript") {
      w.turns.forEach((turn, index) => {
        const text = turn[1];
        add(
          `transcript-${index}`,
          text,
          [
            [
              turn[0] === "assistant" ? `${glyph.dot} ` : `${glyph.ring} `,
              turn[0] === "assistant" ? c.secondary : c.faint,
            ],
            [turn[0], c.text, bold],
            [`  turn ${index + 1}`, c.faint],
          ],
          (box) => {
            const inner = this.box({ paddingLeft: space.between });
            inner.add(turn[0] === "assistant" ? this.code(text) : this.transcriptText(text));
            box.add(inner);
          },
          { group: turn[0], shown: text },
        );
      });
    } else if (w.view === "changes") {
      const root = w.host.directory;
      const relative = (path: string) => fromDirectory(root, path);
      // The scopes of the review stand first, and in the middle of an empty view under why it is empty.
      if (w.changes.length)
        add("change-scopes", `${w.scope}:${this.theme}`, [], (box) => box.add(this.scopes()));
      const counts = (added: number, removed: number): Part[] => [
        [`+${added}`, added ? c.success : c.faint],
        [` -${removed}`, removed ? c.danger : c.faint],
      ];
      if (w.changes.length) {
        // The list of the files: each with the lines that it added and removed, and a click that goes to its diff.
        const added = w.changes.reduce((sum, change) => sum + change.added, 0);
        const removed = w.changes.reduce((sum, change) => sum + change.removed, 0);
        const files = w.changes.length === 1 ? "1 file" : `${w.changes.length} files`;
        add(
          "change-files",
          `${w.scope}:${this.theme}:${w.changes.map((change) => `${change.path}:${change.added}:${change.removed}`).join()}`,
          [[`${files}  `, c.text, bold], ...counts(added, removed)],
          (box) => {
            const width = Math.max(...w.changes.map((change) => Bun.stringWidth(relative(change.path))));
            for (const change of w.changes) {
              const path = relative(change.path);
              const row = this.row({ paddingLeft: space.between });
              row.add(this.link(path, () => this.jumpTo(`change:${path}`)));
              row.add(
                this.text([
                  [" ".repeat(width - Bun.stringWidth(path) + 2), c.text],
                  ...counts(change.added, change.removed),
                ]),
              );
              box.add(row);
            }
          },
        );
      }
      for (const change of w.changes) {
        const path = relative(change.path);
        // The heading of a change is a bar of the panel: the path, what the change did to the file, and the lines it
        // added and removed at its right.
        const [what, color] = !change.before
          ? ["created", c.success]
          : !change.after
            ? ["deleted", c.danger]
            : ["modified", c.warning];
        const tail = `+${change.added} -${change.removed} `;
        const fill = Math.max(
          1,
          this.feedWidth - Bun.stringWidth(` ${glyph.dot} ${path}  ${what}`) - Bun.stringWidth(tail),
        );
        // Two sides need room for two lines of code side by side, and one side reads better below that.
        const split = this.renderer.width >= 140;
        add(
          `change:${path}`,
          `${w.scope}:${change.before.length}:${change.after.length}:${change.added}:${change.removed}:${split}:${this.theme}`,
          [
            [` ${glyph.dot} `, color, 0, c.panel],
            [path, c.text, bold, c.panel],
            [`  ${what}`, c.muted, 0, c.panel],
            [" ".repeat(fill), c.text, 0, c.panel],
            [`+${change.added}`, change.added ? c.success : c.faint, 0, c.panel],
            [` -${change.removed} `, change.removed ? c.danger : c.faint, 0, c.panel],
          ],
          (box) => {
            for (const hunk of change.hunks) {
              // Each hunk has a bar: where it stands in both files, a revert through the World, and a comment on
              // one of its lines, which goes to the chain as a prompt.
              const bar = this.row({ gap: space.between, paddingLeft: space.between });
              bar.add(
                this.text(
                  `@@ -${hunk.oldStart},${hunk.oldLines} +${hunk.newStart},${hunk.newLines} @@`,
                  c.faint,
                ),
              );
              bar.add(this.link("Revert", () => void w.revert(change, hunk).catch(this.report)));
              bar.add(this.link("Comment a line", () => this.commentLine(path, hunk)));
              box.add(bar);
              box.add(
                new DiffRenderable(this.renderer, {
                  diff: hunkPatch(path, hunk),
                  view: split ? "split" : "unified",
                  syntaxStyle: this.style,
                  fg: c.text,
                  showLineNumbers: true,
                  lineNumberFg: c.faint,
                  lineNumberBg: c.background,
                  contextBg: c.background,
                  addedBg: c.added,
                  removedBg: c.removed,
                  addedSignColor: c.success,
                  removedSignColor: c.danger,
                  wrapMode: "word",
                }),
              );
            }
          },
          { shown: change.path },
        );
      }
    }
    // A view with nothing to show says why in the middle of the feed, and what brings something to it.
    const { height } = this.scroll.viewport;
    if (!items && w.loading && !w.error) {
      const loading = (): Part[] => [
        [`${spin()} `, c.accent],
        [`Loading the ${viewLabels[w.view].toLowerCase()}`, c.muted],
      ];
      add(
        "view-loading",
        `${w.view}:${height}`,
        [],
        (box) => this.centered(box, height, this.text(loading())),
        {
          title: loading,
          heading: false,
        },
      );
    }
    if (!items && !w.loading && !w.error) {
      const empty: Record<View, [string, string]> = {
        feed: ["Nothing here yet", "Send a message, or run Python with ⌃R."],
        transcript: ["No transcript yet", "The model reads its first turn here once a prompt runs."],
        changes: (
          {
            turn: [
              "No file changes in this turn",
              "Each file that this chain writes after your last message shows here.",
            ],
            chain: [
              "No file changes on this chain",
              "Each file that this chain writes shows here as a diff.",
            ],
            life: ["No file changes yet", "Each file that the life writes shows here as a diff."],
            branch: [
              "No change on this branch",
              "Each file that differs from main in git shows here as a diff.",
            ],
          } satisfies Record<Scope, [string, string]>
        )[w.scope],
      };
      const [title, hint] = w.search
        ? [`Nothing matches “${w.search}”`, "Change the filter, or press Esc to clear it."]
        : empty[w.view];
      // An empty review offers its other scopes under its hint, where the eye already is.
      const scoped = w.view === "changes" && !w.search;
      add("empty", `${title}:${height}:${this.theme}:${scoped && w.scope}`, [], (box) =>
        this.centered(
          box,
          height,
          this.text([
            [`${glyph.ring} `, c.faint],
            [title, c.text, bold],
          ]),
          this.text(hint, c.muted),
          ...(scoped ? [this.scopes()] : []),
        ),
      );
    }
    for (const id of existing) {
      this.cards.get(id)?.node.destroyRecursively();
      this.cards.delete(id);
    }
  }

  /** A panel of the feed that says a state of the view: its heading, what the state holds, and a link that acts on it
   * with what the link does. */
  private banner(
    box: BoxRenderable,
    color: RGBA,
    heading: Part[],
    detail: Renderable | undefined,
    ...actions: [label: string, does: string, run: () => void][]
  ): void {
    const panel = this.panel(box, color);
    panel.add(this.text(heading));
    if (detail) panel.add(this.inset(space.between, detail));
    for (const [index, [label, does, run]] of actions.entries()) {
      const row = this.box({
        flexDirection: "row",
        marginTop: index ? 0 : space.section,
        paddingLeft: space.between,
      });
      row.add(this.link([[label, c.accent, bold]], run));
      row.add(this.text(`  ${does}`, c.faint));
      panel.add(row);
    }
  }
  /** A text that lights under the pointer, and runs an action at a click. */
  private link(content: string | Part[], run: () => void): TextRenderable {
    return this.hoverable(this.text(content, c.accent, { run }));
  }
  /** A message of the operator: its text, and the name of each image it refers to in the color of an attachment. A
   * click on the message opens the actions of its first image. */
  private message(text: string): TextRenderable {
    const images = imageReferences(text);
    const parts: Part[] = [];
    let rest = text;
    for (const image of images) {
      const at = rest.indexOf(image.text);
      parts.push([rest.slice(0, at), c.text], [`${glyph.dot} ${image.name || "image"}`, c.secondary]);
      rest = rest.slice(at + image.text.length);
    }
    parts.push([rest, c.text]);
    const first = images[0];
    return this.text(parts, c.text, { run: first && (() => this.imageActions(first.uri)) });
  }
  /** What a view says in the middle of the feed when it has nothing else to show. */
  private centered(box: BoxRenderable, height: number, ...nodes: Renderable[]): void {
    const frame = this.box({
      minHeight: Math.max(0, height - space.section * 2),
      justifyContent: "center",
      alignItems: "center",
    });
    for (const node of nodes) frame.add(node);
    box.add(frame);
  }
  /** The screen of a feed that has no turn yet: the logo of furb, what it is, where it works, where to start, and the
   * keys to know. */
  private welcome(box: BoxRenderable, height: number): void {
    const w = this.session;
    const width = Math.min(72, this.feedWidth);
    const column = this.box({ width, alignItems: "center" });
    // The logo shades from the accent to the color of Python, one column at a time. The mode of a screen reader says
    // the name, since a reader reads the blocks of the logo as glyphs.
    const columns = Math.max(...logo.map((line) => line.length));
    if (this.reader) column.add(this.text([["furb", c.accent, bold]]));
    else
      for (const line of logo)
        column.add(
          this.text(
            [...line].map(
              (cell, at): Part => [cell, mix(c.accent, c.secondary, at / Math.max(1, columns - 1))],
            ),
            c.accent,
            { width: columns },
          ),
        );
    column.add(
      this.text("The model answers in Python. Read and steer each word it runs.", c.muted, {
        marginTop: space.section,
      }),
    );
    const model = modelName(w.actorChoice.model).id;
    column.add(
      this.whole(
        this.text(
          [
            [clip(shortenHome(w.workingDirectory), 40, "end"), c.faint],
            ["   ", c.faint],
            [model, c.faint],
          ],
          c.faint,
          { truncate: true },
        ),
        () => `${shortenHome(w.workingDirectory)}   ${model}`,
      ),
    );
    // The check of the setup shows on the first run, and on each run while no model answers, once the worker said
    // what this machine holds. The demo needs no model and checks nothing.
    const setup = w.setup;
    const found = setup && w.catalogRead ? checks(setup, w.catalog.length) : [];
    // With no model, a prompt has nothing to answer it, so the welcome offers only the life to watch.
    const none = Boolean(found.length) && !w.catalog.length;
    if (found.length && (this.firstRun || none)) {
      column.add(this.setupCheck(found, width));
      if (!w.preferences.setupSeen) {
        w.preferences.setupSeen = true;
        try {
          w.preferences.save();
        } catch (error) {
          this.report(error);
        }
      }
    }
    // Each way to start is a card of two lines. The first plays a life to watch, and a click on each other one puts
    // its prompt in the composer. The cards are as wide as their longest line, and stand in the middle as one block.
    const cards: [label: string, line: string, run: () => void][] = [
      ...(w.watching ? [] : [[...watching, () => void this.watch()] as [string, string, () => void]]),
      ...(none ? [] : starters).map(([label, prompt]): [string, string, () => void] => [
        label,
        prompt,
        () => this.insert(prompt),
      ]),
    ];
    const widest = Math.max(
      ...cards.flatMap(([label, line]) => [label, line].map((one) => Bun.stringWidth(one))),
    );
    const starts = this.box({
      width: Math.min(width, widest + space.between * 2 + 1),
      marginTop: space.section * 2,
      gap: space.section,
    });
    // A card has a bar at its left, but in the mode of a screen reader.
    const framed = !this.reader;
    for (const [index, [label, prompt, run]] of cards.entries()) {
      const lead = index === 0 && !w.watching;
      // The pointer lifts a card onto a panel, and its bar takes the accent.
      const card = this.box({
        paddingX: space.between,
        backgroundColor: c.background,
        border: ["left"],
        borderColor: c.border,
        customBorderChars: { ...noBorder, vertical: glyph.bar },
        onMouseUp: this.click(run),
        onMouseOver() {
          this.backgroundColor = c.panel;
          if (framed) this.borderColor = c.accent;
        },
        onMouseOut() {
          this.backgroundColor = c.background;
          if (framed) this.borderColor = lead ? c.accent : c.border;
        },
      });
      // The card that plays a life leads, with its bar in the accent.
      if (lead && framed) card.borderColor = c.accent;
      card.add(this.text([[label, lead ? c.accent : c.text, bold]], c.text, { truncate: true }));
      card.add(this.text(prompt, c.muted, { truncate: true }));
      starts.add(card);
    }
    column.add(starts);
    // Each key under the cards is a button that does what it names.
    const keys = this.box({ flexDirection: "row", marginTop: space.section * 2, gap: space.between + 1 });
    for (const [chord, action, run] of [
      ["/", "commands", () => this.palette()],
      ["@", "files", () => void this.filesPicker().catch(this.report)],
      ["⌃R", "Python", () => this.toggleMode()],
      ["F1", "help", () => this.help()],
    ] as const)
      keys.add(
        this.text(
          [
            [chord, c.muted],
            [` ${action}`, c.faint],
          ],
          c.faint,
          { run },
        ),
      );
    column.add(keys);
    this.centered(box, height, column);
  }

  /** The check of the setup: each way to a model that this machine holds or lacks, with a fix for each one that is
   * missing, and a clear word when no model answers. */
  private setupCheck(found: Check[], width: number): BoxRenderable {
    const none = found.at(-1)?.state === "missing";
    const panel = this.box({
      width,
      marginTop: space.section * 2,
      paddingX: space.between,
      gap: space.section,
    });
    panel.add(
      this.text(
        none
          ? [
              [`${glyph.failed} `, c.danger],
              ["No model answers yet", c.text, bold],
            ]
          : [
              [`${glyph.done} `, c.success],
              ["Setup", c.text, bold],
            ],
        c.text,
      ),
    );
    const marks: Record<Check["state"], Part> = {
      ready: [glyph.done, c.success],
      missing: [glyph.failed, c.danger],
      optional: [glyph.ring, c.faint],
    };
    // A check and its fix stand together, and a line of space stands between two checks.
    for (const check of found) {
      const [mark, color] = marks[check.state];
      const one = this.box({});
      one.add(
        this.text(
          [
            [`${mark} `, color],
            [check.label, c.text],
            [`  ${check.found}`, c.muted],
          ],
          c.text,
        ),
      );
      if (check.fix && check.state !== "ready") one.add(this.text(check.fix, c.faint, { paddingLeft: 2 }));
      panel.add(one);
    }
    return panel;
  }
  /** The bar at the top of the feed of a life that the operator watches: what it costs, which is nothing, what it
   * shows, in common words, and where the moment stands when the operator moved back in time. */
  private watchBanner(box: BoxRenderable): void {
    const w = this.session;
    const panel = this.panel(box, c.accent);
    // The demo asks no model: its script answers each turn, so no call goes out and nothing is spent.
    panel.add(
      this.text([
        [`${glyph.running} `, c.accent],
        ["Watching a life", c.text, bold],
        [`   0 model calls, ${dollars(w.cost)}`, c.muted],
      ]),
    );
    panel.add(
      this.text(
        "Many models answer in one life. A word of Python is a program. Any point can start a new branch.",
        c.muted,
        { paddingLeft: space.between },
      ),
    );
    panel.add(
      this.text(
        w.moment === undefined
          ? "Press [ to step back in time, then b to start a branch from there."
          : "This is an earlier moment. Press ] to step on, b to start a branch here, or Esc to come back to now.",
        w.moment === undefined ? c.faint : c.warning,
        { paddingLeft: space.between },
      ),
    );
  }
  /** Watch a life: a new demo that plays by itself, asks no model, and shows in a workspace of its own. */
  private watch = async (): Promise<void> => {
    try {
      const session = await demoSession({ watch: true, preferences: this.session.preferences });
      await this.options.workspaces.show(session);
      void tour(session).catch(session.fail);
    } catch (error) {
      this.report(error);
    }
  };

  private preview(text: string, reserve = 2): string {
    return clip(text.replace(/\s+/g, " "), Math.max(8, this.feedWidth - reserve));
  }
  /** The first or the last lines of a text under a heading, joined to it by a branch. */
  private excerpt(box: BoxRenderable, content: string, tail = false, color = c.muted): void {
    const lines = shortenHomes(content)
      .trimEnd()
      .split("\n")
      .filter((line) => line.trim());
    const limit = 3;
    const selected = tail ? lines.slice(-limit) : lines.slice(0, limit);
    if (lines.length > limit) {
      if (tail) selected[0] = `… ${selected[0]}`;
      else selected[selected.length - 1] += " …";
    }
    const visible = selected
      .map((line) => clip(line, Math.max(8, this.feedWidth - space.between * 3)))
      .join("\n");
    box.add(this.branched(visible, color));
  }
  /** A text joined by a branch to the heading above it. */
  private branched(text: string, color: RGBA): BoxRenderable {
    const row = this.box({ flexDirection: "row", paddingLeft: space.between });
    row.add(this.text(`${glyph.branch} `, c.faint));
    row.add(this.text(text, color, { flexShrink: 1 }));
    return row;
  }
  /** What an act is doing: the word that says it, the glyph that shows it, and their color. A done act says
   * nothing, since its glyph says it. */
  private actState(act: ActRow): State {
    const w = this.session;
    const running = () => ({ word: `running ${this.progress(act.id)}`, mark: spin(), color: c.accent });
    if (act.kind === "grant" && act.done) return { word: "ended", mark: glyph.ring, color: c.faint };
    // Only work moves: an act of another kind that lives, as a grant or the watcher of an extension, lives until
    // something ends it, as a chain does, and shows a dot.
    if (!act.done && !WORK.includes(act.kind)) return { word: "", mark: glyph.dot, color: c.accent };
    // A rung that a pause holds waits for the wake, and says so, where it would otherwise seem to run.
    if (act.kind === "rung")
      return cancelled(act)
        ? stateOf("cancelled")
        : act.run?.status === "failed"
          ? this.retry(act)
            ? { word: `retried as ${this.retry(act)?.id}`, mark: glyph.failed, color: c.faint }
            : stateOf("failed")
          : act.run?.status === "done"
            ? stateOf("done")
            : act.paused
              ? stateOf("held")
              : running();
    if (act.done) return stateOf(failed(act) ? "failed" : cancelled(act) ? "cancelled" : "done");
    if (w.host.pending.has(act.id)) return { word: "pending", mark: glyph.ring, color: c.faint };
    if (w.host.prompts.has(act.id)) return { word: "needs input", mark: glyph.asks, color: c.warning };
    if (act.paused && act.kind !== "bash") return stateOf("held");
    return running();
  }
  /** The first lines of a text that take at most a count of rows of the feed, and how many lines are left out. */
  private firstRows(text: string, rows: number): [string, number] {
    const lines = text.split("\n");
    const width = Math.max(20, this.feedWidth - 8);
    let used = 0;
    let taken = 0;
    for (const line of lines) {
      used += Math.max(1, Math.ceil(Bun.stringWidth(line) / width));
      if (used > rows) break;
      taken++;
    }
    // A message that is a row or two too long shows whole, since a fold of so little hides nothing.
    if (taken >= lines.length - 2) return [text, 0];
    return [lines.slice(0, Math.max(1, taken)).join("\n"), lines.length - Math.max(1, taken)];
  }
  /** The acts of a rung as chips, each in the color of its state, as many as the room holds. */
  private chipParts(chips: readonly Chip[], room: number): Part[] {
    const parts: Part[] = [];
    let used = 0;
    for (const [index, chip] of chips.entries()) {
      const left = chips.length - index;
      if (used + Bun.stringWidth(chip.label) + 2 > room - (left > 1 ? 6 : 0)) {
        parts.push([`  +${left}`, c.faint]);
        break;
      }
      used += Bun.stringWidth(chip.label) + 2;
      parts.push(["  ", c.faint], [chip.label, chipColors[chip.state]()]);
    }
    return parts;
  }
  private readonly coloring = new Map<string, Promise<void>>();
  /** Once each line of a word that the view asked to color has its colors. */
  async colorsDone(): Promise<void> {
    while (this.coloring.size) await Promise.all(this.coloring.values());
  }
  /** The loom that the threads of the sidebar read last, and what it was woven of, which a frame of the same life
   * reads again. */
  private woven?: { of: unknown[]; loom: Loom };
  /** The loom of the chains of the life as it stands, each chain under the label that the operator gave it last. */
  private lifeLoom(): Loom {
    const w = this.session;
    const of = [
      w.version,
      [...w.host.prompts.keys()].join(" "),
      w.host.measures,
      [...w.host.pending.keys()].join(" "),
    ];
    const same = this.woven?.of.every((one, at) => one === of[at]) ? this.woven.loom : undefined;
    const loom =
      same ??
      weave({
        acts: w.acts,
        prompts: [...w.host.prompts.values()],
        measures: w.host.measures,
        folds: new Set(),
        width: 0,
        pending: w.host.pending,
        label: (id) => id,
        index: w.boardIndex,
      });
    this.woven = { of, loom };
    // A chain that the operator names again keeps its loom, so each label is read again.
    return {
      ...loom,
      rows: loom.rows.map((row) => (row.id === "operator" ? row : { ...row, label: w.labelOf(row.id) })),
    };
  }
  /** The view of the life that shows: main, the loom, or the graph. */
  /** The view of the life: the one that the preferences keep, or main in the mode of a screen reader,
   * which reads text in lines and no picture of the life in time. */
  get lifeView(): LifeView {
    return this.reader ? "main" : this.session.preferences.view;
  }
  /** The view of the life switched to the one named, or to the next one, which the preferences keep. */
  switchView(view?: LifeView): void {
    const w = this.session;
    const next = view ?? lifeViews[(lifeViews.indexOf(this.lifeView) + 1) % lifeViews.length] ?? "main";
    w.preferences.view = next;
    w.preferences.save();
    if (next === "main") {
      this.cursor = undefined;
      if (this.steering) this.steer(false);
    }
    w.notice =
      next === "main"
        ? "Main shows the threads of the life beside the feed. ⌥V switches the view."
        : `The ${next} shows the life. ⌥V switches the view, and ⌥G moves in it.`;
    this.render();
  }
  /** The actor of a chain, as its last answer, its rungs or its prompts name it. */
  private actorOf(chain: string): string {
    const w = this.session;
    return actorOf(chain, w.actsOn(chain), w.host.measures, chain === w.selected ? w.actor : w.host.actor);
  }
  /** The acts of the life as they stood at the time cursor, which the loom and the graph read. */
  private stageActs: readonly ActRow[] = [];
  /** The loom of the life at the time cursor, which the last draw of the stage read. */
  private stageLoom: Loom = { rows: [], owed: [], folded: 0 };
  private weaveStage(width: number): Loom {
    const w = this.session;
    const then = this.cursor;
    const acts = w.at(w.acts);
    this.stageActs = acts;
    this.stageLoom = weave({
      acts,
      prompts: then === undefined ? [...w.host.prompts.values()] : owedAt(acts),
      measures: w.host.measures,
      folds: this.loom.folds,
      width,
      pending: then === undefined ? w.host.pending : new Map(),
      label: (id) => w.labelOf(id),
    });
    return this.stageLoom;
  }
  /** Whether the stage moves with time: a chain works, or the graph shows a message that flows. */
  private moves(): boolean {
    return (
      this.stageLoom.rows.some((row) => row.state === "working") ||
      (this.lifeView === "graph" && this.stageLoom.owed.length > 0)
    );
  }
  /** The loom or the graph over the feed. The graph takes the whole height of the
   * feed for one program and its words, and gives it back to the feed to show one act. */
  private renderStage(): void {
    const w = this.session;
    const view = this.lifeView;
    const shows =
      view !== "main" && w.view === "feed" && !this.tree && !(view === "graph" && this.graph.level === "act");
    const whole = shows && view === "graph" && ["program", "word"].includes(this.graph.level);
    if (this.stage.visible !== shows) this.stage.visible = shows;
    if (this.scroll.visible === whole) this.scroll.visible = !whole;
    this.steps();
    if (!shows) return;
    const width = this.feedWidth;
    const body = Math.max(8, this.renderer.height - 14);
    if (view === "loom") {
      const loom = this.weaveStage(LoomPane.cellWidth(width));
      const rows = this.loom.draw({
        loom,
        width,
        height: Math.min(loom.rows.length + 1, Math.max(4, Math.floor(body * 0.45))),
        selected: w.selected,
        frozen: this.cursor !== undefined,
        steering: this.steering,
        actor: (id) => this.actorOf(id),
      });
      this.stage.flexGrow = 0;
      this.stage.height = rows.length;
      this.stage.content = styled(joined(rows));
      return;
    }
    this.weaveStage(Math.max(8, width - 60));
    if (whole) {
      this.stage.flexGrow = 1;
      this.stage.flexShrink = 1;
      this.stage.minHeight = 0;
    } else {
      this.stage.flexGrow = 0;
      this.stage.flexShrink = 0;
    }
    const height = whole ? Math.max(6, this.renderer.height - 12) : Math.max(8, Math.floor(body * 0.55));
    this.stage.height = height;
    this.stage.content = styled(joined(this.graph.rows(width, height)));
  }
  /** A click on a row of the stage: the loom selects its chain, or answers for the row of the operator, and the graph
   * selects what the row shows, and zooms in on a second click. */
  private clickStage(row: number): void {
    if (this.lifeView === "graph") {
      if (this.graph.click(row)) this.render();
      return;
    }
    const chain = this.loom.click(row);
    if (chain === "operator") this.question();
    else if (chain) void this.selectAt(chain).catch(this.report);
  }
  /** What the graph reads of the App: the life at the time cursor, and the controls that it asks for. */
  private stageOf(): Stage {
    const app = this;
    return {
      get acts() {
        return app.stageActs;
      },
      get loom() {
        return app.stageLoom;
      },
      get owed() {
        return new Set(app.stageLoom.owed.map((prompt) => prompt.id));
      },
      get frozen() {
        return app.cursor !== undefined;
      },
      get selected() {
        return app.session.selected;
      },
      words: () => app.words(),
      label: (chain) => app.session.labelOf(chain),
      actor: (chain) => app.actorOf(chain),
      streams: () => (app.cursor === undefined ? app.session.host.streams : new Map()),
      select: (chain) => app.selectAt(chain),
      // The lens of the graph is the card of the act in the feed, which shows it whole.
      lens: (act) => {
        if (!act) return app.render();
        const w = app.session;
        void (act.on === w.selected ? Promise.resolve() : w.select(act.on))
          .then(() => app.go("feed", act.id))
          .catch(app.report);
      },
    };
  }
  /** The words of the rungs of the life: what the caller of each wrote, what ran, and what its model wrote, the words
   * that the gate refused among them. */
  private words(): Map<string, string> {
    const w = this.session;
    const words = new Map<string, string>();
    for (const item of conversation(w.turns, (one) => w.actOf(one)))
      if (item.type === "python" && item.rung) words.set(item.rung.id, item.code);
    for (const [id, word] of Object.entries(w.program)) words.set(id, word);
    for (const act of w.acts)
      if (act.kind === "rung" && act.words[0]) words.set(act.id, String(act.words[0]));
    return words;
  }
  /** The keys go to the loom or the graph, or back to the input. */
  steer(on = !this.steering): void {
    const w = this.session;
    if (on && this.lifeView === "main") {
      w.notice = "⌥V shows the loom or the graph, which ⌥G moves in.";
      return;
    }
    this.steering = on;
    if (on) {
      this.closeTree();
      this.composer.blur();
    } else this.composer.focus();
    this.render();
  }
  /** The selection of the loom moves by a count of chains, and the session shows the chain that it reaches. */
  private moveLoom(by: number): void {
    const rows = this.stageLoom.rows.filter((row) => row.id !== "operator");
    const at = rows.findIndex((row) => row.id === this.session.selected);
    const next = rows[Math.max(0, Math.min(rows.length - 1, at + by))];
    if (next && next.id !== this.session.selected) void this.selectAt(next.id).catch(this.report);
  }
  /** A chain shown at the moment of the cursor in time, which the loom and the graph keep from chain to chain. */
  private async selectAt(chain: string): Promise<void> {
    const at = this.cursor;
    await this.session.select(chain);
    // A move of the cursor while the chain opened stands.
    if (at !== undefined && this.cursor === undefined) this.cursor = at;
    this.render();
  }
  /** The workers of the selected chain fold into its row of the loom, or show again. */
  private foldLoom(folding: boolean): void {
    const row = this.stageLoom.rows.find((one) => one.id === this.session.selected);
    if (!row?.workers) return;
    if (folding) this.loom.folds.add(row.id);
    else this.loom.folds.delete(row.id);
    this.render();
  }
  /** The time cursor moves back or forward by a count of moments, and past the last moment it stands at the present
   * again. */
  travel(by: number): void {
    const moments = momentsOf(this.session.acts);
    if (!moments.length) return;
    const from = this.cursor === undefined ? moments.length - 1 : moments.indexOf(this.cursor);
    const to = Math.max(0, from + by);
    this.cursor = to >= moments.length - 1 ? undefined : moments[to];
    this.render();
  }
  /** Whether a verb that changes the life can act: none acts in the past. */
  private present(): boolean {
    if (this.cursor === undefined) return true;
    this.session.notice =
      "The cursor stands in the past, where nothing changes. Esc returns to now, and b branches.";
    this.render();
    return false;
  }
  /** The selected chain paused, or woken when a pause holds it. */
  private holdChain(): void {
    if (!this.present()) return;
    const w = this.session;
    const row = this.stageLoom.rows.find((one) => one.id === w.selected);
    this.action(`/${row?.state === "paused" ? "wake" : "pause"} ${w.selected}`);
  }
  /** The work of the selected chain cancelled: the act that the graph selects, or what the chain waits on. */
  private cancelChain(): void {
    if (!this.present()) return;
    const w = this.session;
    const target =
      this.lifeView === "graph" && this.graph.level !== "threads" && this.graph.level !== "map"
        ? this.graph.selection
        : this.stageLoom.rows.find((one) => one.id === w.selected)?.waits;
    if (target) this.action(`/cancel ${target}`);
    else w.notice = `${w.label} waits on nothing to cancel.`;
  }
  /** A step: the selected chain runs until one more of its rungs ends, then a pause holds it again. */
  private async step(): Promise<void> {
    if (!this.present()) return;
    const w = this.session;
    const chain = w.selected;
    this.stepping.set(chain, this.rungsDone(chain));
    if (this.stageLoom.rows.find((row) => row.id === chain)?.state === "paused")
      this.action(`/wake ${chain}`);
    w.notice = `${w.labelOf(chain)} runs one step, then pauses.`;
    this.render();
  }
  private rungsDone(chain: string): number {
    return this.session.acts.filter((act) => act.on === chain && act.kind === "rung" && act.done).length;
  }
  /** Each chain that a step let run pauses again once a rung of it that the step let run ends. */
  private steps(): void {
    for (const [chain, ended] of this.stepping) {
      if (this.rungsDone(chain) <= ended) continue;
      this.stepping.delete(chain);
      this.action(`/pause ${chain}`);
      this.session.notice = `${this.session.labelOf(chain)} paused after one step.`;
    }
  }
  /** A branch of the selected chain at the time cursor, as the feed branches at its moment. */
  private branchAtCursor(): void {
    const w = this.session;
    if (this.cursor === undefined) {
      w.notice = "[ moves the cursor back in time, and b branches the chain there.";
      this.render();
      return;
    }
    void w.branchHere().catch(this.report);
  }
  /** Escape on the stage: the graph zooms out, then the cursor returns to now, then the keys go back to the input. */
  private async leaveStage(): Promise<void> {
    if (this.lifeView === "graph" && (await this.graph.key("escape"))) this.render();
    else if (this.cursor !== undefined) this.travel(Number.MAX_SAFE_INTEGER);
    else this.steer(false);
  }
  /** Enter on the stage: the graph zooms in, and the loom answers for the row of the operator or gives the keys to the
   * input on the selected chain. */
  private async enterStage(): Promise<void> {
    if (this.lifeView === "graph") {
      await this.graph.key("return");
      this.render();
    } else this.steer(false);
  }
  /** An arrow on the stage: the graph moves its selection, and the loom moves between chains or folds their workers. */
  private async arrowStage(name: string): Promise<void> {
    if (this.lifeView === "graph") {
      await this.graph.key(name);
      this.render();
    } else if (name === "up" || name === "down") this.moveLoom(name === "up" ? -1 : 1);
    else if (name === "pageup" || name === "pagedown") this.moveLoom(name === "pageup" ? -8 : 8);
    else this.foldLoom(name === "left");
  }
  /** The desk over the input: one mark for each ask that waits for the operator, in the order that they came, then
   * the ask that he acts on, from which chain, and what it wants, the first line of its message, and its choices or
   * where its answer goes. A click on a mark points at its ask and shows its chain, and a click on the message opens
   * the ask whole. */
  /** The parts once they loaded: their keys again, and each card again, since a part may draw the row of its act. */
  /** The count of the loads of the parts, which a label that a part gives keys on. */
  private partsLoaded = 0;
  private partsChanged = (): void => {
    this.partsLoaded++;
    if (this.closed) return;
    this.partKeysOff?.();
    const keys = this.parts.loaded.flatMap((part) => part.keys);
    // A key of a part stands under every key of the TUI, and one with no modifier acts on an empty input alone.
    this.partKeysOff = keys.length
      ? this.keymap.registerLayer({
          priority: -1000,
          enabled: () => !this.renaming?.input?.focused && !this.overlay,
          commands: keys.map(({ on, run }, index) => ({
            name: `part-${index}`,
            run: () => {
              this.partRun(() => run(this.seen()));
              return true;
            },
            enabled: () => /^(ctrl|meta|alt|super)\+/.test(on) || !this.composer.plainText,
          })),
          bindings: keys.map(({ on }, index) => ({ key: on, cmd: `part-${index}` })),
        })
      : undefined;
    // The cards are made again only when a part draws the rows of a kind, before this load or after it.
    const rows = this.parts.loaded.some((part) => part.rows.size);
    if (rows || this.partRows) {
      this.clear(this.scroll);
      this.cards.clear();
      this.paneKeys.delete(this.scroll);
    }
    this.partRows = rows;
    this.dockKey = "";
    this.schedule();
  };
  /** Runs what a part does, and shows in the footer the error of one that fails. */
  private partRun(run: () => unknown): void {
    const failed = (error: unknown) => {
      this.session.notice = `A part of an extension failed: ${plainError(error)}`;
      this.schedule();
    };
    try {
      const got = run();
      if (got instanceof Promise) got.catch(failed);
    } catch (error) {
      failed(error);
    }
  }
  /** What a component of a part reads of the life, and what it can do in it. */
  private seen(): Seen {
    const w = this.session;
    return {
      chain: w.selected,
      acts: w.acts,
      of: (kind) => w.acts.filter((act) => act.kind === kind),
      run: (word) => w.command(`/run ${word}`),
      say: (text) => {
        w.notice = text;
        this.schedule();
      },
    };
  }
  /** The runs of a line of a part, each in the color of its tone. */
  private partLine(line: PartLine): Part[] {
    const tones: Record<PartTone, RGBA> = {
      text: c.text,
      muted: c.muted,
      faint: c.faint,
      accent: c.accent,
      success: c.success,
      warning: c.warning,
      danger: c.danger,
    };
    if (typeof line === "string") return [[line, c.text]];
    return line.map(([text, tone]) => [String(text), tones[tone ?? "text"] ?? c.text]);
  }
  /** The row that a part draws for an act of its kind, with its error as its label when it throws. */
  private partRow(act: ActRow): Row | undefined {
    const draw = this.parts.row(act.kind);
    if (!draw) return undefined;
    try {
      return draw(act, this.seen()) ?? {};
    } catch (error) {
      return { label: `its row failed: ${plainError(error)}` };
    }
  }
  /** The panels of the parts over the input, while the feed shows: each panel of a part that loaded, and the error of
   * a part that did not load or of a panel that throws, in plain words in its own panel. A panel shows twelve lines at
   * most. */
  private renderDock(): void {
    const w = this.session;
    const shown: { title: string; lines: readonly PartLine[]; error?: string }[] = [];
    if (w.view === "feed" && !this.tree) {
      const seen = this.seen();
      for (const part of this.parts.loaded) {
        if (part.error) shown.push({ title: part.name, lines: [], error: part.error });
        for (const panel of part.panels)
          try {
            const lines = panel.draw(seen);
            if (!Array.isArray(lines)) throw new Error("Its panel gives no list of lines.");
            shown.push({ title: panel.name, lines });
          } catch (error) {
            shown.push({ title: panel.name, lines: [], error: plainError(error) });
          }
      }
    }
    const visible = shown.length > 0;
    if (this.dock.visible !== visible) this.dock.visible = visible;
    if (!visible) {
      this.dockKey = "";
      return;
    }
    const key = JSON.stringify([shown, this.feedWidth, this.theme]);
    if (key === this.dockKey) return;
    this.dockKey = key;
    this.dock.backgroundColor = c.panel;
    this.clear(this.dock);
    for (const one of shown) {
      const box = this.box();
      const mark: Part[] = one.error ? [[`${glyph.failed} `, c.danger]] : [];
      box.add(this.line([...mark, [one.title, c.text, bold]], c.text, { truncate: true }));
      if (one.error) box.add(this.text(one.error, c.danger));
      const lines = one.lines.slice(0, 12);
      for (const line of lines) box.add(this.line(this.partLine(line), c.text, { truncate: true }));
      if (one.lines.length > lines.length)
        box.add(this.line(`${one.lines.length - lines.length} more lines`, c.faint));
      this.dock.add(box);
    }
  }
  private renderDesk(): void {
    const w = this.session;
    const { asks, questions } = w.desk;
    const ask = w.aimed;
    const visible = asks.length > 0 && !this.tree;
    if (this.deskBox.visible !== visible) this.deskBox.visible = visible;
    if (!visible) {
      this.deskKey = "";
      return;
    }
    const key = JSON.stringify([asks, ask?.id, w.selected, this.feedWidth, this.theme]);
    if (key === this.deskKey) return;
    this.deskKey = key;
    this.deskBox.backgroundColor = c.panel;
    this.clear(this.deskBox);
    const room = this.feedWidth - space.inset * 2;
    const head = this.row();
    const marks = this.row({ gap: 1 });
    head.add(marks);
    for (const one of asks)
      marks.add(
        this.text(glyph.asks, one.id === ask?.id ? c.warning : c.faint, {
          run: () => void w.point(one.id).catch(this.report),
        }),
      );
    const count = `${questions} ${questions === 1 ? "question" : "questions"}`;
    if (!ask) {
      head.add(
        this.line(
          [
            ["  ", c.faint],
            [`${count} ${asks.length === 1 ? "waits" : "wait"} on other chains`, c.text, bold],
            ["  ⌃A or a mark goes to the first", c.faint],
          ],
          c.text,
          { truncate: true, flexShrink: 1, run: () => this.question() },
        ),
      );
      this.deskBox.add(head);
      return;
    }
    const place = `${asks.findIndex((one) => one.id === ask.id) + 1} of ${asks.length}`;
    const wanted = `wants ${wants(ask)}`;
    head.add(
      this.line(
        [
          ["  ", c.faint],
          ["Question", c.text, bold],
          [` ${place}`, c.muted],
          ["  from ", c.faint],
          [ask.label, c.text],
          [`  ${ask.id}  `, c.faint],
          [wanted, c.warning],
        ],
        c.text,
        { truncate: true, flexShrink: 1 },
      ),
    );
    this.deskBox.add(head);
    const lines = ask.message.split("\n").filter((line) => line.trim()).length;
    const more = lines > 1 ? `  +${lines - 1} ${lines === 2 ? "line" : "lines"}` : "";
    this.deskBox.add(
      this.line(
        [
          [clip(shortenHomes(gist(ask.message)), room - Bun.stringWidth(more)), c.text],
          [more, c.faint],
        ],
        c.text,
        { truncate: true, run: () => this.question(ask) },
      ),
    );
    const act = this.row({ gap: space.between });
    if (ask.choices.length && ask.id === this.picking()?.id)
      act.add(
        this.text("Choose below with ↑↓ and Enter, or its number, or Other for your own words", c.faint),
      );
    else if (ask.choices.length) {
      for (const choice of ask.choices)
        act.add(
          this.line([[choice, c.warning, bold]], c.warning, {
            run: () => void w.answer(ask, choice).catch(this.report),
          }),
        );
      act.add(this.text("or type it below", c.faint));
    } else
      act.add(
        this.line(
          [
            ["Enter sends the input to ", c.faint],
            [ask.id, c.muted],
            [` as ${ask.shape}`, c.faint],
            [asks.length > 1 ? "   ⌃A opens each in turn" : "   ⌃A opens it", c.faint],
          ],
          c.faint,
          { truncate: true, flexShrink: 1 },
        ),
      );
    this.deskBox.add(act);
  }
  /** The question whose values the input offers as a list in place of the box of the prompt: the question aimed, when
   * its shape is a choice, a Literal or a bool, and no program is under edit. */
  private picking(): Ask | undefined {
    const ask = this.session.operatorPrompt;
    if (!ask?.choices.length || this.session.editing) return undefined;
    if (this.choice.id !== ask.id) this.choice = { id: ask.id, at: 0, other: false };
    return ask;
  }
  /** The values of a question of a choice, each after its number, and a last row, Other, whose words go in place of
   * every value. The row of the pointer is lit. While Other is chosen, the input under the list takes the words. */
  private renderPick(ask: Ask | undefined): void {
    if (this.pickBox.visible !== Boolean(ask)) this.pickBox.visible = Boolean(ask);
    const typing = !ask || this.choice.other;
    if (this.composer.visible !== typing) {
      this.composer.visible = typing;
      // The input takes the keys again once no list holds them.
      if (typing && !this.overlay) this.composer.focus();
      else this.composer.blur();
    }
    if (!ask) {
      this.pickKey = "";
      return;
    }
    const key = JSON.stringify([ask.id, ask.choices, this.choice, this.theme]);
    if (key === this.pickKey) return;
    this.pickKey = key;
    this.clear(this.pickBox);
    for (const [index, label] of [...ask.choices, "Other"].entries()) {
      const here = index === this.choice.at;
      this.pickBox.add(
        this.line(
          [
            [here ? glyph.pointer : " ", c.warning],
            [` ${index + 1}  `, here ? c.muted : c.faint],
            [label, here ? c.text : c.muted, here ? bold : 0],
            [index === ask.choices.length ? "  in your own words" : "", c.faint],
          ],
          c.text,
          { truncate: true, run: () => void this.takePick(index) },
        ),
      );
    }
  }
  /** The pointer of the list moves a step, round from the last row to the first. */
  private stepPick(by: number): void {
    const ask = this.picking();
    if (!ask) return;
    const rows = ask.choices.length + 1;
    this.choice = { ...this.choice, at: (this.choice.at + by + rows) % rows };
    this.render();
  }
  /** The row at a place, the pointer by default, answers: a value answers the question with it, and Other gives the
   * input to the words of the operator. */
  private async takePick(at = this.choice.at): Promise<void> {
    const ask = this.picking();
    if (!ask) return;
    const value = ask.choices[at];
    this.choice = { id: ask.id, at, other: value === undefined };
    if (value !== undefined) await this.session.answer(ask, value).catch(this.report);
    this.render();
  }
  /** The rung that took the place of a rung of a model that failed: a later rung for the same prompt, which the model
   * wrote once it read the failure, so the failure no longer stands. */
  private retry(act: ActRow): ActRow | undefined {
    if (act.kind !== "rung" || act.by === "operator" || !failed(act)) return undefined;
    return this.session.madeAfter(act, (other) => other.kind === "rung");
  }
  /** The words that the turns of the chain shown tell of its rungs, by rung, which the program of the chain does not
   * hold for a word that the gate refused. */
  private told: { turns: readonly Turn[]; at: number; rung?: ActRow; words: Map<string, string> } = {
    turns: [],
    at: 0,
    words: new Map(),
  };
  private toldWord(id: string): string | undefined {
    const w = this.session;
    const { turns } = w;
    const held = this.told;
    if (held.turns !== turns) {
      // The turns before the last are whole, so each is read once while the turns begin with them. The last turn grows
      // as paragraphs are told on the chain, and it is read again.
      if (!(held.at > 0 && turns.length >= held.at && turns[held.at - 1] === held.turns[held.at - 1]))
        Object.assign(held, { at: 0, rung: undefined, words: new Map() });
      const whole = Math.max(held.at, turns.length - 1);
      const read = (from: { at: number; rung?: ActRow }, to?: number) => {
        for (const item of conversation(turns.slice(from.at, to), (one) => w.actOf(one), from))
          if (item.type === "python" && item.rung) held.words.set(item.rung.id, item.code);
        return from;
      };
      held.rung = read({ at: held.at, rung: held.rung }, whole).rung;
      held.at = whole;
      read({ at: whole, rung: held.rung });
      held.turns = turns;
    }
    return held.words.get(id);
  }
  /** What an act is about, whole: the message of a prompt, the program of a rung, the time of a wait, the ceilings of
   * a grant, and the first word of any other act. */
  private subject(act: ActRow): string {
    const label = this.partRow(act)?.label;
    if (label !== undefined) return label;
    if (act.kind === "prompt") return String(act.words[1] ?? "");
    if (act.kind === "rung")
      return String(this.session.program[act.id] || act.word || this.toldWord(act.id) || act.words[0] || "");
    if (act.kind === "wait") return seconds(Number(act.words[0]));
    if (act.kind === "write") {
      // A write names the path of the Text that it writes, which a read gave whole, so the path drops the directory of
      // the chain.
      const path = String((act.words[0] as { path?: unknown } | null)?.path ?? "");
      const inside = isAbsolute(path) ? relative(this.session.directory, path) : path;
      return inside && !inside.startsWith("..") && !isAbsolute(inside) ? inside : shortenHomes(path);
    }
    if (act.kind === "grant")
      return [
        act.words[0] === null ? "" : `${dollars(Number(act.words[0]))} ceiling`,
        act.words[1] === null ? "" : share(Number(act.words[1])),
      ]
        .filter(Boolean)
        .join("  ");
    return String(act.words[0] || "");
  }
  /** Where a message of the operator stands, which its panel says at its right: the type of the answer that it asks
   * for, and a mark and a word only when a cancel or a failure ended it, or a pause holds it. */
  private promptState(act: ActRow): Part[] {
    const shape: Part = [String(act.words[0] ?? ""), c.faint];
    const state = act.done
      ? failed(act)
        ? stateOf("failed")
        : cancelled(act)
          ? stateOf("cancelled")
          : undefined
      : act.paused || this.session.host.pending.has(act.id)
        ? stateOf("held")
        : undefined;
    return state ? [shape, [`   ${state.mark} ${state.word}`, state.color]] : [shape];
  }
  /** The model of an actor by its name alone, with no provider, and its effort. */
  private model(actor: string): { name: string; effort: string } {
    const { model, effort } = actorParts(
      actor || this.session.actor,
      this.session.roster.map(([name]) => name),
    );
    return { name: modelName(model).id, effort };
  }
  /** Who sent a prompt that is not a message of the operator, and to which model: a chain tells its model that an
   * act it waits on is done, and a rung asks a model a question. */
  private sender(act: ActRow): string {
    const model = this.model(String(act.words[2] ?? "")).name;
    const maker = this.session.actOf(act.by);
    return maker?.kind === "chain" ? `the chain told ${model}` : `${act.by} asked ${model}`;
  }
  /** The heading of the answer to a prompt: the model that answered it with its effort, and the prompt when others
   * closed with it. */
  private answerLabel(act: ActRow, parallel: boolean): Part[] {
    const { name, effort } = this.model(String(act.words[2] ?? ""));
    return [
      [`${glyph.dot} `, c.secondary],
      [name, c.text, bold],
      [effort && effort !== "off" ? `  ${effort}` : "", c.faint],
      [
        parallel
          ? `  answers “${this.preview(String(act.words[1]).split("\n")[0] ?? "", Bun.stringWidth(name) + 34)}”`
          : "",
        c.muted,
      ],
    ];
  }
  /** A word of Python in a card: its numbered lines, the reason it failed, and what to do after a refusal of the
   * gate. */
  private word(box: BoxRenderable, word: string, reason?: string, hint?: string): void {
    // The code starts under the name of its rung, past the fold and the glyph of the heading.
    const inner = this.box({ gap: space.stack, paddingLeft: space.inset });
    // A rung whose word has not come yet shows no code.
    if (word) inner.add(this.numbered(word));
    // The parser names the word as <string> and says its line twice, which the reason leaves out.
    if (reason)
      inner.add(this.branched(shortenHomes(reason.replace(/\s*\(<string>, line \d+\)$/, "")), c.danger));
    // The line of what to do stands under the reason, at the column of its text.
    if (hint) inner.add(this.inset(space.between, this.text(hint, c.muted)));
    box.add(inner);
  }
  /** The one line that says what to do after the gate refused the word of a rung, and nothing for any other rung:
   * the operator fixes his own word, and the chain asks a model for its word again with the findings. */
  private gateHint(act: ActRow): string | undefined {
    if (act.kind !== "rung" || !act.done || !act.run?.refused) return undefined;
    return act.by === "operator"
      ? "Press ↑ in the input to bring the word back, fix the line that the gate names, and send it again."
      : "The chain tells the model these findings and asks it for its word again.";
  }
  private actDetails(box: BoxRenderable, act: ActRow): void {
    if (act.kind === "rung") {
      this.word(
        box,
        String(this.session.program[act.id] || act.words[0] || ""),
        act.run?.reason,
        this.gateHint(act),
      );
      return;
    }
    const details = this.box({ paddingLeft: space.between * 2, gap: space.stack });
    box.add(details);
    if (act.kind === "bash") {
      this.commandDetails(details, act);
      return;
    }
    if ((act.kind === "read" || act.kind === "write") && !failed(act)) {
      this.textDetails(details, act);
      return;
    }
    const row = this.partRow(act);
    if (row?.details) {
      for (const line of row.details) details.add(this.line(this.partLine(line), c.text));
      return;
    }
    const fields: Record<string, string[]> = {
      prompt: ["shape", "message", "actor"],
      rung: ["word", "retells", "actor", "returns"],
      wait: ["seconds"],
      grant: ["dollar ceiling", "context ceiling"],
    };
    details.add(this.reference(act.id, act.id));
    for (const [index, value] of act.words.entries()) {
      if (value === null || value === "") continue;
      details.add(
        this.text([
          [`${fields[act.kind]?.[index] ?? `argument ${index + 1}`}: `, c.faint],
          [display(value), c.muted],
        ]),
      );
    }
    if (act.done && act.value !== null)
      details.add(
        this.text(display(act.value), failed(act) ? c.danger : c.text, { marginTop: space.section }),
      );
  }
  /** The text that a read gave or that a write wrote, and under it its name. The row leaves the text out, so the card
   * reads the whole act. */
  private textDetails(details: BoxRenderable, act: ActRow): void {
    const node = this.text("", c.text);
    details.add(node);
    details.add(this.reference(act.id, act.id));
    void this.session.host.act(act.id).then((whole) => {
      const text = (whole?.kind === "write" ? whole.words[0] : whole?.value) as
        | { content?: unknown }
        | undefined;
      if (!node.isDestroyed && typeof text?.content === "string")
        node.content = safeText(text.content.replace(/\n$/, ""));
    }, this.report);
  }
  /** What a command printed, and under it its name and how it was run. The command says its line, its exit and its time
   * in its heading. What it
   * printed to stderr takes the color of a failure, and each stream is named only when the command printed to both. */
  private commandDetails(details: BoxRenderable, act: ActRow): void {
    const exit = (act.value && typeof act.value === "object" ? act.value : {}) as Exit;
    const both = Boolean(exit.stdout?.content && exit.stderr?.content);
    const shown: TextRenderable[] = [];
    for (const [name, color] of [
      ["stdout", c.text],
      ["stderr", c.danger],
    ] as const) {
      const content = exit[name]?.content;
      if (!content) continue;
      if (both)
        details.add(
          this.text(name, name === "stdout" ? c.faint : c.danger, {
            marginTop: shown.length ? space.section : 0,
          }),
        );
      // The line end that closes what a command printed opens no empty row.
      const node = this.text(content.replace(/\n$/, ""), color);
      details.add(node);
      shown.push(node);
    }
    const meta = this.box({ flexDirection: "row", marginTop: shown.length ? space.section : 0 });
    meta.add(this.reference(act.id, act.id));
    const [, input, timeout] = act.words;
    // The heading says the exit of a command and its time, so its card says only how it was run.
    const notes: Part[] = [
      [input === null ? "   input open" : "", c.faint],
      [typeof timeout === "number" && timeout !== TIMEOUT ? `   times out after ${timeout}s` : "", c.faint],
    ];
    meta.add(this.text(notes, c.faint));
    details.add(meta);
    // The row holds the tail of what the command printed, and the card reads the whole of it once it opens.
    if (act.output !== undefined)
      void this.session.host.act(act.id).then((whole) => {
        const streams = whole?.value as Exit | undefined;
        const contents = [streams?.stdout?.content, streams?.stderr?.content].filter(Boolean) as string[];
        for (const [index, node] of shown.entries())
          if (!node.isDestroyed && contents[index] !== undefined)
            node.content = safeText(contents[index].replace(/\n$/, ""));
      }, this.report);
  }

  /** A program of the feed: its lines as one block of python, each numbered as a line of its word, with what the
   * acts of a line came to as a quiet comment beside it, the line that runs lit with its time, and the line that failed
   * marked, with its reason under it as a comment. */
  private programView(
    lines: readonly ProgramLine[],
    room: number,
    took: (step: ActRow) => string,
    since: (act?: ActRow) => string,
    heads: ReadonlySet<ProgramLine>,
  ): Renderable {
    const texts: string[] = [];
    const numbers = new Map<number, number>();
    const hidden = new Set<number>();
    const colors = new Map<number, { content: RGBA; gutter: RGBA }>();
    const signs = new Map<number, { before: string; beforeColor: RGBA }>();
    const widest = Math.max(
      0,
      ...lines.map((line) => (line.kind === "code" ? Bun.stringWidth(line.text) : 0)),
    );
    const noteOf = (line: Extract<ProgramLine, { kind: "code" }>) =>
      [this.lineHint(line), line.runs ? since(line.acts.at(-1)) : "", heads.has(line) ? took(line.step) : ""]
        .filter(Boolean)
        .join("  ");
    // The code takes the room that its hints leave, and the hints take 30 columns at most: a longer hint is cut.
    const notes = Math.max(
      0,
      ...lines.map((line) => (line.kind === "code" ? Bun.stringWidth(noteOf(line)) : 0)),
    );
    const column = Math.max(24, Math.min(100, widest, room - (notes ? Math.min(notes, 26) + 4 : 0)));
    const fit = (text: string, most: number) => clip(text, Math.max(4, most));
    // The line of the program that each row shows, which a click opens or folds.
    const sources: (ProgramLine | undefined)[] = [];
    // The strings that each row folds, and the rows that show the strings of a line whole, each with where it starts
    // in the text of its line.
    const folded = new Map<number, { line: Extract<ProgramLine, { kind: "code" }>; folds: Fold[] }>();
    const whole = new Map<number, number>();
    // The colors come from each statement whole, so a line cut to its room or a folded string changes no color: the
    // program as it ran, each part of a row with where it starts in that program, and the rows that only comment.
    let program = "";
    const pieces: [row: number, from: number, to: number, at: number][] = [];
    const fixed: [row: number, from: number, to: number, group: string][] = [];
    const comment = (row: number, from = 0) => fixed.push([row, from, texts[row]?.length ?? 0, "comment"]);
    const push = (text: string, number?: number, source?: ProgramLine) => {
      const at = texts.length;
      texts.push(text);
      sources.push(source);
      if (number === undefined) hidden.add(at);
      else numbers.set(at, number);
      return at;
    };
    for (const line of lines) {
      if (line.kind === "code") {
        const note = noteOf(line);
        // An open line, and a line whose strings the operator opened, show the whole text, cut at the column into
        // rows of their own. Another line folds its longest strings until it fits.
        const shown =
          this.lineOpen(line) || this.openStrings(line).size ? undefined : foldStrings(line.text, column);
        const parts = shown
          ? [fit(shown.text, column)]
          : Array.from({ length: Math.max(1, Math.ceil(line.text.length / column)) }, (_, k) =>
              line.text.slice(k * column, (k + 1) * column),
            );
        const code = parts[0] ?? "";
        const hint = note ? `# ${fit(note, room - column - 4)}` : "";
        const at = push(
          hint ? `${code}${" ".repeat(Math.max(0, column - Bun.stringWidth(code)))}  ${hint}` : code,
          line.number,
          line,
        );
        if (hint) comment(at, (texts[at]?.length ?? 0) - hint.length);
        for (const more of parts.slice(1)) push(more, undefined, line);
        const folds = shown?.folds.filter((fold) => fold.at + FOLD.length <= Bun.stringWidth(code)) ?? [];
        if (folds.length) folded.set(at, { line, folds });
        if (!shown) for (const k of parts.keys()) whole.set(at + k, k * column);
        const base = program.length;
        program += `${line.text}\n`;
        if (shown) {
          const found = shown.folds.length ? literals(line.text) : [];
          let from = 0;
          let source = base;
          for (const fold of shown.folds) {
            if (fold.at >= code.length) break;
            pieces.push([at, from, fold.at, source]);
            fixed.push([at, fold.at, Math.min(fold.at + FOLD.length, code.length), "string"]);
            from = fold.at + FOLD.length;
            source = base + (found[fold.literal]?.end ?? 0);
          }
          if (from < code.length) pieces.push([at, from, code.length, source]);
        } else
          for (const [k, part] of parts.entries()) pieces.push([at + k, 0, part.length, base + k * column]);
        if (line.runs) {
          colors.set(at, { content: c.selected, gutter: c.selected });
          signs.set(at, { before: glyph.running, beforeColor: c.accent });
        }
        if (line.fails) {
          signs.set(at, { before: glyph.failed, beforeColor: c.danger });
          for (const why of line.fails
            .split("\n")
            .filter((one) => one.trim())
            .slice(0, 3))
            comment(push(`# ${fit(why.replace(/^line \d+: /, ""), room - 4)}`));
        }
      } else if (line.kind === "quote") {
        const at = push(
          `${line.name} = "${fit(line.text, column - line.name.length - 6)}"  # ${line.lines} lines`,
          line.number,
        );
        const end = (texts[at] ?? "").lastIndexOf('"  # ');
        fixed.push([at, line.name.length + 3, end + 1, "string"]);
        comment(at, end + 3);
      } else if (line.kind === "output") comment(push(`#   ${fit(line.text, room - 6)}`));
      else if (line.kind === "writes") {
        const at = push(`# ${fit(line.text, room - 4)}`);
        comment(at);
        signs.set(at, { before: glyph.running, beforeColor: c.accent });
      } else comment(push(fit(line.text, room - 2)));
    }
    // Each fold is underlined, at its place in the text of the program.
    const starts: number[] = [];
    let offset = 0;
    for (const text of texts) {
      starts.push(offset);
      offset += text.length + 1;
    }
    const marks: SimpleHighlight[] = [...folded].flatMap(([row, { folds }]) =>
      folds.map(
        (fold): SimpleHighlight => [
          (starts[row] ?? 0) + fold.at,
          (starts[row] ?? 0) + fold.at + FOLD.length,
          "fold",
        ],
      ),
    );
    // The fold under the pointer: a hover shows its string whole, and a click opens it in place.
    const foldAt = (x: number, y: number) => {
      const row = y - view.y;
      const one = folded.get(row);
      const at = x - target.x;
      const fold = one?.folds.find((fold) => {
        const from = Bun.stringWidth((texts[row] ?? "").slice(0, fold.at));
        return at >= from && at < from + FOLD.length;
      });
      return one && fold ? { line: one.line, fold } : undefined;
    };
    const hoverFold = (event: MouseEvent) => {
      const hit = foldAt(event.x, event.y);
      if (!hit) return false;
      clearTimeout(this.hoverTimer);
      this.unhover();
      this.hoverTimer = setTimeout(() => {
        const raw = literals(hit.line.text)[hit.fold.literal]?.raw;
        const text = raw
          ? hit.fold.whole
          : hit.fold.whole.replace(/\\([nt"'\\])/g, (_, one: string) =>
              one === "n" ? "\n" : one === "t" ? "  " : one,
            );
        this.hoverCard(
          event.x,
          event.y,
          14,
          () => this.openString(hit.line, hit.fold.literal),
          this.text(text.slice(0, 2000), c.text),
        );
      }, 220);
      return true;
    };
    // Each run of characters of one group in a part of a row takes the group that it has in the program.
    const highlights = async (): Promise<SimpleHighlight[]> => {
      const groups = await this.groupsOf(program);
      const out: SimpleHighlight[] = [];
      for (const [row, from, to, at] of pieces) {
        const start = starts[row] ?? 0;
        let run = from;
        for (let k = from + 1; k <= to; k++) {
          const group = groups[at + run - from];
          if (k < to && groups[at + k - from] === group) continue;
          if (group) out.push([start + run, start + k, group]);
          run = k;
        }
      }
      for (const [row, from, to, group] of fixed)
        out.push([(starts[row] ?? 0) + from, (starts[row] ?? 0) + to, group]);
      return [...out, ...marks];
    };
    const target = this.code(texts.join("\n"), marks, hoverFold, highlights);
    const view = new LineNumberRenderable(this.renderer, {
      target,
      paddingRight: space.inset,
      fg: c.faint,
      minWidth: 3,
      lineNumbers: numbers,
      hideLineNumbers: hidden,
      lineColors: colors,
      lineSigns: signs,
    });
    // The gutter of the numbers takes the events of the pointer over the program, so it shows the string of a fold.
    view.onMouseMove = (event) => {
      if (hoverFold(event)) return;
      clearTimeout(this.hoverTimer);
      this.unhover();
    };
    // A press that ends a drag selects text and opens nothing. A press on a fold opens its string, and a press on a
    // string that shows whole folds it again.
    view.onMouseUp = (event) => {
      const row = event.y - view.y;
      const line = sources[row];
      if (line?.kind !== "code" || this.renderer.getSelection()?.getSelectedText()) return;
      const hit = foldAt(event.x, event.y);
      if (hit) return this.openString(hit.line, hit.fold.literal);
      const open = this.openStrings(line);
      const index = (whole.get(row) ?? -1) + event.x - target.x;
      const literal = literals(line.text).findIndex(
        (one, k) => open.has(k) && whole.has(row) && index >= one.start - 1 && index <= one.end,
      );
      if (literal >= 0) return this.openString(line, literal);
      if (line.acts.length || Bun.stringWidth(line.text) > column) this.openLine(line);
    };
    return view;
  }
  /** The strings of a line of a program that the operator opened, which show whole. */
  private openStrings(line: Extract<ProgramLine, { kind: "code" }>): Set<number> {
    const folds = this.session.folds;
    const key = `string:${line.step.id}:${line.number}:`;
    // Each frame asks of each line of each program, so the lines with an open string are read once from the folds.
    if (this.stringLines?.folds !== folds)
      this.stringLines = {
        folds,
        lines: new Set(
          Object.keys(folds)
            .filter((one) => one.startsWith("string:") && folds[one] === false)
            .map((one) => one.slice(0, one.lastIndexOf(":") + 1)),
        ),
      };
    if (!this.stringLines.lines.has(key)) return new Set();
    return new Set([...literals(line.text).keys()].filter((k) => folds[key + k] === false));
  }
  /** The lines of the programs whose strings the operator opened, as read from the folds of the session. */
  private stringLines?: { folds: Record<string, boolean>; lines: Set<string> };
  /** A string of a line of a program opens in place, or folds again. */
  private openString(line: Extract<ProgramLine, { kind: "code" }>, literal: number): void {
    const w = this.session;
    w.folds[`string:${line.step.id}:${line.number}:${literal}`] = this.openStrings(line).has(literal);
    this.stringLines = undefined;
    w.save();
    this.unhover();
    // A feed that follows its end would push the line up as it grows, so the view keeps its offset.
    const top = this.scroll.scrollTop;
    this.renderContent();
    this.scrollNow(top);
    this.scrollAfterLayout(top);
  }
  /** Whether a line of a program stands open: its whole text, and what its acts gave under it. */
  private lineOpen(line: ProgramLine): boolean {
    return line.kind === "code" && this.session.folds[`line:${line.step.id}:${line.number}`] === false;
  }
  /** A line of a program opens, or folds again. */
  private openLine(line: ProgramLine): void {
    if (line.kind !== "code") return;
    const w = this.session;
    w.folds[`line:${line.step.id}:${line.number}`] = this.lineOpen(line);
    w.save();
    this.renderContent();
  }
  /** The quiet hint of a line of a program: what a part of an extension draws as the row of each act of the line, and
   * else what its acts came to. */
  private lineHint(line: Extract<ProgramLine, { kind: "code" }>): string {
    if (!this.partRows) return line.hint;
    const drawn = line.acts.map((act) => this.partRow(act as ActRow)?.label).filter(Boolean);
    return drawn.length ? drawn.join("  ") : line.hint;
  }
  /** Python with the number of each line. */
  private numbered(
    word: string,
    options: LineNumberOptions = { fg: c.faint, minWidth: 3 },
  ): LineNumberRenderable {
    return new LineNumberRenderable(this.renderer, {
      target: this.code(word),
      paddingRight: space.inset,
      ...options,
    });
  }
  /** A text of a dialog, which scrolls past the rows it takes. */
  private document(node: Renderable, options: ScrollBoxOptions): ScrollBoxRenderable {
    const document = new ScrollBoxRenderable(this.renderer, { scrollX: false, scrollY: true, ...options });
    document.add(node);
    return document;
  }
  /** A card under the pointer, which a press opens. */
  private hoverCard(x: number, y: number, rows: number, open: () => void, ...nodes: Renderable[]): void {
    const width = Math.min(58, this.renderer.width - 4);
    this.popup(
      {
        left: Math.max(1, Math.min(x, this.renderer.width - width - 1)),
        top: Math.max(1, Math.min(y + 1, this.renderer.height - rows)),
        width,
        maxHeight: rows,
        paddingX: space.between,
        paddingY: space.inset,
        onMouseDown: open,
      },
      ...nodes,
    );
  }

  private reference(label: string, value: string): TextRenderable {
    const node = this.text(label, c.link, { attributes: underline });
    node.onMouseDown = () => {
      void this.follow(value).catch(this.report);
    };
    node.onMouseOver = (event) => {
      void this.referenceHover(value, event.x, event.y);
    };
    node.onMouseOut = () => {
      this.unhover();
    };
    return node;
  }

  private async follow(value: string): Promise<void> {
    this.unhover();
    const act = this.session.actOf(value);
    if (value.startsWith("furb-image://")) this.imageActions(value);
    else if (act?.kind === "chain") await this.session.select(act.id);
    else if (act && act.id === value && act.on === this.session.selected) this.go("feed", act.id);
    else this.showValue(value, await this.referenced(value));
  }

  /** What a reference holds, as a view reads it: the text of a door, which the outcome of its act holds under the
   * path of that door as an Exit holds its streams; the act the door opens on, whole, while its outcome holds no
   * such text; or the text that the host reads at the path. This read is no act of the operator, so it makes none
   * and the journal keeps nothing of it. */
  private async referenced(value: string): Promise<unknown> {
    const act = this.session.actOf(value);
    if (!act) return (await this.session.host.look(value, this.session.selected)).content;
    const whole = (await this.session.host.act(act.id)) ?? act;
    const held = whole.value && typeof whole.value === "object" ? Object.values(whole.value) : [];
    const door = held.find(
      (one): one is { path: string; content: string } =>
        Boolean(one) && typeof one === "object" && (one as { path?: unknown }).path === value,
    );
    return door ? door.content : whole;
  }

  private async referenceHover(value: string, x: number, y: number): Promise<void> {
    try {
      const act = this.session.acts.find((act) => act.id === value);
      const detail = value.startsWith("furb-image://")
        ? "Image attachment. Click to open its actions."
        : act
          ? `${act.kind}  ${act.done ? display(act.value) : "pending"}`
          : display(await this.referenced(value));
      if (this.closed || this.overlay) return;
      this.hoverCard(
        x,
        y,
        8,
        () => void this.follow(value).catch(this.report),
        this.text(value, c.link, { attributes: bold }),
        this.text(detail.slice(0, 400), c.muted, { maxHeight: 4 }),
      );
    } catch {
      /* A path can have disappeared since the turn was written. */
    }
  }

  /** The python of a user turn: a header is `#` and the name of an act or the kind of a query, a comment is `#` and
   * a space, and a header that names an act and an image attachment are references. */
  private transcriptText(source: string): TextRenderable {
    const text = safeText(source);
    const chunks: TextChunk[] = [];
    let at = 0;
    for (const match of text.matchAll(/^#(?! |$)\S+|^#(?: .*)?$|furb-image:\/\/[\w.]+/gm)) {
      if (match.index > at) chunks.push({ __isChunk: true, text: text.slice(at, match.index), fg: c.text });
      chunks.push({
        __isChunk: true,
        text: match[0],
        fg: match[0].startsWith("furb-image://")
          ? c.syntaxString
          : /^#(?! |$)/.test(match[0])
            ? c.syntaxKeyword
            : c.syntaxComment,
      });
      at = match.index + match[0].length;
    }
    chunks.push({ __isChunk: true, text: text.slice(at), fg: c.text });
    return this.pointable(
      new TextRenderable(this.renderer, { content: new StyledText(chunks), wrapMode: "word", flexShrink: 0 }),
      text,
    );
  }

  private renderRail(): void {
    if (!this.rail.visible) return;
    this.renderRailSession();
    this.renderWorkspaces();
    this.renderRailUsage();
  }
  /** What each act that a thread waits on does, and what each thread that ended came to, which hold while their act
   * and their state stay, so that a frame reads none of them again. */
  private doing = new Map<string, string>();
  /** The sidebar of the session: each question that waits for the operator, which a click answers in place, then the
   * threads of the life as a tree of their chains, each with what it does now, and the finished ones folded at the
   * foot. */
  private renderRailSession(): void {
    const w = this.session;
    // The layout gives the sidebar its width, which the preferences only ask for.
    const width = this.rail.width || w.preferences.sidebarWidth;
    const inner = width - space.between * 2;
    if (this.railChain && w.chains.some((chain) => chain.id === this.railChain)) {
      this.renderThreadList(this.railChain, width, inner);
      return;
    }
    this.railChain = undefined;
    const loom = this.lifeLoom();
    const rows = loom.rows.filter((row) => row.id !== "operator");
    const asks = w.desk.asks.filter((ask) => !ask.notice);
    const find = (id: string) => w.actOf(id);
    const remember = (key: string, make: () => string): string => {
      const known = this.doing.get(key);
      if (known !== undefined) return known;
      if (this.doing.size > 4000) this.doing.clear();
      const made = make();
      this.doing.set(key, made);
      return made;
    };
    const label = (chain: string) => w.labelOf(chain).split("/").at(-1) ?? "";
    // A thread that works says what its newest act does and since when, in minutes, so that a row changes at most once
    // a minute while nothing needs the operator.
    const said = (row: (typeof rows)[number]): [string, RGBA, string] => {
      if (row.state === "asks") return ["asks you", c.warning, ""];
      if (row.state === "paused") return ["paused", c.warning, ""];
      // A thread that ended says what it came to: the first line of its answer, or why it failed.
      if (row.state === "failed" || row.state === "done") {
        const text = remember(`${row.state} ${row.id} ${row.measure?.answers ?? 0}`, () => {
          return nowOf(row, loom, w.acts, (rung) => w.program[rung.id] ?? "").text;
        });
        return row.state === "failed"
          ? [`failed${text ? `: ${text}` : ""}`, c.danger, ""]
          : [text || "done", c.faint, ""];
      }
      if (row.state !== "working") return ["", c.faint, ""];
      if (!row.waits) return ["writes a step", c.muted, ""];
      const waits = row.waits;
      const text = remember(`act ${waits}`, () => {
        const act = find(waits);
        return act ? doingOf(act, find, label) : "writes a step";
      });
      const began = w.started[waits];
      const minutes = began === undefined ? 0 : Math.floor((Date.now() - began) / 60000);
      const since = minutes < 1 ? "" : minutes < 60 ? `${minutes}m` : `${Math.floor(minutes / 60)}h`;
      return [text, minutes >= 15 ? c.warning : c.muted, since];
    };
    // A thread stands in the tree while it or a thread under it needs a look: it works, asks, failed or is paused. The
    // root and the thread shown stand always, and the others fold under Finished at the foot.
    const looks = (row: (typeof rows)[number]) => !["done", "idle"].includes(row.state);
    const depth = (row: (typeof rows)[number]) => row.lines.length / 2;
    const standing = rows.filter((row, at) => {
      if (row.id === w.engine.root || row.id === w.selected || looks(row)) return true;
      for (const below of rows.slice(at + 1)) {
        if (depth(below) <= depth(row)) return false;
        if (looks(below)) return true;
      }
      return false;
    });
    const resting = rows.filter((row) => !standing.includes(row));
    const lines = standing.map((row) => ({ row, said: said(row) }));
    this.railRows = [...standing, ...(this.unfolded.has("finished") ? resting.slice(0, 12) : [])].map(
      (row) => row.id,
    );
    const key = JSON.stringify([
      this.theme,
      width,
      w.selected,
      asks.map((ask) => [ask.id, ask.label, ask.message, ask.choices]),
      lines.map(({ row, said: [text, , since] }) => [row.id, row.label, row.state, row.lines, text, since]),
      resting.map((row) => [row.id, row.label]),
      this.unfolded.has("finished"),
      this.choice,
      this.railKeys && this.railAt,
    ]);
    if (key === this.railKey) return;
    this.railKey = key;
    this.clear(this.railSession);
    const add = (parts: Part[], options: TextShape = {}) => {
      const node = this.line(parts, c.muted, { truncate: true, ...options });
      this.railSession.add(node);
      return node;
    };
    const heading = (name: string, count: Part[]) => {
      const room = Math.max(1, inner - Bun.stringWidth(name + plain(count)));
      add([[name, c.text, bold], [" ".repeat(room)], ...count]);
    };
    this.renderNeeds(asks, inner);
    const working = rows.filter((row) => row.state === "working").length;
    heading("Chains", [
      [working ? `${glyph.running} ${working}` : "", c.accent],
      [working && resting.length ? "  " : ""],
      [resting.length ? `${glyph.done} ${resting.length}` : "", c.faint],
    ]);
    // A row of a thread spans the sidebar past its padding, with the tone of its thread as a bar at its left edge. The
    // threads of the first level stand with no guide, and the lines of the tree show from the second level.
    const edge = { marginX: -space.between, paddingRight: space.inset };
    const named = [...standing, ...(this.unfolded.has("finished") ? resting.slice(0, 12) : [])];
    const name = Math.min(
      18,
      Math.floor(width * 0.38),
      Math.max(8, ...named.map((row) => Bun.stringWidth(label(row.id)) + 2)),
    );
    const threadRow = (row: (typeof rows)[number], [text, color, since]: [string, RGBA, string]) => {
      const selected = row.id === w.selected;
      const [mark, tone] = chainMarks[row.state];
      const guide = depth(row) ? `  ${row.lines.slice(2)}` : "";
      const title = clip(label(row.id), Math.max(4, name - Bun.stringWidth(guide)));
      // The row spans the sidebar, so its room is the width of the sidebar less its bar, its padding at the right, and
      // the parts before and after the words.
      const room = Math.max(
        0,
        width - 1 - Bun.stringWidth(guide) - 2 - name - 1 - (since ? since.length + 1 : 0) - space.inset,
      );
      this.railSession.add(
        this.railRow(
          [
            [glyph.mark, threadTone(row.id)],
            [guide, c.faint],
            [`${mark()} `, tone()],
            [
              title.padEnd(Math.max(4, name - Bun.stringWidth(guide))),
              selected ? c.text : c.muted,
              selected ? bold : 0,
            ],
            [" "],
            [clip(text, room).padEnd(room), color],
            [since ? ` ${since}` : "", c.faint],
          ],
          () => this.openThreads(row.id),
          edge,
          selected || this.cursorOn(row.id),
        ),
      );
    };
    for (const { row, said: told } of lines) threadRow(row, told);
    if (resting.length) {
      this.railSession.add(
        this.foldRow("finished", " Finished", resting.length, { ...edge, paddingLeft: 1 }),
      );
      if (this.unfolded.has("finished")) for (const row of resting.slice(0, 12)) threadRow(row, said(row));
    }
  }
  /** The context of the chain shown, at the foot of the sidebar: how full its window is and where a grant pauses it,
   * its tokens, and what it spent. */
  private renderRailUsage(): void {
    const w = this.session;
    const window = w.filled;
    const grant = w.open.find((act) => act.kind === "grant");
    const width = this.rail.width || w.preferences.sidebarWidth;
    if (
      !this.paneChanged(this.railUsage, [this.theme, w.spend, window, width, w.context, grant?.words, w.demo])
    )
      return;
    this.clear(this.railUsage);
    const inner = width - space.between * 2;
    const add = (parts: Part[], options: TextShape = {}) => {
      const node = this.line(parts, c.muted, { truncate: true, ...options });
      this.railUsage.add(node);
      return node;
    };
    // A line of a table: its name and its note at its left, and its value at its right.
    const spread = (name: Part, note: string, value: Part, options: TextShape = {}) => {
      const room = Math.max(1, inner - Bun.stringWidth(name[0] + note + value[0]));
      return add([name, [note, c.faint], [" ".repeat(room)], value], options);
    };
    const row = (name: string, value: string, color = c.text) => spread([name, c.muted], "", [value, color]);
    const section = (name: string, value = "", note = "") =>
      spread([name, c.text, bold], note, [value, c.faint]);
    const spend = w.spend;
    const ceiling =
      grant?.words[1] === null || grant?.words[1] === undefined ? undefined : Number(grant.words[1]);
    const known = window !== undefined && Number.isFinite(window);
    const tokens = spend.input + spend.output + spend.cacheRead + spend.cacheWrite;
    // A chain with a source reads the prompt of its origin before it answers once, so its context shows from its birth.
    this.railUsage.visible = tokens > 0 || spend.dollars > 0 || Boolean(grant) || w.context !== undefined;
    if (this.railUsage.visible) {
      section(
        "Context",
        known ? `${Number(((window ?? 0) * 100).toFixed(1))}%` : "",
        w.demo ? "  simulated" : "",
      );
      // The meter fills with the share of the window that the last answer used, and marks the ceiling of a grant. It
      // stands empty until an answer tells the share, and a hover over it says the share and where the chain pauses.
      const part = known ? (window ?? 0) : 0;
      const used = Math.min(inner, Math.max(part > 0 ? 1 : 0, Math.round(part * inner)));
      const mark = ceiling === undefined ? -1 : Math.min(inner - 1, Math.round(ceiling * inner));
      const tone = part >= 0.9 ? c.danger : part >= 0.7 ? c.warning : c.accent;
      const cells: Part[] = [];
      for (let at = 0; at < inner; at++)
        cells.push(at === mark ? ["╋", c.warning] : [glyph.meter, at < used ? tone : c.border]);
      const tip: Part[] = [
        [known ? `${Number((part * 100).toFixed(1))}%` : "No answer yet", c.text, bold],
        [known ? " of the context window" : "", c.muted],
        [ceiling === undefined ? "" : `, pauses at ${Number((ceiling * 100).toFixed(1))}%`, c.warning],
      ];
      this.tipped(add(cells), tip);
      // The last prompt is the whole prompt of the last answer, its system prompt and the reads of the cache included,
      // which the share of the window measures. The fresh input, the output and the cache are what every answer of the
      // chain read past the cache, wrote, and read and wrote in the cache, each token counted once.
      const prompt = w.context;
      if (prompt !== undefined)
        this.tipped(row("Last prompt", count(prompt)), [
          ["The whole prompt of the last answer, its system prompt and the cache included.", c.text],
        ]);
      if (tokens > 0) {
        this.tipped(row("Fresh input", count(spend.input)), [
          ["The input of the answers of the chain that the cache did not hold.", c.text],
        ]);
        row("Output", count(spend.output));
        if (spend.cacheRead) row("Cache read", count(spend.cacheRead));
        if (spend.cacheWrite) row("Cache write", count(spend.cacheWrite));
      }
      row("Spent", dollars(spend.dollars));
      if (grant && grant.words[0] !== null) row("Ceiling", dollars(Number(grant.words[0])), c.muted);
    }
  }
  /** Each question that waits for the operator, on top of the sidebar over the tree and over the list of threads, which
   * a click answers in place: a question never hides. */
  private renderNeeds(asks: readonly Ask[], inner: number): void {
    const w = this.session;
    const add = (parts: Part[], options: TextShape = {}) => {
      const node = this.line(parts, c.muted, { truncate: true, ...options });
      this.railSession.add(node);
      return node;
    };
    const heading = (name: string, count: Part[]) => {
      const room = Math.max(1, inner - Bun.stringWidth(name + plain(count)));
      add([[name, c.text, bold], [" ".repeat(room)], ...count]);
    };
    const label = (chain: string) => w.labelOf(chain).split("/").at(-1) ?? "";
    if (asks.length) {
      heading("Needs you", [[`${asks.length}`, c.warning, bold]]);
      for (const ask of asks) {
        const parent = w.labelOf(ask.chain).split("/").slice(0, -1).join("/");
        add(
          [
            [`${glyph.asks} `, c.warning],
            [label(ask.chain), c.text, bold],
            [parent ? `  in ${parent}` : "", c.faint],
          ],
          { marginTop: space.section, run: () => void w.point(ask.id).catch(this.report) },
        );
        for (const text of wrapped(shortenHomes(gist(ask.message)), inner - 2, 2))
          add([["  "], [text, c.text]], { run: () => this.question(ask) });
        const controls = this.row({ gap: 1, paddingLeft: 2, flexWrap: "wrap" });
        const chip = (text: string, run: () => void) =>
          controls.add(this.text([[` ${text} `, c.text, 0, c.raised]], c.text, { run }));
        if (ask.choices.length) {
          for (const [at, value] of ask.choices.entries())
            chip(`${at + 1} ${value}`, () => void w.answer(ask, value).catch(this.report));
          chip(`${ask.choices.length + 1} Other`, () => {
            this.choice = { id: ask.id, at: ask.choices.length, other: true };
            void w.point(ask.id).catch(this.report);
          });
        } else chip(`Answer as ${ask.shape}`, () => void w.point(ask.id).catch(this.report));
        this.railSession.add(controls);
      }
      add([], { marginTop: space.section });
    }
  }
  /** The threads of the chain shown, and each by its id. */
  private chainThreadList(chain: string): ChainThread[] {
    return this.chainThreadsBy(chain).list;
  }
  /** What each prompt that the list of the threads read is: a message to a thread, or a thread and its title. */
  private readonly promptsRead = new Map<string, { message: string | undefined; title: string }>();
  private chainThreadsBy(chain: string): ThreadList {
    const w = this.session;
    if (w.selected !== chain) return { acts: undefined, owed: "", list: [], by: new Map(), of: new Map() };
    const owed = [...w.host.prompts.keys()].join(" ");
    if (this.threadList.acts !== w.version || this.threadList.owed !== owed) {
      const index = w.ladderIndex;
      const list = chainThreads(index.on, index.ids, new Set(w.host.prompts.keys()), this.promptsRead);
      this.threadList = {
        acts: w.version,
        owed,
        list,
        by: new Map(list.map((one) => [one.act.id, one])),
        of: new Map(
          list.flatMap((one) => one.messages.map((message): [string, ChainThread] => [message.id, one])),
        ),
      };
    }
    return this.threadList;
  }
  /** How many threads the list of a chain shows at once. */
  private get railRoom(): number {
    return Math.max(5, Math.floor(this.renderer.height / 2) - 5);
  }
  /** Whether the keys of the sidebar stand on a row. */
  private cursorOn(id: string): boolean {
    return this.railKeys && this.railAt === id;
  }
  /** The threads of a chain in the sidebar, in place of the tree of the chains: a row that goes back to the tree, then
   * a row for each prompt of the chain, in the color of its thread, with its state and the first line of its message.
   * The thread chosen, or under the keys, shows its controls under its row. The list scrolls by the wheel and by the
   * keys. */
  private renderThreadList(chain: string, width: number, inner: number): void {
    const w = this.session;
    const list = this.chainThreadList(chain);
    const room = this.railRoom;
    this.railFrom = Math.max(0, Math.min(Math.max(0, list.length - room), this.railFrom));
    const from = this.railFrom;
    const questions = w.desk.asks.filter((ask) => !ask.notice);
    const key = JSON.stringify([
      "threads",
      questions.map((ask) => [ask.id, ask.label, ask.message, ask.choices]),
      this.choice,
      this.theme,
      width,
      chain,
      w.labelOf(chain),
      w.thread,
      this.railKeys && this.railAt,
      from,
      room,
      list.map((one) => [one.act.id, one.state, one.resolved, one.title, this.threadFolded(one.act.id)]),
    ]);
    if (key === this.railKey) return;
    this.railKey = key;
    this.clear(this.railSession);
    this.railSession.onMouseScroll = (event) => {
      const direction = event.scroll?.direction;
      if (!this.railChain || (direction !== "up" && direction !== "down")) return;
      this.railFrom = Math.max(0, this.railFrom + (direction === "up" ? -3 : 3));
      this.render();
    };
    this.renderNeeds(questions, inner);
    const edge = { marginX: -space.between, paddingRight: space.inset };
    const line = (parts: Part[], run?: () => void) =>
      this.railSession.add(this.line(parts, c.muted, { truncate: true, ...(run ? { run } : {}) }));
    this.railSession.add(
      this.railRow(
        [
          ["← ", c.accent],
          ["Chains", c.muted],
        ],
        () => this.closeThreads(),
        { ...edge, paddingLeft: 1 },
      ),
    );
    let asks = 0;
    let running = 0;
    let resolved = 0;
    for (const one of list)
      if (one.state === "asks") asks++;
      else if (one.resolved) resolved++;
      else running++;
    const tally: Part[] = [
      [asks ? `${glyph.asks} ${asks}  ` : "", c.warning],
      [running ? `${glyph.running} ${running}  ` : "", c.accent],
      [resolved ? `${glyph.done} ${resolved}` : "", c.faint],
    ];
    const name = clip(w.labelOf(chain), Math.max(4, inner - Bun.stringWidth(plain(tally)) - 1));
    line([
      [name, c.text, bold],
      [" ".repeat(Math.max(1, inner - Bun.stringWidth(name + plain(tally))))],
      ...tally,
    ]);
    if (!list.length) line([["No thread on this chain yet.", c.faint]]);
    if (from > 0)
      line([[`↑ ${from} earlier`, c.faint]], () => {
        this.railFrom = Math.max(0, from - room);
        this.render();
      });
    for (const one of list.slice(from, from + room)) {
      const id = one.act.id;
      const chosen = w.thread === id;
      const folded = this.threadFolded(id);
      const [mark, tone] = threadMarks(one);
      const text = clip(shortenHomes(one.title) || id, Math.max(4, width - 6 - space.inset));
      this.railSession.add(
        this.railRow(
          [
            [glyph.mark, threadTone(id)],
            [`${mark} `, tone],
            [folded ? `${glyph.closed} ` : "", c.faint],
            [text, chosen ? c.text : one.resolved ? c.faint : c.muted, chosen ? bold : 0],
          ],
          () => this.chooseThread(id),
          edge,
          chosen || this.cursorOn(id),
        ),
      );
      if (!chosen && !this.cursorOn(id)) continue;
      const controls = this.row({ gap: 1, paddingLeft: 3, flexWrap: "wrap" });
      const chip = (label: string, run: () => void) =>
        controls.add(this.text([[` ${label} `, c.text, 0, c.raised]], c.text, { run }));
      chip(folded ? "Expand" : "Collapse", () => this.foldThread(id));
      if (one.resolved) chip("Open again", () => this.reopenThread(id));
      else chip("Resolve", () => this.resolveThread(one));
      const ask = one.asks[0];
      if (ask) chip("Answer", () => void w.point(ask).catch(this.report));
      this.railSession.add(controls);
    }
    const after = list.length - from - room;
    if (after > 0)
      line([[`↓ ${after} later`, c.faint]], () => {
        this.railFrom = from + room;
        this.render();
      });
  }
  /** A chain chosen in the tree: the feed shows it, and the sidebar lists its threads in place of the tree. */
  openThreads(chain: string): void {
    this.railChain = chain;
    this.railFrom = Number.MAX_SAFE_INTEGER;
    void this.session.select(chain).catch(this.report);
    const list = this.chainThreadList(chain);
    this.railAt = (list.findLast((one) => !one.resolved) ?? list.at(-1))?.act.id;
    this.render();
  }
  /** The tree of the chains again, in place of the list of the threads of a chain, with the keys on that chain. */
  closeThreads(): void {
    if (this.railChain) this.railAt = this.railChain;
    this.railChain = undefined;
    this.session.thread = undefined;
    this.render();
  }
  /** A thread chosen in the list: the feed unfolds it and scrolls to it, and the input sends each message to it. */
  private chooseThread(id: string): void {
    const w = this.session;
    this.railAt = id;
    w.thread = id;
    if (this.threadFolded(id)) {
      w.folds[`thread:${id}`] = false;
      w.save();
    }
    if (this.railKeys) this.railSteer(false);
    this.go("feed", id);
  }
  /** Whether a thread stands folded to its header in the feed: as the operator left it, and else when it is resolved,
   * is not the last thread of its chain, and the preference folds what is over. */
  private threadFolded(id: string): boolean {
    const w = this.session;
    const set = w.folds[`thread:${id}`];
    if (set !== undefined) return set;
    const own = this.chainThreadsBy(w.selected);
    const thread = own.by.get(id);
    return Boolean(w.preferences.foldThreads && thread?.resolved && own.list.at(-1) !== thread);
  }
  /** A thread folds in the feed to its header line alone, or unfolds. */
  private foldThread(id: string, fold = !this.threadFolded(id)): void {
    const w = this.session;
    w.folds[`thread:${id}`] = fold;
    w.save();
    this.renderContent();
    this.render();
  }
  /** A thread resolves: its open prompt closes, and the model that works for it stops. */
  private resolveThread(thread: ChainThread): void {
    const open = [thread.act, ...thread.messages].findLast((act) => !act.done);
    if (open) void this.session.resolve(open.id).catch(this.report);
  }
  /** A thread opens again: the input takes the message that goes to it. */
  private reopenThread(id: string): void {
    const w = this.session;
    w.thread = id;
    this.railAt = id;
    w.notice = `Your next message goes to ${id} and opens it again.`;
    if (this.railKeys) this.railSteer(false);
    else this.composer.focus();
    this.render();
  }
  /** The keys move in the sidebar, or go back to the input. */
  private railSteer(on = !this.railKeys): void {
    this.railKeys = on;
    if (on) {
      if (this.steering) this.steer(false);
      this.composer.blur();
      if (!this.railAt)
        this.railAt = this.railChain
          ? (this.session.thread ?? this.chainThreadList(this.railChain).at(-1)?.act.id)
          : this.session.selected;
    } else this.composer.focus();
    this.render();
  }
  /** A step of the keys of the sidebar, in the tree or in the list, which scrolls the list so that the row shows. */
  private moveRail(by: number): void {
    const ids = this.railChain
      ? this.chainThreadList(this.railChain).map((one) => one.act.id)
      : this.railRows;
    if (!ids.length) return;
    const at = ids.indexOf(this.railAt ?? "");
    const next = Math.max(0, Math.min(ids.length - 1, at < 0 ? ids.length - 1 : at + by));
    this.railAt = ids[next];
    if (this.railChain) {
      if (next < this.railFrom) this.railFrom = next;
      else if (next >= this.railFrom + this.railRoom) this.railFrom = next - this.railRoom + 1;
    }
    this.render();
  }
  /** Enter or → in the sidebar: a chain lists its threads, and a thread is chosen. */
  private enterRail(): void {
    const at = this.railAt;
    if (!at) return;
    if (this.railChain) this.chooseThread(at);
    else this.openThreads(at);
  }
  /** ← or Esc in the sidebar: the list goes back to the tree, and the tree gives the keys back to the input. */
  private leaveRail(): void {
    if (this.railChain) this.closeThreads();
    else this.railSteer(false);
  }
  /** The thread under the keys of the list. */
  private railThread(): ChainThread | undefined {
    return this.railChain ? this.chainThreadsBy(this.railChain).by.get(this.railAt ?? "") : undefined;
  }
  /** The controls of a thread on the header of the thread in the feed: collapse it, and resolve it or open it again.
   * A message of a thread and a prompt that heads no thread have none. */
  private threadControls(act: ActRow): BoxRenderable | undefined {
    const thread = this.chainThreadsBy(this.session.selected).by.get(act.id);
    if (!thread) return undefined;
    const id = thread.act.id;
    const row = this.row({ gap: 3, paddingLeft: space.between });
    row.add(
      this.link(
        [
          [`${glyph.open} `, c.faint],
          ["Collapse", c.faint],
        ],
        () => this.foldThread(id, true),
      ),
    );
    if (thread?.resolved)
      row.add(
        this.link(
          [
            [`${glyph.ring} `, c.faint],
            ["Open again", c.faint],
          ],
          () => this.reopenThread(id),
        ),
      );
    else if (thread)
      row.add(
        this.link(
          [
            [`${glyph.done} `, c.faint],
            ["Resolve", c.faint],
          ],
          () => this.resolveThread(thread),
        ),
      );
    return row;
  }
  /** A row of the sidebar, whose click runs, which spans the sidebar so that the pointer and the selection show from
   * edge to edge. It lights while the pointer is on it, and stays lit while it is selected. */
  private railRow(parts: Part[], run: () => void, options: BoxOptions, selected = false): BoxRenderable {
    const row = this.hoverable(
      this.row({ backgroundColor: selected ? c.selected : c.panel, onMouseUp: this.click(run), ...options }),
      selected ? c.selected : c.raised,
    );
    row.add(this.text(parts, c.muted, { truncate: true, flexGrow: 1, flexShrink: 1 }));
    return row;
  }
  /** A row of the sidebar that folds a list under it, with its label and the count of the list, which a click folds
   * or unfolds. */
  private foldRow(key: string, label: string, count: number, options: BoxOptions): BoxRenderable {
    return this.railRow(
      [
        [this.unfolded.has(key) ? glyph.open : glyph.closed, c.faint],
        [label, c.faint],
        [`  ${count}`, c.faint],
      ],
      () => {
        if (!this.unfolded.delete(key)) this.unfolded.add(key);
        this.render();
      },
      options,
    );
  }
  /** A node that shows a tip while the pointer is over it. */
  private tipped<T extends Renderable>(node: T, parts: Part[]): T {
    node.onMouseOver = (event) => this.tip(parts, event.x, event.y);
    node.onMouseOut = () => this.unhover();
    return node;
  }
  /** The composer holds a text in place of the draft that the session shows, and an undo gives the draft back. */
  private insert(text: string): void {
    this.closeOverlay();
    if (this.draftKey !== this.session.draftKey) this.showDraft(this.session.draftKey);
    this.composer.replaceText(text);
    this.composer.focus();
  }
  /** The workspaces in the sidebar, under the session: each folder with its sessions, which scroll on their own. */
  private renderWorkspaces(): void {
    const library = this.options.workspaces;
    const width = this.session.preferences.sidebarWidth;
    if (
      !this.paneChanged(this.railSpaces, [
        this.theme,
        library.current?.path,
        width,
        library.notice,
        library.groups.map((group) => [
          group.name,
          group.directory,
          group.collapsed,
          group.sessions.map((entry) => [entry.path, entry.name, entry.status, entry.error, entry.archived]),
        ]),
        [...this.unfolded],
        this.renaming && this.itemKey(this.renaming.item),
        this.menuItem && this.itemKey(this.menuItem),
      ])
    )
      return;
    this.clear(this.railHeading);
    this.clear(this.railSpaces);
    this.unhover();
    const inner = width - space.between * 2;
    this.railHeading.add(this.text("Workspaces", c.text, { attributes: bold, flexGrow: 1 }));
    this.railHeading.add(this.text("⌃W", c.faint, { run: () => void this.workspacePicker() }));
    const padding = { paddingLeft: space.inset, paddingRight: space.between };
    const line = (parts: Part[], run: () => void, options: BoxOptions = {}) =>
      this.railSpaces.add(this.railRow(parts, run, { ...padding, ...options }));
    if (library.notice) this.railSpaces.add(this.inset(space.between, this.text(library.notice, c.warning)));
    if (!library.groups.length)
      this.railSpaces.add(
        this.inset(space.between, this.text("No workspaces yet. Add a project folder below.", c.muted)),
      );
    // A session and a workspace take one shape of row. While the pointer is on the row, it lights, and at its end a
    // button renames it and a button removes it, which asks first. The buttons are drawn in the color of the row until
    // then, so the row keeps its layout. The mark of the state tells the state in a tip, and a right click opens the
    // menu of the row. A row under a new name holds an input in place of its name.
    const itemRow = (
      item: SessionEntry | Workspace,
      parts: { lead: Part[]; name: Part; state?: Part; after?: boolean },
      tip: () => Part[],
      run: () => void,
      options: BoxOptions = {},
      selected = false,
    ) => {
      const noun = "sessions" in item ? "workspace" : "session";
      const lit = selected ? c.selected : c.raised;
      const ground = selected ? c.selected : this.menuItem === item ? lit : c.panel;
      // A click in the input of a new name only moves its cursor.
      const renaming = this.renaming?.item === item ? this.renaming : undefined;
      const row = this.row({
        paddingLeft: space.inset,
        paddingRight: space.between,
        backgroundColor: renaming ? lit : ground,
        ...(renaming ? {} : { onMouseUp: this.click(run) }),
        onMouseDown: (event) => {
          if (event.button !== 2) return;
          event.stopPropagation();
          this.itemMenu(item, event.x, event.y);
        },
        ...options,
      });
      row.add(this.line(parts.lead, c.muted));
      const state = parts.state && this.line([parts.state], c.muted);
      if (state && !parts.after) row.add(state);
      if (renaming) {
        const input = new InputRenderable(this.renderer, {
          flexGrow: 1,
          flexShrink: 1,
          value: renaming.value,
          ...inputColors(c.background),
        });
        input.on(InputRenderableEvents.INPUT, (value: string) => {
          renaming.value = value;
        });
        // A row that the sidebar draws again keeps the cursor where it was.
        if (renaming.cursor !== undefined) input.cursorOffset = renaming.cursor;
        input.onKeyDown = () => {
          queueMicrotask(() => {
            if (!input.isDestroyed) renaming.cursor = input.cursorOffset;
          });
        };
        input.on(InputRenderableEvents.ENTER, () => this.endRename(true));
        // The name is kept when the input loses its focus to another part of the screen, and not when the sidebar
        // draws the row again, which focuses its new input at once.
        input.on("blurred", () =>
          queueMicrotask(() => {
            if (this.renaming === renaming && !renaming.input?.focused) this.endRename(true);
          }),
        );
        renaming.input = input;
        row.add(input);
        this.railSpaces.add(row);
        input.focus();
        return row;
      }
      const name = this.line([parts.name], c.muted, {
        truncate: true,
        flexShrink: 1,
        flexGrow: parts.after ? 0 : 1,
      });
      row.add(name);
      if (state && parts.after) {
        state.marginLeft = space.between;
        row.add(state);
      }
      if (parts.after) row.add(this.box({ flexGrow: 1 }));
      const button = (mark: string, act: () => void) =>
        this.line(mark, ground, {
          marginLeft: space.inset,
          onMouseUp: this.click((event) => {
            event.stopPropagation();
            act();
          }),
        });
      const rename = button(glyph.rename, () => this.startRename(item));
      const remove = button(glyph.remove, () => this.removeItem(item));
      row.add(rename);
      row.add(remove);
      row.onMouseOver = (event) => {
        row.backgroundColor = lit;
        rename.fg = event.target === rename ? c.accent : c.muted;
        remove.fg = event.target === remove ? c.danger : c.muted;
        // A name that its room cuts shows whole in a tip of its own.
        if (event.target === name) return;
        this.unhover();
        if (event.isDragging) return;
        const told: Part[] | undefined =
          event.target === rename
            ? [[`Rename this ${noun}`, c.text]]
            : event.target === remove
              ? [
                  [
                    "sessions" in item
                      ? "Remove this workspace from the list"
                      : "Archive or remove this session",
                    c.text,
                  ],
                ]
              : event.target === state
                ? tip()
                : undefined;
        if (told) this.tip(told, event.x, event.y);
      };
      row.onMouseOut = () => {
        row.backgroundColor = ground;
        rename.fg = ground;
        remove.fg = ground;
        this.unhover();
      };
      this.railSpaces.add(row);
      return row;
    };
    for (const [index, group] of library.groups.entries()) {
      const status = library.groupStatus(group);
      const current = group === library.groupOf();
      const quiet = status === "saved" || status === "idle";
      const count = group.sessions.filter((entry) => entry.status === status).length;
      itemRow(
        group,
        {
          lead: [[" "], [`${group.collapsed ? glyph.closed : glyph.open} `, c.faint]],
          name: [group.name, current ? c.text : c.muted, bold],
          state: quiet ? undefined : statusMark(status),
          after: true,
        },
        () => [
          statusMark(status),
          [statusLabels[status], c.text, bold],
          [
            `  ${count} of ${group.sessions.length} ${group.sessions.length === 1 ? "session" : "sessions"}`,
            c.muted,
          ],
        ],
        () => library.toggle(group),
        { marginTop: index ? space.section : space.stack },
      );
      // A folded workspace shows its name alone, and an open one its folder under its name.
      if (group.collapsed) continue;
      this.railSpaces.add(
        this.inset(
          space.inset + 3,
          this.whole(
            this.line(clip(shortenHome(group.directory), inner - space.inset - 1, "end"), c.faint, {
              truncate: true,
            }),
            () => shortenHome(group.directory),
          ),
        ),
      );
      const sessionRow = (entry: SessionEntry) => {
        const selected = library.current === entry;
        itemRow(
          entry,
          {
            lead: [[selected ? glyph.mark : " ", c.accent], ["  "]],
            state: statusMark(entry.status),
            name: [entry.name, selected ? c.text : entry.archived ? c.faint : c.muted, selected ? bold : 0],
          },
          () => [
            statusMark(entry.status),
            [statusLabels[entry.status], c.text, bold],
            [entry.error ? `  ${entry.error}` : "", c.danger],
          ],
          () => {
            void library.select(entry).catch(this.report);
          },
          {},
          selected,
        );
      };
      // The sidebar shows the sessions that live: the one shown, and each that works, asks, waits for a wake or
      // failed, and the newest of the others up to three rows. The older sessions fold under a row of their own,
      // which a click opens.
      const kept = group.sessions.filter((entry) => !entry.archived);
      const live = (entry: SessionEntry) =>
        library.current === entry ||
        ["working", "opening", "blocked", "paused", "error"].includes(entry.status);
      const resting = kept.filter((entry) => !live(entry));
      const room = Math.max(0, 3 - (kept.length - resting.length));
      const shown = new Set([...kept.filter(live), ...resting.slice(0, room)]);
      for (const entry of kept.filter((entry) => shown.has(entry))) sessionRow(entry);
      const earlier = kept.filter((entry) => !shown.has(entry));
      if (earlier.length) {
        this.railSpaces.add(
          this.foldRow(`${group.directory}\nearlier`, "  Earlier", earlier.length, padding),
        );
        if (this.unfolded.has(`${group.directory}\nearlier`)) for (const entry of earlier) sessionRow(entry);
      }
      line([["   "], ["+ ", c.faint], ["New session", c.faint]], () => {
        void library.create(group).catch(this.report);
      });
      // The archived sessions fold under a row of their own, as the finished chains do, which a click opens.
      const archived = group.sessions.filter((entry) => entry.archived);
      if (archived.length) {
        this.railSpaces.add(this.foldRow(group.directory, "  Archived", archived.length, padding));
        if (this.unfolded.has(group.directory)) for (const entry of archived) sessionRow(entry);
      }
    }
    line([[" "], ["+ ", c.faint], ["Add a workspace", c.faint]], () => this.insert("/workspace "), {
      marginTop: space.section,
    });
  }
  /** A tip that stands under the pointer and ends at its left, or above it where the rows under it cannot hold it,
   * inside the screen. It never covers the row of the pointer, or the pointer would leave the text that shows it. */
  private tip(parts: Part[], x: number, y: number): void {
    const size = Math.min(
      this.renderer.width - 2,
      Math.max(25, Bun.stringWidth(plain(parts)) + space.between),
    );
    const rows = Math.max(1, Math.ceil(Bun.stringWidth(plain(parts)) / Math.max(1, size - space.inset * 2)));
    this.popup(
      {
        left: Math.max(1, Math.min(x - size, this.renderer.width - size - 1)),
        top: y + 1 + rows <= this.renderer.height ? y + 1 : Math.max(0, y - rows),
        width: size,
        paddingX: space.inset,
      },
      this.text(parts),
    );
  }
  /** A popup over the screen, in place of the one before it, which holds nodes until the pointer leaves what shows
   * it. */
  private popup(options: BoxOptions, ...nodes: Renderable[]): void {
    this.unhover();
    this.hover = this.box({ position: "absolute", backgroundColor: c.raised, zIndex: 30, ...options });
    for (const node of nodes) this.hover.add(node);
    this.root.add(this.hover);
  }
  /** The popup gone, where one shows. */
  private unhover(): void {
    this.hover?.destroyRecursively();
    this.hover = undefined;
  }
  /** Ask how to remove a session: archive it, which keeps its record under a row of its own, or move it to the trash.
   * An archived session offers to come back instead. */
  private removeSession(entry: SessionEntry): void {
    const library = this.options.workspaces;
    const group = library.groupOf(entry);
    if (!group) return;
    this.openPalette(
      `Remove the session “${entry.name}”?`,
      [
        entry.archived
          ? {
              label: "Restore",
              detail: "Bring it back to the list of its workspace",
              run: () => library.restore(entry),
            }
          : {
              label: "Archive",
              detail: "Fold it under Archived. Its record stays, and a click opens it again.",
              run: () => void library.archive(entry).catch(this.report),
            },
        {
          label: "Move to trash",
          color: c.danger,
          detail: "Stop this session and move its saved record to the trash",
          run: () => library.delete(entry).then(() => {}),
        },
        { label: "Keep", detail: "Return without changes", run() {} },
      ],
      {
        note: `The trash is ${shortenHome(join(group.directory, ".furb", "trash"))}, where you can get it back.`,
      },
    );
  }
  /** What names a session or a workspace for as long as it lives: the path of its record, or its folder. */
  private itemKey(item: SessionEntry | Workspace): string {
    return "sessions" in item ? item.directory : item.path;
  }
  /** Rename a session or a workspace in its row: an input takes the place of its name, Enter keeps what it holds,
   * and Escape leaves the name as it was. */
  private startRename(item: SessionEntry | Workspace): void {
    this.closeOverlay();
    this.renaming = { item, value: item.name };
    this.session.notice = "Enter keeps the new name, and Esc leaves it as it was.";
    this.render();
  }
  private endRename(keep: boolean): void {
    const renaming = this.renaming;
    if (!renaming) return;
    this.renaming = undefined;
    const name = renaming.value.trim();
    if (keep && name && name !== renaming.item.name) this.options.workspaces.rename(renaming.item, name);
    if (this.session.notice.startsWith("Enter keeps the new name")) this.session.notice = "";
    // The input gives its focus back to the composer, and a part that took the focus from it keeps it.
    const focused = this.renderer.currentFocusedRenderable;
    if (!focused || focused === renaming.input || focused.isDestroyed) this.composer.focus();
    this.render();
  }
  /** A cancel that ends running work asks first: it says how many acts it ends, and on which chains. */
  private confirm = (cancel: Cancel): void => {
    const { act, reach, back } = cancel;
    const ended = reach.acts.length;
    const acts = `${ended} running ${ended === 1 ? "act" : "acts"}`;
    const where = reach.chains.length === 1 ? "one chain" : `${reach.chains.length} chains`;
    this.openPalette(
      `${back ? "Take back" : "Cancel"} ${act.id}, and end ${acts} on ${where}?`,
      [
        {
          label: back ? "Take it back" : "Cancel it",
          color: c.danger,
          detail: `End ${act.id} and the ${acts} under it`,
          run: () => this.session.end(cancel),
        },
        { label: "Keep", detail: "Return without changes", run() {} },
      ],
      { note: `It ends work on ${this.session.chainsOf(reach)}.` },
    );
  };
  /** Ask how to remove a session, or whether a workspace leaves the list. */
  private removeItem(item: SessionEntry | Workspace): void {
    if (!("sessions" in item)) {
      this.removeSession(item);
      return;
    }
    const library = this.options.workspaces;
    this.openPalette(
      `Remove the workspace “${item.name}” from the list?`,
      [
        {
          label: "Remove from the list",
          detail: "Its folder and its sessions stay on disk, and /workspace adds it back",
          run: () => library.remove(item),
        },
        { label: "Keep", detail: "Return without changes", run() {} },
      ],
      { note: `The folder is ${shortenHome(item.directory)}.` },
    );
  }
  /** The menu that a right click on the row of a session or a workspace opens, at the pointer. */
  private itemMenu(item: SessionEntry | Workspace, x: number, y: number): void {
    const library = this.options.workspaces;
    const rename = { label: "Rename", detail: "", run: () => this.startRename(item) };
    const choices: Choice[] =
      "sessions" in item
        ? [
            { label: "New session", detail: "", run: () => library.create(item).then(() => {}) },
            rename,
            { label: item.collapsed ? "Unfold" : "Fold", detail: "", run: () => library.toggle(item) },
            { label: "Remove from the list", detail: "", run: () => this.removeItem(item) },
          ]
        : [
            ...(library.current === item
              ? []
              : [{ label: "Open", detail: "", run: () => library.select(item) }]),
            rename,
            item.archived
              ? { label: "Restore", detail: "", run: () => library.restore(item) }
              : { label: "Archive", detail: "", run: () => library.archive(item) },
            { label: "Move to trash", detail: "", color: c.danger, run: () => this.removeItem(item) },
          ];
    this.openPalette(item.name, choices, { at: { x, y } });
    this.menuItem = item;
    this.render();
  }
  /** The workspaces and their sessions in a palette, which opens once the workspaces are read again. */
  workspacePicker = (): Promise<void> => {
    const library = this.options.workspaces;
    return library
      .refresh()
      .then(() =>
        this.openPalette(
          "Workspaces and sessions",
          [
            {
              label: "Add a workspace",
              detail: "Open a project folder",
              mark: ["+ ", c.accent],
              run: () => this.insert("/workspace "),
            },
            // Each workspace is a part of the list with its folder in its title, and its sessions under it.
            ...library.groups.flatMap((group) => [
              ...group.sessions.map((entry) => ({
                label: entry.name,
                detail: sessionDetail(entry, library.current === entry),
                status: entry.status,
                heading: `${group.name}   ${shortenHome(group.directory)}`,
                run: () => library.select(entry),
              })),
              {
                label: "New session",
                detail: `Start a fresh life in ${group.name}`,
                mark: ["+ ", c.faint] as Part,
                heading: `${group.name}   ${shortenHome(group.directory)}`,
                run: async () => {
                  await library.create(group);
                },
              },
            ]),
          ],
          { note: "Open sessions keep running while you work in another one." },
        ),
      )
      .catch(this.report);
  };
  /** The commands that the view answers itself, by their text. */
  private async globalCommand(text: string): Promise<boolean> {
    const [name, ...words] = text.startsWith("/") ? text.slice(1).split(" ") : [];
    const argument = words.join(" ").trim();
    const library = this.options.workspaces;
    // A command with no argument that opens a picker or acts at once, and /inspect, which opens the value it names.
    const bare: Record<string, () => unknown> = {
      details: this.details,
      rewind: this.rewind,
      tree: this.sessionTree,
      queue: this.queuePicker,
      model: this.models,
      providers: this.providers,
      effort: this.effortPicker,
      shape: this.shapes,
      theme: () => this.themes(),
      exit: () => this.options.quit(),
      end: () => this.options.quit(true),
      editor: () => this.editDraft(),
      files: this.filesPicker,
      new: () => library.create(),
      sidebar: () => library.toggle(),
      workspace: this.workspacePicker,
      watch: this.watch,
      repair: () => this.session.repairJournal(),
      renew: () => this.session.renewJournal(),
    };
    if (name && !argument && Object.hasOwn(bare, name)) {
      await bare[name]?.();
      return true;
    }
    if (name === "serve") {
      const served = await this.session.host.serve(argument);
      const copy = (address: string, what: string) => () => {
        this.renderer.copyToClipboardOSC52(address);
        this.session.notice = `Copied the address ${what}.`;
      };
      this.openPalette(
        "Serve this life to other operators",
        [
          {
            label: "Copy the address to read and drive it",
            detail: served.write,
            run: copy(served.write, "to read and drive this life"),
          },
          {
            label: "Copy the address to read it only",
            detail: served.read,
            run: copy(served.read, "to read this life only"),
          },
        ],
        {
          note: "An operator attaches with furb-tui --attach <address>. Each address holds its key, so give it only to the operator that it is for.",
        },
      );
      return true;
    }
    if (name === "access") {
      if (!argument) this.accessPicker();
      else if (argument === "reader" || argument === "motion") this.toggleAccess(argument);
      else throw new Error("Choose reader or motion.");
      return true;
    }
    if (name === "view") {
      if (!argument) this.viewPicker();
      else if (isLifeView(argument)) this.switchView(argument);
      else throw new Error(`Choose ${lifeViews.join(", ")}.`);
      return true;
    }
    if (name === "inspect" && argument) {
      await this.inspect(argument);
      return true;
    }
    if (text === "/image" || text.startsWith("/image ")) {
      const path = text.slice(6).trim();
      if (path) await this.session.attachImage(path);
      else await clipboardImage((path) => this.session.attachImage(path));
      return true;
    }
    if (text === "/delete") {
      this.openPalette(
        "Delete a session",
        library.groups.flatMap((group) =>
          group.sessions.map((entry) => ({
            label: entry.name,
            detail: group.name,
            run: () => this.removeItem(entry),
          })),
        ),
      );
      return true;
    }
    if (text.startsWith("/workspace ")) {
      const group = await library.add(text.slice(11).trim());
      const entry = group.sessions[0];
      if (entry) await library.select(entry);
      else await library.create(group);
      return true;
    }
    // A command of a part of an extension, which no command of the TUI shadows.
    const part = name ? this.parts.command(name) : undefined;
    if (part) {
      this.partRun(() => part.run(argument, this.seen()));
      return true;
    }
    return false;
  }
  /** The views of the life to choose from, the one that shows marked. */
  private viewPicker(): void {
    this.openPalette(
      "View of the life",
      lifeViews.map((view) => ({
        label: `${lifeViewLabels[view]}${view === this.lifeView ? "  (shows)" : ""}`,
        detail: lifeViewDetails[view],
        run: () => this.switchView(view),
      })),
    );
  }
  private imageActions(uri: string): void {
    const content = imageContent(this.session.host.imageDirectory, uri);
    const file = imagePath(this.session.host.imageDirectory, uri).path;
    const pending = this.session.images[this.session.selected]?.find((image) => image.uri === uri);
    this.openPalette(pending?.name ?? "Image attachment", [
      {
        label: "Open image",
        detail: `${content.mimeType}  ${kibibytes(Buffer.byteLength(content.data, "base64"))}`,
        run: () => openFile(file),
      },
      ...(pending
        ? [
            {
              label: "Remove from this draft",
              detail: "Keep the saved image file",
              run: () => {
                this.session.images[this.session.selected] = (
                  this.session.images[this.session.selected] ?? []
                ).filter((image) => image.uri !== uri);
                this.session.save();
                this.render();
              },
            },
          ]
        : []),
    ]);
  }
  async editDraft(): Promise<void> {
    const value = await externalEditor(
      this.renderer,
      this.composer.plainText,
      this.writesPython,
      this.session.directory || this.session.host.directory,
    );
    // An editor ends a file with a line end, which the draft leaves out.
    if (!this.closed) {
      this.insert(value.replace(/\n$/, ""));
      this.session.notice = "The draft is back from the editor.";
      this.render();
    }
  }
  private filesPicker = async (): Promise<void> => {
    const files = await this.session.projectFiles(true);
    if (this.closed) return;
    this.openPalette(
      "Attach a project file",
      files.map((path) => ({
        label: path,
        detail: "Read into this chain with the next message",
        run: () => {
          const token = this.beforeCursor().match(/(?:^|\s)(@[^\s]*)$/)?.[1] ?? "";
          this.replaceBefore(token, `@${/\s/.test(path) ? JSON.stringify(path) : path} `);
        },
      })),
    );
  };
  private queuePicker = (): void => {
    const session = this.session;
    this.openPalette(
      "Queued follow-ups",
      [
        ...(session.queueHeld
          ? [
              {
                label: "Resume queue",
                detail: session.queueError || "Send when this chain's current work is complete",
                run: async () => {
                  session.queueHeld = false;
                  session.queueError = "";
                  await session.drainQueue();
                },
              },
            ]
          : []),
        ...session.queued.map((entry) => ({
          label: entry.text.split("\n")[0] ?? "Follow-up",
          // A follow-up asks the actor of its chain when it is sent, and the session reads that actor for the
          // selected chain alone.
          detail: `${entry.chain === session.selected ? `to ${this.model("").name} ` : ""}on ${session.labelOf(entry.chain)}`,
          mark: [`${glyph.ring} `, c.faint] as Part,
          run: () =>
            this.openPalette("Queued message", [
              {
                label: "Edit in composer",
                detail: "Remove from the queue and edit before sending again",
                run: async () => {
                  session.removeQueued(entry.id);
                  await session.select(entry.chain);
                  this.insert(entry.text);
                },
              },
              {
                label: "Remove",
                detail: "Remove this queued message",
                run: () => {
                  session.removeQueued(entry.id);
                },
              },
            ]),
        })),
      ],
      { note: "Sent in turn after the current work. Choose one to edit or remove it." },
    );
  };
  private shared = (path: string, markdown: string): void => {
    this.openPalette(
      "Conversation ready to share",
      [
        {
          label: "Open the page",
          detail: "Show the conversation in the browser",
          run: () => openFile(path),
        },
        {
          label: "Copy the path",
          detail: "Put the path of the page on the clipboard",
          run: () => {
            this.renderer.copyToClipboardOSC52(path);
            this.session.notice = "Export path copied.";
          },
        },
        {
          label: "Upload a secret gist",
          detail: "Anyone with the link can read it. Needs the GitHub CLI, signed in.",
          run: async () => {
            const url = await publishShare(path, markdown, this.session.sessionName);
            this.session.notice = `Shared: ${url}`;
            if (!this.closed)
              this.openPalette("Share link", [
                {
                  label: "Copy link",
                  detail: url,
                  run: () => {
                    this.renderer.copyToClipboardOSC52(url);
                    this.session.notice = "Share link copied.";
                  },
                },
              ]);
          },
        },
      ],
      { note: `The page is ${shortenHome(path)}.` },
    );
  };
  private action(command: string): void {
    this.closeOverlay();
    void this.globalCommand(command)
      .then((handled) => {
        if (!handled) return this.session.submit(command);
      })
      .catch(this.report);
  }
  private report = (error: unknown): void => {
    const message = error instanceof Error ? error.message : String(error);
    if (this.closed) {
      const session = this.options.workspaces.current?.session;
      if (session) session.notice = message;
    } else this.showValue("Could not complete action", message);
  };
  /** What the operator can do with an act that a right click chose. */
  private actActions(act: ActRow): void {
    const w = this.session;
    const message = w.isUserPrompt(act) && !asksOperator(act);
    this.openPalette(`${title(act.kind)} ${act.id}`, [
      {
        label: "Inspect",
        detail: "Its words, its state, and its value",
        run: () =>
          this.showValue(act.id, {
            kind: act.kind,
            by: act.by,
            words: act.words,
            done: act.done,
            value: act.value,
          }),
      },
      ...(w.editable(act)
        ? [
            {
              label: "Edit its program",
              detail: "Change the Python the model wrote for it, and replay it",
              run: () => this.action(`/edit ${act.id}`),
            },
          ]
        : []),
      ...(!act.done && WORK.includes(act.kind)
        ? [
            w.lastOwn?.id === act.id && message
              ? {
                  label: "Take back",
                  detail: "Cancel this message alone, and give it back to the input to write again",
                  run: () => this.action("/cancel"),
                }
              : {
                  label: "Cancel",
                  detail: "End this act and the work under it, and not the thread or the chain it stands on",
                  run: () => this.action(`/cancel ${act.id}`),
                },
          ]
        : []),
      ...(this.isPoint(act)
        ? [
            {
              label: message ? "Edit and send again" : "Branch after it",
              detail: message
                ? "A new branch that has not read this message, with the message in the input"
                : "A new branch that reads the chain up to this act",
              run: () => this.rewindTo(act),
            },
          ]
        : []),
      {
        label: "Copy its name",
        detail: act.id,
        run: () => {
          this.renderer.copyToClipboardOSC52(act.id);
          w.notice = `${act.id} copied.`;
        },
      },
    ]);
  }
  /** The chord of the key that runs an action, as this terminal sends it. */
  private chordOf(action: Action): string {
    const found = bindings.find(({ binding }) => binding.run === action);
    return found ? shown(found.key, found.binding, this.kitty) : "";
  }
  /** Every action in one list: each with its label, the slash command that does it, what it does, and its keys. The
   * table of keys gives the chord of each command, and each action that has keys and no command, which stands before
   * the command that it goes with. */
  palette(): void {
    const views_: Record<View, string> = {
      feed: "Messages, the Python each model wrote, and answers",
      transcript: "The exact text that the model reads",
      changes: "The diff of each file that the life wrote",
    };
    const keysOf = new Map<string, string>();
    const before = new Map<string, Choice>();
    for (const { key, binding } of bindings) {
      if (binding.command) keysOf.set(binding.command, shown(key, binding, this.kitty));
      const { choice, run } = binding;
      if (choice && run)
        before.set(choice.before, {
          label: choice.label,
          detail: choice.detail,
          command: "",
          keys: shown(key, binding, this.kitty),
          run: () => void this.actions[run](),
        });
    }
    const choices: Choice[] = [
      { label: "New session", detail: commands.new[2], command: "/new", run: () => this.action("/new") },
    ];
    // One key names the three views, as 1-3.
    const toggle = bindings.find(({ binding }) => binding.run === "view");
    for (const [index, view] of views.entries())
      choices.push({
        label: `${viewLabels[view]} view`,
        detail: views_[view],
        command: "",
        keys: toggle && shown(toggle.key, toggle.binding, this.kitty).replace("1-3", String(index + 1)),
        run: () => this.showView(view),
      });
    for (const { name, label, argument, detail, usage } of slashes) {
      if (name === "new") continue;
      const aside = before.get(name);
      if (aside) choices.push(aside);
      choices.push({
        label,
        detail,
        command: usage,
        keys: keysOf.get(name),
        run: this.command(name, argument),
      });
    }
    for (const part of this.parts.loaded)
      for (const one of part.commands)
        choices.push({
          label: title(one.name),
          detail: `${one.detail} (${part.name})`,
          command: `/${one.name}`,
          run: () => this.action(`/${one.name}`),
        });
    this.openPalette("Commands", choices, { rich: true });
  }
  /** How a command runs when it is picked from a list: one that needs its argument waits for it in the input. */
  private command(name: string, argument: string): () => void {
    return () => (argument && !argument.startsWith("[") ? this.insert(`/${name} `) : this.action(`/${name}`));
  }
  /** The text before the cursor. The offset of the cursor counts cells of the terminal, not units of the text. */
  private beforeCursor(): string {
    return this.composer.getTextRange(0, this.composer.cursorOffset);
  }
  /** The text before the cursor that ends with `old` replaced. The composer deletes one grapheme at a time. */
  private replaceBefore(old: string, replacement: string): void {
    for (const _ of graphemes.segment(old)) this.composer.deleteCharBackward();
    this.composer.insertText(replacement);
  }
  /** The token the cursor ends: a `/command` that opens the input, or an `@path` or a `:shortcode` that starts a word
   * of a prompt. */
  private token(): { kind: "/" | "@" | ":" | "value"; text: string; command?: string } | undefined {
    const before = this.beforeCursor();
    const slash = before.match(/^\/([\w-]*)$/);
    if (slash) return { kind: "/", text: slash[1] ?? "" };
    // The first word after a command whose values are known is a value, which the suggestions complete.
    const value = before.match(/^\/([\w-]+) (\S*)$/);
    if (value && Object.hasOwn(this.values, value[1] ?? ""))
      return { kind: "value", text: value[2] ?? "", command: value[1] ?? "" };
    const at = before.match(/(?:^|\s)@([^\s"']*)$/);
    if (at && this.session.mode === "prompt" && !this.session.editing)
      return { kind: "@", text: at[1] ?? "" };
    const colon = before.match(/(?:^|\s):([\w+-]*)$/);
    if (colon && this.session.mode === "prompt" && !this.session.editing)
      return { kind: ":", text: colon[1] ?? "" };
    return undefined;
  }
  /** The token before the cursor as it is now, replaced with the text of a suggestion. */
  private complete(suggestion: Suggestion): void {
    const token = this.token();
    if (token)
      this.replaceBefore(token.kind === "value" ? token.text : `${token.kind}${token.text}`, suggestion.text);
  }
  /** The paths of the project, which the suggestions read, as a pick of them gives them: none until the read of the
   * session ends, which draws the suggestions again. An `@` that starts a word reads them afresh. */
  private paths(pick = (paths: string[]) => paths, fresh = false): Value[] {
    void this.session.projectFiles(fresh);
    return pick(this.session.files?.paths ?? []).map((path) => ({ value: path, detail: "" }));
  }
  /** The values that the first argument of a command may take, for each command whose values the TUI knows. A value
   * of a command that takes more after it completes, and a value of any other command runs the command when it is
   * chosen. */
  private readonly values: Partial<Record<string, () => Value[]>> = {
    model: () => {
      const w = this.session;
      const roster = w.roster.filter(([name]) => name !== "operator");
      // The models of the catalog that the roster does not hold follow it, and one of them joins the roster when it is
      // chosen.
      const added = w.catalog.filter(([name]) => !roster.some(([held]) => held === name));
      return [...roster, ...added].map(([name, , window]) => {
        const { provider, id } = modelName(name);
        return {
          // A catalog can hold the same id under a provider that the roster does not offer.
          value: name,
          detail: `${count(window)} tokens of context`,
          current: name === w.actorChoice.model,
          label: id,
          heading: provider || undefined,
        };
      });
    },
    effort: () => {
      const { model, effort } = this.session.actorChoice;
      return (this.session.roster.find(([name]) => name === model)?.[1] ?? []).map((name) => ({
        value: name,
        detail: efforts[name] ?? "",
        current: name === effort,
      }));
    },
    shape: () =>
      shapes().map((shape) => ({
        value: shape,
        detail: `The answer is a ${shape}`,
        current: shape === this.session.shape,
      })),
    theme: () =>
      (["terminal", ...Object.keys(palettes)] as ThemeChoice[]).map((name) => ({
        value: name,
        detail: name === "terminal" ? terminalDetail : themeLabels[name].join(", "),
        current: name === this.session.preferences.theme,
      })),
    access: () =>
      (["reader", "motion"] as const).map((name) => ({
        value: name,
        detail: name === "reader" ? "Screen reader mode, on or off" : "Reduced motion, on or off",
        current:
          name === "reader" ? this.session.preferences.screenReader : this.session.preferences.reducedMotion,
      })),
    read: () => this.paths(),
    image: () => this.paths((paths) => paths.filter((path) => /\.(png|jpe?g|gif|webp)$/i.test(path))),
    cd: () =>
      this.paths((paths) =>
        [
          ...new Set(
            paths.flatMap((path) => (path.includes("/") ? [path.slice(0, path.lastIndexOf("/"))] : [])),
          ),
        ].sort(),
      ),
    workspace: () =>
      this.options.workspaces.groups.map((group) => ({
        value: shortenHome(group.directory),
        detail: group.name,
      })),
    edit: () =>
      this.session.activity
        .filter((act) => this.session.editable(act))
        .map((act) => ({
          value: act.id,
          detail: clip(String(act.words[1] ?? "").split("\n")[0] ?? "", 48),
        })),
    pause: () => this.chainValues(),
    wake: () => this.chainValues(),
    cancel: () =>
      this.session.open
        .filter((act) => working(act))
        .map((act) => ({
          value: act.id,
          detail: `${act.kind}  ${clip(String(act.words[act.kind === "prompt" ? 1 : 0] ?? ""), 40)}`,
        })),
    feed: () =>
      this.session.open
        .filter((act) => act.kind === "bash" && act.words[1] === null)
        .map((act) => ({ value: act.id, detail: clip(String(act.words[0] ?? ""), 48), more: true })),
    close: () => this.session.open.map((act) => ({ value: act.id, detail: act.kind, more: true })),
    grant: () =>
      ["1", "2", "5", "10", "20"].map((value) => ({
        value,
        detail: `Pause this chain at ${dollars(Number(value))}`,
      })),
    context: () =>
      ["0.5", "0.7", "0.8", "0.9"].map((value) => ({
        value,
        detail: `Pause this chain at ${share(Number(value))}`,
      })),
  };
  /** The chains of the session as the values of a command, which pause and wake take. */
  private chainValues(): Value[] {
    const w = this.session;
    return w.chains.map((chain) => ({
      value: chain.id,
      detail: w.labelOf(chain.id),
      current: chain.id === w.selected,
    }));
  }
  private suggest = (): void => {
    // A whole `:shortcode:` of a prompt that names an emoji becomes that emoji.
    const whole = this.beforeCursor().match(/(?:^|\s)(:([\w+-]+):)$/);
    const named =
      whole && this.session.mode === "prompt" && !this.session.editing && emojiNamed(whole[2] ?? "");
    if (whole && named) {
      this.replaceBefore(whole[1] ?? "", named);
      return;
    }
    const token = this.token();
    const key = token ? `${token.kind}${token.text}` : "";
    const started = key === "@" && this.lastToken !== "@";
    // A new filter chooses anew from its first suggestion, and a move of the cursor keeps the choice.
    if (key !== this.lastToken) this.suggestionIndex = 0;
    this.lastToken = key;
    if (key !== this.dismissed) this.dismissed = "";
    if (!token || this.overlay || this.dismissed) {
      this.suggestions = [];
      this.renderSuggestions();
      return;
    }
    if (token.kind === "/") {
      // The command that the token names whole comes first.
      this.suggestions = slashes
        .filter(({ name }) => name.startsWith(token.text))
        .sort((one, other) => Number(other.name === token.text) - Number(one.name === token.text))
        .map(({ name, argument, detail, usage }) => ({
          label: usage,
          detail,
          text: `/${name} `,
          submit: () => {
            this.composer.setText("");
            this.command(name, argument)();
          },
        }));
    } else if (token.kind === "value") {
      const command = token.command ?? "";
      const wanted = token.text.toLowerCase();
      // A value that starts with what is typed comes first, then a value that holds it.
      this.suggestions = (this.values[command]?.() ?? [])
        .filter(({ value }) => value.toLowerCase().includes(wanted))
        .sort(
          (one, other) =>
            Number(other.value.toLowerCase().startsWith(wanted)) -
            Number(one.value.toLowerCase().startsWith(wanted)),
        )
        .slice(0, 50)
        .map(({ value, detail, current, more }) => ({
          label: value,
          detail: `${detail}${current ? "   current" : ""}`,
          text: `${value} `,
          ...(more
            ? {}
            : {
                submit: () => {
                  this.composer.setText(`/${command} ${value}`);
                  void this.submit();
                },
              }),
        }));
    } else if (token.kind === ":") {
      this.suggestions = emojis(token.text)
        .slice(0, 50)
        .map(({ emoji, name, description }) => ({
          label: `${emoji} :${name}:`,
          detail: description,
          text: emoji,
        }));
    } else {
      const wanted = token.text.toLowerCase();
      this.suggestions = this.paths(
        (paths) => paths.filter((path) => path.toLowerCase().includes(wanted)),
        started,
      )
        .slice(0, 50)
        .map(({ value: path }) => ({
          label: `@${path}`,
          detail: "",
          text: `@${/\s/.test(path) ? JSON.stringify(path) : path} `,
        }));
    }
    this.suggestionIndex = Math.min(this.suggestionIndex, Math.max(0, this.suggestions.length - 1));
    this.renderSuggestions();
  };
  private renderSuggestions(): void {
    const token = this.overlay || this.dismissed ? undefined : this.token();
    const shown = token ? this.suggestions : [];
    const files = this.session.files;
    // An emoji list that matches nothing closes.
    const state =
      !token || token.kind === ":"
        ? ""
        : (token.kind === "@" || pathCommands.has(token.command ?? "")) && files?.error
          ? files.error
          : (token.kind === "@" || pathCommands.has(token.command ?? "")) && !files?.paths
            ? "Finding project files..."
            : shown.length
              ? ""
              : token.kind === "/"
                ? "No command starts with that name."
                : token.kind === "value"
                  ? "No known value matches. Enter sends what is typed."
                  : "No project file matches.";
    if (
      !this.paneChanged(this.suggestionBox, [
        this.theme,
        state,
        this.suggestionIndex,
        shown.map((one) => one.label),
      ])
    )
      return;
    this.clear(this.suggestionBox);
    this.suggestionBox.visible = Boolean(state || shown.length);
    if (state)
      this.suggestionBox.add(this.inset(space.between, this.line(state, files?.error ? c.danger : c.muted)));
    const rows = 8;
    const first = Math.max(0, Math.min(this.suggestionIndex - rows + 1, shown.length - rows));
    const visible = shown.slice(first, first + rows);
    // The details stand in one column after the longest label, and a long label pushes none of them.
    const column = Math.min(
      36,
      Math.max(0, ...visible.map((one) => Bun.stringWidth(one.label))) + space.between,
    );
    for (const [offset, one] of visible.entries()) {
      const index = first + offset;
      const selected = index === this.suggestionIndex;
      const row = this.row({
        paddingX: space.inset,
        backgroundColor: selected ? c.selected : c.raised,
        onMouseUp: this.click(() => {
          this.suggestionIndex = index;
          this.complete(one);
        }),
        onMouseOver: () => {
          if (this.suggestionIndex === index) return;
          this.suggestionIndex = index;
          this.renderSuggestions();
        },
      });
      row.add(this.text(selected ? `${glyph.pointer} ` : "  ", c.accent));
      row.add(
        this.text(one.label, selected ? c.accent : c.text, {
          width: one.detail ? column : undefined,
          truncate: true,
          attributes: selected ? bold : 0,
        }),
      );
      if (one.detail) row.add(this.text(one.detail, c.muted, { truncate: true, flexShrink: 1 }));
      this.suggestionBox.add(row);
    }
    if (shown.length > rows)
      this.suggestionBox.add(
        this.inset(space.between + space.inset, this.line(`${shown.length - rows} more`, c.faint)),
      );
    this.renderStatus();
  }
  /** The mark of a choice that is the current one, and the room of that mark for the others. */
  private current(yes: boolean): Part {
    return yes ? [`${glyph.done} `, c.accent] : ["  "];
  }
  /** The values of a command in a picker, with the current one marked and selected, which runs the command on the
   * value that is chosen. */
  private pick(command: string, label: string, note: string, only?: (value: string) => boolean): void {
    const values = (this.values[command]?.() ?? []).filter((one) => !only || only(one.value));
    this.openPalette(
      label,
      values.map((one) => ({
        label: one.label ?? one.value,
        detail: one.detail,
        heading: one.heading,
        mark: this.current(Boolean(one.current)),
        run: () => this.action(`/${command} ${one.value}`),
      })),
      { selected: values.findIndex((one) => one.current), note },
    );
  }
  /** The models of the roster and of the catalog under their providers, with the one the chain uses marked. */
  models = (): void => {
    this.pick("model", "Model", "The chain sends its next prompt to this model.");
  };
  private codexModels = (): void => this.pickModels("codex");
  private pickModels(provider?: string): void {
    void this.session
      .reloadCatalog()
      .then(() => {
        if (!this.closed)
          this.pick(
            "model",
            provider ? "Codex models" : "Model",
            provider
              ? "These models use your ChatGPT subscription."
              : "The chain sends its next prompt to this model.",
            provider ? (value) => value.startsWith(`${provider}:`) : undefined,
          );
      })
      .catch(this.report);
  }
  private authenticationOpen = false;
  private authenticationUrl = "";
  private authentication = (status: import("furb").CodexAuthInfo): void => {
    if (status.state !== "waiting" || !status.url) {
      if (this.authenticationOpen) this.closeOverlay();
      return;
    }
    const url = status.url;
    this.openPalette(
      "Sign in to Codex",
      [
        {
          label: "Open browser",
          detail: "Finish sign-in with your ChatGPT subscription",
          run: () => openFile(url),
        },
        {
          label: "Copy sign-in link",
          detail: "Use this if the browser did not open",
          run: () => {
            this.renderer.copyToClipboardOSC52(url);
          },
        },
        {
          label: "Cancel sign-in",
          detail: "Keep using the current model",
          run: async () => {
            await this.session.host.codexAuth("cancel");
          },
        },
      ],
      { note: "Furb will connect when sign-in finishes. No API key is needed." },
    );
    this.authenticationOpen = true;
    if (this.authenticationUrl !== url) {
      this.authenticationUrl = url;
      void openFile(url).catch(() => {
        this.session.notice = "Open the sign-in link to connect Codex.";
        this.render();
      });
    }
  };
  providers = (): void => {
    void this.session.host
      .providers()
      .then((choices) => {
        const order = ["codex", "claude-cli", "openai", "local"];
        const rank = (name: string) => (order.includes(name) ? order.indexOf(name) : order.length);
        choices.sort((one, other) => rank(one.name) - rank(other.name) || one.name.localeCompare(other.name));
        this.openPalette(
          "Providers",
          [
            ...choices.map((one) => ({
              label: one.name,
              detail:
                one.name === "codex"
                  ? one.enabled && one.models > 0
                    ? `Choose from ${one.models} models on your ChatGPT subscription`
                    : "Connect your ChatGPT subscription"
                  : one.name === "openai"
                    ? `OpenAI API, ${one.models} models available`
                    : `${one.enabled ? "Remove" : "Add"} provider, ${one.models} models available`,
              mark: this.current(one.enabled),
              run: () =>
                one.name === "codex" && one.enabled && one.models > 0
                  ? this.codexModels()
                  : this.action(
                      `/provider ${one.enabled && !(one.name === "codex" && one.models === 0) ? "remove" : "add"} ${one.name}`,
                    ),
            })),
            ...choices
              .filter((one) => one.name === "codex" && one.enabled)
              .map(() => ({
                label: "Disable codex",
                detail: "Stop offering ChatGPT subscription models in this project",
                run: () => this.action("/provider remove codex"),
              })),
            {
              label: "Add a custom provider",
              detail: "Enter its name, URL, model and context window",
              run: () => this.insert("/provider add "),
            },
            {
              label: "Reload settings",
              detail: "Read the JSON config files again",
              run: () => this.action("/reload"),
            },
          ],
          { note: "Changes apply to this project and are saved in .furb/config.json." },
        );
      })
      .catch(this.report);
  };
  effortPicker = (): void =>
    this.pick(
      "effort",
      `Effort of ${modelName(this.session.actorChoice.model).id}`,
      "More effort thinks longer and costs more. ⇧Tab moves to the next.",
    );
  details = (): void => {
    const w = this.session;
    // Each thread of the chain folds to its header and opens to its program, as a click on its header does.
    const threads = w.view === "feed" ? this.chainThreadList(w.selected) : [];
    this.openPalette("Details", [
      ...threads.map((one) => {
        const folded = this.threadFolded(one.act.id);
        return {
          label: clip(shortenHomes(gist(one.title)) || one.act.id, 72),
          detail: folded ? "Expand" : "Collapse",
          run: () => {
            this.foldThread(one.act.id, !folded);
            this.scrollAfterLayout({ card: one.act.id });
          },
        };
      }),
      ...[...this.cards]
        .filter(([, card]) => card.compact || card.collapsible)
        .map(([id, card]) => ({
          label: card.heading.plainText.replace(/^[▸▾] /, ""),
          detail: card.closed ? "Expand" : "Collapse",
          run: () => {
            w.folds[card.state] = !card.closed;
            this.renderContent();
            this.scrollAfterLayout({ card: id });
          },
        })),
    ]);
  };
  themes(): void {
    // The theme of the terminal comes first, then the dark themes, then the light ones.
    const names = (Object.keys(palettes) as (keyof typeof palettes)[]).sort(
      (one, other) => Number(themeLabels[one][1] === "Light") - Number(themeLabels[other][1] === "Light"),
    );
    const choices: ThemeChoice[] = ["terminal", ...names];
    const chosen = this.session.preferences.theme;
    this.openPalette(
      "Color theme",
      choices.map((name) => {
        const palette = paletteOf(name === "terminal" ? this.theme : name);
        return {
          label: name === "terminal" ? "Terminal" : themeLabels[name][0],
          detail: name === "terminal" ? terminalDetail : themeLabels[name][1],
          mark: this.current(name === chosen),
          swatch: [palette.accent, palette.secondary, palette.success, palette.warning, palette.danger],
          run: () => {
            this.session.theme = name;
            this.render();
            this.session.save();
          },
        };
      }),
      {
        selected: choices.indexOf(chosen),
        note: colorless.on
          ? "NO_COLOR is set, so every theme shows the colors of the terminal alone."
          : "Every session shares this choice.",
      },
    );
  }
  shapes = (): void =>
    this.pick("shape", "Response shape", "The engine validates the result against this Python type.");
  toggleMode(): void {
    this.session.mode = this.session.mode === "python" ? "prompt" : "python";
    this.render();
    void this.highlightEditor();
  }
  private applyTheme(name: ThemeName): void {
    const old = { ...c };
    setTheme(name);
    this.theme = name;
    const replacements = new Map(
      Object.keys(old).map((key) => [old[key as keyof typeof old], c[key as keyof typeof c]]),
    );
    const recolor = (node: Renderable) => {
      for (const property of [
        "fg",
        "bg",
        "backgroundColor",
        "borderColor",
        "titleColor",
        "textColor",
        "placeholderColor",
      ]) {
        const color: unknown = Reflect.get(node, property);
        if (replacements.has(color as RGBA)) Reflect.set(node, property, replacements.get(color as RGBA));
      }
      for (const child of node.getChildren()) recolor(child);
    };
    recolor(this.root);
    // An input keeps the colors that it shows focused, which the walk above cannot read.
    for (const input of [this.composer, this.search]) Object.assign(input, inputColors(c.panel));
    this.clear(this.scroll);
    this.cards.clear();
    const previous = this.style;
    this.style = syntax();
    this.composer.syntaxStyle = this.style;
    previous.destroy();
  }
  private async highlightEditor(): Promise<void> {
    if (this.closed) return;
    const content = this.composer.plainText;
    this.keepDraft();
    const version = ++this.editorVersion;
    this.composer.clearAllHighlights();
    if (this.session.mode !== "python" && !this.session.editing && !content.startsWith("/run ")) return;
    const result = await getTreeSitterClient()
      .highlightOnce(content, "python")
      .catch((error) => {
        if (!this.closed) this.session.fail(error);
        return undefined;
      });
    if (version !== this.editorVersion || this.closed) return;
    // The composer counts the characters of its text without the line ends, so each offset of the text loses the
    // line ends before it.
    const ends: number[] = [];
    for (let at = content.indexOf("\n"); at >= 0; at = content.indexOf("\n", at + 1)) ends.push(at);
    const flat = (offset: number) => offset - ends.filter((end) => end < offset).length;
    const mark = (start: number, end: number, styleId: number) =>
      this.composer.addHighlightByCharRange({ start: flat(start), end: flat(end), styleId });
    for (const [start, end, group] of result?.highlights ?? []) {
      const styleId =
        this.style.resolveStyleId(group) ?? this.style.resolveStyleId(group.split(".")[0] ?? "default");
      if (styleId !== null) mark(start, end, styleId);
    }
    if (content === this.session.rejectedWord || content === `/run ${this.session.rejectedWord}`) {
      const styleId = this.style.resolveStyleId("diagnostic");
      const lines = content.split("\n");
      for (const finding of this.session.findings) {
        const line = Number(finding.match(/line (\d+)/)?.[1] ?? 0) - 1;
        if (styleId !== null && line >= 0) {
          const start = lines.slice(0, line).reduce((size, line) => size + line.length + 1, 0);
          mark(start, start + (lines[line]?.length ?? 0), styleId);
        }
      }
    }
    const cursor = this.beforeCursor().length;
    const at = "()[]{}".includes(content[cursor] ?? " ") ? cursor : cursor - 1;
    const bracket = content[at] ?? "";
    const pair = "()[]{}".indexOf(bracket);
    if (bracket && pair >= 0) {
      const direction = pair % 2 === 0 ? 1 : -1;
      const other = "()[]{}"[pair + direction];
      let depth = 0;
      for (let index = at; index >= 0 && index < content.length; index += direction) {
        if (
          (result?.highlights ?? []).some(
            ([start, end, group]) => /^(string|comment)/.test(group) && index >= start && index < end,
          )
        )
          continue;
        if (content[index] === bracket) depth++;
        else if (content[index] === other && --depth === 0) {
          const styleId = this.style.resolveStyleId("matching");
          if (styleId !== null) for (const start of [at, index]) mark(start, start + 1, styleId);
          break;
        }
      }
    }
  }
  /** The time since an act started. */
  private progress(id: string): string {
    return elapsed(Date.now() - (this.session.started[id] ?? Date.now()));
  }
  private resume = (): void => {
    const pending = this.session.host.pending;
    this.openPalette("Saved work is paused", [
      {
        label: "Resume saved work",
        detail: `${pending.size} unfinished ${pending.size === 1 ? "act starts" : "acts start"} again.`,
        run: async () => {
          await this.session.host.resume();
          await this.session.refresh();
        },
      },
      {
        label: "Keep it paused",
        detail: "Inspect the session first. /wake brings this choice back.",
        run: () => {},
      },
    ]);
  };
  /** The dialog of an ask: its whole message, and the field of its answer or the choices of its shape. Its title says
   * which ask of the desk it is, and from which chain. Tab and ⇧Tab go to the next and the previous ask, and an
   * answer goes on to the next ask that waits, until none waits. */
  question(ask = this.session.aimed ?? this.session.desk.asks[0]): void {
    const w = this.session;
    if (!ask) return;
    if (w.pointed !== ask.id || ask.chain !== w.selected) void w.point(ask.id).catch(this.report);
    const { asks } = w.desk;
    const title = `Question ${asks.findIndex((one) => one.id === ask.id) + 1} of ${asks.length} from ${ask.label}`;
    const moves = asks.length > 1 ? " Tab goes to the next one." : "";
    // An answer goes on to the ask after this one, or closes the dialog when no other ask waits.
    const next = () => {
      const rest = w.desk.asks;
      const after = rest.find((one) => one.began > ask.began) ?? rest[0];
      if (after) this.question(after);
      else this.closeOverlay();
    };
    if (ask.choices.length) {
      this.openPalette(
        title,
        ask.choices.map((choice) => ({
          label: choice,
          detail: "",
          run: () => w.answer(ask, choice).then(next),
        })),
      );
      this.showQuestionText(ask.message);
      this.answering = ask.id;
      return;
    }
    this.openPalette(title, []);
    // The field of the dialog takes the answer, and shows its prompt from the start.
    const prompt = this.paletteInputRow?.getChildren()[0];
    if (prompt) prompt.visible = true;
    if (this.paletteInputRow) this.paletteInputRow.height = space.bar;
    this.showQuestionText(ask.message);
    if (this.paletteInput) this.paletteInput.placeholder = `Type the answer to ${ask.id}, as ${ask.shape}`;
    const error = this.text(`Enter sends it. Esc leaves the question open.${moves}`, c.muted);
    this.paletteList?.add(error);
    this.answering = ask.id;
    const input = this.paletteInput;
    if (!input) return;
    input.removeAllListeners(InputRenderableEvents.INPUT);
    input.removeAllListeners(InputRenderableEvents.ENTER);
    input.on(InputRenderableEvents.ENTER, () => {
      void w
        .answer(ask, input.value)
        .then(next)
        .catch((failure) => {
          error.fg = c.danger;
          error.content = String(failure);
        });
    });
  }
  /** The dialog of the ask a step after or before the one that it shows. */
  private moveAsk = (key?: KeyEvent) => {
    const next = beside(this.session.desk.asks, this.answering, key?.shift ? -1 : 1);
    if (next && next.id !== this.answering) this.question(next);
  };
  private showQuestionText(message: string): void {
    if (!this.overlay || !this.paletteInput) return;
    // The text of the question takes the rows it wraps to, up to what the screen leaves, and scrolls past them. A
    // long question moves the dialog up to the top of the screen, and a short one leaves it where every dialog stands.
    const inner = this.paletteWidth - space.inset * 2 - space.between * 2;
    const rows = message
      .split("\n")
      .reduce((sum, line) => sum + Math.max(1, Math.ceil(Bun.stringWidth(line) / inner)), 0);
    if (rows > 3) this.overlay.top = 1;
    this.questionDocument = this.document(this.markdown(message), {
      height: Math.max(1, Math.min(rows, 8, this.renderer.height - 18)),
      // A field that shows stands a row under the text, and a field that takes no row leaves its own space under it.
      marginBottom: this.paletteInputRow?.height ? space.section : 0,
      contentOptions: { paddingLeft: space.between, paddingRight: space.between },
    });
    this.overlay.insertBefore(this.questionDocument, this.paletteInputRow);
    this.overlay.maxHeight = this.paletteHeight;
  }
  private async showHover(name: string, x: number, y: number): Promise<void> {
    try {
      const inspected = await this.session.engine.inspect(name, this.session.selected);
      if (this.closed || this.overlay) return;
      this.hoverCard(
        x,
        y,
        9,
        () => this.inspect(name),
        this.text([
          [name, c.text, bold],
          [`: ${inspected.kind}`, c.secondary],
        ]),
        this.text(inspected.representation.slice(0, 280), c.muted, { maxHeight: 4 }),
        this.text([
          ["Click", c.muted],
          [" opens it   ", c.faint],
          ["⌃G", c.muted],
          [" inspects a name", c.faint],
        ]),
      );
    } catch {
      this.unhover();
    }
  }
  /** The value a name holds in the module of the chain, shown once the life has read it. */
  inspect = (name: string): Promise<void> => {
    this.unhover();
    return this.session.engine
      .inspect(name, this.session.selected)
      .then((value) => {
        this.showValue(`${name}: ${value.kind}`, value.value ?? value.representation, name);
      })
      .catch(this.report);
  };
  private showValue(label: string, value: unknown, name?: string, back?: () => void): void {
    const choices: Choice[] = [];
    if (back) choices.push({ label: "← Back", detail: "Return to the parent value", run: back });
    if (name && /^[\p{L}_][\p{L}\p{N}_]*$/u.test(name)) {
      const definition = [...Object.entries(this.session.program)]
        .reverse()
        .find(([, source]) =>
          source
            .split("\n")
            .some((line) => new RegExp(`^(?:def |class )?${name}\\b(?:\\s*[:=(])`).test(line)),
        );
      if (definition)
        choices.push({
          label: "Go to definition",
          detail: definition[0],
          run: () => {
            this.go("feed", definition[0]);
          },
        });
      else
        choices.push({
          label: "Engine definition",
          detail: `Find ${name} in the engine source`,
          run: async () => {
            const lines = (await this.session.host.source()).split("\n");
            const at = lines.findIndex((line) =>
              new RegExp(`^(?:(?:async )?def |class )?${name}\\b`).test(line),
            );
            if (at < 0) {
              this.session.notice = "This name has no definition in the engine source.";
              return;
            }
            const next = lines.slice(at + 1).findIndex((line) => /^(?:def |class |[A-Z_]+\s*=)/.test(line));
            this.openPalette(`${name} in engine.py, line ${at + 1}`, [
              {
                label: "← Back to value",
                detail: label,
                run: () => this.showValue(label, value, name, back),
              },
            ]);
            this.paletteList?.add(
              this.document(
                this.numbered(lines.slice(at, next < 0 ? undefined : at + next + 1).join("\n"), {
                  fg: c.muted,
                  lineNumberOffset: at,
                }),
                { height: Math.min(24, this.renderer.height - 16) },
              ),
            );
          },
        });
    }
    if (value && typeof value === "object") {
      for (const [key, child] of Object.entries(value))
        choices.push({
          label: `${key}  ${typeof child === "object" && child !== null ? glyph.closed : ""}`,
          detail: shortenHome(display(child)).replaceAll("\n", " ").slice(0, 110),
          run: () =>
            this.showValue(`${label}.${key}`, child, undefined, () =>
              this.showValue(label, value, name, back),
            ),
        });
    } else
      choices.push({
        label:
          typeof value === "string" && (value.length > 100 || value.includes("\n"))
            ? "Copy value"
            : display(value),
        detail: "Copy this value",
        run: () => {
          this.renderer.copyToClipboardOSC52(display(value));
          this.session.notice = "Value copied.";
        },
      });
    const act = typeof value === "string" ? this.session.actOf(value) : undefined;
    if (typeof value === "string" && act)
      choices.push({
        label: "Follow this act or door",
        detail: value,
        run: () => {
          if (act.kind === "chain") return this.session.select(act.id);
          void this.referenced(value)
            .then((held) => this.showValue(value, held))
            .catch(this.report);
        },
      });
    this.openPalette(label, choices);
    if (typeof value === "string" && (value.length > 100 || value.includes("\n"))) {
      this.questionDocument = this.document(this.text(value), {
        height: Math.max(3, Math.min(18, this.renderer.height - 18)),
      });
      this.paletteList?.add(this.questionDocument);
    }
  }
  async names(): Promise<void> {
    const w = this.session;
    const names = (await w.engine.names(w.selected)).filter((name) => !name.startsWith("_"));
    // The names that the words of this chain bind come first, then the acts of the chain, then what the engine gives.
    const identifier = "[\\p{L}_][\\p{L}\\p{N}_]*";
    // A name is bound by an assignment, a def or a class, a for loop, an import, or an as.
    const binds = [
      `^\\s*(${identifier})\\s*(?::[^=\\n]+)?=(?!=)`,
      `^\\s*(?:async\\s+)?(?:def|class)\\s+(${identifier})`,
      `^\\s*(?:async\\s+)?for\\s+(${identifier})\\s+in\\b`,
      `^\\s*(?:from\\s+\\S+\\s+)?import\\s+(${identifier})`,
      `\\bas\\s+(${identifier})`,
    ];
    const program = Object.values(w.program).join("\n");
    const bound = new Set(
      binds.flatMap((pattern) =>
        [...program.matchAll(new RegExp(pattern, "gmu"))].map((match) => match[1] ?? ""),
      ),
    );
    const acts = new Set(w.acts.map((act) => act.id));
    const rank = (key: string) => (bound.has(key) ? 0 : acts.has(key) ? 1 : 2);
    const headings = ["Named in this chain", "Acts of this chain", "Given by the engine"];
    this.openPalette(
      "Inspect a name",
      names
        .map((key, index) => ({ key, index, group: rank(key) }))
        .sort((one, other) => one.group - other.group || one.index - other.index)
        .map(({ key, group }) => ({
          label: key,
          detail: "",
          heading: headings[group],
          run: () => this.inspect(key),
        })),
      { note: "Enter reads the live value of the name, its type, and its fields." },
    );
  }
  private async completeNames(): Promise<void> {
    const names = await this.session.engine.names(this.session.selected);
    const prefix = this.beforeCursor().match(/[\p{L}_][\p{L}\p{N}_]*$/u)?.[0] ?? "";
    this.openPalette(
      "Complete Python name",
      names
        .filter((name) => name.startsWith(prefix) && !name.startsWith("_"))
        .map((name) => ({
          label: name,
          detail: "Insert this name at the cursor",
          run: () => {
            this.replaceBefore(prefix, name);
            this.composer.focus();
          },
        })),
    );
  }
  /** The prompts of the chain, whose program an edit changes and replays. */
  ladders(): void {
    const prompts = this.session.activity.filter((act) => this.session.editable(act));
    this.openPalette(
      "Edit a prompt program",
      prompts.map((act) => {
        const { mark, color } = this.actState(act);
        return {
          label: act.id,
          detail: String(act.words[1]),
          mark: [`${mark} `, color],
          run: () => this.action(`/edit ${act.id}`),
        };
      }),
      {
        selected: Math.max(0, prompts.length - 1),
        note: "The program is the Python the model wrote for the prompt. An edit replays it.",
      },
    );
  }
  /** The rewind tree in the feed, with the pointer on the last message of the chain: Enter then gives that message
   * back on a new branch that has not read it. A paused chain is woken first. */
  rewind = (): void => {
    if (this.session.paused) {
      this.openPalette("Wake this chain before rewinding", [
        {
          label: "Wake chain",
          detail: "Then choose the point that the new branch starts from.",
          run: () => (this.session.host.pending.size ? this.resume() : this.action("/wake")),
        },
      ]);
      return;
    }
    const points = this.session.activity.filter((act) => this.isPoint(act));
    const last = points.findLast((act) => this.session.isUserPrompt(act)) ?? points.at(-1);
    this.openTree(last?.id ?? this.session.selected, "Rewind");
  };
  /** A rewind to a point. When a write changed a file after it, it asks first what comes back: the conversation on a
   * new branch, the files on this chain, or both. A change that a command made is not restored. */
  private async rewindTo(act: ActRow): Promise<void> {
    const w = this.session;
    const found = await w.filesAt(act.id);
    const { files, commands } = found;
    if (!files.length) {
      await w.rewind(act.id);
      return;
    }
    const message = w.isUserPrompt(act);
    const count = files.length === 1 ? "the file" : `the ${files.length} files`;
    const named = files.map(({ path }) => {
      const near = relative(w.workingDirectory, path);
      return near.startsWith("..") || isAbsolute(near) ? shortenHome(path) : near;
    });
    const stays = commands
      ? `${commands} ${commands === 1 ? "command" : "commands"} ran after it: a change that a command made is not restored.`
      : "A change that a command made is not restored.";
    this.openPalette(
      `Rewind to ${act.id}`,
      [
        {
          label: "Restore the conversation",
          detail: message
            ? "A new branch that has not read this message, with the message in the input. The files stay."
            : "A new branch that reads the chain up to this point. The files stay as they are.",
          run: () => w.rewind(act.id).then(() => {}),
        },
        {
          label: "Restore the files",
          detail: `Put back ${count} that a write changed after this point. The conversation stays.`,
          run: () => w.restoreAt(act.id, found),
        },
        {
          label: "Both",
          detail: `A new branch from this point, and ${count} put back`,
          run: () => w.rewind(act.id, undefined, found).then(() => {}),
        },
        { label: "Keep", detail: "Return without changes", run() {} },
      ],
      { note: `${clip(named.join(", "), 200)}. ${stays}` },
    );
  }
  /** The tree of the session in the feed, with the pointer on the chain shown. */
  sessionTree = (): void => this.openTree(this.session.selected, "Session tree");
  /** Whether an act is a point that a branch can start from: an act that a turn of its chain tells, which is no
   * chain, no grant, and no rung that its chain wrote to retell its source. */
  private isPoint(act: ActRow): boolean {
    return (
      !["chain", "grant"].includes(act.kind) &&
      (act.kind !== "rung" || this.session.actOf(act.by)?.kind !== "chain")
    );
  }
  private openTree(selected: string, title: string): void {
    this.closeOverlay();
    this.closeSearch();
    this.treeFolds.clear();
    this.tree = { rows: [], selected, title };
    this.tree.rows = this.treeRows();
    if (!this.tree.rows.some((row) => row.id === selected)) this.tree.selected = this.tree.rows[0]?.id ?? "";
    this.composer.blur();
    this.render();
  }
  closeTree = (): void => {
    if (!this.tree) return;
    this.tree = undefined;
    this.composer.focus();
    this.render();
  };
  /** The row of the tree that the pointer stands on. */
  private treeRow(): TreeRow | undefined {
    return this.tree?.rows.find((row) => row.id === this.tree?.selected);
  }
  /** The rows of the tree. A chain holds its acts, each act holds the acts it made, and a branch stands under the
   * point it starts from. The chain shown and the chains above it are open, and the others folded. */
  private treeRows(): TreeRow[] {
    const w = this.session;
    const chains = w.chains;
    const order = new Map(w.acts.map((act, index) => [act.id, index]));
    const source = (chain: ActRow) => {
      const id = chain.words[1];
      return typeof id === "string" && chains.some((one) => one.id === id) ? id : undefined;
    };
    const path = new Set<string>();
    for (let id: string | undefined = w.selected; id && !path.has(id); ) {
      path.add(id);
      const chain = chains.find((one) => one.id === id);
      id = chain && source(chain);
    }
    // A branch starts after the last point of its source that it reads: the points made before it, but those that the
    // rung that made it took out, and that rung.
    const branches = new Map<string, ActRow[]>();
    for (const chain of chains) {
      const from = source(chain);
      if (!from) continue;
      const maker = w.acts.find((act) => act.id === chain.by);
      const taken = String(maker?.words[0] ?? "").match(/take\(([^)]*)\)/)?.[1] ?? "";
      const omitted = new Set([...taken.matchAll(/"([^"]+)"/g)].map((match) => match[1]));
      const made = order.get(chain.id) ?? 0;
      const point = w.acts.findLast(
        (act) =>
          act.on === from &&
          this.isPoint(act) &&
          (order.get(act.id) ?? 0) < made &&
          !omitted.has(act.id) &&
          act.id !== chain.by &&
          act.by !== chain.by,
      );
      const at = point?.id ?? from;
      branches.set(at, [...(branches.get(at) ?? []), chain]);
    }
    const rows: TreeRow[] = [];
    type Node = { act: ActRow; chain: boolean };
    const visit = (node: Node, lines: string, last: boolean, depth: number, up?: string) => {
      const { act } = node;
      const children: Node[] = [
        ...(node.chain
          ? w.acts.filter(
              (one) =>
                one.on === act.id &&
                this.isPoint(one) &&
                !w.acts.some((maker) => maker.id === one.by && maker.on === act.id && this.isPoint(maker)),
            )
          : w.acts.filter((one) => one.by === act.id && one.on === act.on && this.isPoint(one))
        ).map((one) => ({ act: one, chain: false })),
        ...(branches.get(act.id) ?? []).map((one) => ({ act: one, chain: true })),
      ];
      const folded = this.treeFolds.get(act.id) ?? (node.chain && !path.has(act.id));
      const branch = depth === 0 ? "" : `${lines}${last ? "└─ " : "├─ "}`;
      rows.push({
        ...this.treeLabel(node.act, node.chain),
        id: act.id,
        lines: branch,
        parent: children.length > 0,
        folded,
        up,
      });
      if (folded) return;
      const inner = depth === 0 ? "" : `${lines}${last ? "   " : "│  "}`;
      for (const [index, child] of children.entries())
        visit(child, inner, index === children.length - 1, depth + 1, act.id);
    };
    const roots = chains.filter((chain) => !source(chain));
    for (const root of roots) visit({ act: root, chain: true }, "", true, 0);
    return rows;
  }
  /** The label of a row of the tree, the tag at its right, and what choosing it does. */
  private treeLabel(act: ActRow, chain: boolean): Pick<TreeRow, "parts" | "tag" | "hint" | "run"> {
    const w = this.session;
    if (chain) {
      const current = act.id === w.selected;
      const status = this.session.chainStatus(act.id);
      return {
        parts: [statusMark(status), [w.labelOf(act.id), current ? c.text : c.muted, bold]],
        tag: current ? "current" : act.id,
        hint: "open it",
        run: async () => {
          this.closeTree();
          await w.select(act.id);
        },
      };
    }
    const branch = async () => {
      this.closeTree();
      await this.rewindTo(act);
    };
    if (w.isUserPrompt(act) && !asksOperator(act))
      return {
        parts: [
          [`${glyph.prompt} `, c.accent],
          [String(act.words[1] ?? "").split("\n")[0] ?? "", c.text],
        ],
        tag: act.id,
        hint: "edit it on a new branch",
        run: branch,
      };
    const { mark, color } = this.actState(act);
    const subject = this.subject(act);
    const observation = act.kind === "prompt" && !asksOperator(act) && !w.isUserPrompt(act);
    const name = asksOperator(act) ? "question" : act.kind === "rung" ? act.id : act.kind;
    return {
      parts: observation
        ? [
            [`${mark} `, color],
            [subject.split("\n")[0] ?? "", c.muted],
            [`  ${this.sender(act)}`, c.faint],
          ]
        : [
            [`${mark} `, color],
            [name, c.text],
            // A rung shows the first line of its program, and any other act its words on one line, as the feed does.
            [
              `  ${act.kind === "rung" ? (subject.split("\n")[0] ?? "") : subject.replace(/\s+/g, " ").trim()}`,
              c.muted,
            ],
          ],
      tag: act.kind === "rung" ? "" : act.id,
      hint: "branch after it",
      run: branch,
    };
  }
  private renderTree(): void {
    const tree = this.tree;
    if (!tree) return;
    const selected = tree.selected;
    tree.rows = this.treeRows();
    if (!tree.rows.some((row) => row.id === selected)) tree.selected = tree.rows[0]?.id ?? "";
    const width = this.feedWidth;
    if (this.paneChanged(this.treeBar, [this.theme, width, tree.title])) {
      this.clear(this.treeBar);
      const note =
        tree.title === "Rewind"
          ? "Choose a point to go back to, with the files as they were or not. A change by a command stays."
          : "Open a chain, or go back to a point, with the files as they were or not. A change by a command stays.";
      // The note is cut at its end where the bar has no room for it beside the title and the key that leaves.
      const room =
        width - space.inset * 2 - Bun.stringWidth(tree.title) - space.between * 2 - Bun.stringWidth("Esc");
      this.treeBar.add(
        this.line(
          [
            [tree.title, c.text, bold],
            [`${" ".repeat(space.between)}${clip(note, Math.max(0, room))}`, c.muted],
          ],
          c.muted,
          { flexGrow: 1, flexShrink: 1 },
        ),
      );
      this.treeBar.add(this.text("Esc", c.faint, { run: () => this.closeTree() }));
    }
    if (
      !this.paneChanged(this.scroll, [
        tree.rows.map((row) => [row.id, row.lines, plain(row.parts), row.tag, row.folded, row.parent]),
        tree.selected,
        width,
        this.theme,
      ])
    )
      return;
    this.clear(this.scroll);
    for (const [index, row] of tree.rows.entries()) {
      const active = row.id === tree.selected;
      const line = this.row({
        id: `tree-${row.id}`,
        // Each tree of chains stands apart from the one above it.
        marginTop: index && !row.lines ? space.section : 0,
        paddingX: space.inset,
        backgroundColor: active ? c.selected : c.background,
        onMouseUp: this.click((event) => {
          // A click on the fold of a row folds it, a click on another row points at it, and a click on the row
          // that the pointer is on chooses it.
          const fold = event.x - (this.scroll.x + space.inset + Bun.stringWidth(row.lines)) <= 1;
          if (row.parent && fold) this.foldTree(row.id, !row.folded);
          else if (active) void this.chooseTreeRow();
          else this.pointTree(row.id);
        }),
      });
      if (!active) this.hoverable(line, c.panel);
      const fold = row.parent ? `${row.folded ? glyph.closed : glyph.open} ` : "  ";
      const tag = row.tag ? `  ${row.tag}` : "";
      const room = Math.max(8, width - space.inset * 2 - Bun.stringWidth(row.lines + fold + tag));
      const label = plain(row.parts);
      line.add(
        this.line(
          [
            [row.lines, c.faint],
            [fold, c.faint],
            ...(Bun.stringWidth(label) > room ? this.clipParts(row.parts, room) : row.parts),
          ],
          c.text,
          { flexGrow: 1, flexShrink: 1, truncate: true },
        ),
      );
      if (tag) line.add(this.line(tag, active ? c.muted : c.faint));
      this.scroll.add(line);
    }
    this.scrollAfterLayout({ card: `tree-${tree.selected}` });
  }
  /** Parts cut to a width at their end, with the mark of a cut. */
  private clipParts(parts: readonly Part[], width: number): Part[] {
    const cut: Part[] = [];
    let left = width;
    for (const [text, fg, attributes, bg] of parts) {
      if (left <= 0) break;
      const size = Bun.stringWidth(text);
      if (size < left || (size === left && cut.length === parts.length - 1))
        cut.push([text, fg, attributes, bg]);
      else cut.push([clip(text, left), fg, attributes, bg]);
      left -= size;
    }
    return cut;
  }
  private pointTree(id: string): void {
    if (!this.tree) return;
    this.tree.selected = id;
    this.renderContent();
    this.renderStatus();
  }
  private moveTree(step: number): void {
    const tree = this.tree;
    if (!tree?.rows.length) return;
    const at = tree.rows.findIndex((row) => row.id === tree.selected);
    const next = tree.rows[Math.max(0, Math.min(tree.rows.length - 1, at + step))];
    if (next) this.pointTree(next.id);
  }
  private foldTree(id: string, folded: boolean): void {
    this.treeFolds.set(id, folded);
    this.renderContent();
    this.renderStatus();
  }
  private async chooseTreeRow(): Promise<void> {
    const row = this.treeRow();
    if (!row) return;
    if (row.id === this.session.selected && this.session.chains.some((chain) => chain.id === row.id)) {
      this.closeTree();
      return;
    }
    await Promise.resolve(row.run()).catch(this.report);
  }
  private go(view: View, id?: string): void {
    this.navigation.push({
      chain: this.session.selected,
      view: this.session.view,
      search: this.session.search,
      place: this.place,
      mode: this.session.mode,
    });
    this.closeTree();
    this.session.show(view);
    if (id) this.reveal(id);
    this.render();
    if (id) this.scrollAfterLayout({ card: id });
  }
  /** Open each fold of the feed that holds an act, so that a jump to it finds its card: the earlier steps of its
   * chain, the earlier steps of each prompt and rung over it, and the rung that made it. */
  private reveal(id: string): void {
    const w = this.session;
    const rows = new Map(w.acts.map((one) => [one.id, one]));
    let at = rows.get(id);
    if (!at || at.on !== w.selected) return;
    w.folds[`prompts:${w.selected}`] = false;
    for (let step = 0; at && step < 64; step++) {
      const maker = rows.get(at.by);
      if (!maker || maker.on !== w.selected) break;
      if (maker.kind === "prompt") w.folds[`rungs:${maker.id}`] = false;
      if (maker.kind === "rung") w.folds[at.kind === "prompt" ? `prompts:${maker.id}` : maker.id] = false;
      at = maker;
    }
  }
  private async back(): Promise<void> {
    const previous = this.navigation.pop();
    if (!previous) return;
    await this.session.select(previous.chain);
    this.session.show(previous.view);
    this.session.search = previous.search;
    this.session.mode = previous.mode;
    this.render();
    this.scrollNow(previous.place);
    this.scrollAfterLayout(previous.place);
  }
  /** Scroll to a place, or bring a card into view, once the scroll box has laid out the cards that it holds now,
   * since it clamps an offset to the layout it has and places a card by the layout it has. */
  private scrollAfterLayout(target?: Scroll | { card: string }): void {
    if (target === undefined) return;
    if (this.scrollTarget === undefined) this.renderer.once("frame", this.laidOut);
    this.scrollTarget = target;
  }
  private laidOut = (): void => {
    const target = this.scrollTarget;
    if (this.closed || target === undefined) return;
    // A card that shows an act is found by the act, since the feed keys the card by the turn that tells it.
    if (typeof target === "object")
      this.scroll.scrollChildIntoView(
        this.cards.has(target.card)
          ? target.card
          : ([...this.cards].find(([, card]) => card.act === target.card)?.[0] ?? target.card),
      );
    else this.scrollNow(target);
    this.scrollTarget = undefined;
  };
  /** Scroll to a place as the scroll box is laid out now, which clamps the end to the last offset it has. */
  private scrollNow(place: Scroll): void {
    this.scroll.scrollTo(place === "end" ? this.scroll.scrollHeight : place);
  }
  /** Where the view stands, which is where it scrolls to once it is laid out: the end of a view that follows its end
   * while it stands there, and its offset otherwise. */
  private get place(): Scroll {
    const target = this.scrollTarget;
    if (target !== undefined && typeof target !== "object") return target;
    const { scrollTop, scrollHeight, viewport, stickyScroll } = this.scroll;
    return stickyScroll && scrollTop >= scrollHeight - viewport.height ? "end" : scrollTop;
  }
  /** The chain a step away from the one shown, in the order of the chains, round from the last to the first. */
  private rollChain(step: number): void {
    const w = this.session;
    const chains = w.chains;
    if (chains.length < 2) {
      w.notice = "This session has one chain. ⌃N makes another.";
      this.render();
      return;
    }
    const at = Math.max(
      0,
      chains.findIndex((chain) => chain.id === w.selected),
    );
    const next = chains[(at + step + chains.length) % chains.length];
    if (!next) return;
    void w
      .select(next.id)
      .then(() => {
        w.notice = `${w.labelOf(next.id)}, chain ${chains.indexOf(next) + 1} of ${chains.length}`;
      })
      .catch(w.fail);
  }
  /** The chains of the session, each with its state and the chain it branched from. */
  chains(): void {
    const w = this.session;
    const chains = w.chains;
    this.openPalette(
      "Chains",
      chains.map((chain) => {
        const source = chains.find((one) => one.id === chain.words[1]);
        return {
          label: w.labelOf(chain.id),
          detail: [
            source ? `branched from ${w.labelOf(source.id)}` : "a root chain",
            chain.id === w.selected ? "current" : "",
          ]
            .filter(Boolean)
            .join("   "),
          status: this.session.chainStatus(chain.id),
          run: () => w.select(chain.id),
        };
      }),
      {
        selected: Math.max(
          0,
          chains.findIndex((chain) => chain.id === w.selected),
        ),
        note: "Each chain has its own transcript and module. /tree shows the branches.",
      },
    );
  }
  /** A dialog of choices, which a filter narrows: with the choice that it selects first, a note under its title, and
   * the rows of a rich choice. At a point, it is a menu as wide as its labels need, which stands under the point, or
   * over it where the rows under it cannot hold it, and leaves the screen behind it as it is. */
  openPalette(
    label: string,
    choices: Choice[],
    {
      selected = 0,
      note = "",
      rich = false,
      at,
    }: { selected?: number; note?: string; rich?: boolean; at?: { x: number; y: number } } = {},
  ): void {
    // A dialog or a menu takes the keys, so a name that a row takes ends first, and is kept.
    this.endRename(true);
    this.closeOverlay();
    this.rich = rich;
    this.paletteStart = 0;
    this.composer.blur();
    this.unhover();
    // The dialog is as wide as its longest label and detail need, from 80 columns to 110. It stands over the middle
    // of the feed when the feed holds it, so that it cuts no word of the sidebar, and over the middle of the screen
    // when it is wider, so that it cuts none of its own.
    const widest = (pick: (choice: Choice) => string) =>
      Math.max(0, ...choices.map((choice) => Bun.stringWidth(pick(choice))));
    const wanted = Math.max(
      80,
      Math.min(110, widest((choice) => choice.label) + widest((choice) => choice.detail) + 10),
    );
    const column = this.feedWidth + space.gutter * 2;
    const stage = column - 4 >= wanted ? column : this.renderer.width;
    // A menu holds its title with the key that closes it, and each label after the pointer of the list.
    const menu = Math.min(
      this.renderer.width - 2,
      space.inset * 2 +
        Math.max(
          Bun.stringWidth(label) + space.between * 2 + 3,
          widest((choice) => choice.label) + 2 + space.between,
        ),
    );
    this.paletteWidth = at ? menu : Math.min(stage - 4, wanted);
    const width = this.paletteWidth;
    // The rows of a menu: its inset, its title with the space under it, and its choices.
    const rows = space.inset * 2 + space.bar + space.section + choices.length;
    // A veil dims the screen behind the dialog, and a click on it closes the dialog. The veil of a menu is clear, and a
    // right click on it closes the menu too.
    this.backdrop = this.box({
      position: "absolute",
      left: 0,
      top: 0,
      width: "100%",
      height: "100%",
      backgroundColor: at ? "transparent" : c.backdrop,
      zIndex: 19,
      onMouseDown: (event) => {
        if (event.button === 2) this.closeOverlay();
      },
      onMouseUp: this.click(() => this.closeOverlay()),
    });
    this.root.add(this.backdrop);
    this.overlay = this.box({
      id: "palette",
      position: "absolute",
      left: at
        ? Math.max(1, Math.min(at.x, this.renderer.width - width - 1))
        : Math.floor((stage - width) / 2),
      top: at
        ? at.y + 1 + rows <= this.renderer.height
          ? at.y + 1
          : Math.max(0, at.y - rows)
        : Math.max(1, Math.min(4, Math.floor(this.renderer.height / 8))),
      width,
      maxHeight: this.paletteHeight,
      paddingX: space.inset,
      paddingY: space.inset,
      gap: space.stack,
      backgroundColor: c.raised,
      zIndex: 20,
    });
    this.root.add(this.overlay);
    // The title and the filter start where the labels of the list start, and the prompt of the filter stands over the
    // pointer of the list.
    const heading = this.row({ paddingLeft: space.between, paddingRight: space.inset });
    heading.add(this.text(label, c.text, { attributes: bold, truncate: true, flexGrow: 1, flexShrink: 1 }));
    heading.add(this.text("Esc", c.faint, { run: () => this.closeOverlay() }));
    this.overlay.add(heading);
    this.paletteNote = note ? space.bar : 0;
    if (note)
      this.overlay.add(this.inset(space.between, this.line(clip(note, this.paletteWidth - 8), c.muted)));
    // A short list shows its filter only once the operator types in it, and until then the filter takes no row.
    const quiet = choices.length <= 5;
    this.paletteInputRow = this.row({
      height: quiet ? 0 : space.bar,
      paddingRight: space.inset,
      marginBottom: space.section,
    });
    const prompt = this.text(`${glyph.prompt} `, c.accent, { visible: !quiet });
    this.paletteInputRow.add(prompt);
    this.paletteInput = new InputRenderable(this.renderer, {
      id: "palette-search",
      flexGrow: 1,
      placeholder: quiet ? "" : "Type to filter",
      ...inputColors(c.raised),
    });
    this.paletteInputRow.add(this.paletteInput);
    this.overlay.add(this.paletteInputRow);
    this.paletteList = this.box({
      gap: space.stack,
      onMouseScroll: (event) => {
        const direction = event.scroll?.direction;
        if (direction !== "up" && direction !== "down") return;
        this.selection = Math.max(
          0,
          Math.min(this.filtered.length - 1, this.selection + (direction === "up" ? -1 : 1)),
        );
        this.renderChoices();
      },
    });
    this.overlay.add(this.paletteList);
    // A label that starts with the filter comes first, then a label that holds it, then a detail that holds it.
    const filter = (value: string) => {
      const wanted = value.toLowerCase();
      const rank = (choice: Choice) => {
        const label = choice.label.trim().toLowerCase();
        const command = choice.command?.toLowerCase() ?? "";
        return label.startsWith(wanted) || command.startsWith(wanted) || command.startsWith(`/${wanted}`)
          ? 0
          : label.includes(wanted)
            ? 1
            : 2;
      };
      return choices
        .filter((choice) =>
          `${choice.label} ${choice.detail} ${choice.command ?? ""} ${choice.keys ?? ""}`
            .toLowerCase()
            .includes(wanted),
        )
        .map((choice, index) => ({ choice, index, rank: wanted ? rank(choice) : 0 }))
        .sort((one, other) => one.rank - other.rank || one.index - other.index)
        .map(({ choice }) => choice);
    };
    this.filtered = filter("");
    this.selection = Math.max(0, selected);
    this.paletteInput.on(InputRenderableEvents.INPUT, (value: string) => {
      prompt.visible = !quiet || value !== "";
      if (this.paletteInputRow) this.paletteInputRow.height = prompt.visible ? space.bar : 0;
      this.filtered = filter(value);
      this.selection = 0;
      this.paletteStart = 0;
      this.renderChoices();
    });
    this.paletteInput.on(InputRenderableEvents.ENTER, () => this.choose());
    this.renderChoices();
    this.paletteInput.focus();
  }
  /** The rows the palette may take: all but a row above and below for a question, whose text stands above its
   * input, and all but eight otherwise. */
  private get paletteHeight(): number {
    return this.overlay && this.questionDocument?.parent === this.overlay
      ? this.renderer.height - 2
      : Math.max(12, Math.min(34, this.renderer.height - 8));
  }
  private renderChoices(): void {
    if (!this.paletteList) return;
    this.clear(this.paletteList);
    // The list has the rows of the palette but its inset, its heading, its note, its input with the space under it,
    // and the text it asks.
    const asked = this.questionDocument?.parent === this.overlay ? (this.questionDocument?.height ?? 0) : 0;
    const room = Math.max(2, this.paletteHeight - space.inset * 2 - 3 * space.bar - this.paletteNote - asked);
    // A rich choice takes a row for its label, a row for its command when it has one, a row for its detail, and a
    // row of space before the next one.
    // A choice that opens a part of the list takes a row more for its title, and a row of space above it.
    const titled = (index: number) =>
      Boolean(this.filtered[index]?.heading) &&
      this.filtered[index]?.heading !== this.filtered[index - 1]?.heading;
    // The first choice that the list shows carries the title of its part, with no space above it.
    const cost = (index: number, first: boolean) => {
      const choice = this.filtered[index];
      if (!choice) return 0;
      const above = first ? (choice.heading ? 1 : 0) : titled(index) ? 2 : 0;
      return (this.rich ? (choice.command ? 3 : 2) : 1) + above;
    };
    const gap = this.rich ? space.section : 0;
    const span = (from: number, to: number) => {
      let sum = (to - from) * gap;
      for (let index = from; index <= to; index++) sum += cost(index, index === from);
      return sum;
    };
    // A list that the palette cannot hold whole keeps a row of space and a row for its count under it, and scrolls as
    // little as it takes to show the selected choice.
    const fits = span(0, this.filtered.length - 1) <= room ? room : room - space.section - space.bar;
    let start = Math.min(this.paletteStart, this.selection);
    while (start < this.selection && span(start, this.selection) > fits) start++;
    this.paletteStart = start;
    let end = start;
    while (end + 1 < this.filtered.length && span(start, end + 1) <= fits) end++;
    const shown = this.filtered.slice(start, end + 1);
    const inner = this.paletteWidth - space.inset * 2;
    // The details stand in one column after the labels, which take at most half of the row, or what short details
    // leave.
    const widest = (width: (choice: Choice) => number) => Math.max(0, ...this.filtered.map(width));
    const column = Math.min(
      Math.max(
        Math.floor(inner / 2),
        inner -
          space.between -
          widest(
            (choice) =>
              Bun.stringWidth(choice.detail.replace(/\s*\n\s*/g, " ")) +
              (choice.swatch ? choice.swatch.length * 2 + space.between : 0),
          ),
      ),
      widest((choice) => Bun.stringWidth(choice.label) + (choice.mark || choice.status ? 2 : 0)) +
        space.between,
    );
    for (const [offset, choice] of shown.entries()) {
      const index = start + offset;
      const selected = index === this.selection;
      if (titled(index) || (offset === 0 && choice.heading))
        this.paletteList.add(
          this.inset(
            space.between,
            this.line([[choice.heading ?? "", c.muted, bold]], c.muted, {
              marginTop: offset ? space.section : 0,
            }),
          ),
        );
      const block = this.box({
        backgroundColor: selected ? c.selected : c.raised,
        marginTop: offset && this.rich ? gap : 0,
        onMouseUp: this.click(() => {
          this.selection = index;
          this.choose();
        }),
        onMouseOver: () => {
          if (this.selection === index) return;
          this.selection = index;
          this.renderChoices();
        },
      });
      if (this.rich) {
        // The bar of the selected choice runs down all its rows.
        const bar: Part = [`${selected ? glyph.mark : " "} `, c.accent];
        const width = inner - 2;
        const keys = choice.keys ?? "";
        const top = this.row();
        top.add(
          this.text([bar, [clip(choice.label, width - Bun.stringWidth(keys) - 2), c.text, bold]], c.text, {
            flexGrow: 1,
            flexShrink: 1,
            truncate: true,
          }),
        );
        if (keys) top.add(this.text(keys, selected ? c.muted : c.faint, { paddingRight: space.inset }));
        block.add(top);
        if (choice.command)
          block.add(this.line([bar, [clip(choice.command, width), selected ? c.accent : c.secondary]]));
        block.add(this.line([bar, [clip(choice.detail.replace(/\s*\n\s*/g, " "), width), c.muted]], c.muted));
        this.paletteList.add(block);
        continue;
      }
      block.flexDirection = "row";
      block.height = space.bar;
      const fg = choice.color ?? c.text;
      block.add(this.text(`${selected ? glyph.pointer : " "} `, c.accent, { attributes: bold }));
      const mark: Part | undefined = choice.mark ?? (choice.status ? statusMark(choice.status) : undefined);
      // A label or a detail that the row cut shows whole in a tip while the pointer is over it.
      const markText = mark ? mark[0] : "";
      block.add(
        this.whole(
          this.text(
            [
              ...(mark ? [mark] : []),
              // A label is cut at its end where the column of the details starts, a gap before it.
              [
                choice.detail
                  ? clip(
                      choice.label,
                      Math.max(4, column - space.between - (mark ? Bun.stringWidth(mark[0]) : 0)),
                    )
                  : choice.label,
                fg,
                selected ? bold : 0,
              ],
            ],
            fg,
            {
              width: choice.detail ? column : undefined,
              flexShrink: choice.detail ? 0 : 1,
              truncate: true,
            },
          ),
          () => `${markText}${choice.label}`,
        ),
      );
      if (choice.swatch)
        block.add(
          this.text(
            choice.swatch.map((hex): Part => [`${glyph.chip} `, RGBA.fromHex(hex)]),
            c.text,
            { width: choice.swatch.length * 2 + space.between },
          ),
        );
      if (choice.detail)
        block.add(
          this.whole(
            this.text(
              clip(
                choice.detail.replace(/\s*\n\s*/g, " "),
                Math.max(
                  8,
                  inner -
                    space.between -
                    column -
                    (choice.swatch ? choice.swatch.length * 2 + space.between : 0),
                ),
              ),
              selected ? c.text : c.muted,
              { truncate: true, flexShrink: 1 },
            ),
            () => choice.detail.replace(/\s*\n\s*/g, " "),
          ),
        );
      this.paletteList.add(block);
    }
    if (!this.filtered.length && this.paletteInput?.value)
      this.paletteList.add(this.inset(space.between, this.text("Nothing matches this filter.", c.muted)));
    else if (shown.length < this.filtered.length)
      this.paletteList.add(
        this.inset(
          space.between,
          this.text(`${this.selection + 1} of ${this.filtered.length}`, c.faint, {
            marginTop: space.section,
          }),
        ),
      );
  }
  private choose(): void {
    const choice = this.filtered[this.selection];
    if (!choice) return;
    this.closeOverlay();
    void Promise.resolve()
      .then(() => choice.run())
      .catch(this.report);
  }
  closeOverlay(): void {
    this.authenticationOpen = false;
    if (this.menuItem) {
      this.menuItem = undefined;
      this.schedule();
    }
    this.overlay?.destroyRecursively();
    this.backdrop?.destroyRecursively();
    this.overlay = undefined;
    this.backdrop = undefined;
    this.paletteInput = undefined;
    this.paletteInputRow = undefined;
    this.paletteList = undefined;
    this.questionDocument = undefined;
    this.answering = undefined;
    this.composer?.focus();
  }
  /** The keys that this terminal sends, what each mark means, and the commands. */
  help(): void {
    const legend: [string, RGBA, string][] = [
      [`${spin(0)} Working`, c.accent, "A model or a command runs"],
      [`${glyph.running} Running`, c.accent, "A chain or a session has work in progress"],
      [`${glyph.asks} Input needed`, c.warning, "A question waits for your answer"],
      [`${glyph.notice} Notice`, c.faint, "A notice tells you something and asks nothing"],
      [`${glyph.held} Paused`, c.warning, "Work waits until you wake it"],
      [`${glyph.done} Done`, c.success, "The act ended and gave its value"],
      [
        `${glyph.dot} Finished, unread`,
        c.success,
        "A chain you started, or a session, ended while you were away",
      ],
      [`${glyph.failed} Failed`, c.danger, "The act raised, or the gate refused its Python"],
      [`${glyph.ring} Ready`, c.faint, "Nothing runs, or the work is saved or pending"],
    ];
    this.openPalette(
      "Keys and commands",
      keys
        .map(
          (key): Choice => ({
            label: chords(key, this.kitty),
            detail: key.action,
            heading: "Keys",
            run: () => {},
          }),
        )
        .concat(
          legend.map(([mark, color, meaning]) => ({
            label: mark,
            detail: meaning,
            color,
            heading: "Marks",
            run: () => {},
          })),
        )
        .concat(
          slashes.map(({ usage, detail }) => ({
            label: usage,
            detail,
            heading: "Slash commands",
            run: () => {},
          })),
        ),
      { note: "The chords that this terminal sends, what each mark means, and every command." },
    );
  }
  /** The layers of the keys of the App, from the top: a layer that holds takes a key before the layers under it, a
   * command that does not hold lets its key go on, and a key that no layer takes goes to the part that has the focus.
   * The footer offers the keys of the top layer that offers any. It gives what takes each layer away again. */
  private layers(): (() => void)[] {
    const offs: (() => void)[] = [];
    // Each layer that comes later stands under the ones before it.
    const layer = (enabled: () => boolean, group: string, keys: KeyCommand[]) =>
      offs.push(
        this.keymap.registerLayer({
          priority: -offs.length,
          enabled,
          commands: keys.map(({ run, when }, index) => ({
            name: `${offs.length}-${index}`,
            run: ({ event }) => run(event) !== false,
            ...(when ? { enabled: when } : {}),
          })),
          bindings: keys.flatMap(({ on, hint }, index) =>
            [on].flat().map((key, first) => ({
              key,
              cmd: `${offs.length}-${index}`,
              ...(hint && !first ? { hint: [...hint, group] } : {}),
            })),
          ),
        }),
      );
    const free = () => !this.renaming?.input?.focused;
    const bare = () => free() && !this.overlay;
    const row = () => this.treeRow();
    const chosen = () => Boolean(this.suggestions[this.suggestionIndex]);
    // The keys of the table, which act while no dialog is open, or at all times.
    const table = (always: boolean): KeyCommand[] =>
      bindings.flatMap(({ key, binding }) => {
        const { run, on = [], hint } = binding;
        if (!run || Boolean(binding.always) !== always) return [];
        const offered: Hint | undefined = hint ? [() => shown(key, binding, this.kitty), hint] : undefined;
        return { on, run: (event) => void this.actions[run](event), when: this.when[run], hint: offered };
      });
    // While a row takes a new name, Escape or Ctrl+C leaves the name as it was, and Ctrl+Q keeps it and quits.
    layer(() => !free(), "rename", [
      { on: ["escape", "ctrl+c"], run: () => this.endRename(false) },
      {
        on: "ctrl+q",
        run: () => {
          this.endRename(true);
          void this.options.quit();
        },
      },
    ]);
    // While a question of a choice is aimed, its list takes the arrows, Enter and the numbers, and Escape gives the
    // list back the keys that Other gave the input.
    const choosing = () => bare() && !this.tree && Boolean(this.picking());
    layer(() => choosing() && !this.choice.other, "choices", [
      { on: "up", run: () => this.stepPick(-1), hint: ["↑↓", "choose"] },
      { on: "down", run: () => this.stepPick(1) },
      { on: ["return", "enter"], run: () => void this.takePick(), hint: ["Enter", "answer"] },
      ...Array.from(
        { length: 9 },
        (_, at): KeyCommand => ({
          on: String(at + 1),
          run: () => void this.takePick(at),
          when: () => at <= (this.picking()?.choices.length ?? -1),
        }),
      ),
    ]);
    layer(() => choosing() && this.choice.other, "other", [
      {
        on: "escape",
        run: () => {
          this.choice = { ...this.choice, other: false };
          this.render();
        },
        hint: ["Esc", "back to the values"],
      },
    ]);
    layer(() => bare() && Boolean(this.tree), "tree", [
      ...Object.entries(steps).map(
        ([on, step]): KeyCommand => ({
          on,
          run: () => this.moveTree(step),
          hint: on === "up" ? ["↑↓", "move"] : undefined,
        }),
      ),
      { on: "right", run: () => this.unfoldTree(), when: () => Boolean(row()?.parent), hint: ["←→", "fold"] },
      { on: "left", run: () => this.foldTreeUp(), when: () => Boolean(row()) },
      {
        on: ["return", "enter"],
        run: () => void this.chooseTreeRow(),
        hint: ["Enter", () => row()?.hint ?? "choose"],
      },
      { on: "escape", run: () => this.closeTree(), hint: ["Esc", "back"] },
    ]);
    layer(() => bare() && this.suggestionBox.visible, "suggestions", [
      { on: "up", run: () => this.stepSuggestion(-1), hint: ["↑↓", "choose"] },
      { on: "down", run: () => this.stepSuggestion(1) },
      { on: "tab", run: () => this.takeSuggestion(false), when: chosen, hint: ["Tab", "complete"] },
      { on: ["return", "enter"], run: () => this.takeSuggestion(true), when: chosen },
      { on: "escape", run: () => this.dismissSuggestions(), hint: ["Esc", "hide"] },
    ]);
    layer(() => bare() && Boolean(this.session.editing), "edit", [
      { on: ["return", "enter"], run: () => void this.submit(), hint: ["Enter", "run"] },
      { on: "escape", run: this.escape, hint: ["Esc", "leave the program"] },
    ]);
    // While the operator watches a life and the input is empty, [ and ] move the moment of the feed, b starts a
    // branch there, and Escape comes back to now.
    const watched = () => bare() && this.session.watching && !this.composer.plainText;
    const past = () => this.session.moment !== undefined;
    layer(watched, "watch", [
      { on: "[", run: () => this.session.travel(-1), hint: ["[ ]", "move in time"] },
      { on: "]", run: () => this.session.travel(1) },
      {
        on: "b",
        run: () => void this.session.branchHere().catch(this.report),
        when: past,
        hint: ["b", "branch here"],
      },
      {
        on: "escape",
        run: () => {
          this.session.moment = undefined;
          this.renderContent();
        },
        when: past,
        hint: ["Esc", "now"],
      },
    ]);
    // While the keys move on the loom or the graph, the arrows, Enter and Escape move there, and a letter acts on the
    // selected chain.
    const stageKey = (on: string | string[], run: () => unknown, hint?: Hint): KeyCommand => ({
      on,
      run: () => void Promise.resolve(run()).catch(this.report),
      hint,
    });
    layer(() => bare() && this.steering && this.lifeView !== "main", "stage", [
      stageKey("up", () => this.arrowStage("up"), ["↑↓←→", "move"]),
      ...["down", "left", "right", "pageup", "pagedown"].map((on) => stageKey(on, () => this.arrowStage(on))),
      stageKey(["return", "enter"], () => this.enterStage(), [
        "Enter",
        () => (this.lifeView === "graph" ? "zoom in" : "input"),
      ]),
      stageKey("escape", () => this.leaveStage(), [
        "Esc",
        () => (this.lifeView === "graph" && this.graph.level !== "threads" ? "zoom out" : "back"),
      ]),
      stageKey("[", () => this.travel(-1), ["[ ]", "time"]),
      stageKey("]", () => this.travel(1)),
      stageKey("{", () => this.travel(-10)),
      stageKey("}", () => this.travel(10)),
      stageKey("space", () => this.holdChain(), ["Space", "pause"]),
      stageKey("x", () => this.cancelChain(), ["x", "cancel"]),
      stageKey("n", () => this.step(), ["n", "step"]),
      stageKey("b", () => this.branchAtCursor(), ["b", "branch"]),
      stageKey("tab", () => this.question(), ["Tab", "answer"]),
    ]);
    // While the keys move in the sidebar, the arrows move them, Enter opens a chain or chooses a thread, Escape goes
    // back, and a letter acts on the thread under them.
    const onThread = (run: (thread: ChainThread) => void) => () => {
      const thread = this.railThread();
      if (thread) run(thread);
    };
    layer(() => bare() && this.railKeys, "rail", [
      { on: "up", run: () => this.moveRail(-1), hint: ["↑↓", "move"] },
      { on: "down", run: () => this.moveRail(1) },
      {
        on: ["return", "enter", "right"],
        run: () => this.enterRail(),
        hint: ["Enter", () => (this.railChain ? "choose" : "threads")],
      },
      {
        on: ["escape", "left"],
        run: () => this.leaveRail(),
        hint: ["Esc", () => (this.railChain ? "chains" : "input")],
      },
      {
        on: "space",
        run: onThread((thread) => this.foldThread(thread.act.id)),
        when: () => Boolean(this.railThread()),
        hint: ["Space", "collapse"],
      },
      {
        on: "r",
        run: onThread((thread) => this.resolveThread(thread)),
        when: () => this.railThread()?.resolved === false,
        hint: ["r", "resolve"],
      },
      {
        on: "o",
        run: onThread((thread) => this.reopenThread(thread.act.id)),
        when: () => this.railThread()?.resolved === true,
        hint: ["o", "open again"],
      },
    ]);
    layer(bare, "", [
      // As in Claude Code, Ctrl+D acts on an empty input.
      { on: "ctrl+d", run: () => this.exit(), when: () => !this.composer.plainText },
      ...table(false),
      {
        on: ["up", "down", "meta+up", "meta+down"],
        run: this.walkHistory,
        when: () => this.composer.focused,
      },
      // Ctrl+J arrives as a line feed from a terminal with no kitty keyboard protocol.
      {
        on: ["shift+return", "shift+enter", "ctrl+j", "linefeed"],
        run: this.newline,
        when: () => this.writesPython,
      },
    ]);
    layer(free, "", [
      { on: ["tab", "shift+tab"], run: this.moveAsk, when: () => Boolean(this.overlay && this.answering) },
      ...table(true),
      { on: "ctrl+c", run: () => this.cancel() },
      { on: "escape", run: this.escape },
      {
        on: ["pageup", "pagedown"],
        run: this.pageQuestion,
        when: () => Boolean(this.overlay && this.questionDocument),
      },
      ...["up", "down", "pageup", "pagedown"].map(
        (on): KeyCommand => ({
          on,
          run: () => this.stepChoice(steps[on] ?? 0),
          when: () => Boolean(this.overlay),
        }),
      ),
    ]);
    return offs;
  }
  /** Whether this terminal sends the chords of the kitty keyboard protocol. */
  private get kitty(): boolean {
    return this.renderer.capabilities?.kitty_keyboard === true;
  }
  /** The suggestion a step away from the one chosen, round from the last to the first. */
  private stepSuggestion(step: number): void {
    const count = this.suggestions.length;
    if (count) this.suggestionIndex = (this.suggestionIndex + step + count) % count;
    this.renderSuggestions();
  }
  /** The suggestion chosen, completed in the input, or sent when Enter takes one that sends itself. */
  private takeSuggestion(enter: boolean): void {
    const chosen = this.suggestions[this.suggestionIndex];
    if (enter && chosen?.submit) chosen.submit();
    else if (chosen) this.complete(chosen);
  }
  /** The text that a question asks, scrolled by a page. */
  private pageQuestion = (key?: KeyEvent) => this.questionDocument?.scrollBy(key?.name === "pageup" ? -8 : 8);
  /** The choice of the dialog a step away from the one chosen, within the list. */
  private stepChoice(step: number): void {
    this.selection = Math.max(0, Math.min(this.filtered.length - 1, this.selection + step));
    this.renderChoices();
  }
  /** The row of the rewind tree unfolded, or the pointer on its first child when it is open. */
  private unfoldTree(): void {
    const row = this.treeRow();
    if (row?.folded) this.foldTree(row.id, false);
    else this.moveTree(1);
  }
  /** The row of the rewind tree folded, or the pointer on the row above it when it is folded. */
  private foldTreeUp(): void {
    const row = this.treeRow();
    if (row?.parent && !row.folded) this.foldTree(row.id, true);
    else if (row?.up) this.pointTree(row.up);
  }
  /** Ctrl+D on an empty input asks once, and exits when it is pressed again while it asks. */
  private exit(): void {
    if (this.session.notice === exitNotice) void this.options.quit();
    else this.session.notice = exitNotice;
  }
  /** Whether the input holds Python: Python input, or a program under edit. */
  private get writesPython(): boolean {
    return this.session.mode === "python" || Boolean(this.session.editing);
  }
  /** A new line in the input, as deep as the line before it, and deeper after a colon. */
  private newline = (): void => {
    const before = this.beforeCursor().split("\n").at(-1) ?? "";
    this.composer.insertText(
      `\n${before.match(/^\s*/)?.[0] ?? ""}${before.trimEnd().endsWith(":") ? "  " : ""}`,
    );
  };
  /** Ctrl+C closes a dialog or the rewind tree, or clears the input, or cancels the last act that the operator made on
   * the chain, which takes back a message of his. */
  private cancel(): void {
    if (this.overlay) this.closeOverlay();
    else if (this.tree) this.closeTree();
    else if (this.composer.plainText) this.composer.replaceText("");
    else this.action("/cancel");
  }
  /** Up on an empty input takes back the last message queued on the chain, and Up and Down at the first and the last
   * line of the input walk the history of what it sent, as a shell does. Alt+Up and Alt+Down walk it from any line.
   * It gives false when it does nothing, and the input moves its cursor. */
  private walkHistory = (key?: KeyEvent): boolean => {
    const up = key?.name === "up";
    const anywhere = key?.meta;
    const queued = this.session.queued.findLast((entry) => entry.chain === this.session.selected);
    if (up && !anywhere && !this.composer.plainText && queued && this.historyIndex < 0) {
      this.session.removeQueued(queued.id);
      this.insert(queued.text);
      this.session.notice = "The queued message is back in the input. ⌥Enter queues it again.";
      return true;
    }
    const cursor = this.composer.visualCursor.visualRow;
    const edge = up ? cursor === 0 : cursor >= this.composer.virtualLineCount - 1;
    const history = this.history();
    if (!(anywhere || edge) || !history.length || !(up || this.historyIndex >= 0)) return false;
    if (this.historyIndex < 0) {
      this.historyDraft = this.composer.plainText;
      this.historyIndex = history.length;
    }
    this.historyIndex = Math.max(0, Math.min(history.length, this.historyIndex + (up ? -1 : 1)));
    const text = history[this.historyIndex] ?? this.historyDraft;
    if (this.historyIndex === history.length) this.historyIndex = -1;
    this.composer.replaceText(text);
    return true;
  };
  /** Escape closes what is open, or pauses the model at work. An Escape with nothing to do asks for a second one,
   * which opens the rewind tree. */
  private escape = (): void => {
    const open = Boolean(this.overlay || this.searchRow.visible || this.session.editing);
    if (!open && this.pausing()) this.action("/pause");
    else if (!open) {
      const now = Date.now();
      if (now - this.escapedAt <= twice) {
        this.escapedAt = 0;
        if (this.session.notice === rewindNotice) this.session.notice = "";
        this.rewind();
        return;
      }
      this.escapedAt = now;
      this.session.notice = rewindNotice;
      // The notice lasts as long as a second Escape rewinds.
      setTimeout(() => {
        if (!this.closed && this.session.notice === rewindNotice) this.session.notice = "";
      }, twice).unref();
    }
    this.closeOverlay();
    this.closeSearch();
    this.leaveEdit();
  };
  /** Whether Escape pauses the chain: a model is at work on it, and no pause holds it. */
  private pausing(): boolean {
    const w = this.session;
    return !w.paused && w.open.some((act) => act.kind === "prompt" && !asksOperator(act));
  }
  /** What each action of the table of keys does, with the key that ran it. */
  private readonly actions: Record<Action, (key?: KeyEvent) => unknown> = {
    queue: () => {
      // The queue holds messages to a model. Python input and a program under edit run when Enter sends them.
      if (this.writesPython) {
        this.session.notice = "Only a message can wait in the queue. Enter runs this Python now.";
        return;
      }
      this.session.enqueue(this.composer.plainText);
      this.composer.setText("");
    },
    stash: () => this.stash(),
    view: (key) => this.showView(views[Number(key?.name) - 1] ?? "feed"),
    // ⌘Tab rolls the chains too where the system and the terminal pass it, which macOS does not, since it keeps ⌘Tab
    // to switch applications.
    roll: (key) => this.rollChain(key?.shift ? -1 : 1),
    palette: () => this.palette(),
    chains: () => this.chains(),
    newChain: () => this.insert("/chain "),
    models: () => this.models(),
    effort: () => this.effortPicker(),
    themes: () => this.themes(),
    workspaces: () => void this.workspacePicker(),
    sidebar: () => this.options.workspaces.toggle(),
    details: () => this.details(),
    editor: () => void this.editDraft().catch(this.report),
    image: () => void clipboardImage((path) => this.session.attachImage(path)).catch(this.report),
    search: () => this.openSearch(),
    scroll: (key) =>
      this.scroll.scrollBy((key?.name === "pageup" ? -1 : 1) * Math.max(1, this.scroll.viewport.height - 2)),
    python: () => this.toggleMode(),
    complete: () => void this.completeNames().catch(this.report),
    ladders: () => this.ladders(),
    answer: () => this.question(),
    names: () => void this.names().catch(this.report),
    copy: () => {
      const selection = this.renderer.getSelection()?.getSelectedText();
      if (selection) this.renderer.copyToClipboardOSC52(selection);
    },
    back: () => void this.back().catch(this.report),
    jump: (key) => this.jumpMessage(["]", "n"].includes(key?.name ?? "") ? 1 : -1),
    scope: (key) => this.session.review(key?.name === "pageup" ? -1 : 1),
    pause: this.escape,
    help: () => this.help(),
    quit: () => void this.options.quit(),
    lifeView: () => this.switchView(),
    steer: () => this.steer(),
    rail: () => this.railSteer(),
    repair: () => void this.session.repairJournal().catch(this.session.fail),
    renew: () => void this.session.renewJournal().catch(this.session.fail),
  };
  /** When an action of the table acts, where it does not act at all times: a key that does not act goes on to the
   * input, and the footer does not offer it. */
  private readonly when: Partial<Record<Action, () => boolean>> = {
    answer: () => this.session.desk.asks.length > 0,
    scope: () => this.session.view === "changes",
    repair: () => this.session.journalDamaged,
    renew: () => this.session.journalDamaged,
    pause: () => !this.searchRow.visible && this.pausing(),
  };
  /** The view shown, with the rewind tree closed. */
  showView(view: View): void {
    this.closeTree();
    this.session.show(view);
  }
  private openSearch(): void {
    this.closeTree();
    this.searchRow.visible = true;
    this.search.focus();
  }
  private closeSearch(): void {
    if (!this.searchRow.visible && !this.session.search) return;
    this.searchRow.visible = false;
    this.search.value = "";
    this.session.search = "";
    this.composer.focus();
    this.render();
  }
  private dismissSuggestions(): void {
    const token = this.token();
    this.dismissed = token ? `${token.kind}${token.text}` : "";
    this.suggest();
  }
  private leaveEdit(): void {
    if (!this.session.editing) return;
    this.session.editing = undefined;
    this.render();
  }
  /** The input put aside, or brought back: the input and the text put aside for its draft trade places. */
  stash(): void {
    const w = this.session;
    const key = this.draftKey;
    const text = this.composer.plainText;
    const kept = w.stashes[key] ?? "";
    if (!text && !kept) {
      w.notice = "⌃S puts the input aside. The input is empty.";
      return;
    }
    if (text) w.stashes[key] = text;
    else delete w.stashes[key];
    this.composer.setText(kept);
    this.composer.cursorOffset = kept.length;
    w.notice = text
      ? kept
        ? "The input and the text put aside traded places."
        : "The input is put aside. ⌃S brings it back."
      : "The text put aside is back.";
    w.save();
    this.render();
  }
  /** The feed scrolled to the message of the operator before or after the top of the view. */
  private jumpMessage(step: number): void {
    const w = this.session;
    if (this.tree || w.view !== "feed") return;
    const top = this.scroll.viewport.y;
    const messages = this.scroll
      .getChildren()
      .filter((node) => {
        const act = w.acts.find((one) => one.id === this.cards.get(node.id)?.act);
        return act && w.isUserPrompt(act) && !asksOperator(act);
      })
      .map((node) => node.y - top);
    const offset = step > 0 ? messages.find((at) => at > 0) : messages.findLast((at) => at < 0);
    if (offset !== undefined) this.scroll.scrollBy(offset);
  }
  /** The App ends once, by its own quit or as its renderer ends. It stops each timer and each frame to come first,
   * so that nothing draws on a buffer that is gone, and it keeps the draft while the input still holds it. */
  dispose = (): void => {
    if (this.closed) return;
    this.closed = true;
    clearInterval(this.tick);
    clearTimeout(this.redraw);
    clearTimeout(this.hoverTimer);
    this.redraw = this.hoverTimer = undefined;
    this.renderer.off("frame", this.laidOut);
    this.renderer.off("destroy", this.dispose);
    if (!this.composer.isDestroyed) this.keepDraft();
    this.session.scrolls[this.lastView] = this.place;
    this.session.save();
    this.session.off("change", this.schedule);
    this.options.workspaces.off("change", this.schedule);
    this.session.off("compose", this.compose);
    this.session.off("resume", this.resume);
    this.session.off("shared", this.shared);
    this.session.off("confirm", this.confirm);
    this.session.off("authentication", this.authentication);
    this.session.off("codexModels", this.codexModels);
    for (const off of this.layersOff) off();
    this.partKeysOff?.();
    this.parts.dispose();
    this.renderer.off("resize", this.render);
    this.renderer.off("selection", this.copySelection);
    this.renderer.off("theme_mode", this.modeChanged);
    this.renderer.removeFrameCallback(this.drawDue);
    this.renderer.off("focus", this.focusTaken);
    this.renderer.off("blur", this.focusLost);
    this.root.destroyRecursively();
    this.style.destroy();
  };
  /** The scopes of the review of the changes: the one shown in bold, and each other one a link to it. */
  private scopes(): BoxRenderable {
    const w = this.session;
    const row = this.row({ gap: space.between, flexShrink: 0 });
    for (const scope of scopes)
      row.add(
        scope === w.scope
          ? this.text([[scopeLabels[scope], c.text, bold | underline]])
          : this.link(scopeLabels[scope], () => w.review(scope)),
      );
    row.add(this.text("⌃PageUp / ⌃PageDown", c.faint));
    return row;
  }
  /** The feed scrolled so that the card of an id stands at its top. */
  private jumpTo(id: string): void {
    const node = this.scroll.getChildren().find((one) => one.id === id);
    if (node) this.scroll.scrollBy(node.y - this.scroll.viewport.y);
  }
  /** A comment on one line of a hunk: the operator picks the line, and the input holds the start of a message that
   * names it, which Enter sends to the chain as a prompt. */
  private commentLine(path: string, hunk: Hunk): void {
    const shown = lines(hunk);
    this.openPalette(
      `Comment a line of ${path}`,
      shown.map((line) => ({
        label: `${line.sign} ${line.text}`.trimEnd(),
        detail: `${line.sign === "-" ? "removed " : ""}line ${line.number}`,
        run: () => {
          const start = commentStart(path, line);
          this.insert(start);
          this.composer.cursorOffset = start.length;
        },
      })),
      {
        selected: Math.max(
          0,
          shown.findIndex((line) => line.sign !== " "),
        ),
      },
    );
  }
}

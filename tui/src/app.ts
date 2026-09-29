import { join } from "node:path";
import {
  actorParts,
  display,
  imageContent,
  imagePath,
  imageReferences,
  OPERATOR,
  type Stream,
  safeText,
  shapes,
  TIMEOUT,
  WORK,
} from "@furb/engine";
import {
  type BoxOptions,
  BoxRenderable,
  type CliRenderer,
  CodeRenderable,
  DiffRenderable,
  getTreeSitterClient,
  InputRenderable,
  InputRenderableEvents,
  type KeyEvent,
  type LineColorConfig,
  type LineNumberOptions,
  LineNumberRenderable,
  type LineSign,
  type MarkdownOptions,
  MarkdownRenderable,
  type MouseEvent,
  type OptimizedBuffer,
  type Renderable,
  RGBA,
  type ScrollBoxOptions,
  ScrollBoxRenderable,
  type SimpleHighlight,
  TextareaRenderable,
  type TextOptions,
  TextRenderable,
} from "@opentui/core";
import type { Keymap } from "@opentui/keymap";
import { createDefaultOpenTuiKeymap } from "@opentui/keymap/opentui";
import { parsePatch } from "diff";
import { clipboardImage } from "./clipboard.ts";
import { commands, slashes } from "./commands.ts";
import {
  actsOf,
  asksOperator,
  conversation,
  fromOperator,
  type Item,
  type Note,
  notesOf,
  operatorNote,
  refusal,
  steps,
} from "./conversation.ts";
import { externalEditor, openFile } from "./editor.ts";
import { shortenHome, shortenHomes } from "./files.ts";
import { ago, clip, count, dollars, elapsed, graphemes, kibibytes, modelName, share } from "./format.ts";
import { type Action, bindings, chords, keys, shown } from "./keys.ts";
import { loadParsers } from "./parsers.ts";
import {
  type ActRow,
  cancelled,
  type Exit,
  failed,
  type Scroll,
  type Session,
  type SessionStatus,
  type ShownChange,
  statusLabels,
  type View,
  views,
  working,
} from "./session.ts";
import { publishShare } from "./share.ts";
import {
  theme as c,
  defaultTheme,
  glyph,
  motion,
  palettes,
  selectedMix,
  setTheme,
  spacing as space,
  spin,
  syntax,
  type ThemeName,
  themeLabels,
} from "./theme.ts";
import { bold, inline, italic, lineCounts, logo, mix, type Part, plain, styled, underline } from "./ui.ts";
import type { SessionEntry, Workspace, Workspaces } from "./workspaces.ts";

const exitNotice = "Press ⌃D again to exit.";
const rewindNotice = "Press Escape again to rewind.";
/** The time within which a second Escape rewinds. */
const twice = 800;
/** The name of each view as the toggle and the commands show it. */
const viewLabels: Record<View, string> = { feed: "Feed", transcript: "Transcript", changes: "Changes" };
/** A text with its first letter in upper case. */
const title = (text: string) => text.charAt(0).toUpperCase() + text.slice(1);
/** Why a word failed, as the operator reads it: each home as ~, and without the name <string> that the parser gives
 * the word and the line that it says a second time. */
const readable = (reason: string) => shortenHomes(reason).replace(/\s*\(<string>, line \d+\)$/gm, "");
/** What an act is doing: the word that says it, the glyph that shows it, and their color. */
type State = { word: string; mark: string; color: RGBA };
/** The state of an act that failed, that a cancel ended, that is done, or that a pause holds. */
const stateOf = (state: "failed" | "cancelled" | "done" | "held"): State =>
  ({
    failed: { word: "failed", mark: glyph.failed, color: c.warm },
    cancelled: { word: "cancelled", mark: glyph.cancelled, color: c.faint },
    done: { word: "", mark: glyph.done, color: c.done },
    held: { word: "waits for a wake", mark: glyph.held, color: c.warm },
  })[state];
/** The mark of a state in its color, one shape for each: at work, waiting on the operator, paused, failed, finished
 * and not yet seen, and at rest. */
const statusMark = (status: SessionStatus): Part =>
  (
    ({
      working: [`${glyph.running} `, c.operator],
      opening: [`${glyph.running} `, c.operator],
      blocked: [`${glyph.asks} `, c.warm],
      paused: [`${glyph.held} `, c.warm],
      error: [`${glyph.failed} `, c.warm],
      done: [`${glyph.dot} `, c.done],
      idle: [`${glyph.ring} `, c.faint],
      saved: [`${glyph.ring} `, c.faint],
    }) satisfies Record<SessionStatus, Part>
  )[status];
/** The rows that a key moves the pointer of a list by, in the rewind tree and in a dialog. */
const moves: Record<string, number> = { up: -1, down: 1, pageup: -8, pagedown: 8, home: -1e9, end: 1e9 };
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
  textColor: c.bright,
  focusedTextColor: c.bright,
  placeholderColor: c.faint,
  cursorColor: c.warm,
});
/** The suggestions of an empty feed: what each says, and the prompt it puts in the composer. */
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
  /** The columns the card stands in from the edge of the feed. */
  indent?: number;
  /** Whether the card draws no heading, as a word or a command does, whose body shows it folded and open. */
  headless?: boolean;
}
/** A word of the conversation. */
type Word = Extract<Item, { type: "word" }>;
/** A card added to the view: its name, what makes it again when it changes, its heading, and its body, which is given
 * whether the card is folded, and what folds or opens it. */
type Add = (
  id: string,
  key: string,
  label: Part[],
  body: (box: BoxRenderable, closed: boolean, toggle: () => void) => void,
  options?: BlockOptions & { shown?: string },
) => void;
/** A text of numbers and units, each number bright and each unit faint. */
const quantity = (text: string): Part[] =>
  text
    .split(/(\d[\d.,]*)/)
    .filter(Boolean)
    .map((part): Part => [part, /^\d/.test(part) ? c.bright : c.faint]);
/** The languages whose colors the TUI knows, by the extensions of their files. */
const extensions: Record<string, string> = {
  py: "python",
  ts: "typescript",
  tsx: "typescript",
  mts: "typescript",
  cts: "typescript",
  js: "javascript",
  jsx: "javascript",
  mjs: "javascript",
  cjs: "javascript",
  md: "markdown",
  zig: "zig",
};
const languages = new Set(Object.values(extensions));
/** The language of a file, by its extension, for the languages whose colors the TUI knows. */
function filetype(path: string): string | undefined {
  return extensions[path.slice(path.lastIndexOf(".") + 1).toLowerCase()];
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
  quit(): void | Promise<void>;
  workspaces: Workspaces;
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
  options.workspaces.on("select", (next: Session) => {
    if (app.session === next) return;
    app.dispose();
    app = new App(renderer, next, options);
  });
  return () => app;
}

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
/** Parts cut to a width at their end, with the mark of a cut. */
function clipParts(parts: readonly Part[], width: number): Part[] {
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
/** A text of one line that shows its parts cut at the end to its width, since OpenTUI cuts a line in its middle and a
 * line of furb keeps its start. It holds the whole text, which OpenTUI measures, and cuts only what it draws. */
class CutText extends TextRenderable {
  protected override renderSelf(buffer: OptimizedBuffer): void {
    const parts = this.content.chunks.map(
      (chunk): Part => [chunk.text, chunk.fg, chunk.attributes, chunk.bg],
    );
    if (Bun.stringWidth(plain(parts)) <= this.width) {
      super.renderSelf(buffer);
      return;
    }
    let x = this.screenX;
    for (const [text, fg, attributes, bg] of clipParts(parts, this.width)) {
      buffer.drawText(
        text,
        x,
        this.screenY,
        fg ?? this._defaultFg,
        bg,
        (attributes ?? 0) | this._defaultAttributes,
      );
      x += Bun.stringWidth(text);
    }
  }
}

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
      /** The text of the heading, set again only when it changes. */
      label: string;
      key: string;
      compact: boolean;
      collapsible: boolean;
      state: string;
      closed: boolean;
      /** The act that the card shows. */
      act?: string;
    }
  >();
  /** Whether the pass of the view that draws now is the first of the view, whose steps land at once. */
  private opening = false;
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
  private submitting = false;
  private historyIndex = -1;
  private historyDraft = "";
  private diagnosticsKey = "";
  /** What the footer shows, set again only when it changes. */
  private statusKey = "";
  /** Whether the footer shows a spinner or the bar of a reply, which the tick moves. */
  private statusMoves = false;
  /** The whole text of the footer, which a notice cut to its room shows in a tip. */
  private statusWhole = "";
  /** The texts that move while their cause holds: what each shows now, and the time after which it shows that for
   * good and moves no more. The tick draws each one again, and stops when none moves. */
  private readonly movers = new Map<
    TextRenderable,
    { parts: () => Part[]; until?: number; shown?: string }
  >();
  private tick?: ReturnType<typeof setInterval>;
  /** When each step of the feed first showed, so that a new step lands bright and settles, by its place in its word. */
  private readonly landed = new Map<string, number>();
  /** When each thread closed while the App watched, which its check shows for a moment, and the threads open at the
   * last draw. */
  private readonly closedAt = new Map<string, number>();
  private readonly openThreads = new Set<string>();
  private readonly navigation: {
    chain: string;
    view: View;
    search: string;
    place: Scroll;
    mode: "markdown" | "python";
  }[] = [];

  constructor(
    readonly renderer: CliRenderer,
    readonly session: Session,
    readonly options: AppOptions,
  ) {
    setTheme(session.theme);
    this.theme = session.theme;
    this.style = syntax();
    this.root = this.box({
      id: "furb",
      width: "100%",
      height: "100%",
      flexDirection: "row",
      backgroundColor: c.ground,
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
        backgroundColor: c.surface2,
        ...options,
      });
    this.searchRow = bar();
    this.searchRow.add(
      this.text(
        [
          ["Filter  ", c.bright, bold],
          [`${glyph.prompt} `, c.operator],
        ],
        c.bright,
      ),
    );
    this.search = new InputRenderable(renderer, {
      id: "search",
      flexGrow: 1,
      placeholder: "Filter this view",
      ...inputColors(c.surface2),
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
    this.queueBox = this.row({
      id: "queued-follow-ups",
      visible: false,
      onMouseUp: this.click(() => this.queuePicker()),
    });
    center.add(this.queueBox);
    this.imageBox = this.row({ id: "attached-images", gap: space.between, visible: false });
    center.add(this.imageBox);
    this.suggestionBox = this.box({ id: "suggestions", visible: false, backgroundColor: c.surface2 });
    center.add(this.suggestionBox);
    this.modeBox = this.row();
    // The composer is a panel, whose three rows the mode colors.
    this.composeBox = this.panel(center, c.operator, { id: "composer-box" });
    this.composeEdges = center.getChildren().slice(-3) as BoxRenderable[];
    this.composer = new TextareaRenderable(renderer, {
      id: "composer",
      flexGrow: 1,
      minHeight: 1,
      placeholder: "What would you like to build?",
      ...inputColors(c.surface2),
      // The caret is a block that the terminal blinks, and holds still while it moves.
      cursorStyle: { style: "block", blinking: session.preferences.motion },
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
      this.line("", c.prose, { truncate: true, flexGrow: 1, flexShrink: 1 }),
      () => this.statusWhole,
    );
    footer.add(this.status);
    this.hints = this.row();
    footer.add(this.hints);
    center.add(footer);
    this.rail = this.box({
      id: "sidebar",
      width: session.preferences.sidebarWidth,
      backgroundColor: c.surface1,
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
      backgroundColor: c.surface1,
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
      borderColor: c.rule,
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
    this.keymap = keymapOf(renderer);
    this.layersOff = this.layers();
    renderer.on("resize", this.render);
    renderer.on("selection", this.copySelection);
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
  }

  private box(options: BoxOptions = {}): BoxRenderable {
    return new BoxRenderable(this.renderer, { flexDirection: "column", flexShrink: 0, ...options });
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
  private hoverable<T extends BoxRenderable | TextRenderable>(node: T, color = c.surface2): T {
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
  /** The text that a drag selected goes to the clipboard as the drag ends. */
  private copySelection = (selection: { getSelectedText(): string } | null): void => {
    const text = selection?.getSelectedText() ?? "";
    if (this.closed || !text.trim()) return;
    this.renderer.copyToClipboardOSC52(text);
    const length = [...graphemes.segment(text)].length;
    this.session.notice = `Copied ${length} ${length === 1 ? "character" : "characters"}.`;
  };
  /** A text, which runs an action at a click when it has one. */
  private text(content: string | Part[], fg = c.bright, { run, ...options }: TextShape = {}): TextRenderable {
    const shape = {
      content: typeof content === "string" ? safeText(content) : styled(content),
      fg,
      // A text truncates only on a line it does not wrap, so a text that truncates keeps one line with an ellipsis.
      wrapMode: options.truncate ? ("none" as const) : ("word" as const),
      flexShrink: 0,
      onMouseUp: run && this.click(run),
      ...options,
    };
    if (!options.truncate) return new TextRenderable(this.renderer, shape);
    const node = new CutText(this.renderer, shape);
    return options.onMouseOver ? node : this.whole(node);
  }
  /** A text one bar high. */
  private line(content: string | Part[], fg = c.bright, options: TextShape = {}): TextRenderable {
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
      if (cut && text) this.tip([[text, c.bright]], event.x, event.y);
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
  private schedule = () => {
    if (this.closed) return;
    if (!this.redraw)
      this.redraw = setTimeout(() => {
        this.redraw = undefined;
        this.render();
      }, 35);
  };
  /** What the input sent on this chain in this mode. A chain that holds messages the input did not send, as a record
   * that opens with no saved input, walks those messages instead. */
  private history(): string[] {
    const w = this.session;
    const kept = w.histories[w.draftKey] ?? [];
    if (kept.length || w.mode !== "markdown" || w.editing) return kept;
    return w.activity
      .filter((act) => w.isUserThread(act))
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
      if (!(await this.globalCommand(content.trim()))) {
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
    if (w.theme !== this.theme) this.applyTheme(w.theme);
    if (this.draftKey !== w.draftKey) this.showDraft(w.draftKey);
    this.rail.visible = w.preferences.sidebar && this.renderer.width >= 100;
    this.rail.width = w.preferences.sidebarWidth;
    if (this.composer.cursorStyle.blinking !== w.preferences.motion)
      this.composer.cursorStyle = { style: "block", blinking: w.preferences.motion };
    this.watchThreads();
    this.renderTop();
    const images = w.images[w.selected] ?? [];
    this.imageBox.visible = images.length > 0;
    if (this.paneChanged(this.imageBox, [images, this.theme])) {
      this.clear(this.imageBox);
      this.imageBox.add(
        this.line(
          [
            [`${glyph.chip} `, c.operator],
            [`${images.length} ${images.length === 1 ? "image" : "images"} attached  `, c.bright],
            [
              images
                .map((image) => image.name)
                .join("  ")
                .replace(/\s+/g, " "),
              c.prose,
            ],
          ],
          c.prose,
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
            [`${w.queueHeld ? glyph.held : glyph.ring} `, w.queueHeld ? c.warm : c.faint],
            [
              `${w.queueHeld ? "Queue held" : "Queued"} ${w.queued.length}  `,
              w.queueHeld ? c.warm : c.bright,
            ],
            [first, c.prose],
          ],
          c.prose,
          { truncate: true, flexGrow: 1, flexShrink: 1 },
        ),
      );
      this.queueBox.add(
        this.line(
          queued.length
            ? [
                ["↑", c.prose],
                [" takes the last back", c.faint],
              ]
            : [],
          c.faint,
        ),
      );
    }
    const enter = this.enter;
    for (const edge of this.composeEdges)
      if (edge.borderColor !== enter.color) edge.borderColor = enter.color;
    this.composer.placeholder = enter.placeholder;
    // The box holds the lines of the text, up to six, a row of space, and the line under the text.
    this.composeBox.height =
      Math.min(6, Math.max(space.bar, this.composer.lineCount, this.composer.lineInfo.lineSources.length)) +
      space.section +
      space.bar;
    this.renderMeta(enter);
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
   * holds a segment for each choice, two columns of space at each side of its label. The chosen segment is tinted
   * with the color of its choice from edge to edge, as a selected row is, and its label stands in that color. The
   * segment under the pointer lights in place, and a click chooses a segment. The chord that moves the switch stands
   * after the track. */
  private switcher(
    box: BoxRenderable,
    choices: { label: string; badge?: string; color: RGBA; run?: () => void }[],
    chosen: number,
    chord: string,
    track: RGBA,
  ): void {
    for (const [index, choice] of choices.entries()) {
      const active = index === chosen;
      const fill = active ? mix(track, choice.color, selectedMix) : track;
      const parts = (lit: boolean): Part[] => [
        [`  ${choice.label}`, active ? choice.color : lit ? c.bright : c.prose, active ? bold : 0, fill],
        [choice.badge ? ` ${choice.badge}` : "", active ? choice.color : lit ? c.bright : c.faint, 0, fill],
        ["  ", c.bright, 0, fill],
      ];
      // The pointer recolors the segment that it is over, and builds no node, so that a press and its release land on
      // the same segment.
      const segment: TextRenderable = this.line(parts(false), c.bright, {
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
  /** The top line: the session, the chain, and the thread at its left, each a button, and the toggle of the views at
   * its right. */
  private renderTop(): void {
    const w = this.session;
    const thread = w.acts.find((act) => act.id === w.thread && act.on === w.selected);
    const crumb = thread ? this.firstLine(String(thread.words[1] ?? "")) : "";
    const changes = w.host.changes;
    const shown = this.tree ? "feed" : w.view;
    const directory = shortenHome(w.workingDirectory);
    // The top line spends its room in this order: the session and the chain, the switch of the views, the thread, the
    // key of the switch, then the directory. A part that finds no room is left out, the thread is cut at its end to
    // the room that is left, and a session name that is still too long is cut at its end.
    const chord = `${this.kitty ? "⌃" : "⌥"}1-3`;
    const toggle = views.reduce(
      (sum, view) =>
        sum + viewLabels[view].length + 4 + (view === "changes" && changes ? String(changes).length + 1 : 0),
      0,
    );
    const head =
      Bun.stringWidth(w.sessionName) +
      3 +
      Bun.stringWidth(w.label) +
      (crumb ? 3 + Math.min(24, Bun.stringWidth(crumb)) : 0);
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
          color: c.operator,
          run: () => this.showView(view),
        })),
        views.indexOf(shown),
        // F1 lists the chord where the top line has no room for it.
        hint ? chord : "",
        c.surface2,
      );
    }
    if (this.paneChanged(this.headline, [name, w.label, crumb, folder && directory, this.theme])) {
      this.clear(this.headline);
      this.headline.add(this.line([[name, c.bright]], c.bright, { run: () => void this.workspacePicker() }));
      this.headline.add(this.line(` ${glyph.crumb} `, c.faint));
      // The chain opens its feed from a thread, and the chains from its feed.
      this.headline.add(
        this.line(w.label, crumb ? c.prose : c.bright, {
          run: () => (crumb ? void w.open("").catch(w.fail) : this.chains()),
        }),
      );
      if (crumb) {
        this.headline.add(this.line(` ${glyph.crumb} `, c.faint));
        this.headline.add(this.line(crumb, c.bright, { truncate: true, flexShrink: 1 }));
      }
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
  /** What Enter does in the composer, by the mode of the input and what the operator selected: the words of Enter,
   * the type of the answer that it asks for, the actor that reads it, the color of the bar of the composer, and the
   * text of an empty input. */
  private get enter(): { word: string; shape?: string; actor?: string; color: RGBA; placeholder: string } {
    const w = this.session;
    if (w.editing)
      return { word: "Replay the program", color: c.model, placeholder: "Edit this thread's Python program" };
    if (w.mode === "python")
      return {
        word: "Run Python",
        color: c.model,
        placeholder: "Write Python. The gate reads it before it runs.",
      };
    const intent = w.intent;
    if (intent.does === "answer") {
      const { shape } = intent.question;
      return {
        word: "Answer",
        shape,
        color: c.warm,
        placeholder: shape === "bool" ? "Answer yes or no" : `Your answer, as ${shape}`,
      };
    }
    if (intent.does === "notify")
      return {
        word: "Notify this thread",
        actor: String(intent.thread.words[2] || w.actor),
        color: c.operator,
        placeholder: "A note for the model, which it reads at its next reply",
      };
    return {
      word: "New thread",
      shape: w.shape,
      actor: w.actor,
      color: c.operator,
      placeholder: "Start a thread, or type / for commands",
    };
  }
  /** The line under the input, which reads as a sentence: the switch of its mode, what Enter does, the type of the
   * answer, then the model that reads it, its provider, and its effort. A part that the operator can change is a
   * button that changes it. */
  private renderMeta(enter: App["enter"]): void {
    const w = this.session;
    const { model, effort } = actorParts(
      enter.actor ?? "",
      w.roster.map(([name]) => name),
    );
    const { provider, id: name } = modelName(model);
    const stash = w.stashes[w.draftKey];
    // A new thread chooses its type and its model, and a note goes to the model that works the thread.
    const chooses = enter.word === "New thread";
    const effortShown = Boolean(enter.actor) && effort !== "off";
    // The line spans the input but its bar and its padding.
    const room = this.feedWidth - 3;
    if (
      !this.paneChanged(this.meta, [
        enter.word,
        enter.shape,
        name,
        provider,
        effort,
        stash,
        w.editing,
        room,
        this.theme,
      ])
    )
      return;
    this.clear(this.meta);
    // A narrow input leaves out, in this order, the provider, the effort, the type of the answer, and the chord of the
    // switch, and the stash shows less of what waits in it.
    const shown = {
      provider: Boolean(enter.actor && provider),
      effort: effortShown,
      shape: Boolean(enter.shape),
      chord: !w.editing,
    };
    const width = () =>
      (w.editing ? "python".length + 4 : "markdown".length + "python".length + 8) +
      (shown.chord ? 4 : 0) +
      3 +
      Bun.stringWidth(enter.word) +
      (shown.shape ? Bun.stringWidth(` · ${enter.shape}`) : 0) +
      (enter.actor ? Bun.stringWidth(` with ${name}`) : 0) +
      (shown.provider ? provider.length + 1 : 0) +
      (shown.effort ? Bun.stringWidth(` at ${effort} effort`) : 0) +
      // The stash takes a gap, its word, its quotes and their space, and its key, and its preview takes the rest.
      (stash ? 3 + "stashed “”  ⌃S".length : 0);
    for (const part of ["provider", "effort", "shape", "chord"] as const)
      if (width() > room) shown[part] = false;
    const button = (parts: Part[], run?: () => void) => this.meta.add(this.line(parts, c.prose, { run }));
    // The mode of the input is a switch between markdown and Python, and a program under edit is Python alone, which
    // only its key leaves.
    this.clear(this.modeBox);
    if (w.editing) this.switcher(this.modeBox, [{ label: "python", color: c.model }], 0, "", c.surface2);
    else
      this.switcher(
        this.modeBox,
        [
          { label: "markdown", color: c.operator, run: () => w.mode === "markdown" || this.toggleMode() },
          { label: "python", color: c.model, run: () => w.mode === "python" || this.toggleMode() },
        ],
        w.mode === "python" ? 1 : 0,
        shown.chord ? "⌃R" : "",
        c.surface2,
      );
    this.meta.add(this.text("   ", c.faint));
    button([[enter.word, c.operator, bold]]);
    if (shown.shape && enter.shape)
      button(
        [
          [" · ", c.faint],
          [enter.shape, c.prose],
        ],
        chooses ? this.shapes : undefined,
      );
    if (enter.actor) {
      button(
        [
          [" with ", c.faint],
          [name, c.prose],
          [shown.provider ? ` ${provider}` : "", c.faint],
        ],
        chooses ? () => this.models() : undefined,
      );
      if (shown.effort)
        button(
          [
            [" at ", c.faint],
            [effort, c.warm],
            [" effort", c.faint],
          ],
          chooses ? () => this.effortPicker() : undefined,
        );
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
          [preview >= 6 ? `“${clip(first, preview)}${more}”  ` : "", c.prose],
          ["⌃S", c.faint],
        ],
        () => this.stash(),
      );
    }
  }

  /** The state of the session and the keys that act on it, in the footer. A notice stands after the state until it
   * ends, and each key is a button that does what it says. */
  private renderStatus(): void {
    const w = this.session;
    // A refresh follows each fact, so the footer says Loading only while the chain has nothing to show yet.
    const loading = w.loading && !w.turns.length;
    const status = loading ? "opening" : w.status(w.selected);
    // The footer moves while work goes on: a spinner, or a bar while a reply streams.
    const moving = (status === "working" || status === "opening") && w.preferences.motion;
    const streaming = moving && w.host.streams.size > 0;
    this.statusMoves = moving;
    this.animate();
    const [mark, color] = statusMark(status);
    const label = loading
      ? "Loading"
      : status === "error"
        ? w.error
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
      [chord, c.prose],
      [` ${action}`, c.faint],
    ]);
    const hints = buttons.flat();
    const notice = w.error ? "" : w.notice;
    // The state stays in the footer, and a notice stands after it, cut at its end where the footer has no room for it
    // beside the keys.
    const state: Part[] = [
      ...(streaming ? [...this.walker(), [" "] as Part] : [[moving ? `${spin()} ` : mark, color] as Part]),
      [label, moving ? c.bright : c.prose],
    ];
    const room =
      this.feedWidth - Bun.stringWidth(plain(hints)) - Bun.stringWidth(plain(state)) - space.between * 2;
    this.statusWhole = notice ? `${plain(state)}   ${notice}` : plain(state);
    if (notice && room >= 8)
      state.push(
        ["   ", c.bright],
        [clip(notice, room - 3), [exitNotice, rewindNotice].includes(notice) ? c.warm : c.bright],
      );
    const key = JSON.stringify([state.map(([text, fg]) => [text, fg?.toInts()]), plain(hints), this.theme]);
    if (key === this.statusKey) return;
    this.statusKey = key;
    this.status.content = styled(state);
    this.clear(this.hints);
    for (const [index, [, , run]] of keys.entries())
      this.hints.add(this.line(buttons[index] ?? [], c.faint, { run }));
  }

  /** The bar of a reply that streams: a bright head that walks its cells, with a tail of three steps that fade. */
  private walker(now = Date.now()): Part[] {
    const head = Math.floor(now / motion.walk) % motion.bar;
    return Array.from({ length: motion.bar }, (_, at): Part => {
      const behind = (head - at + motion.bar) % motion.bar;
      return [
        glyph.meter,
        behind === 0 ? c.bright : behind <= 3 ? mix(c.bright, c.rule, behind / 4) : c.rule,
      ];
    });
  }
  /** The heading of a card set to a line, only when the line changed. */
  private label(card: { heading: TextRenderable; label: string }, parts: Part[]): void {
    const key = JSON.stringify(parts.map(([text, fg, attributes]) => [text, fg?.toInts(), attributes]));
    if (card.label === key) return;
    card.label = key;
    card.heading.content = styled(parts);
  }
  private card(
    id: string,
    key: string,
    label: Part[],
    body: (box: BoxRenderable, closed: boolean, toggle: () => void) => void,
    index: number,
    options: BlockOptions = {},
  ): void {
    const rung = options.act?.kind === "rung" && !options.headless ? options.act : undefined;
    const state = options.headless ? (options.act?.id ?? id) : (rung?.id ?? id);
    const closed = options.headless
      ? (this.session.folds[state] ?? this.session.preferences.foldRungs)
      : this.folded(state, rung, Boolean(options.compact));
    key =
      options.compact && closed && !options.preview
        ? "closed"
        : `${key}:${closed}:${options.preview && closed ? this.feedWidth : ""}`;
    const marker: Part[] =
      !options.headless && (rung || options.compact || options.collapsible)
        ? [[`${closed ? glyph.closed : glyph.open} `, c.faint]]
        : [];
    const heading = [...marker, ...label];
    const visible =
      !options.headless && Boolean(plain(label)) && (options.compact || options.heading !== false || closed);
    const margin = options.separate ? space.section : space.stack;
    const { title } = options;
    const prior = this.cards.get(id);
    if (prior?.key === key) {
      if (title) this.move(prior.heading, () => [...marker, ...title()]);
      else {
        this.still(prior.heading);
        this.label(prior, heading);
      }
      prior.closed = closed;
      if (prior.heading.visible !== visible) prior.heading.visible = visible;
      if (prior.node.marginTop !== margin) prior.node.marginTop = margin;
      if (this.scroll.getChildren()[index] !== prior.node) this.scroll.add(prior.node, index);
      return;
    }
    prior?.node.destroyRecursively();
    const box = this.box({
      id,
      gap: space.stack,
      marginTop: margin,
      paddingLeft: options.indent ?? 0,
    });
    const toggle = () => {
      this.session.folds[state] = !closed;
      this.renderContent();
    };
    const labelNode = this.line(heading, c.prose, {
      truncate: true,
      visible,
      onMouseDown: (event) => {
        if (event.button === 2 && options.act)
          this.actActions(this.session.acts.find((act) => act.id === options.act?.id) ?? options.act);
      },
      run: toggle,
    });
    box.add(labelNode);
    if (options.headless || !closed) body(box, closed, toggle);
    else if (!rung) options.preview?.(box);
    this.scroll.add(box, index);
    const card = {
      key,
      node: box,
      heading: labelNode,
      label: "",
      compact: options.compact ?? false,
      collapsible: Boolean(rung || options.collapsible || options.headless),
      state,
      closed,
      act: options.act?.id,
    };
    if (title) this.move(labelNode, () => [...marker, ...title()]);
    else this.label(card, heading);
    this.cards.set(id, card);
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
          borderColor: c.surface2,
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
      backgroundColor: c.surface2,
      ...options,
    });
    box.add(edge("top"));
    box.add(inner);
    box.add(edge("bottom"));
    return inner;
  }
  /** Python in the colors of its syntax. In a user turn, a header is a comment to Python, and the start of an entry to
   * the reader: the name of its act or the kind of its query stands in the color of a reference, and the rest of its
   * line as text. An image attachment there is a reference too. */
  private code(content: string, user = false): CodeRenderable {
    const code = new CodeRenderable(this.renderer, {
      content: safeText(content),
      filetype: "python",
      syntaxStyle: this.style,
      wrapMode: "word",
      drawUnstyledText: true,
      onHighlight: user
        ? (highlights, { content: text }) => [
            ...highlights,
            ...[...text.matchAll(/^(#(?! |$)\S+)(.*)$/gm)].flatMap((header): SimpleHighlight[] => [
              [header.index, header.index + (header[1]?.length ?? 0), "reference"],
              [header.index + (header[1]?.length ?? 0), header.index + header[0].length, "header"],
            ]),
            ...[...text.matchAll(/furb-image:\/\/[\w.]+/g)].map(
              (image): SimpleHighlight => [image.index, image.index + image[0].length, "attachment"],
            ),
          ]
        : undefined,
    });
    return this.pointable(code, content);
  }
  /** Python under the pointer. A name shows what it holds in a card once the pointer rests on it, and a click with ⌃
   * or ⌥ opens it in the inspector. A header that names an act and an image attachment are references, which show
   * what they name in the card and open it at a click. */
  private pointable<T extends CodeRenderable | TextRenderable>(node: T, content: string): T {
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
  private markdown(content: string, fg = c.prose): MarkdownRenderable {
    // A block of code stands on the surface of a block, in the colors of its language. OpenTUI spaces the other blocks
    // only while the renderer says it draws code alone, and it gives no margin to a node of its own, so the block keeps
    // the blank line under it but at the end of the text.
    const text = safeText(content);
    const code: NonNullable<MarkdownOptions["renderNode"]> = (token) => {
      if (token.type !== "code") return undefined;
      const end = text.trimEnd().endsWith(token.raw.trimEnd());
      const block = this.box({
        backgroundColor: c.surface2,
        paddingX: space.inset,
        marginBottom: end ? 0 : 1,
      });
      const language = token.lang?.trim().toLowerCase() ?? "";
      // A diff tints each line that it adds or removes, as the diffs of the feed do.
      if (["diff", "patch"].includes(language)) {
        for (const line of safeText(token.text).split("\n")) {
          const [tone, tint] = /^(\+\+\+|---)( |$)/.test(line)
            ? [c.faint, undefined]
            : line.startsWith("+")
              ? [c.done, c.added]
              : line.startsWith("-")
                ? [c.warm, c.removed]
                : line.startsWith("@@")
                  ? [c.faint, undefined]
                  : [c.bright, undefined];
          const row = this.box(tint ? { backgroundColor: tint } : {});
          row.add(this.text(line || " ", tone));
          block.add(row);
        }
        return block;
      }
      block.add(
        new CodeRenderable(this.renderer, {
          content: safeText(token.text),
          filetype: filetype(`.${language}`) ?? (languages.has(language) ? language : undefined),
          syntaxStyle: this.style,
          wrapMode: "word",
          drawUnstyledText: true,
          conceal: false,
        }),
      );
      return block;
    };
    return new MarkdownRenderable(this.renderer, {
      content: text,
      syntaxStyle: this.style,
      fg,
      renderNode: Object.assign(code, { codeBlockOnly: true }),
    });
  }

  /** Whether a card is folded: by the operator's click, or else as its kind starts. A rung that runs, or that failed
   * with no rung that took its place, stands open, and any other rung starts folded when the preference says so. */
  private folded(state: string, rung?: ActRow, compact = false): boolean {
    return (
      this.session.folds[state] ??
      (rung
        ? this.session.preferences.foldRungs && !working(rung) && (!failed(rung) || Boolean(this.retry(rung)))
        : compact)
    );
  }
  renderContent(): void {
    const w = this.session;
    // The feed of each thread is a view of its own, which keeps its place.
    const view = this.tree
      ? "tree"
      : `${w.selected}${w.view === "feed" && w.thread ? `/${w.thread}` : ""}:${w.view}`;
    this.treeBar.visible = Boolean(this.tree);
    this.opening = this.lastView !== view;
    if (this.opening) {
      // The steps that a view shows as it opens land at once.
      this.landed.clear();
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
    if (this.tree) {
      this.renderTree();
      return;
    }
    const existing = new Set(this.cards.keys());
    const used = new Map<string, number>();
    const matches = (text: string) => !w.search || text.toLowerCase().includes(w.search.toLowerCase());
    let order = 0,
      items = 0,
      group = "";
    // A card that shows an item of the view says the text that the filter reads, and counts as an item once it shows.
    const add: Add = (id, key, label, body, options = {}) => {
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
      this.card(id, key, label, body, order, {
        ...options,
        heading: options.heading ?? heading,
        separate: order > 0 && (options.separate ?? heading),
      });
      order++;
    };
    if (w.preferences.notice)
      add("preferences-notice", w.preferences.notice, [], (box) => {
        const panel = this.panel(box, c.warm);
        panel.add(
          this.text(w.preferences.notice, c.bright, {
            run: () => {
              w.preferences.notice = "";
              this.renderContent();
            },
          }),
        );
      });
    if (w.error)
      add("view-error", `${w.view}:${w.error}`, [], (box) =>
        this.banner(
          box,
          c.warm,
          [
            [`${glyph.failed} `, c.warm],
            [`Error in the ${viewLabels[w.view].toLowerCase()} view`, c.bright, bold],
          ],
          this.text(w.error, c.prose),
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
      const threads = w.threads;
      const rows = new Map(w.acts.map((act) => [act.id, act]));
      const listed = conversation(w.turns, w.acts, w.asked);
      const words = new Map(
        listed.flatMap((item) => (item.type === "word" && item.rung ? [[item.rung.id, item] as const] : [])),
      );
      // An act that a turn tells, or that a word a turn tells made.
      const told = new Set(
        listed.flatMap((item) =>
          item.type === "word"
            ? [item.rung?.id ?? "", ...actsOf(item).map((act) => act.id)]
            : [item.act?.id ?? ""],
        ),
      );
      // A rung whose model has not begun to write has no stream yet, and waits as the words of a stream do.
      const writing = [...w.host.streams].filter(([, stream]) => stream.chain === w.selected);
      for (const act of w.activity)
        if (
          act.kind === "rung" &&
          working(act) &&
          fromOperator(act, rows) &&
          !told.has(act.id) &&
          !w.program[act.id] &&
          !w.host.streams.has(act.id)
        )
          writing.push([act.id, { chain: w.selected, text: "", thinking: "" }]);
      const waiting = new Set(writing.map(([id]) => id));
      // An act that no turn tells yet stands with the word that made it, or where it came in time among the items that
      // the turns tell, so the feed never moves a card once it shows it. A rung that a cancel or a close ended before
      // its model wrote a word shows nothing, since the cancel or the answer says how its thread ended.
      const position = new Map(w.activity.map((act, index) => [act.id, index]));
      const loose: { at: number; item: Item }[] = [];
      for (const act of w.activity) {
        if (told.has(act.id) || waiting.has(act.id) || !this.isPoint(act) || !fromOperator(act, rows))
          continue;
        if (act.kind === "rung" && act.done && !failed(act) && !w.program[act.id]) continue;
        const maker = words.get(act.by);
        if (maker && act.kind !== "thread") maker.told.push(act);
        else
          loose.push({
            at: position.get(act.id) ?? Number.POSITIVE_INFINITY,
            item:
              act.kind === "thread"
                ? { type: "thread", key: act.id, act }
                : { type: "act", key: act.id, act, notes: [] },
          });
      }
      if (loose.length) {
        const merged: Item[] = [];
        let next = 0;
        for (const item of listed) {
          const id = item.type === "word" ? item.rung?.id : item.act?.id;
          const at = id === undefined ? undefined : position.get(id);
          while (at !== undefined && next < loose.length && (loose[next]?.at ?? 0) < at)
            merged.push(loose[next++]?.item as Item);
          merged.push(item);
        }
        while (next < loose.length) merged.push(loose[next++]?.item as Item);
        listed.splice(0, listed.length, ...merged);
      }
      // The feed shows the items of the selected thread, or the items of no thread and a card for each thread. A run
      // of words of one speaker stands under one line that names the speaker.
      const zoom = rows.get(w.thread)?.on === w.selected ? w.thread : "";
      const place = (id: string) =>
        Math.max(
          0,
          threads.on(w.selected).findIndex((act) => act.id === id),
        );
      const here = zoom ? place(zoom) : 0;
      type Run = { speaker: string; rungs: string[] };
      type Entry =
        | { is: "item"; item: Item }
        | { is: "stream"; id: string; stream: Stream }
        | { is: "speaker"; key: string; run: Run }
        | { is: "card"; thread: ActRow };
      const plan: Entry[] = [];
      const carded = new Set<string>();
      let run = undefined as Run | undefined;
      const speak = (speaker: string, key: string, rung?: string) => {
        if (run?.speaker !== speaker) {
          run = { speaker, rungs: [] };
          plan.push({ is: "speaker", key: `speaker-${key}`, run });
        }
        if (rung) run.rungs.push(rung);
      };
      const card = (thread: ActRow) => {
        if (carded.has(thread.id)) return;
        carded.add(thread.id);
        plan.push({ is: "card", thread });
        run = undefined;
      };
      // Who speaks in an act that no word made: the speaker of the word that made it, or the operator.
      const who = (act: ActRow) => {
        const maker = act.kind === "rung" ? act : rows.get(act.by);
        return maker?.kind === "rung" ? threads.speaker(maker) : OPERATOR;
      };
      for (const item of listed) {
        const holder = threads.of(item.type === "word" ? item.rung : item.act);
        if ((holder ?? "") !== zoom) {
          const thread = holder === undefined ? undefined : rows.get(holder);
          if (!zoom && thread) card(thread);
          continue;
        }
        if (item.type === "word" && !this.aside(item))
          speak(item.rung ? threads.speaker(item.rung) : w.actor, item.key, item.rung?.id);
        else if (item.type === "act") speak(who(item.act), item.key, item.act.id);
        else if (item.type === "result" && !asksOperator(item.act)) speak(this.answerer(item.act), item.key);
        else run = undefined;
        plan.push({ is: "item", item });
      }
      // A thread that no turn tells yet has its card after the others.
      if (!zoom) for (const thread of threads.on(w.selected)) card(thread);
      for (const [id, stream] of writing) {
        const act = rows.get(id);
        if ((threads.of(act) ?? "") !== zoom) continue;
        speak(act ? threads.speaker(act) : w.actor, `stream-${id}`, id);
        plan.push({ is: "stream", id, stream });
      }
      // The last step of each thread, which its card says: the last comment of the last word under it, or of the words
      // that its model writes now.
      const lasts = new Map<string, string>();
      const said = (holder: string | undefined, code: string) => {
        const lines = steps(code);
        if (holder !== undefined && (lines.length || code.trim()))
          lasts.set(holder, lines.at(-1) ?? this.firstLine(code));
      };
      for (const item of listed) if (item.type === "word") said(threads.of(item.rung), item.code);
      for (const [id, stream] of writing)
        said(threads.of(rows.get(id)), stream.text.slice(0, stream.text.lastIndexOf("\n") + 1));
      for (const entry of plan) {
        if (entry.is === "speaker") {
          const { run } = entry;
          const live = () =>
            run.rungs.some((id) => {
              const act = rows.get(id);
              return w.host.streams.has(id) || (act !== undefined && working(act));
            });
          const label = () => this.speaker(run.speaker, live(), run.rungs[0], here);
          const moves = live() && w.preferences.motion;
          add(entry.key, `${run.speaker}:${live()}:${this.theme}`, label(), () => {}, {
            heading: true,
            separate: true,
            title: moves ? label : undefined,
          });
        } else if (entry.is === "card") {
          const { thread } = entry;
          const last = lasts.get(thread.id) ?? "";
          const state = this.threadState(thread, place(thread.id));
          add(
            `card-${thread.id}`,
            JSON.stringify([
              thread.words,
              thread.done,
              thread.paused,
              plain(state.label),
              last,
              this.feedWidth,
            ]),
            [],
            (box) => this.threadCard(box, thread, state, last),
            { separate: true, act: thread, shown: `${thread.words[1]}\n${last}` },
          );
        } else if (entry.is === "stream") {
          const { id, stream } = entry;
          items++;
          add(
            `stream-${id}`,
            `${stream.text}${stream.thinking}:${this.feedWidth}`,
            [],
            (box) => this.streamBody(box, id, stream),
            { act: rows.get(id) },
          );
        } else this.item(add, entry.item);
      }
      // A paused chain says so at the end of its feed, with the reason that the last failure gave, and a button that
      // wakes it, since nothing new runs on it until then.
      if (w.paused && !w.search) {
        const failure = w.activity.findLast((act) => act.kind === "rung" && act.run?.status === "failed")?.run
          ?.reason;
        add("paused", `paused:${failure ?? ""}:${this.theme}`, [], (box) =>
          this.banner(
            box,
            c.warm,
            [
              [`${glyph.held} `, c.warm],
              ["This chain is paused", c.bright],
              ["  New work waits for a wake.", c.prose],
            ],
            failure
              ? this.whole(
                  this.text(clip(readable(failure).split("\n")[0] ?? "", this.feedWidth - 8), c.warm),
                  () => readable(failure),
                )
              : undefined,
            ["Wake", "runs what waits, once the cause is fixed", () => this.action("/wake")],
          ),
        );
        items++;
      }
      if (!items && !w.search && !w.loading && !w.error) {
        const { width, height } = this.scroll.viewport;
        add("welcome", `welcome:${width}:${height}:${this.theme}`, [], (box) => this.welcome(box, height));
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
              turn[0] === "assistant" ? c.model : c.faint,
            ],
            [turn[0], c.bright, bold],
            [`  turn ${index + 1}`, c.faint],
          ],
          (box) => {
            const inner = this.box({ paddingLeft: space.between });
            inner.add(this.code(text, turn[0] === "user"));
            box.add(inner);
          },
          { group: turn[0], shown: text },
        );
      });
    } else if (w.view === "changes") {
      if (w.host.changes > 20)
        add("change-pages", String(w.changePage), [], (box) => {
          const row = this.row({ gap: space.between });
          row.add(
            this.text(
              `Writes ${w.changePage * 20 + 1} to ${Math.min((w.changePage + 1) * 20, w.host.changes)} of ${w.host.changes}`,
              c.prose,
            ),
          );
          row.add(this.link("Previous page", () => this.changePage(-1)));
          row.add(this.link("Next page", () => this.changePage(1)));
          box.add(row);
        });
      const root = w.host.directory;
      for (const [index, change] of w.changes.entries()) {
        // A change never changes once it is written, so its position in the life keys its card.
        const position = String(w.changePage * 20 + index);
        const { added, removed } = lineCounts(change.patch);
        const path = change.path.startsWith(`${root}/`) ? change.path.slice(root.length + 1) : change.path;
        // The heading of a change is a bar of the panel: the path, what the write did to the file, and the lines it
        // added and removed at its right.
        const [what, color] = !change.before
          ? ["created", c.done]
          : !change.after
            ? ["deleted", c.warm]
            : ["modified", c.warm];
        const counts = `+${added} -${removed} `;
        const fill = Math.max(
          1,
          this.feedWidth - Bun.stringWidth(` ${glyph.dot} ${path}  ${what}`) - Bun.stringWidth(counts),
        );
        add(
          `change-${position}`,
          position,
          [
            [` ${glyph.dot} `, color, 0, c.surface2],
            [path, c.bright, bold, c.surface2],
            [`  ${what}`, c.prose, 0, c.surface2],
            [" ".repeat(fill), c.bright, 0, c.surface2],
            [`+${added}`, added ? c.done : c.faint, 0, c.surface2],
            [` -${removed} `, removed ? c.warm : c.faint, 0, c.surface2],
          ],
          // Two sides need room for two lines of code side by side, and one side reads better below that.
          (box) => box.add(this.diff(change.patch, path, this.feedWidth >= 160)),
          { shown: change.path },
        );
      }
    }
    // A view with nothing to show says why in the middle of the feed, and what brings something to it.
    const { height } = this.scroll.viewport;
    if (!items && w.loading && !w.error) {
      const loading = (): Part[] => [
        [`${spin()} `, c.operator],
        [`Loading the ${viewLabels[w.view].toLowerCase()}`, c.prose],
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
        changes: ["No file changes yet", "Each file that the life writes shows here as a diff."],
      };
      const [title, hint] = w.search
        ? [`Nothing matches “${w.search}”`, "Change the filter, or press Esc to clear it."]
        : empty[w.view];
      add("empty", `${title}:${height}:${this.theme}`, [], (box) =>
        this.centered(
          box,
          height,
          this.text([
            [`${glyph.ring} `, c.faint],
            [title, c.bright, bold],
          ]),
          this.text(hint, c.prose),
        ),
      );
    }
    for (const id of existing) {
      this.cards.get(id)?.node.destroyRecursively();
      this.cards.delete(id);
    }
  }

  /** An item of the feed as its card: a word, the open of a thread, its close, an act that no word made, or a note. */
  private item(add: Add, item: Item): void {
    const w = this.session;
    if (item.type === "word") {
      const { code, rung } = item;
      // A word of the operator that only made a chain, as a branch or a new chain does, reads as the chain that it
      // made, and a click on it opens that chain.
      const made = this.madeChain(item);
      if (made && rung) {
        const name = w.labelOf(made.id);
        // The branch itself reads the same word as the point where it starts.
        const here = made.id === w.selected;
        add(
          rung.id,
          `made:${made.id}:${name}:${here}:${this.theme}`,
          [],
          (box) =>
            box.add(
              here
                ? this.text([
                    ["↳ ", c.faint],
                    ["This branch starts here", c.prose],
                  ])
                : this.link(
                    [
                      ["↳ ", c.faint],
                      [made.words[1] ? "Branched to " : "Started the chain ", c.prose],
                      [name, c.bright],
                    ],
                    () => void w.select(made.id).catch(w.fail),
                  ),
            ),
          { shown: code, separate: true },
        );
        return;
      }
      // A note of the operator stands as a message of the operator does, in a smaller block.
      if (rung && this.aside(item)) {
        add(
          rung.id,
          `${code}:${this.theme}`,
          [],
          (box) => {
            const block = this.box({
              border: ["left"],
              borderColor: c.operator,
              customBorderChars: { ...noBorder, vertical: glyph.bar },
              backgroundColor: c.surface2,
              paddingLeft: space.between - space.inset,
              paddingRight: space.inset,
            });
            for (const line of steps(code)) block.add(this.text(this.inline(line, c.bright)));
            box.add(block);
          },
          { act: rung, shown: code, separate: true },
        );
        return;
      }
      const retried = Boolean(rung && this.retry(rung));
      const changes = rung ? (w.made[rung.id] ?? []) : [];
      const findings = rung && failed(rung) ? refusal(w.turns, rung.id) : [];
      const said = steps(code);
      add(
        rung?.id ?? item.key,
        JSON.stringify([
          code,
          rung?.run,
          item.told.map((one) =>
            "id" in one ? [one.id, one.done, one.paused, working(one) ? one.value : null] : one.key,
          ),
          changes.map((change) => change.patch),
          findings,
          retried,
          this.feedWidth,
          this.theme,
        ]),
        [[said[0] ?? this.firstLine(code), c.prose]],
        (box, closed, toggle) => this.wordBody(box, item, { closed, toggle, retried, changes, findings }),
        {
          headless: true,
          act: rung,
          shown: [code, ...notesOf(item).map((note) => note.body)].join("\n"),
        },
      );
    } else if (item.type === "thread" && asksOperator(item.act)) {
      const { act } = item;
      const message = String(act.words[1] ?? "");
      const waiting = w.host.threads.has(act.id);
      add(
        item.key,
        `${message}\n${waiting}\n${this.theme}`,
        [
          [`${glyph.asks} `, waiting ? c.warm : c.faint],
          // A question that its answer closed is past, and says so in the tone of the chrome.
          waiting ? ["Question for you", c.bright] : ["Asked you", c.faint],
        ],
        (box) => {
          this.panel(box, waiting ? c.warm : c.rule).add(this.text(message, c.bright));
          if (waiting)
            box.add(
              this.inset(
                space.between,
                this.text(
                  [
                    ["Answer in the input below", c.prose],
                    [act.words[0] === "bool" ? " with yes or no" : `, as ${act.words[0]}`, c.prose],
                    [", or press ", c.faint],
                    ["⌃A", c.prose],
                  ],
                  c.prose,
                  { run: () => this.question() },
                ),
              ),
            );
        },
        { act, heading: true, separate: true, shown: message },
      );
    } else if (item.type === "thread") {
      const { act } = item;
      const message = String(act.words[1] ?? "");
      const user = w.isUserThread(act);
      // A message of the operator shows as the operator typed it, with each image it attached named by a mark that
      // opens it, and says at its right the type of the answer that it asks for, and where it stands. A thread that a
      // model started shows its markdown, and the model it goes to.
      const state = user
        ? this.messageState(act)
        : [[`to ${this.model(String(act.words[2] ?? "")).name}`, c.faint]];
      add(
        item.key,
        `${message}\n${plain(state as Part[])}\n${this.theme}`,
        [],
        (box) => {
          const row = this.box({ flexDirection: "row" });
          const text = user ? this.message(message) : this.markdown(message, c.bright);
          text.flexGrow = 1;
          text.flexShrink = 1;
          row.add(text);
          row.add(this.text(state as Part[], c.faint, { marginLeft: space.between }));
          this.panel(box, user ? c.operator : c.model).add(row);
        },
        { act, separate: true, shown: message },
      );
    } else if (item.type === "result" && asksOperator(item.act)) {
      const { act } = item;
      const value = typeof act.value === "boolean" ? (act.value ? "yes" : "no") : display(act.value);
      const line = !value.includes("\n") && Bun.stringWidth(value) < this.feedWidth - 20;
      add(
        item.key,
        `${value}\n${line}`,
        [
          [`${glyph.done} `, c.done],
          ["You", c.bright, bold],
          [" answered", c.faint],
          [line ? `  ${value}` : "", c.bright],
        ],
        (box) => {
          if (!line) box.add(this.inset(space.between, this.markdown(value, c.bright)));
        },
        { act, heading: true, separate: false, shown: value },
      );
    } else if (item.type === "result") {
      const { act } = item;
      const value = display(act.value);
      // An answer that closed with others says which thread it answers, but in the view of that thread itself.
      const named = item.parallel && act.id !== w.thread;
      add(
        item.key,
        `${value}:${named}:${this.theme}`,
        [],
        (box) => {
          const answer = this.box({ paddingLeft: space.between });
          if (named)
            answer.add(
              this.text(`answers “${this.firstLine(String(act.words[1] ?? ""))}”`, c.faint, {
                truncate: true,
              }),
            );
          answer.add(this.markdown(value, c.bright));
          box.add(answer);
        },
        { act, separate: true, shown: value },
      );
    } else if (item.type === "act" && item.act.kind === "bash") {
      const { act } = item;
      add(
        item.key,
        JSON.stringify(act),
        [],
        (box, closed, toggle) => this.commandBlock(box, act, !closed, toggle),
        { act, headless: true, indent: space.between - space.inset, shown: this.describe(act) },
      );
    } else if (item.type === "act") {
      const { act, notes } = item;
      const heading = () => this.actHeading(act);
      add(
        item.key,
        JSON.stringify([act, notes.map((note) => note.key)]),
        heading(),
        (box) => this.actDetails(box, act, notes),
        {
          compact: true,
          preview: this.actPreview(act),
          act,
          title: working(act) && w.preferences.motion ? heading : undefined,
          indent: space.inset,
          shown: [this.describe(act), ...notes.map((note) => note.body)].join("\n"),
        },
      );
    } else {
      const { label, detail, body, act } = item;
      const text = [label, detail, body].filter(Boolean).join("\n");
      const danger = ["raised", "refused"].includes(label);
      add(
        item.key,
        text,
        [
          [`${danger ? glyph.failed : glyph.small} `, danger ? c.warm : c.faint],
          [label, danger ? c.warm : c.prose],
          [`  ${this.preview(detail, Bun.stringWidth(label) + 8)}`, c.faint],
        ],
        (box) => {
          const details = this.box({ paddingLeft: space.between * 2 });
          // A read and a write name the path they were of, which the reference opens.
          if (!act && detail && ["read", "write"].includes(label))
            details.add(this.reference(detail, detail));
          else if (detail) details.add(this.text(detail, c.prose));
          if (body) details.add(this.text(body, danger ? c.warm : c.bright));
          box.add(details);
        },
        {
          compact: true,
          act,
          indent: space.inset,
          shown: text,
          ...(label === "refused"
            ? { preview: (box: BoxRenderable) => this.excerpt(box, body, false, c.warm) }
            : {}),
        },
      );
    }
  }
  /** The chain that a word of the operator made, when the word made one. */
  private madeChain(item: Word): ActRow | undefined {
    const rung = item.rung;
    return rung?.by === OPERATOR && /^\s*chain\(/.test(item.code)
      ? this.session.chains.find((chain) => chain.by === rung.id)
      : undefined;
  }
  /** Whether a word stands apart from the runs of words: a note of the operator, or a word of the operator that made a
   * chain. */
  private aside(item: Word): boolean {
    return Boolean((item.rung && operatorNote(item.rung)) || this.madeChain(item));
  }
  /** The first line of a text that holds anything. */
  private firstLine(text: string): string {
    return text.split("\n").find((line) => line.trim()) ?? "";
  }
  /** A line of markdown in a tone, with the colors of code, of what stands out, and of a link. */
  private inline(line: string, tone: RGBA): Part[] {
    return inline(line, tone, { strong: c.bright, code: c.model, link: c.operator });
  }
  /** Who speaks, above a run of words: a dot, or a spinner while the run works, the name in bold, the effort, and the
   * time since the run began while it works. */
  private speaker(actor: string, live: boolean, first: string | undefined, place: number): Part[] {
    const moving = live && this.session.preferences.motion;
    const operator = actor === OPERATOR;
    const mark: Part = [
      `${moving ? spin(Date.now(), place) : live ? glyph.running : glyph.dot} `,
      operator ? c.operator : c.model,
    ];
    if (operator) return [mark, ["You", c.bright, bold]];
    const { name, effort } = this.model(actor);
    return [
      mark,
      [name, c.bright, bold],
      ...(effort && effort !== "off" ? ([["  "], [effort, c.warm]] as Part[]) : []),
      ...(live && first ? ([["  "], ...quantity(this.progress(first))] as Part[]) : []),
    ];
  }
  /** Who answered a thread: the speaker of the last rung the thread made, or the actor that it went to. */
  private answerer(thread: ActRow): string {
    const w = this.session;
    const rung = w.acts.findLast((act) => act.kind === "rung" && act.by === thread.id);
    return rung ? w.threads.speaker(rung) : String(thread.words[2] || w.actor);
  }
  /** Where a thread stands, as its mark and the words of its state: a spinner while a model works it, a diamond while a
   * question in it waits for the operator, a ring while a pause holds it, a check for a moment after it closes and a
   * dot after that, and a cross when it failed. The mark moves until the time it gives, or while the thread works. */
  private threadState(
    thread: ActRow,
    place: number,
  ): { mark: () => Part; label: Part[]; moves: boolean; until?: number } {
    const w = this.session;
    const moving = w.preferences.motion;
    const threads = w.threads;
    const asks = [...w.host.threads.keys()].some((id) => {
      const act = w.acts.find((one) => one.id === id);
      return act !== undefined && threads.of(act) === thread.id;
    });
    if (asks) return { mark: () => [glyph.asks, c.warm], label: [["asks you", c.warm]], moves: false };
    if (thread.done) {
      if (failed(thread))
        return { mark: () => [glyph.failed, c.warm], label: [["failed", c.warm]], moves: false };
      if (cancelled(thread))
        return { mark: () => [glyph.cancelled, c.faint], label: [["cancelled", c.faint]], moves: false };
      const at = this.closedAt.get(thread.id);
      const until = moving && at !== undefined ? at + motion.check : undefined;
      return {
        mark: () =>
          until !== undefined && Date.now() < until ? [glyph.done, c.done] : [glyph.small, c.faint],
        label: [[asksOperator(thread) ? "answered" : "closed", c.faint]],
        moves: until !== undefined && Date.now() < until,
        until,
      };
    }
    if (thread.paused || w.host.pending.has(thread.id))
      return { mark: () => [glyph.held, c.warm], label: [["paused", c.warm]], moves: false };
    return {
      mark: () => [moving ? spin(Date.now(), place) : glyph.running, c.model],
      label: [["working", c.faint]],
      moves: moving,
    };
  }
  /** A thread as a card of the feed of its chain: its mark, the first line of its markdown, and its state, then its
   * last step. A click opens the thread. */
  private threadCard(
    box: BoxRenderable,
    thread: ActRow,
    state: ReturnType<App["threadState"]>,
    last: string,
  ): void {
    const w = this.session;
    const open = () => void w.open(thread.id).catch(w.fail);
    const block = this.hoverable(
      this.box({
        backgroundColor: c.surface2,
        paddingLeft: space.between - space.inset,
        paddingRight: space.inset,
        onMouseUp: this.click(open),
      }),
      c.selected,
    );
    const top = this.row();
    const title = this.firstLine(String(thread.words[1] ?? ""));
    const head = (): Part[] => [state.mark(), [" "], [title, c.bright]];
    const heading = this.line(head(), c.bright, { truncate: true, flexGrow: 1, flexShrink: 1 });
    if (state.moves) this.move(heading, head, state.until);
    top.add(heading);
    top.add(this.line(state.label, c.faint, { marginLeft: space.between }));
    block.add(top);
    if (last) block.add(this.line([["  "], ...this.inline(last, c.prose)], c.prose, { truncate: true }));
    box.add(block);
  }
  /** The words that a model writes now: the last line of its thought, and each step that it has written whole. */
  private streamBody(box: BoxRenderable, id: string, stream: { text: string; thinking: string }): void {
    // The words of a model stand where they stand once they land: each mark in the column of the marks, and each text
    // in the column of the text of the steps.
    const inner = this.box({ paddingLeft: space.between - space.inset });
    const lead = Bun.stringWidth(`${glyph.closed} `);
    const thought = stream.thinking.trim().split("\n").at(-1) ?? "";
    if (thought)
      inner.add(this.text(thought, c.faint, { attributes: italic, truncate: true, marginLeft: lead }));
    const whole = stream.text.slice(0, stream.text.lastIndexOf("\n") + 1);
    const said = steps(whole);
    for (const [at, line] of said.entries())
      inner.add(this.step([`${id}:${at}`], line, c.prose, {}, at ? "  " : `${glyph.closed} `));
    if (!said.length && whole.trim())
      inner.add(
        this.text(
          [
            [`${glyph.closed} `, c.faint],
            [this.firstLine(whole), c.faint],
          ],
          c.faint,
          { truncate: true },
        ),
      );
    // A model that has said nothing yet is waited for, and the card says so.
    if (!thought && !whole.trim())
      inner.add(
        this.text("Waiting for the first words of the model", c.faint, {
          attributes: italic,
          marginLeft: lead,
        }),
      );
    box.add(inner);
  }
  /** A word as the steps that its comments say, or its first line where it says none. Open, it shows its Python and
   * what each act it made came to. Under it stand the diff of each file that its writes changed, and what it raised,
   * or the findings of the gate that refused it. A word that a later word replaced stands in the color of the chrome. */
  private wordBody(
    box: BoxRenderable,
    item: Word,
    how: {
      closed: boolean;
      toggle: () => void;
      retried: boolean;
      changes: ShownChange[];
      findings: string[];
    },
  ): void {
    const w = this.session;
    const { code, rung } = item;
    const tone = how.retried ? c.faint : c.prose;
    const inner = this.box({ paddingLeft: space.between - space.inset });
    const shape: TextShape = {
      run: how.toggle,
      onMouseDown: (event) => {
        if (event.button === 2 && rung) this.actActions(w.acts.find((act) => act.id === rung.id) ?? rung);
      },
    };
    const said = steps(code);
    // The fold of the word marks its first line, so that its steps read as work that Enter opens, apart from what
    // someone said.
    const fold = `${how.closed ? glyph.closed : glyph.open} `;
    for (const [at, line] of said.entries())
      inner.add(
        this.step(
          [rung?.id, item.key].flatMap((one) => (one ? [`${one}:${at}`] : [])),
          line,
          tone,
          shape,
          at ? "  " : fold,
        ),
      );
    if (!said.length)
      inner.add(
        this.text(
          [
            [fold, c.faint],
            [this.firstLine(code), c.faint],
          ],
          c.faint,
          { truncate: true, ...shape },
        ),
      );
    const lines = inner.getChildren().length;
    if (!how.closed) {
      const detail = this.box({ marginTop: space.section, gap: space.stack });
      if (code) detail.add(this.numbered(code, { fg: c.faint, minWidth: 3 }));
      // Each act stands in the order the word made it, with what it told, and each note of a query among them. The code
      // of the word and each block of a command stand apart by one line from the lines that follow them.
      const acts = new Set(actsOf(item).map((act) => act.id));
      const notes = notesOf(item);
      let apart = Boolean(code);
      for (const one of item.told) {
        const at = detail.getChildren().length;
        if ("id" in one)
          this.made(
            detail,
            one,
            notes.filter((note) => note.act?.id === one.id),
            how.retried,
          );
        else if (!acts.has(one.act?.id ?? "")) this.noteLine(detail, one);
        const first = detail.getChildren()[at];
        if (first && apart && !("id" in one && one.kind === "bash")) first.marginTop = space.section;
        if (first) apart = "id" in one && one.kind === "bash";
      }
      inner.add(detail);
    } else
      for (const act of actsOf(item))
        if (act.kind === "bash") this.commandBlock(inner, act, false);
        else if (working(act)) this.made(inner, act, [], false);
    this.diffs(inner, how.changes);
    // A word that a later word replaced says so in one line, and what refused it, or what it raised, no longer counts.
    // Each line stands under the text of the steps, after the fold.
    const under = this.box({
      marginLeft: Bun.stringWidth(fold),
      marginTop: inner.getChildren().length > lines ? space.section : 0,
    });
    if (rung?.run?.status === "failed" && !cancelled(rung) && how.retried)
      under.add(
        this.text(
          `${how.findings.length ? "The gate refused this word" : "This word raised"}, and the next took its place.`,
          c.faint,
        ),
      );
    else if (rung?.run?.status === "failed" && !cancelled(rung)) {
      if (how.findings.length) {
        under.add(this.text("The gate refused this word", c.warm));
        for (const finding of how.findings.filter(Boolean))
          under.add(this.branched(readable(finding), c.warm));
      } else under.add(this.text(readable(rung.run.reason ?? ""), c.warm));
    }
    if (under.getChildren().length) inner.add(under);
    box.add(inner);
  }
  /** A command as one block wherever it stands, under the steps of a word or as a command of the operator: its line,
   * with a spinner and its time while it runs, or its end when it failed or a cancel ended it. Folded, the block shows
   * the last line that the command printed; open, all that it printed and how it ended. A click on its line folds or
   * opens it, where the block folds apart from a word. The block stands where the marks of the steps stand, and its
   * background reaches the edge of the feed, as the card of a thread does. It stands apart by one line from what
   * stands before it. */
  private commandBlock(box: BoxRenderable, act: ActRow, whole: boolean, toggle?: () => void): void {
    const exit = (act.value && typeof act.value === "object" ? act.value : {}) as Exit;
    const block = this.box({
      backgroundColor: c.surface2,
      paddingX: space.inset,
      marginLeft: -space.inset,
      marginTop: box.getChildren().some((child) => child.visible) ? space.section : 0,
    });
    const head = (): Part[] => {
      const live = working(act);
      const ended = cancelled(act)
        ? "cancelled"
        : act.done && exit.code !== 0
          ? `exit ${exit.code ?? "timeout"}`
          : "";
      return [
        live ? [this.session.preferences.motion ? spin() : glyph.running, c.model] : ["$", c.faint],
        [" "],
        [this.firstLine(String(act.words[0] ?? "")), c.bright],
        [live ? `  ${this.actState(act).word}` : "", c.faint],
        [!whole && ended ? `  ${ended}` : "", cancelled(act) ? c.faint : c.warm],
      ];
    };
    const line = this.line(head(), c.bright, { truncate: true, run: toggle });
    if (working(act) && this.session.preferences.motion) this.move(line, head);
    block.add(line);
    if (whole) this.commandDetails(block, act);
    else {
      const last = [exit.stdout?.content, exit.stderr?.content]
        .filter(Boolean)
        .join("\n")
        .split("\n")
        .map((one) => one.trim())
        .findLast(Boolean);
      if (last) block.add(this.line(last, c.faint, { truncate: true, run: toggle }));
    }
    box.add(block);
  }
  /** A step of a word: one line of markdown in a tone, after its lead in the tone of the chrome. A step that wraps goes
   * on under its own text, and not under its lead. A step that shows first after its view opened lands bright, and
   * settles to its tone in three steps. It lands once under all its keys: the key of its turn, and the key of its rung
   * once the rung is known, which a word that its model streams has from the start. */
  private step(
    keys: readonly string[],
    line: string,
    tone: RGBA,
    options: TextShape,
    lead: string,
  ): BoxRenderable {
    const born =
      keys.map((key) => this.landed.get(key)).find((one) => one !== undefined) ??
      (this.opening || !this.session.preferences.motion ? 0 : Date.now());
    for (const key of keys) this.landed.set(key, born);
    const parts = () => {
      const settled = Math.min(3, Math.floor(((Date.now() - born) * 3) / motion.settle));
      return this.inline(line, settled >= 3 ? tone : mix(c.bright, tone, settled / 3));
    };
    const row = this.box({ flexDirection: "row" });
    row.add(this.text(lead, c.faint, options));
    const node = this.text(parts(), tone, { ...options, flexShrink: 1 });
    row.add(node);
    if (Date.now() - born < motion.settle) this.move(node, parts, born + motion.settle);
    return row;
  }
  /** An act that a word made, on one line with no name: its state and what it is, then what it printed while it runs,
   * or what it told. */
  private made(box: BoxRenderable, act: ActRow, notes: Note[], quiet: boolean): void {
    // A command is the block of a command, open, with all that it printed.
    if (act.kind === "bash") {
      this.commandBlock(box, act, true);
      return;
    }
    const parts = (): Part[] => {
      const { word, mark, color } = this.actState(act);
      return [
        [`${mark} `, quiet ? c.faint : color],
        [this.describe(act), quiet ? c.faint : c.prose],
        [word ? `  ${word}` : "", c.faint],
      ];
    };
    const node = this.line(parts(), c.prose, { truncate: true });
    if (working(act) && this.session.preferences.motion) this.move(node, parts);
    box.add(node);
    this.actPreview(act)?.(box);
    for (const note of notes) if (note.body) this.excerpt(box, note.body, false, quiet ? c.faint : c.prose);
  }
  /** What a query of a word told: its kind and what it was about, then the start of what it told. */
  private noteLine(box: BoxRenderable, note: Note): void {
    const danger = ["raised", "refused"].includes(note.label);
    box.add(
      this.line(
        [
          [`${danger ? glyph.failed : glyph.small} `, danger ? c.warm : c.faint],
          [note.label, c.faint],
          [note.detail ? `  ${note.detail}` : "", c.prose],
        ],
        c.faint,
        { truncate: true },
      ),
    );
    if (note.body) this.excerpt(box, note.body, false, danger ? c.warm : c.prose);
  }
  /** What an act is, in words and with no name: the line of a command, the time of a wait, whom a thread asks, the
   * first step of a rung, the name of a chain, the ceilings of a grant, and the kind and the first word of any other. */
  private describe(act: ActRow): string {
    const w = this.session;
    if (act.kind === "bash") return `$ ${this.firstLine(String(act.words[0] ?? ""))}`;
    if (act.kind === "thread") {
      const to = asksOperator(act) ? "you" : this.model(String(act.words[2] ?? "")).name;
      const where = act.on === w.selected ? "" : ` on ${w.labelOf(act.on)}`;
      return `asked ${to}${where}: ${this.firstLine(String(act.words[1] ?? ""))}`;
    }
    if (act.kind === "rung") {
      const word = String(w.program[act.id] || act.words[0] || "");
      return steps(word)[0] ?? this.firstLine(word);
    }
    if (act.kind === "chain") return `started the chain ${w.labelOf(act.id)}`;
    if (["wait", "grant"].includes(act.kind)) return `${act.kind} ${this.subject(act)}`;
    return `${act.kind} ${this.firstLine(String(act.words[0] ?? ""))}`.trim();
  }
  /** The heading of an act that no word made: its state, what it is, and the word of its state. */
  private actHeading(act: ActRow): Part[] {
    const { word, mark, color } = this.actState(act);
    return [
      [`${mark} `, color],
      [this.preview(this.describe(act), Bun.stringWidth(word) + 8), c.bright],
      [word ? `  ${word}` : "", color],
    ];
  }
  /** The diff of each file that a word changed, each under its path and the lines it added and removed. */
  private diffs(box: BoxRenderable, changes: ShownChange[]): void {
    const root = this.session.host.directory;
    for (const change of changes) {
      const { added, removed } = lineCounts(change.patch);
      const path = change.path.startsWith(`${root}/`) ? change.path.slice(root.length + 1) : change.path;
      box.add(
        this.line(
          [
            [shortenHome(path), c.prose],
            ["  "],
            [`+${added}`, added ? c.done : c.faint],
            [` -${removed}`, removed ? c.warm : c.faint],
          ],
          c.prose,
          { truncate: true, marginTop: space.section },
        ),
      );
      box.add(this.diff(change.patch, path, false));
    }
  }
  /** A patch as a diff: each added and removed line tinted, in the colors of the language of its file, and in two
   * sides where the view asks for them. One side is one gutter for the whole patch, in which the lines that two hunks
   * leave out between them stand as one row with no number and the mark ⋯, so a jump of the line numbers never reads
   * as a line that was there, and every hunk keeps one column of numbers. */
  private diff(patch: string, path: string, split: boolean): Renderable {
    if (split)
      return new DiffRenderable(this.renderer, {
        diff: patch,
        view: "split",
        filetype: filetype(path),
        conceal: false,
        syntaxStyle: this.style,
        fg: c.bright,
        showLineNumbers: true,
        lineNumberFg: c.faint,
        lineNumberBg: c.ground,
        contextBg: c.ground,
        addedBg: c.added,
        removedBg: c.removed,
        addedSignColor: c.done,
        removedSignColor: c.warm,
        wrapMode: "word",
      });
    const lines: string[] = [];
    const lineColors = new Map<number, LineColorConfig>();
    const lineSigns = new Map<number, LineSign>();
    const lineNumbers = new Map<number, number>();
    const hideLineNumbers = new Set<number>();
    for (const [at, hunk] of (parsePatch(patch)[0]?.hunks ?? []).entries()) {
      if (at) {
        hideLineNumbers.add(lines.length);
        lineSigns.set(lines.length, { after: " ⋯", afterColor: c.faint });
        lines.push("");
      }
      let before = hunk.oldStart;
      let after = hunk.newStart;
      for (const line of hunk.lines) {
        const row = lines.length;
        if (line.startsWith("+")) {
          lineColors.set(row, { gutter: c.ground, content: c.added });
          lineSigns.set(row, { after: " +", afterColor: c.done });
          lineNumbers.set(row, after++);
        } else if (line.startsWith("-")) {
          lineColors.set(row, { gutter: c.ground, content: c.removed });
          lineSigns.set(row, { after: " -", afterColor: c.warm });
          lineNumbers.set(row, before++);
        } else if (line.startsWith(" ")) {
          lineNumbers.set(row, after++);
          before++;
        } else continue;
        lines.push(line.slice(1));
      }
    }
    return new LineNumberRenderable(this.renderer, {
      // A diff shows the source as it stands, so markdown keeps its marks.
      target: new CodeRenderable(this.renderer, {
        content: safeText(lines.join("\n")),
        filetype: filetype(path),
        syntaxStyle: this.style,
        fg: c.bright,
        wrapMode: "word",
        drawUnstyledText: true,
        conceal: false,
      }),
      fg: c.faint,
      bg: c.ground,
      lineColors,
      lineSigns,
      lineNumbers,
      hideLineNumbers,
    });
  }
  /** A text that moves while its cause holds: the tick draws its parts again, until a time after which it shows them
   * for good. It shows its parts at once. */
  private move(node: TextRenderable, parts: () => Part[], until?: number): void {
    const mover: { parts: () => Part[]; until?: number; shown?: string } = { parts, until };
    this.movers.set(node, mover);
    this.draw(node, mover);
    this.animate();
  }
  /** A text that moved, still. */
  private still(node: TextRenderable): void {
    this.movers.delete(node);
  }
  /** A text of the movers drawn with its parts of now, when they changed. */
  private draw(node: TextRenderable, mover: { parts: () => Part[]; shown?: string }): void {
    const parts = mover.parts();
    const shown = JSON.stringify(parts.map(([text, fg, attributes]) => [text, fg?.toInts(), attributes]));
    if (shown === mover.shown) return;
    mover.shown = shown;
    node.content = styled(parts);
  }
  /** The tick runs while anything moves, and stops when nothing does. */
  private animate(): void {
    const moves = this.movers.size > 0 || this.statusMoves;
    if (moves && !this.tick) this.tick = setInterval(this.advance, 20);
    else if (!moves && this.tick) {
      clearInterval(this.tick);
      this.tick = undefined;
    }
  }
  /** One tick: each mover drawn as it stands now, and the footer while it moves. */
  private advance = (): void => {
    if (this.closed) return;
    const now = Date.now();
    for (const [node, mover] of this.movers) {
      if (node.isDestroyed) {
        this.movers.delete(node);
        continue;
      }
      this.draw(node, mover);
      if (mover.until !== undefined && now >= mover.until) this.movers.delete(node);
    }
    if (this.statusMoves) this.renderStatus();
    this.animate();
  };
  /** The time each thread closed while the App watched it, which its check shows for a moment. */
  private watchThreads(): void {
    const now = Date.now();
    for (const act of this.session.acts) {
      if (act.kind !== "thread") continue;
      if (!act.done) this.openThreads.add(act.id);
      else if (this.openThreads.delete(act.id)) this.closedAt.set(act.id, now);
    }
  }
  /** A panel of the feed that says a state of the view: its heading, what the state holds, and a link that acts on it
   * with what the link does. */
  private banner(
    box: BoxRenderable,
    color: RGBA,
    heading: Part[],
    detail: Renderable | undefined,
    [label, does, run]: [string, string, () => void],
  ): void {
    const panel = this.panel(box, color);
    panel.add(this.text(heading));
    if (detail) panel.add(this.inset(space.between, detail));
    const row = this.box({ flexDirection: "row", marginTop: space.section, paddingLeft: space.between });
    row.add(this.link([[label, c.operator, bold]], run));
    row.add(this.text(`  ${does}`, c.faint));
    panel.add(row);
  }
  /** A text that lights under the pointer, and runs an action at a click. */
  private link(content: string | Part[], run: () => void): TextRenderable {
    return this.hoverable(this.text(content, c.operator, { run }));
  }
  /** A message of the operator: its text, and the name of each image it refers to in the color of an attachment. A
   * click on the message opens the actions of its first image. */
  private message(text: string): TextRenderable {
    const images = imageReferences(text);
    const parts: Part[] = [];
    let rest = text;
    for (const image of images) {
      const at = rest.indexOf(image.text);
      parts.push([rest.slice(0, at), c.bright], [`${glyph.dot} ${image.name || "image"}`, c.model]);
      rest = rest.slice(at + image.text.length);
    }
    parts.push([rest, c.bright]);
    const first = images[0];
    return this.text(parts, c.bright, { run: first && (() => this.imageActions(first.uri)) });
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
    // The logo shades from the accent to the color of Python, one column at a time.
    const columns = Math.max(...logo.map((line) => line.length));
    for (const line of logo)
      column.add(
        this.text(
          [...line].map((cell, at): Part => [cell, mix(c.operator, c.model, at / Math.max(1, columns - 1))]),
          c.operator,
          { width: columns },
        ),
      );
    column.add(
      this.text("The model answers in Python. Read and steer each word it runs.", c.prose, {
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
    // Each way to start is a card of two lines that a click puts in the composer. The cards are as wide as their
    // longest line, and stand in the middle as one block.
    const widest = Math.max(...starters.flat().map((line) => Bun.stringWidth(line)));
    const starts = this.box({
      width: Math.min(width, widest + space.between * 2 + 1),
      marginTop: space.section * 2,
      gap: space.section,
    });
    for (const [label, prompt] of starters) {
      // The pointer lifts a card onto a panel, and its bar takes the accent.
      const card = this.box({
        paddingX: space.between,
        backgroundColor: c.ground,
        border: ["left"],
        borderColor: c.rule,
        customBorderChars: { ...noBorder, vertical: glyph.bar },
        onMouseUp: this.click(() => this.insert(prompt)),
        onMouseOver() {
          this.backgroundColor = c.surface2;
          this.borderColor = c.operator;
        },
        onMouseOut() {
          this.backgroundColor = c.ground;
          this.borderColor = c.rule;
        },
      });
      card.add(this.text([[label, c.bright, bold]], c.bright, { truncate: true }));
      card.add(this.text(prompt, c.prose, { truncate: true }));
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
            [chord, c.prose],
            [` ${action}`, c.faint],
          ],
          c.faint,
          { run },
        ),
      );
    column.add(keys);
    this.centered(box, height, column);
  }

  private preview(text: string, reserve = 2): string {
    return clip(text.replace(/\s+/g, " "), Math.max(8, this.feedWidth - reserve));
  }
  /** The first or the last lines of a text under a heading, joined to it by a branch. */
  private excerpt(box: BoxRenderable, content: string, tail = false, color = c.prose): void {
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
  private actPreview(act: ActRow): BlockOptions["preview"] {
    if (act.run)
      return act.run.reason && !cancelled(act)
        ? (box) => this.excerpt(box, readable(act.run?.reason ?? ""), false, c.warm)
        : undefined;
    if (failed(act)) {
      const fault = act.value as { is: string; args: unknown[] };
      return (box) => this.excerpt(box, `${fault.is}: ${fault.args.map(display).join(", ")}`, false, c.warm);
    }
    // The answer of a thread is markdown, whose marks of a heading, of emphasis, and of code the preview leaves out.
    if (act.kind === "thread" && act.done && act.value !== null)
      return (box) =>
        this.excerpt(
          box,
          display(act.value)
            .replace(/^#{1,6} /gm, "")
            .replace(/(\*\*|__|\*|`)(\S(?:.*?\S)?)\1/g, "$2"),
        );
    return undefined;
  }
  /** What an act is doing: the word that says it, the glyph that shows it, and their color. A done act says
   * nothing, since its glyph says it. */
  private actState(act: ActRow): State {
    const w = this.session;
    const running = () => ({ word: `running ${this.progress(act.id)}`, mark: spin(), color: c.operator });
    if (act.kind === "grant" && act.done) return { word: "ended", mark: glyph.ring, color: c.faint };
    // Only work moves: an act of another kind that lives, as a grant or the watcher of an extension, lives until
    // something ends it, as a chain does, and shows a dot.
    if (!act.done && !WORK.includes(act.kind)) return { word: "", mark: glyph.dot, color: c.operator };
    // A rung that a pause holds waits for the wake, and says so, where it would otherwise seem to run.
    if (act.kind === "rung")
      return cancelled(act)
        ? stateOf("cancelled")
        : act.run?.status === "failed"
          ? this.retry(act)
            ? { word: "retried", mark: glyph.failed, color: c.faint }
            : stateOf("failed")
          : act.run?.status === "done"
            ? stateOf("done")
            : act.paused
              ? stateOf("held")
              : running();
    if (act.done) return stateOf(failed(act) ? "failed" : cancelled(act) ? "cancelled" : "done");
    if (w.host.pending.has(act.id)) return { word: "pending", mark: glyph.ring, color: c.faint };
    if (w.host.threads.has(act.id)) return { word: "needs input", mark: glyph.asks, color: c.warm };
    if (act.paused && act.kind !== "bash") return stateOf("held");
    return running();
  }
  /** The rung that took the place of a rung of a model that failed: a later rung for the same thread, which the model
   * wrote once it read the failure, so the failure no longer stands. */
  private retry(act: ActRow): ActRow | undefined {
    if (act.kind !== "rung" || act.by === "operator" || !failed(act)) return undefined;
    const acts = this.session.acts;
    return acts.slice(acts.indexOf(act) + 1).find((other) => other.kind === "rung" && other.by === act.by);
  }
  /** What an act is about, whole: the markdown of a thread, the program of a rung, the time of a wait, the ceilings of
   * a grant, and the first word of any other act. */
  private subject(act: ActRow): string {
    if (act.kind === "thread") return String(act.words[1] ?? "");
    if (act.kind === "rung") return String(this.session.program[act.id] || act.words[0] || "");
    if (act.kind === "wait") return seconds(Number(act.words[0]));
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
  private messageState(act: ActRow): Part[] {
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
  /** Who sent a thread that is not a message of the operator, and to which model: a chain tells its model that an
   * act it waits on is done, and a rung asks a model a question. */
  private sender(act: ActRow): string {
    const model = this.model(String(act.words[2] ?? "")).name;
    const maker = this.session.actOf(act.by);
    return maker?.kind === "chain" ? `the chain told ${model}` : `${act.by} asked ${model}`;
  }
  /** A word of Python in a card: its numbered lines, and the reason it failed. */
  private python(box: BoxRenderable, word: string, reason?: string): void {
    // The code starts under the name of its rung, past the fold and the glyph of the heading.
    const inner = this.box({ gap: space.stack, paddingLeft: space.inset });
    // A rung whose word has not come yet shows no code.
    if (word) inner.add(this.numbered(word));
    if (reason) inner.add(this.branched(readable(reason), c.warm));
    box.add(inner);
  }
  /** What an act that no word made holds, open: the Python of a rung, or the words and the value of any other act, then
   * the start of what each note told of it. A command is a block of its own. */
  private actDetails(box: BoxRenderable, act: ActRow, notes: Note[] = []): void {
    if (act.kind === "rung") {
      this.python(box, String(this.session.program[act.id] || act.words[0] || ""), act.run?.reason);
      return;
    }
    const details = this.box({ paddingLeft: space.between, gap: space.stack });
    box.add(details);
    for (const note of notes) if (note.body) this.excerpt(details, note.body);
    const fields: Record<string, string[]> = {
      thread: ["shape", "markdown", "actor"],
      rung: ["word", "retells", "actor", "returns"],
      wait: ["seconds"],
      grant: ["dollar ceiling", "context ceiling"],
    };
    for (const [index, value] of act.words.entries()) {
      if (value === null || value === "") continue;
      details.add(
        this.text([
          [`${fields[act.kind]?.[index] ?? `argument ${index + 1}`}: `, c.faint],
          [display(value), c.prose],
        ]),
      );
    }
    if (act.done && act.value !== null)
      details.add(
        this.text(display(act.value), failed(act) ? c.warm : c.bright, { marginTop: space.section }),
      );
  }
  /** What a command printed, and under it its name and how it ended. The command says its line in its heading. What it
   * printed to stderr takes the color of a failure, and each stream is named only when the command printed to both. */
  private commandDetails(details: BoxRenderable, act: ActRow): void {
    const exit = (act.value && typeof act.value === "object" ? act.value : {}) as Exit;
    const both = Boolean(exit.stdout?.content && exit.stderr?.content);
    const shown: TextRenderable[] = [];
    for (const [name, color] of [
      ["stdout", c.bright],
      ["stderr", c.warm],
    ] as const) {
      const content = exit[name]?.content;
      if (!content) continue;
      if (both)
        details.add(
          this.text(name, name === "stdout" ? c.faint : c.warm, {
            marginTop: shown.length ? space.section : 0,
          }),
        );
      // The line end that closes what a command printed opens no empty row.
      const node = this.text(content.replace(/\n$/, ""), color);
      details.add(node);
      shown.push(node);
    }
    const [, input, timeout] = act.words;
    const notes: Part[] = [
      ...(act.done
        ? ([
            ["exit ", c.faint],
            [String(exit.code ?? "timeout"), exit.code === 0 ? c.done : c.warm],
          ] as Part[])
        : []),
      [input === true ? `${act.done ? "   " : ""}input open` : "", c.faint],
      // A limit of time holds while the command runs; once it ends, its exit says whether the limit ended it.
      [
        !act.done && typeof timeout === "number" && timeout !== TIMEOUT
          ? `${input === true ? "   " : ""}times out after ${timeout}s`
          : "",
        c.faint,
      ],
    ];
    // A command that runs with no limit of its own and no open input has nothing to note until it ends.
    if (notes.some(([text]) => text))
      details.add(this.text(notes, c.faint, { marginTop: shown.length ? space.section : 0 }));
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
    const node = this.text(label, c.operator, { attributes: underline });
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
        this.text(value, c.operator, { attributes: bold }),
        this.text(detail.slice(0, 400), c.prose, { maxHeight: 4 }),
      );
    } catch {
      /* A path can have disappeared since the turn was written. */
    }
  }

  private renderRail(): void {
    if (!this.rail.visible) return;
    this.renderRailSession();
    this.renderWorkspaces();
  }
  /** The session in the sidebar: its name and directory, its chains, and what its context and its model cost. */
  private renderRailSession(): void {
    const w = this.session;
    const window = w.filled;
    const grant = w.activity.find((act) => act.kind === "grant" && !act.done);
    const width = w.preferences.sidebarWidth;
    const threads = w.threads;
    if (
      !this.paneChanged(this.railSession, [
        this.theme,
        w.selected,
        w.thread,
        w.spend,
        window,
        width,
        w.chains.map((chain) => [
          chain.id,
          w.labelOf(chain.id),
          this.session.chainStatus(chain.id),
          threads
            .on(chain.id)
            .map((thread) => [thread.id, thread.words[1], plain(this.threadState(thread, 0).label)]),
        ]),
        grant?.words,
        this.unfolded.has("finished"),
        w.preferences.motion,
      ])
    )
      return;
    this.clear(this.railSession);
    this.clear(this.railUsage);
    const inner = width - space.between * 2;
    // The chains fill the head of the sidebar, and the usage its foot.
    let target: BoxRenderable = this.railSession;
    const add = (parts: Part[], options: TextShape = {}) => {
      const node = this.line(parts, c.prose, { truncate: true, ...options });
      target.add(node);
      return node;
    };
    // A line of a table: its name and its note at its left, and its value at its right.
    const spread = (name: Part, note: string, value: Part, options: TextShape = {}) => {
      const room = Math.max(1, inner - Bun.stringWidth(name[0] + note + value[0]));
      return add([name, [note, c.faint], [" ".repeat(room)], value], options);
    };
    // A row of the usage: its name, and its value, each number bright and each unit faint.
    const row = (name: string, value: string) => {
      const room = Math.max(1, inner - Bun.stringWidth(name + value));
      return add([[name, c.prose], [" ".repeat(room)], ...quantity(value)]);
    };
    // The heading of a section, which stands first in its part of the sidebar.
    const section = (name: string, value = "", note = "", run?: () => void) =>
      spread([name, c.bright], note, [value, c.faint], { run });
    section("Chains", "⌃B", "", () => this.chains());
    // The root chain, the chain that is shown, and a chain that has something to say stand in the list, and the other
    // finished chains fold under a row of their own, which a click opens. The list takes six rows at most, and the rest
    // wait behind a button that lists them all.
    const states = new Map(w.chains.map((chain) => [chain.id, this.session.chainStatus(chain.id)]));
    const active = w.chains.filter(
      (chain) => chain.id === w.engine.root || chain.id === w.selected || states.get(chain.id) !== "idle",
    );
    const resting = w.chains.filter((chain) => !active.includes(chain));
    // A row of a chain spans the sidebar past its padding.
    const edge = { marginX: -space.between, paddingX: space.inset };
    // A chain names itself, and its feed shows when its name is chosen. Under it stand its threads: the open ones, then
    // the last closed ones, dim, and a row that opens the feed of the chain where more closed.
    const chainLine = (chain: ActRow) => {
      const selected = chain.id === w.selected;
      const status = states.get(chain.id) ?? "idle";
      this.railSession.add(
        this.railRow(
          [
            [selected ? glyph.mark : " ", c.operator],
            [" "],
            statusMark(status),
            [w.labelOf(chain.id), selected ? c.bright : status === "idle" ? c.faint : c.prose],
          ],
          () => void w.select(chain.id).catch(w.fail),
          edge,
          selected && !w.thread,
        ),
      );
      const all = threads.on(chain.id);
      const closed = all.filter((thread) => thread.done);
      const kept = closed.slice(-3);
      for (const thread of [...all.filter((one) => !one.done), ...kept]) {
        const zoomed = thread.id === w.thread;
        const state = this.threadState(thread, all.indexOf(thread));
        const title = this.firstLine(String(thread.words[1] ?? ""));
        const parts = (): Part[] => [
          [zoomed ? glyph.mark : " ", c.operator],
          ["   "],
          state.mark(),
          [" "],
          [title, zoomed ? c.bright : thread.done ? c.faint : c.prose, zoomed ? bold : 0],
        ];
        const line = this.railRow(parts(), () => void w.open(thread.id).catch(w.fail), edge, zoomed);
        if (state.moves) this.move(line.getChildren()[0] as TextRenderable, parts, state.until);
        this.railSession.add(line);
      }
      if (closed.length > kept.length)
        this.railSession.add(
          this.railRow(
            [["      "], [`${closed.length - kept.length} more closed`, c.faint]],
            () => void w.select(chain.id).catch(w.fail),
            edge,
          ),
        );
    };
    const shown = active.length > 7 ? 6 : active.length;
    for (const chain of active.slice(0, shown)) chainLine(chain);
    if (active.length > shown)
      add(
        [
          ["   ", c.faint],
          [`${active.length - shown} more`, c.operator],
        ],
        { run: () => this.chains() },
      );
    if (resting.length) {
      // The row of the finished chains stands as a chain row does: its fold mark in the column of the bar of the
      // selected chain, and its name where the names of the chains start.
      this.railSession.add(this.foldRow("finished", " Finished", resting.length, edge));
      if (this.unfolded.has("finished")) for (const chain of resting.slice(0, 12)) chainLine(chain);
    }
    target = this.railUsage;
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
      const tone = part >= 0.7 ? c.warm : c.operator;
      const cells: Part[] = [];
      for (let at = 0; at < inner; at++)
        cells.push(at === mark ? ["╋", c.warm] : [glyph.meter, at < used ? tone : c.rule]);
      const tip: Part[] = [
        [known ? `${Number((part * 100).toFixed(1))}%` : "No answer yet", c.bright],
        [known ? " of the context window" : "", c.prose],
        [ceiling === undefined ? "" : `, pauses at ${Number((ceiling * 100).toFixed(1))}%`, c.warm],
      ];
      this.tipped(add(cells), tip);
      // The last prompt is the whole prompt of the last answer, its system prompt and the reads of the cache included,
      // which the share of the window measures. The fresh input, the output and the cache are what every answer of the
      // chain read past the cache, wrote, and read and wrote in the cache, each token counted once.
      const prompt = w.context;
      if (prompt !== undefined)
        this.tipped(row("Last prompt", count(prompt)), [
          ["The whole prompt of the last answer, its system prompt and the cache included.", c.bright],
        ]);
      if (tokens > 0) {
        this.tipped(row("Fresh input", count(spend.input)), [
          ["The input of the answers of the chain that the cache did not hold.", c.bright],
        ]);
        row("Output", count(spend.output));
        if (spend.cacheRead) row("Cache read", count(spend.cacheRead));
        if (spend.cacheWrite) row("Cache write", count(spend.cacheWrite));
      }
      row("Spent", dollars(spend.dollars));
      if (grant && grant.words[0] !== null) row("Ceiling", dollars(Number(grant.words[0])));
    }
  }
  /** A row of the sidebar, whose click runs, which spans the sidebar so that the pointer and the selection show from
   * edge to edge. It lights while the pointer is on it, and stays lit while it is selected. */
  private railRow(parts: Part[], run: () => void, options: BoxOptions, selected = false): BoxRenderable {
    const row = this.hoverable(
      this.row({
        backgroundColor: selected ? c.selected : c.surface1,
        onMouseUp: this.click(run),
        ...options,
      }),
      selected ? c.selected : c.surface2,
    );
    row.add(this.text(parts, c.prose, { truncate: true, flexGrow: 1, flexShrink: 1 }));
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
    this.railHeading.add(this.text("Workspaces", c.bright, { attributes: bold, flexGrow: 1 }));
    this.railHeading.add(this.text("⌃W", c.faint, { run: () => void this.workspacePicker() }));
    const padding = { paddingLeft: space.inset, paddingRight: space.between };
    const line = (parts: Part[], run: () => void, options: BoxOptions = {}) =>
      this.railSpaces.add(this.railRow(parts, run, { ...padding, ...options }));
    if (library.notice) this.railSpaces.add(this.inset(space.between, this.text(library.notice, c.warm)));
    if (!library.groups.length)
      this.railSpaces.add(
        this.inset(space.between, this.text("No workspaces yet. Add a project folder below.", c.prose)),
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
      const lit = selected ? c.selected : c.surface2;
      const ground = selected ? c.selected : this.menuItem === item ? lit : c.surface1;
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
      row.add(this.line(parts.lead, c.prose));
      const state = parts.state && this.line([parts.state], c.prose);
      if (state && !parts.after) row.add(state);
      if (renaming) {
        const input = new InputRenderable(this.renderer, {
          flexGrow: 1,
          flexShrink: 1,
          value: renaming.value,
          ...inputColors(c.ground),
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
      const name = this.line([parts.name], c.prose, {
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
        rename.fg = event.target === rename ? c.operator : c.prose;
        remove.fg = event.target === remove ? c.warm : c.prose;
        // A name that its room cuts shows whole in a tip of its own.
        if (event.target === name) return;
        this.unhover();
        if (event.isDragging) return;
        const told: Part[] | undefined =
          event.target === rename
            ? [[`Rename this ${noun}`, c.bright]]
            : event.target === remove
              ? [
                  [
                    "sessions" in item
                      ? "Remove this workspace from the list"
                      : "Archive or remove this session",
                    c.bright,
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
          name: [group.name, current ? c.bright : c.prose, bold],
          state: quiet ? undefined : statusMark(status),
          after: true,
        },
        () => [
          statusMark(status),
          [statusLabels[status], c.bright, bold],
          [
            `  ${count} of ${group.sessions.length} ${group.sessions.length === 1 ? "session" : "sessions"}`,
            c.prose,
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
            lead: [[selected ? glyph.mark : " ", c.operator], ["  "]],
            state: statusMark(entry.status),
            name: [entry.name, selected ? c.bright : entry.archived ? c.faint : c.prose, selected ? bold : 0],
          },
          () => [
            statusMark(entry.status),
            [statusLabels[entry.status], c.bright, bold],
            [entry.error ? `  ${entry.error}` : "", c.warm],
          ],
          () => {
            void library.select(entry).catch(this.report);
          },
          {},
          selected,
        );
      };
      for (const entry of group.sessions.filter((entry) => !entry.archived)) sessionRow(entry);
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
    this.hover = this.box({ position: "absolute", backgroundColor: c.surface2, zIndex: 30, ...options });
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
          color: c.warm,
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
            { label: "Move to trash", detail: "", color: c.warm, run: () => this.removeItem(item) },
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
              mark: ["+ ", c.operator],
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
      effort: this.effortPicker,
      shape: this.shapes,
      theme: () => this.themes(),
      exit: () => this.options.quit(),
      editor: () => this.editDraft(),
      files: this.filesPicker,
      new: () => library.create(),
      sidebar: () => library.toggle(),
      workspace: this.workspacePicker,
    };
    if (name && !argument && Object.hasOwn(bare, name)) {
      await bare[name]?.();
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
    return false;
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
          detail: `to ${this.model(entry.actor).name} on ${session.labelOf(entry.chain)}`,
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
    const message = w.isUserThread(act) && !asksOperator(act);
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
      ...(this.isPoint(act)
        ? [
            {
              label: message ? "Edit and send again" : "Branch after it",
              detail: message
                ? "A new branch that has not read this message, with the message in the input"
                : "A new branch that reads the chain up to this act",
              run: () => w.rewind(act.id).then(() => {}),
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
  /** The token the cursor ends: a `/command` that opens the input, or an `@path` that starts a word of a prompt. */
  private token(): { kind: "/" | "@" | "value"; text: string; command?: string } | undefined {
    const before = this.beforeCursor();
    const slash = before.match(/^\/([\w-]*)$/);
    if (slash) return { kind: "/", text: slash[1] ?? "" };
    // The first word after a command whose values are known is a value, which the suggestions complete.
    const value = before.match(/^\/([\w-]+) (\S*)$/);
    if (value && Object.hasOwn(this.values, value[1] ?? ""))
      return { kind: "value", text: value[2] ?? "", command: value[1] ?? "" };
    const at = before.match(/(?:^|\s)@([^\s"']*)$/);
    if (at && this.session.mode === "markdown" && !this.session.editing)
      return { kind: "@", text: at[1] ?? "" };
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
      return [...roster, ...added].map(([name, , window], at) => {
        const { provider, id } = modelName(name);
        const shared = roster.filter(([other]) => modelName(other).id === id).length > 1;
        return {
          // A model of the roster goes by its id alone, unless two providers offer a model of that id.
          value: at < roster.length && !shared ? id : name,
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
      (Object.keys(palettes) as ThemeName[]).map((name) => ({
        value: name,
        detail: themeLabels[name].join(", "),
        current: name === this.theme,
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
      this.session.activity
        .filter((act) => working(act))
        .map((act) => ({
          value: act.id,
          detail: `${act.kind}  ${clip(String(act.words[act.kind === "thread" ? 1 : 0] ?? ""), 40)}`,
        })),
    feed: () =>
      this.session.activity
        .filter((act) => act.kind === "bash" && !act.done && act.words[1] === true)
        .map((act) => ({ value: act.id, detail: clip(String(act.words[0] ?? ""), 48), more: true })),
    close: () =>
      this.session.activity
        .filter((act) => !act.done && act.kind !== "chain")
        .map((act) => ({ value: act.id, detail: act.kind, more: true })),
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
    const state = !token
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
      this.suggestionBox.add(this.inset(space.between, this.line(state, files?.error ? c.warm : c.prose)));
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
        backgroundColor: selected ? c.selected : c.surface2,
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
      row.add(this.text(selected ? `${glyph.pointer} ` : "  ", c.operator));
      row.add(
        this.text(one.label, selected ? c.operator : c.bright, {
          width: one.detail ? column : undefined,
          truncate: true,
          attributes: selected ? bold : 0,
        }),
      );
      if (one.detail) row.add(this.text(one.detail, c.prose, { truncate: true, flexShrink: 1 }));
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
    return yes ? [`${glyph.done} `, c.operator] : ["  "];
  }
  /** The values of a command in a picker, with the current one marked and selected, which runs the command on the
   * value that is chosen. */
  private pick(command: string, label: string, note: string): void {
    const values = this.values[command]?.() ?? [];
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
  models = (): void => this.pick("model", "Model", "The chain sends its next prompt to this model.");
  effortPicker = (): void =>
    this.pick(
      "effort",
      `Effort of ${modelName(this.session.actorChoice.model).id}`,
      "More effort thinks longer and costs more. ⇧Tab moves to the next.",
    );
  details = (): void => {
    this.openPalette(
      "Details",
      [...this.cards]
        .filter(([, card]) => card.compact || card.collapsible)
        .map(([id, card]) => ({
          label: card.heading.plainText.replace(/^[▸▾] /, ""),
          detail: card.closed ? "Expand" : "Collapse",
          run: () => {
            this.session.folds[card.state] = !card.closed;
            this.renderContent();
            this.scrollAfterLayout({ card: id });
          },
        })),
    );
  };
  themes(): void {
    // The dark themes come first, then the light one.
    const names = (Object.keys(palettes) as ThemeName[]).sort(
      (one, other) => Number(themeLabels[one][1] === "Light") - Number(themeLabels[other][1] === "Light"),
    );
    this.openPalette(
      "Color theme",
      names.map((name) => {
        const palette = palettes[name];
        return {
          label: themeLabels[name][0],
          detail: themeLabels[name][1],
          mark: this.current(name === this.theme),
          swatch: [palette.operator, palette.model, palette.warm, palette.done, palette.bright],
          run: () => {
            this.session.theme = name;
            this.render();
            this.session.save();
          },
        };
      }),
      { selected: names.indexOf(this.theme), note: "Every session shares this choice." },
    );
  }
  shapes = (): void =>
    this.pick("shape", "Response shape", "The engine validates the result against this Python type.");
  toggleMode(): void {
    this.session.mode = this.session.mode === "python" ? "markdown" : "python";
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
        "cursorColor",
      ]) {
        const color: unknown = Reflect.get(node, property);
        if (replacements.has(color as RGBA)) Reflect.set(node, property, replacements.get(color as RGBA));
      }
      // An input hides the colors that it shows focused, and every input of the view shows the colors it shows
      // unfocused.
      if (node instanceof TextareaRenderable)
        Object.assign(node, {
          focusedBackgroundColor: node.backgroundColor,
          focusedTextColor: node.textColor,
        });
      for (const child of node.getChildren()) recolor(child);
    };
    recolor(this.root);
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
  question(): void {
    const question = this.session.operatorThread;
    if (!question) return;
    if (question.shape === "bool") {
      this.openPalette("Answer yes or no", [
        {
          label: "Yes",
          detail: "",
          run: () => this.session.host.answer(question.id, "yes").then(() => {}),
        },
        {
          label: "No",
          detail: "",
          run: () => this.session.host.answer(question.id, "no").then(() => {}),
        },
      ]);
      this.showQuestionText(question.markdown);
      return;
    }
    this.openPalette(`Your answer, as ${question.shape}`, []);
    // The field of the dialog takes the answer, and shows its prompt from the start.
    const prompt = this.paletteInputRow?.getChildren()[0];
    if (prompt) prompt.visible = true;
    if (this.paletteInputRow) this.paletteInputRow.height = space.bar;
    this.showQuestionText(question.markdown);
    if (this.paletteInput) this.paletteInput.placeholder = `Type the answer, as ${question.shape}`;
    const error = this.text("Enter submits. Esc leaves the question open.", c.prose);
    this.paletteList?.add(error);
    const input = this.paletteInput;
    if (!input) return;
    input.removeAllListeners(InputRenderableEvents.INPUT);
    input.removeAllListeners(InputRenderableEvents.ENTER);
    input.on(InputRenderableEvents.ENTER, () => {
      void this.session.host
        .answer(question.id, input.value)
        .then(() => this.closeOverlay())
        .catch((failure) => {
          error.fg = c.warm;
          error.content = String(failure);
        });
    });
  }
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
          [name, c.bright, bold],
          [`: ${inspected.kind}`, c.model],
        ]),
        this.text(inspected.representation.slice(0, 280), c.prose, { maxHeight: 4 }),
        this.text([
          ["Click", c.prose],
          [" opens it   ", c.faint],
          ["⌃G", c.prose],
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
      // The last word that binds the name defines it, and the row shows the line that binds it.
      const binds = (line: string) => new RegExp(`^(?:def |class )?${name}\\b(?:\\s*[:=(])`).test(line);
      const definition = [...Object.entries(this.session.program)]
        .reverse()
        .map(([id, source]) => [id, source.split("\n").find(binds)] as const)
        .find(([, line]) => line !== undefined);
      if (definition)
        choices.push({
          label: "Go to definition",
          detail: definition[1]?.trim() ?? "",
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
                  fg: c.prose,
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
  /** The threads of the chain, whose program an edit changes and replays. */
  ladders(): void {
    const threads = this.session.activity.filter((act) => this.session.editable(act));
    this.openPalette(
      "Edit a thread program",
      threads.map((act) => {
        const { mark, color } = this.actState(act);
        return {
          label: act.id,
          detail: String(act.words[1]),
          mark: [`${mark} `, color],
          run: () => this.action(`/edit ${act.id}`),
        };
      }),
      {
        selected: Math.max(0, threads.length - 1),
        note: "The program is the Python the model wrote for the thread. An edit replays it.",
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
    const last = points.findLast((act) => this.session.isUserThread(act)) ?? points.at(-1);
    this.openTree(last?.id ?? this.session.selected, "Rewind");
  };
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
        parts: [statusMark(status), [w.labelOf(act.id), current ? c.bright : c.prose, bold]],
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
      await w.rewind(act.id);
    };
    if (w.isUserThread(act) && !asksOperator(act))
      return {
        parts: [
          [`${glyph.prompt} `, c.operator],
          [String(act.words[1] ?? "").split("\n")[0] ?? "", c.bright],
        ],
        tag: act.id,
        hint: "edit it on a new branch",
        run: branch,
      };
    const { mark, color } = this.actState(act);
    const subject = this.subject(act);
    const observation = act.kind === "thread" && !asksOperator(act) && !w.isUserThread(act);
    const name = asksOperator(act) ? "question" : act.kind === "rung" ? act.id : act.kind;
    return {
      parts: observation
        ? [
            [`${mark} `, color],
            [subject.split("\n")[0] ?? "", c.prose],
            [`  ${this.sender(act)}`, c.faint],
          ]
        : [
            [`${mark} `, color],
            [name, c.bright],
            // A rung shows the first line of its program, and any other act its words on one line, as the feed does.
            [
              `  ${act.kind === "rung" ? (subject.split("\n")[0] ?? "") : subject.replace(/\s+/g, " ").trim()}`,
              c.prose,
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
          ? "Choose a point to branch from. The module and the files keep their state."
          : "Open a chain, or branch from a point. The module and the files keep their state.";
      // The note is cut at its end where the bar has no room for it beside the title and the key that leaves.
      const room =
        width - space.inset * 2 - Bun.stringWidth(tree.title) - space.between * 2 - Bun.stringWidth("Esc");
      this.treeBar.add(
        this.line(
          [
            [tree.title, c.bright, bold],
            [`${" ".repeat(space.between)}${clip(note, Math.max(0, room))}`, c.prose],
          ],
          c.prose,
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
        backgroundColor: active ? c.selected : c.ground,
        onMouseUp: this.click((event) => {
          // A click on the fold of a row folds it, a click on another row points at it, and a click on the row
          // that the pointer is on chooses it.
          const fold = event.x - (this.scroll.x + space.inset + Bun.stringWidth(row.lines)) <= 1;
          if (row.parent && fold) this.foldTree(row.id, !row.folded);
          else if (active) void this.chooseTreeRow();
          else this.pointTree(row.id);
        }),
      });
      if (!active) this.hoverable(line, c.surface2);
      const fold = row.parent ? `${row.folded ? glyph.closed : glyph.open} ` : "  ";
      const tag = row.tag ? `  ${row.tag}` : "";
      const room = Math.max(8, width - space.inset * 2 - Bun.stringWidth(row.lines + fold + tag));
      const label = plain(row.parts);
      line.add(
        this.line(
          [
            [row.lines, c.faint],
            [fold, c.faint],
            ...(Bun.stringWidth(label) > room ? clipParts(row.parts, room) : row.parts),
          ],
          c.bright,
          { flexGrow: 1, flexShrink: 1, truncate: true },
        ),
      );
      if (tag) line.add(this.line(tag, active ? c.prose : c.faint));
      this.scroll.add(line);
    }
    this.scrollAfterLayout({ card: `tree-${tree.selected}` });
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
    this.render();
    if (id) this.scrollAfterLayout({ card: id });
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
      backgroundColor: c.surface2,
      zIndex: 20,
    });
    this.root.add(this.overlay);
    // The title and the filter start where the labels of the list start, and the prompt of the filter stands over the
    // pointer of the list.
    const heading = this.row({ paddingLeft: space.between, paddingRight: space.inset });
    heading.add(this.text(label, c.bright, { attributes: bold, truncate: true, flexGrow: 1, flexShrink: 1 }));
    heading.add(this.text("Esc", c.faint, { run: () => this.closeOverlay() }));
    this.overlay.add(heading);
    this.paletteNote = note ? space.bar : 0;
    if (note)
      this.overlay.add(this.inset(space.between, this.line(clip(note, this.paletteWidth - 8), c.prose)));
    // A short list shows its filter only once the operator types in it, and until then the filter takes no row.
    const quiet = choices.length <= 5;
    this.paletteInputRow = this.row({
      height: quiet ? 0 : space.bar,
      paddingRight: space.inset,
      marginBottom: space.section,
    });
    const prompt = this.text(`${glyph.prompt} `, c.operator, { visible: !quiet });
    this.paletteInputRow.add(prompt);
    this.paletteInput = new InputRenderable(this.renderer, {
      id: "palette-search",
      flexGrow: 1,
      placeholder: quiet ? "" : "Type to filter",
      ...inputColors(c.surface2),
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
            this.line([[choice.heading ?? "", c.prose, bold]], c.prose, {
              marginTop: offset ? space.section : 0,
            }),
          ),
        );
      const block = this.box({
        backgroundColor: selected ? c.selected : c.surface2,
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
        const bar: Part = [`${selected ? glyph.mark : " "} `, c.operator];
        const width = inner - 2;
        const keys = choice.keys ?? "";
        const top = this.row();
        top.add(
          this.text(
            [bar, [clip(choice.label, width - Bun.stringWidth(keys) - 2), c.bright, bold]],
            c.bright,
            {
              flexGrow: 1,
              flexShrink: 1,
              truncate: true,
            },
          ),
        );
        if (keys) top.add(this.text(keys, selected ? c.prose : c.faint, { paddingRight: space.inset }));
        block.add(top);
        if (choice.command)
          block.add(this.line([bar, [clip(choice.command, width), selected ? c.operator : c.model]]));
        block.add(this.line([bar, [clip(choice.detail.replace(/\s*\n\s*/g, " "), width), c.prose]], c.prose));
        this.paletteList.add(block);
        continue;
      }
      block.flexDirection = "row";
      block.height = space.bar;
      const fg = choice.color ?? c.bright;
      block.add(this.text(`${selected ? glyph.pointer : " "} `, c.operator, { attributes: bold }));
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
            c.bright,
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
              selected ? c.bright : c.prose,
              { truncate: true, flexShrink: 1 },
            ),
            () => choice.detail.replace(/\s*\n\s*/g, " "),
          ),
        );
      this.paletteList.add(block);
    }
    if (!this.filtered.length && this.paletteInput?.value)
      this.paletteList.add(this.inset(space.between, this.text("Nothing matches this filter.", c.prose)));
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
    this.composer?.focus();
  }
  /** The keys that this terminal sends, what each mark means, and the commands. */
  help(): void {
    const legend: [string, RGBA, string][] = [
      [`${spin(0)} Working`, c.operator, "A model or a command runs"],
      [`${glyph.running} Running`, c.operator, "A chain or a session has work in progress"],
      [`${glyph.asks} Input needed`, c.warm, "A question waits for your answer"],
      [`${glyph.held} Paused`, c.warm, "Work waits until you wake it"],
      [`${glyph.done} Done`, c.done, "The act ended and gave its value"],
      [
        `${glyph.dot} Finished, unread`,
        c.done,
        "A chain you started, or a session, ended while you were away",
      ],
      [`${glyph.failed} Failed`, c.warm, "The act raised, or the gate refused its Python"],
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
    layer(() => bare() && Boolean(this.tree), "tree", [
      ...Object.entries(moves).map(
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
          run: () => this.stepChoice(moves[on] ?? 0),
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
  /** Ctrl+C closes a dialog or the rewind tree, or clears the input, or cancels the work of the chain. */
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
    return !w.paused && w.activity.some((act) => act.kind === "thread" && !act.done && !asksOperator(act));
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
    page: (key) => this.changePage(key?.name === "pageup" ? -1 : 1),
    pause: this.escape,
    help: () => this.help(),
    quit: () => void this.options.quit(),
  };
  /** When an action of the table acts, where it does not act at all times: a key that does not act goes on to the
   * input, and the footer does not offer it. */
  private readonly when: Partial<Record<Action, () => boolean>> = {
    answer: () => Boolean(this.session.operatorThread),
    page: () => this.session.view === "changes",
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
        return act && w.isUserThread(act) && !asksOperator(act);
      })
      .map((node) => node.y - top);
    const offset = step > 0 ? messages.find((at) => at > 0) : messages.findLast((at) => at < 0);
    if (offset !== undefined) this.scroll.scrollBy(offset);
  }
  dispose(): void {
    clearInterval(this.tick);
    this.closed = true;
    this.keepDraft();
    this.session.scrolls[this.lastView] = this.place;
    this.renderer.off("frame", this.laidOut);
    this.session.save();
    if (this.redraw) clearTimeout(this.redraw);
    if (this.hoverTimer) clearTimeout(this.hoverTimer);
    this.session.off("change", this.schedule);
    this.options.workspaces.off("change", this.schedule);
    this.session.off("compose", this.compose);
    this.session.off("resume", this.resume);
    this.session.off("shared", this.shared);
    for (const off of this.layersOff) off();
    this.renderer.off("resize", this.render);
    this.renderer.off("selection", this.copySelection);
    this.root.destroyRecursively();
    this.style.destroy();
  }
  private changePage(step: number): void {
    this.session.changePage = Math.max(
      0,
      Math.min(Math.ceil(this.session.host.changes / 20) - 1, this.session.changePage + step),
    );
    void this.session.refresh().catch(this.session.fail);
  }
}

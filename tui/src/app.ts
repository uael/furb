import { join } from "node:path";
import { actorParts, imageContent, imagePath, imageReferences, shapes } from "@furb/engine";
import { display, safeText } from "@furb/engine/world";
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
  LineNumberRenderable,
  MarkdownRenderable,
  type MouseEvent,
  type Renderable,
  RGBA,
  ScrollBoxRenderable,
  StyledText,
  TextareaRenderable,
  type TextChunk,
  TextRenderable,
} from "@opentui/core";
import { clipboardImage } from "./clipboard.ts";
import { commands } from "./commands.ts";
import { conversation } from "./conversation.ts";
import { externalEditor, openFile } from "./editor.ts";
import { shortenHome, shortenHomes } from "./files.ts";
import { clip, count, dollars, elapsed, graphemes, kibibytes } from "./format.ts";
import { chords, keys } from "./keys.ts";
import { loadParsers } from "./parsers.ts";
import {
  type ActRow,
  cancelled,
  failed,
  type Scroll,
  type Session,
  type SessionStatus,
  statusLabels,
  type View,
  views,
  working,
} from "./session.ts";
import { publishShare } from "./share.ts";
import {
  theme as c,
  defaultTheme,
  frame,
  glyph,
  palettes,
  setTheme,
  spacing as space,
  spin,
  syntax,
  type ThemeName,
  themeLabels,
} from "./theme.ts";
import { bold, italic, lineCounts, logo, mix, type Part, plain, styled } from "./ui.ts";
import type { SessionEntry, Workspace, Workspaces } from "./workspaces.ts";

const exitNotice = "Press ⌃D again to exit.";
const rewindNotice = "Press Escape again to rewind.";
/** The time within which a second Escape rewinds. */
const twice = 800;
/** The name of each view as the toggle and the commands show it. */
const viewLabels: Record<View, string> = { feed: "Feed", transcript: "Transcript", changes: "Changes" };
/** Whether an act is a question to the operator: a prompt whose actor is the operator. */
const title = (text: string) => text.charAt(0).toUpperCase() + text.slice(1);
const asksOperator = (act: ActRow) => act.kind === "prompt" && act.words[2] === "operator";
/** A span of seconds as a wait says it: 1 second, 0.2 seconds. */
const seconds = (value: number) => `${value} ${value === 1 ? "second" : "seconds"}`;
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
  prompt?: boolean;
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
  toggle?: () => void;
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
  sessions?: () => Promise<Choice[]>;
  newSession?: () => Promise<void>;
  workspaces?: Workspaces;
}

/** The commands of the TUI whose first argument takes a value that the TUI knows, which the suggestions offer as it
 * is typed. A command of an extension that says its values joins them. */
const valued = new Set([
  "model",
  "effort",
  "shape",
  "theme",
  "image",
  "workspace",
  "edit",
  "pause",
  "wake",
  "cancel",
  "close",
  "extensions",
]);
/** The commands of the TUI whose value is a path of the project, which the suggestions wait for. A command of an
 * extension whose value is a path joins them. */
const pathCommands = new Set(["image"]);
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
  private readonly folds = new Map<string, boolean>();
  /** The sidebar at the right: the session, its chains and its usage, then the workspaces, which scroll. */
  private readonly rail: BoxRenderable;
  private readonly railSession: BoxRenderable;
  private readonly railHeading: BoxRenderable;
  private readonly railSpaces: ScrollBoxRenderable;
  private readonly railUsage: BoxRenderable;
  /** The workspaces whose archived sessions the sidebar lists, by their folder. */
  private readonly showArchived = new Set<string>();
  /** Whether the sidebar lists the finished chains, which fold under a row of their own. */
  private showResting = false;
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
  private readonly composeEdges: BoxRenderable[] = [];
  private readonly composeBox: BoxRenderable;
  private readonly queueBox: BoxRenderable;
  private readonly imageBox: BoxRenderable;
  /** The suggestions for a `/command` or an `@path` typed in the input, drawn above it while the input keeps focus. */
  private readonly suggestionBox: BoxRenderable;
  private suggestions: Suggestion[] = [];
  private suggestionIndex = 0;
  /** The token whose suggestions Escape hid, which a change of the token shows again. */
  private dismissed = "";
  /** The read of the project files an `@` suggests from, which each `@` that starts a word asks afresh. */
  private files?: { read: Promise<string[]>; paths?: string[]; error?: string };
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
      title?: () => Part[];
      marker: Part[];
      key: string;
      compact: boolean;
      collapsible: boolean;
      state: string;
      closed: boolean;
      /** The act that the card shows. */
      act?: string;
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
    setTheme(session.theme);
    for (const [id, closed] of Object.entries(session.folds)) this.folds.set(id, closed);
    this.theme = session.theme;
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
    const top = this.box({ height: space.bar, flexDirection: "row", gap: space.between });
    this.headline = this.box({
      height: space.bar,
      flexDirection: "row",
      flexGrow: 1,
      flexShrink: 1,
      minWidth: 0,
      overflow: "hidden",
    });
    top.add(this.headline);
    this.toggle = this.box({ id: "views", height: space.bar, flexDirection: "row" });
    top.add(this.toggle);
    center.add(top);
    // The filter of the view is a field of the panel, with its title, the prompt of every filter, and the key that
    // leaves it.
    this.searchRow = this.box({
      flexDirection: "row",
      height: space.bar,
      visible: false,
      marginTop: space.section,
      paddingX: space.inset,
      backgroundColor: c.panel,
    });
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
      backgroundColor: c.panel,
      focusedBackgroundColor: c.panel,
      textColor: c.text,
      focusedTextColor: c.text,
      placeholderColor: c.faint,
      cursorColor: c.accent,
    });
    this.search.on(InputRenderableEvents.INPUT, (value: string) => {
      session.search = value;
      this.renderContent();
    });
    this.searchRow.add(this.search);
    this.searchRow.add(this.text("Esc", c.faint, { onMouseUp: this.click(() => this.closeSearch()) }));
    center.add(this.searchRow);
    this.treeBar = this.box({
      id: "rewind-bar",
      flexDirection: "row",
      height: space.bar,
      visible: false,
      marginTop: space.section,
      paddingX: space.inset,
      backgroundColor: c.panel,
    });
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
      verticalScrollbarOptions: { visible: false },
      horizontalScrollbarOptions: { visible: false },
    });
    // The ScrollBar constructor resets manual visibility; set it after construction.
    this.scroll.verticalScrollBar.visible = false;
    this.scroll.horizontalScrollBar.visible = false;
    center.add(this.scroll);
    this.queueBox = this.box({
      id: "queued-follow-ups",
      height: space.bar,
      visible: false,
      flexDirection: "row",
      onMouseUp: this.click(() => this.queuePicker()),
    });
    center.add(this.queueBox);
    this.imageBox = this.box({
      id: "attached-images",
      height: space.bar,
      flexDirection: "row",
      gap: space.between,
      visible: false,
    });
    center.add(this.imageBox);
    this.suggestionBox = this.box({
      id: "suggestions",
      flexShrink: 0,
      visible: false,
      backgroundColor: c.raised,
    });
    center.add(this.suggestionBox);
    // The composer is a panel with a bar at its left, and half a row of the panel above and below its text.
    const edge = (side: "top" | "bottom") => {
      const bar = this.box({
        height: space.bar,
        border: ["left"],
        borderColor: c.accent,
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
      this.composeEdges.push(bar);
      return bar;
    };
    this.modeBox = this.box({ flexDirection: "row", height: space.bar, flexShrink: 0 });
    center.add(edge("top"));
    this.composeBox = this.box({
      id: "composer-box",
      flexShrink: 0,
      paddingLeft: space.between - space.inset,
      paddingRight: space.inset,
      border: ["left"],
      borderColor: c.accent,
      customBorderChars: { ...noBorder, vertical: glyph.bar },
      backgroundColor: c.panel,
    });
    center.add(this.composeBox);
    this.composeEdges.push(this.composeBox);
    this.composer = new TextareaRenderable(renderer, {
      id: "composer",
      flexGrow: 1,
      minHeight: 1,
      placeholder: "What would you like to build?",
      textColor: c.text,
      placeholderColor: c.faint,
      backgroundColor: c.panel,
      focusedBackgroundColor: c.panel,
      focusedTextColor: c.text,
      cursorColor: c.accent,
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
    this.meta = this.box({
      height: space.bar,
      flexDirection: "row",
      overflow: "hidden",
      flexGrow: 1,
    });
    // The line under the text holds the switch of the mode and where the input goes, a row of space under the text.
    const metaLine = this.box({
      flexDirection: "row",
      height: space.bar,
      flexShrink: 0,
      marginTop: space.section,
    });
    metaLine.add(this.modeBox);
    metaLine.add(this.text("   ", c.faint));
    metaLine.add(this.meta);
    this.composeBox.add(metaLine);
    center.add(edge("bottom"));
    const footer = this.box({ height: space.bar, flexDirection: "row", gap: space.between });
    this.status = this.whole(
      this.text("", c.muted, { height: space.bar, truncate: true, flexGrow: 1, flexShrink: 1 }),
      () => this.statusWhole,
    );
    footer.add(this.status);
    this.hints = this.box({ height: space.bar, flexDirection: "row" });
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
    this.railHeading = this.box({
      flexDirection: "row",
      height: space.bar,
      marginTop: space.section,
      paddingX: space.between,
    });
    this.rail.add(this.railHeading);
    this.railSpaces = new ScrollBoxRenderable(renderer, {
      id: "workspaces",
      flexGrow: 1,
      flexShrink: 1,
      minHeight: 0,
      scrollX: false,
      backgroundColor: c.panel,
      contentOptions: { paddingBottom: space.section },
      verticalScrollbarOptions: { visible: false },
      horizontalScrollbarOptions: { visible: false },
    });
    this.railSpaces.verticalScrollBar.visible = false;
    this.railSpaces.horizontalScrollBar.visible = false;
    this.rail.add(this.railSpaces);
    // The usage of the chain stands at the foot of the sidebar, under a rule, where the list above it does not move it.
    this.railUsage = this.box({
      flexShrink: 0,
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
    options.workspaces?.on("change", this.schedule);
    session.on("compose", this.compose);
    session.on("resume", this.resume);
    session.on("shared", this.shared);
    renderer.keyInput.on("keypress", this.key);
    renderer.on("resize", this.render);
    renderer.on("selection", this.copySelection);
    // A label that moves with time is read again, and no other part of the view is drawn again.
    this.tick = setInterval(() => {
      for (const card of this.cards.values())
        if (card.title) this.label(card, [...card.marker, ...card.title()]);
      if (this.statusMoves) this.renderStatus();
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
    if (session.world.pending.size) this.resume();
  }

  private box(options: BoxOptions = {}): BoxRenderable {
    return new BoxRenderable(this.renderer, { flexDirection: "column", flexShrink: 0, ...options });
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
  /** The text that a drag selected goes to the clipboard as the drag ends. */
  private copySelection = (selection: { getSelectedText(): string } | null): void => {
    const text = selection?.getSelectedText() ?? "";
    if (this.closed || !text.trim()) return;
    this.renderer.copyToClipboardOSC52(text);
    const length = [...graphemes.segment(text)].length;
    this.session.notice = `Copied ${length} ${length === 1 ? "character" : "characters"}.`;
  };
  private text(
    content: string | Part[],
    fg = c.text,
    options: ConstructorParameters<typeof TextRenderable>[1] = {},
  ): TextRenderable {
    const node = new TextRenderable(this.renderer, {
      content: typeof content === "string" ? safeText(content) : styled(content),
      fg,
      // A text truncates only on a line it does not wrap, so a text that truncates keeps one line with an ellipsis.
      wrapMode: options.truncate ? "none" : "word",
      flexShrink: 0,
      ...options,
    });
    return options.truncate && !options.onMouseOver ? this.whole(node) : node;
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
      this.hover?.destroyRecursively();
      this.hover = undefined;
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
      if (!(await this.globalCommand(content.trim()))) {
        const woke = !content.startsWith("/") && (await this.session.wakeForInput());
        await this.session.submit(python ? `/run ${content}` : content);
        if (woke) this.session.notice = "The paused chain resumed with this message.";
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
    // A view that stands at its end stays there when a row above the input opens or the input grows, which makes the
    // view shorter.
    const end = !this.tree && this.scrollTarget === undefined && this.place === "end";
    const shown = this.lastView;
    if (w.theme !== this.theme) this.applyTheme(w.theme);
    if (this.draftKey !== w.draftKey) this.showDraft(w.draftKey);
    this.rail.visible = w.preferences.sidebar && this.renderer.width >= 100;
    this.rail.width = w.preferences.sidebarWidth;
    this.renderTop();
    const pending = w.operatorPrompt;
    const images = w.images[w.selected] ?? [];
    this.imageBox.visible = images.length > 0;
    if (this.paneChanged(this.imageBox, [images, this.theme])) {
      this.clear(this.imageBox);
      this.imageBox.add(
        this.text(
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
            height: space.bar,
            truncate: true,
            flexShrink: 1,
            onMouseUp: this.click(() => {
              this.openPalette(
                "Image attachments",
                images.map((image) => ({
                  label: image.name,
                  detail: `${image.mimeType}  ${kibibytes(image.size)}`,
                  run: () => this.imageActions(image.uri),
                })),
              );
            }),
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
        this.text(
          [
            [`${w.queueHeld ? glyph.held : glyph.ring} `, w.queueHeld ? c.warning : c.faint],
            [
              `${w.queueHeld ? "Queue held" : "Queued"} ${w.queued.length}  `,
              w.queueHeld ? c.warning : c.text,
            ],
            [first, c.muted],
          ],
          c.muted,
          { height: space.bar, truncate: true, flexGrow: 1, flexShrink: 1 },
        ),
      );
      this.queueBox.add(
        this.text(
          queued.length
            ? [
                ["↑", c.muted],
                [" takes the last back", c.faint],
              ]
            : [],
          c.faint,
          { height: space.bar },
        ),
      );
    }
    const [mode, modeColor] = w.editing
      ? ["Edit program", c.warning]
      : pending
        ? ["Answer", c.warning]
        : w.mode === "python"
          ? ["Python", c.secondary]
          : ["Prompt", c.accent];
    for (const edge of this.composeEdges) if (edge.borderColor !== modeColor) edge.borderColor = modeColor;
    this.composer.cursorColor = modeColor;
    this.composer.placeholder = w.editing
      ? "Edit this prompt's Python program"
      : pending
        ? pending.shape === "bool"
          ? "Answer yes or no"
          : `Your answer, as ${pending.shape}`
        : w.mode === "python"
          ? "Write Python. The gate reads it before it runs."
          : "Ask anything, or type / for commands";
    // The box holds the lines of the text, up to six, a row of space, and the line under the text.
    this.composeBox.height =
      Math.min(6, Math.max(space.bar, this.composer.lineCount, this.composer.lineInfo.lineSources.length)) +
      space.section +
      space.bar;
    this.renderMeta(mode, modeColor);
    this.renderStatus();
    this.renderContent();
    this.renderRail();
    this.renderSuggestions();
    if (end && this.lastView === shown && this.scrollTarget === undefined) this.scrollAfterLayout("end");
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
      const segment: TextRenderable = this.text(parts(false), c.text, {
        height: space.bar,
        ...(choice.run ? { onMouseUp: this.click(choice.run) } : {}),
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
    if (chord) box.add(this.text(`  ${chord}`, c.faint, { height: space.bar }));
  }
  /** The top line: the session and the chain at its left, each a button, and the toggle of the views at its right. */
  private renderTop(): void {
    const w = this.session;
    const changes = w.world.changes;
    const shown = this.tree ? "feed" : w.view;
    const kitty = this.renderer.capabilities?.kitty_keyboard === true;
    const directory = shortenHome(w.workingDirectory);
    // The top line spends its room in this order: the session and the chain, the switch of the views, the key of the
    // switch, then the directory. A part that finds no room is left out, and a session name that is still too long is
    // cut at its end.
    const chord = `${kitty ? "⌃" : "⌥"}1-3`;
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
    if (this.paneChanged(this.toggle, [shown, changes, kitty, hint, this.theme])) {
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
      const button = (parts: Part[], run: () => void) =>
        this.text(parts, c.text, {
          height: space.bar,
          flexShrink: 0,
          onMouseUp: this.click(run),
        });
      this.headline.add(
        button([[name, c.text, bold]], () => {
          if (this.options.workspaces) void this.workspacePicker();
          else this.openSessions();
        }),
      );
      this.headline.add(
        this.text([[` ${glyph.crumb} `, c.faint]], c.faint, { height: space.bar, flexShrink: 0 }),
      );
      this.headline.add(button([[w.label, c.muted]], () => this.chains()));
      if (folder)
        this.headline.add(
          this.text([[`   ${directory}`, c.faint]], c.faint, {
            height: space.bar,
            truncate: true,
            flexShrink: 1,
            onMouseUp: this.click(() => this.showValue("Directory", w.workingDirectory)),
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
    const name = model.includes(":") ? model.slice(model.indexOf(":") + 1) : model;
    const provider = model.includes(":") ? model.slice(0, model.indexOf(":")) : "";
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
      (pending ? gaps + Bun.stringWidth(`returns ${pending.shape}`) : 0) +
      (prompt ? gaps + Bun.stringWidth(name) : 0) +
      (shown.provider ? provider.length + 1 : 0) +
      (shown.effort ? gaps + Bun.stringWidth(`${effort} effort`) : 0) +
      (shown.shape ? gaps + Bun.stringWidth(`returns ${w.shape}`) : 0) +
      // The stash takes a gap, its word, its quotes and their space, and its key, and its preview takes the rest.
      (stash ? gaps + "stashed “”  ⌃S".length : 0);
    for (const part of ["provider", "shape", "effort", "chord"] as const)
      if (width() > room) shown[part] = false;
    const gap = () => this.meta.add(this.text("   ", c.faint));
    const button = (parts: Part[], run?: () => void) =>
      this.meta.add(
        this.text(parts, c.muted, {
          height: space.bar,
          flexShrink: 0,
          ...(run ? { onMouseUp: this.click(run) } : {}),
        }),
      );
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
          () => this.shapes(),
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
  private renderStatus(): void {
    const w = this.session;
    // A refresh follows each fact, so the footer says Loading only while the chain has nothing to show yet.
    const loading = w.loading && !w.turns.length;
    const status = loading ? "opening" : w.status(w.selected);
    const moving = status === "working" || status === "opening";
    this.statusMoves = moving;
    const [mark, color] = moving
      ? [spin(), c.accent]
      : status === "blocked" || status === "paused"
        ? [status === "blocked" ? glyph.asks : glyph.held, c.warning]
        : status === "error"
          ? [glyph.failed, c.danger]
          : status === "done"
            ? [glyph.dot, c.success]
            : [glyph.ring, c.faint];
    const label = loading
      ? "Loading"
      : status === "error"
        ? w.error
          ? "The view could not load"
          : "The last act failed"
        : statusLabels[status];
    const row = this.treeRow();
    type Key = readonly [chord: string, action: string, run?: () => void];
    const keys: readonly Key[] = this.tree
      ? [
          ["↑↓", "move"],
          ...(row?.parent ? ([["←→", "fold"]] as const) : []),
          ["Enter", row?.hint ?? "choose", () => void this.chooseTreeRow()],
          ["Esc", "back", () => this.closeTree()],
        ]
      : this.suggestionBox.visible
        ? [
            ["↑↓", "choose"],
            ["Tab", "complete"],
            ["Esc", "hide", () => this.dismissSuggestions()],
          ]
        : this.session.editing
          ? [
              ["Enter", "run", () => void this.submit()],
              ["Esc", "leave the program", () => this.leaveEdit()],
            ]
          : [
              ...(w.paused ? ([["/wake", "resume", () => this.action("/wake")]] as const) : []),
              ...(w.operatorPrompt ? ([["⌃A", "answer", () => this.question()]] as const) : []),
              ...(!w.paused &&
              w.activity.some((act) => act.kind === "prompt" && !act.done && !asksOperator(act))
                ? ([["Esc", "pause", () => this.action("/pause")]] as const)
                : []),
              ["⌃P", "commands", () => this.palette()],
              ["F1", "help", () => this.help()],
            ];
    const hints = keys.flatMap(([key, action], index): Part[] => [
      [index ? "   " : "", c.faint],
      [key, c.muted],
      [` ${action}`, c.faint],
    ]);
    const notice = w.error ? "" : w.notice;
    // The state stays in the footer, and a notice stands after it, cut at its end where the footer has no room for it
    // beside the keys.
    const state: Part[] = [
      [`${mark} `, color],
      [label, moving ? c.text : c.muted],
    ];
    const room =
      this.feedWidth - Bun.stringWidth(plain(hints)) - Bun.stringWidth(plain(state)) - space.between * 2;
    this.statusWhole = notice ? `${plain(state)}   ${notice}` : plain(state);
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
    for (const [index, [chord, action, run]] of keys.entries())
      this.hints.add(
        this.text(
          [
            [index ? "   " : "", c.faint],
            [chord, c.muted],
            [` ${action}`, c.faint],
          ],
          c.faint,
          { height: space.bar, ...(run ? { onMouseUp: this.click(run) } : {}) },
        ),
      );
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
      if (this.scroll.getChildren()[index] !== prior.node) this.scroll.add(prior.node, index);
      return;
    }
    prior?.node.destroyRecursively();
    const box = this.box({
      id,
      gap: space.stack,
      flexShrink: 0,
      marginTop: margin,
      paddingLeft: options.indent ?? 0,
    });
    const labelNode = this.text(heading, c.muted, {
      height: space.bar,
      truncate: true,
      visible,
      onMouseDown: (event) => {
        if (event.button === 2 && options.act)
          this.actActions(this.session.acts.find((act) => act.id === options.act?.id) ?? options.act);
      },
      onMouseUp: this.click(() => {
        this.folds.set(state, !closed);
        this.renderContent();
      }),
    });
    box.add(labelNode);
    if (!closed) body(box);
    else if (!rung) options.preview?.(box);
    this.scroll.add(box, index);
    const card = {
      key,
      node: box,
      heading: labelNode,
      label: "",
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
  /** A panel in a card: a bar of a color at its left, and half a row of the panel above and below what it holds. */
  private panel(box: BoxRenderable, color: RGBA): BoxRenderable {
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
    });
    box.add(edge("top"));
    box.add(inner);
    box.add(edge("bottom"));
    return inner;
  }
  private code(content: string): CodeRenderable {
    const code = new CodeRenderable(this.renderer, {
      content: safeText(content),
      filetype: "python",
      syntaxStyle: this.style,
      wrapMode: "word",
      drawUnstyledText: true,
    });
    const nameAt = (x: number, y: number) => {
      const { line, column } = this.sourcePoint(code, content, x, y);
      return [...line.matchAll(/[\p{L}_][\p{L}\p{N}_]*/gu)].find(
        (match) =>
          column >= Bun.stringWidth(line.slice(0, match.index)) &&
          column < Bun.stringWidth(line.slice(0, match.index + match[0].length)),
      )?.[0];
    };
    code.onMouseMove = (event) => {
      if (this.hoverTimer) clearTimeout(this.hoverTimer);
      const name = nameAt(event.x, event.y);
      if (name)
        this.hoverTimer = setTimeout(() => {
          void this.showHover(name, event.x, event.y);
        }, 220);
    };
    code.onMouseOut = () => {
      if (this.hoverTimer) clearTimeout(this.hoverTimer);
      this.hover?.destroyRecursively();
      this.hover = undefined;
    };
    code.onMouseDown = (event) => {
      const name = nameAt(event.x, event.y);
      if (name && (event.modifiers.ctrl || event.modifiers.alt)) this.inspect(name);
    };
    return code;
  }
  private sourcePoint(node: CodeRenderable | TextRenderable, content: string, x: number, y: number) {
    const row = y - node.y;
    const source = node.getLineSources(row, 1)[0] ?? row;
    const info = node.lineInfo;
    const first = row - (info.lineWraps[row] ?? 0);
    return {
      line: content.split("\n")[source] ?? "",
      column: x - node.x + (info.lineStartCols[row] ?? 0) - (info.lineStartCols[first] ?? 0),
    };
  }
  private markdown(content: string, fg = c.prose): MarkdownRenderable {
    return new MarkdownRenderable(this.renderer, {
      content: safeText(content),
      syntaxStyle: this.style,
      fg,
    });
  }

  /** Whether a card is folded: by the operator's click, or else as its kind starts. A rung that runs, or that failed
   * with no rung that took its place, stands open, and any other rung starts folded when the preference says so. */
  private folded(state: string, rung?: ActRow, compact = false): boolean {
    return (
      this.folds.get(state) ??
      (rung
        ? this.session.preferences.foldRungs && !working(rung) && (!failed(rung) || Boolean(this.retry(rung)))
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
    if (this.tree) {
      this.renderTree();
      return;
    }
    const existing = new Set(this.cards.keys());
    const used = new Map<string, number>();
    let order = 0,
      group = "";
    const add = (
      id: string,
      key: string,
      label: Part[],
      body: (box: BoxRenderable) => void,
      options: BlockOptions = {},
    ) => {
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
    const matches = (text: string) => !w.search || text.toLowerCase().includes(w.search.toLowerCase());
    // The label of an act that works moves with time, so the tick reads it again.
    const moving = (act?: ActRow, indent = 0) =>
      act && working(act) ? () => this.actLabel(act, false, indent) : undefined;
    // An act that a rung made stands under that rung.
    const under = (act?: ActRow) => (act && act.by !== "operator" ? space.between : 0);
    if (w.preferences.notice)
      add("preferences-notice", w.preferences.notice, [], (box) => {
        const panel = this.panel(box, c.warning);
        panel.add(
          this.text(w.preferences.notice, c.text, {
            onMouseUp: this.click(() => {
              w.preferences.notice = "";
              this.renderContent();
            }),
          }),
        );
      });
    if (w.error)
      add("view-error", `${w.view}:${w.error}`, [], (box) => {
        const panel = this.panel(box, c.danger);
        panel.add(
          this.text([
            [`${glyph.failed} `, c.danger],
            [`Error in the ${viewLabels[w.view].toLowerCase()} view`, c.text, bold],
          ]),
        );
        panel.add(this.inset(space.between, this.text(w.error, c.muted)));
        const retry = this.box({
          flexDirection: "row",
          marginTop: space.section,
          paddingLeft: space.between,
        });
        retry.add(
          this.hoverable(
            this.text([["Refresh view", c.accent, bold]], c.accent, {
              onMouseUp: this.click(() => {
                w.error = "";
                void w.refresh().catch(w.fail);
              }),
            }),
          ),
        );
        retry.add(this.text("  reads the view again", c.faint));
        panel.add(retry);
      });
    let items = 0;
    const named = new Set<string>();
    if (w.view === "feed") {
      const listed = conversation(w.turns, w.acts, w.world.parts, w.played);
      // An act that no turn tells yet, such as a command that the operator started, stands at the end of the feed
      // until a turn tells it.
      const told = new Set(
        listed.flatMap((item) => (item.type === "python" ? [item.rung?.id] : [item.act?.id])).filter(Boolean),
      );
      // A rung whose model has not begun to write has no stream yet, and its card waits as the card of a stream does.
      const writing = [...w.world.streams].filter(([, stream]) => stream.chain === w.selected);
      for (const act of w.activity)
        if (
          act.kind === "rung" &&
          working(act) &&
          !told.has(act.id) &&
          !w.program[act.id] &&
          !w.world.streams.has(act.id)
        )
          writing.push([act.id, { chain: w.selected, text: "", thinking: "" }]);
      const waiting = new Set(writing.map(([id]) => id));
      // An act that no turn tells yet stands where it came in time, among the items that the turns tell, so the
      // feed never moves a card once it shows it.
      const position = new Map(w.activity.map((act, index) => [act.id, index]));
      const untold = w.activity
        // A rung that a cancel ended before it wrote a word shows nothing, since its message says the cancel.
        .filter(
          (act) =>
            !told.has(act.id) &&
            !waiting.has(act.id) &&
            this.isPoint(act) &&
            !(act.kind === "rung" && cancelled(act) && !w.program[act.id]),
        )
        .map((act) => ({
          at: position.get(act.id) ?? Number.POSITIVE_INFINITY,
          item: (w.isUserPrompt(act) || asksOperator(act)
            ? { type: "prompt", key: act.id, act }
            : { type: "act", key: act.id, act }) as (typeof listed)[number],
        }));
      if (untold.length) {
        const merged: typeof listed = [];
        let next = 0;
        for (const item of listed) {
          const id = item.type === "python" ? item.rung?.id : item.act?.id;
          const at = id === undefined ? undefined : position.get(id);
          while (at !== undefined && next < untold.length && (untold[next]?.at ?? 0) < at)
            merged.push(untold[next++]?.item as (typeof listed)[number]);
          merged.push(item);
        }
        while (next < untold.length) merged.push(untold[next++]?.item as (typeof listed)[number]);
        listed.splice(0, listed.length, ...merged);
      }
      for (const item of listed) {
        if (item.type === "python") {
          const { code, rung } = item;
          if (!matches(code)) continue;
          items++;
          // A word of the operator that only made a chain, as a branch or a new chain does, reads as the chain that it
          // made, and a click on it opens that chain.
          const made =
            rung?.by === "operator" && /^\s*chain\(/.test(code)
              ? w.chains.find((chain) => chain.by === rung.id)
              : undefined;
          if (made && rung) {
            const name = w.labelOf(made.id);
            // The branch itself reads the same word as the point where it starts.
            const here = made.id === w.selected;
            add(rung.id, `made:${made.id}:${name}:${here}:${this.theme}`, [], (box) =>
              box.add(
                here
                  ? this.text([
                      ["↳ ", c.secondary],
                      ["This branch starts here", c.muted],
                    ])
                  : this.hoverable(
                      this.text(
                        [
                          ["↳ ", c.secondary],
                          [made.words[1] ? "Branched to " : "Started the chain ", c.muted],
                          [name, c.text, bold],
                        ],
                        c.text,
                        { onMouseUp: this.click(() => void w.select(made.id).catch(w.fail)) },
                      ),
                    ),
              ),
            );
            continue;
          }
          const closed = this.folded(rung?.id ?? item.key, rung);
          // The card of the word of a rung is named by the rung, so that a jump to the rung finds it.
          const id = rung && !named.has(rung.id) ? rung.id : item.key;
          named.add(id);
          add(
            id,
            `${code}\n${rung?.run?.reason ?? ""}`,
            rung ? this.actLabel(rung, closed, 0, code) : [["Python", c.muted, bold]],
            (box) => this.word(box, code, rung?.run?.reason),
            { collapsible: true, act: rung, title: moving(rung) },
          );
        } else if (item.type === "prompt" && asksOperator(item.act)) {
          const { act } = item;
          const message = String(act.words[1] ?? "");
          if (!matches(message)) continue;
          items++;
          const waiting = w.world.prompts.has(act.id);
          add(
            item.key,
            `${message}\n${waiting}`,
            [
              [`${glyph.asks} `, waiting ? c.warning : c.faint],
              ["Question", c.text, bold],
              [`  ${act.by === "operator" ? "for you" : `from ${act.by}`}`, c.faint],
            ],
            (box) => {
              this.panel(box, waiting ? c.warning : c.border).add(this.text(message));
              if (waiting)
                box.add(
                  this.inset(
                    space.between,
                    this.text(
                      [
                        ["Answer in the input below", c.muted],
                        [act.words[0] === "bool" ? " with yes or no" : `, as ${act.words[0]}`, c.muted],
                        [", or press ", c.faint],
                        ["⌃A", c.muted],
                      ],
                      c.muted,
                      { onMouseUp: this.click(() => this.question()) },
                    ),
                  ),
                );
            },
            { group: "question", act },
          );
        } else if (item.type === "prompt") {
          const { act } = item;
          const message = String(act.words[1] ?? "");
          if (!matches(message)) continue;
          items++;
          const user = w.isUserPrompt(act);
          // A message that the chain or a rung sent is its heading when it is one short line, and stands under its
          // heading otherwise. The heading says who sent it, and to which model.
          const sent = user ? "" : this.sender(act);
          const line =
            !message.includes("\n") && Bun.stringWidth(message) + Bun.stringWidth(sent) < this.feedWidth - 8;
          const state = user ? this.promptState(act) : [];
          add(
            item.key,
            `${message}\n${line}\n${sent}\n${plain(state)}`,
            user
              ? [["You", c.muted]]
              : [
                  [`${glyph.ring} `, c.faint],
                  [line ? message : sent, c.muted, line ? bold : 0],
                  [line ? `  ${sent}` : "", c.faint],
                ],
            (box) => {
              // A message shows as the operator typed it, with each image it attached named by a mark that opens it.
              // Its first line says at its right the type of the answer that it asks for, and where it stands.
              if (user) {
                const row = this.box({ flexDirection: "row" });
                const text = this.message(message);
                text.flexGrow = 1;
                text.flexShrink = 1;
                row.add(text);
                row.add(this.text(state, c.faint, { flexShrink: 0, marginLeft: space.between }));
                this.panel(box, c.accent).add(row);
              } else if (!line) {
                const note = this.box({ paddingLeft: space.between });
                note.add(this.markdown(message, c.muted));
                box.add(note);
              }
            },
            { group: user ? "user" : "sent", act, heading: user ? false : undefined },
          );
        } else if (item.type === "result" && asksOperator(item.act)) {
          const { act } = item;
          const value = typeof act.value === "boolean" ? (act.value ? "yes" : "no") : display(act.value);
          if (!matches(value)) continue;
          items++;
          const line = !value.includes("\n") && Bun.stringWidth(value) < this.feedWidth - 20;
          add(
            item.key,
            `${value}\n${line}`,
            [
              [`${glyph.done} `, c.success],
              ["You answered", c.text, bold],
              [line ? `  ${value}` : "", c.muted],
            ],
            (box) => {
              if (!line) box.add(this.inset(space.between, this.markdown(value)));
            },
            { group: "question", act, heading: true },
          );
        } else if (item.type === "result") {
          const { act } = item;
          const value = display(act.value);
          if (!matches(value)) continue;
          items++;
          add(
            item.key,
            value,
            this.answerLabel(act, item.parallel),
            (box) => {
              const answer = this.box({ paddingLeft: space.between });
              answer.add(this.markdown(value));
              box.add(answer);
            },
            { group: item.parallel ? `assistant:${act.id}` : "assistant", act },
          );
        } else if (item.type === "act") {
          const { act } = item;
          if (!matches(JSON.stringify(act))) continue;
          items++;
          const indent = under(act);
          add(
            item.key,
            JSON.stringify(act),
            this.actLabel(act, false, indent),
            (box) => this.actDetails(box, act),
            {
              compact: true,
              group: "tools",
              preview: this.actPreview(act),
              act,
              title: moving(act, indent),
              indent,
            },
          );
        } else {
          const { label, detail, body, act } = item;
          const text = [act?.id, label, detail, body].filter(Boolean).join("\n");
          if (!matches(text)) continue;
          // The line of the words that the World played is no work of the chain, so the feed of a new chain still
          // welcomes the operator.
          if (!item.played) items++;
          const danger = ["raised", "refused"].includes(label);
          const indent = act ? under(act) : space.between;
          add(
            item.key,
            text,
            [
              [`${danger ? glyph.failed : glyph.done} `, danger ? c.danger : c.success],
              [label, danger ? c.danger : c.text, bold],
              [
                `  ${this.preview(detail || (act ? act.id : ""), Bun.stringWidth(label) + indent + 8)}`,
                c.muted,
              ],
            ],
            (box) => {
              const details = this.box({ paddingLeft: space.between * 2 });
              if (act) details.add(this.reference(act.id, act.id));
              // A note whose header word names a path, as a read and a write do, makes the path a reference.
              if (!act && detail && this.session.world.parts.paths.has(label))
                details.add(this.reference(detail, detail));
              else if (detail) details.add(this.text(detail, c.muted));
              if (body) details.add(this.text(body, danger ? c.danger : c.text));
              box.add(details);
            },
            {
              compact: true,
              group: "tools",
              act,
              indent,
              ...(label === "refused"
                ? { preview: (box: BoxRenderable) => this.excerpt(box, body, false, c.danger) }
                : {}),
            },
          );
        }
      }
      for (const [id, stream] of writing) {
        const act = w.acts.find((act) => act.id === id);
        items++;
        const label = (): Part[] =>
          act
            ? this.actLabel(act)
            : [
                [`${spin()} `, c.accent],
                ["writing", c.text, bold],
                [`  ${this.progress(id)}`, c.accent],
              ];
        add(
          `stream-${id}`,
          stream.text + stream.thinking,
          label(),
          (box) => {
            const inner = this.box({ paddingLeft: space.between * 2 });
            if (stream.thinking) inner.add(this.text(stream.thinking, c.faint, { attributes: italic }));
            // The words that stream have no color yet, since half a string reads as code, and a cursor ends them.
            if (stream.text)
              inner.add(
                this.text([
                  [stream.text, c.text],
                  [glyph.mark, c.accent],
                ]),
              );
            // A model that has said nothing yet is waited for, and the card says so.
            if (!stream.thinking && !stream.text)
              inner.add(
                this.text("Waiting for the first words of the model", c.faint, { attributes: italic }),
              );
            box.add(inner);
          },
          { act, collapsible: true, title: label },
        );
      }
      // A paused chain says so at the end of its feed, with the reason that the last failure gave, and a button that
      // resumes it, since nothing new runs on it until then.
      if (w.paused && !w.search) {
        const failure = w.activity.findLast((act) => act.kind === "rung" && act.run?.status === "failed")?.run
          ?.reason;
        add("paused", `paused:${failure ?? ""}:${this.theme}`, [], (box) => {
          const panel = this.panel(box, c.warning);
          panel.add(
            this.text([
              [`${glyph.held} `, c.warning],
              ["This chain is paused", c.text, bold],
              ["  New work waits until it resumes.", c.muted],
            ]),
          );
          if (failure)
            panel.add(
              this.inset(
                space.between,
                this.whole(
                  this.text(clip(shortenHomes(failure).split("\n")[0] ?? "", this.feedWidth - 8), c.danger),
                  () => shortenHomes(failure),
                ),
              ),
            );
          const row = this.box({
            flexDirection: "row",
            marginTop: space.section,
            paddingLeft: space.between,
          });
          row.add(
            this.hoverable(
              this.text([["Resume", c.accent, bold]], c.accent, {
                onMouseUp: this.click(() => this.action("/wake")),
              }),
            ),
          );
          row.add(this.text("  runs what waits, once the cause is fixed", c.faint));
          panel.add(row);
        });
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
        if (!matches(text)) return;
        items++;
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
          { group: turn[0] },
        );
      });
    } else if (w.view === "changes") {
      if (w.world.changes > 20)
        add("change-pages", String(w.changePage), [], (box) => {
          const row = this.box({ flexDirection: "row", gap: space.between, height: space.bar });
          row.add(
            this.text(
              `Writes ${w.changePage * 20 + 1} to ${Math.min((w.changePage + 1) * 20, w.world.changes)} of ${w.world.changes}`,
              c.muted,
            ),
          );
          for (const [label, step] of [
            ["Previous page", -1],
            ["Next page", 1],
          ] as const)
            row.add(
              this.hoverable(
                this.text([[label, c.accent]], c.accent, {
                  onMouseUp: this.click(() => this.changePage(step)),
                }),
              ),
            );
          box.add(row);
        });
      const root = w.world.directory;
      for (const [index, change] of w.changes.entries())
        if (matches(change.path)) {
          items++;
          // A change never changes once it is written, so its position in the life keys its card.
          const position = String(w.changePage * 20 + index);
          const { added, removed } = lineCounts(change.patch);
          const path = change.path.startsWith(`${root}/`) ? change.path.slice(root.length + 1) : change.path;
          // The heading of a change is a bar of the panel: the path, what the write did to the file, and the lines it
          // added and removed at its right.
          const [what, color] = !change.before
            ? ["created", c.success]
            : !change.after
              ? ["deleted", c.danger]
              : ["modified", c.warning];
          const counts = `+${added} -${removed} `;
          const fill = Math.max(
            1,
            this.feedWidth - Bun.stringWidth(` ${glyph.dot} ${path}  ${what}`) - Bun.stringWidth(counts),
          );
          add(
            `change-${position}`,
            position,
            [
              [` ${glyph.dot} `, color, 0, c.panel],
              [path, c.text, bold, c.panel],
              [`  ${what}`, c.muted, 0, c.panel],
              [" ".repeat(fill), c.text, 0, c.panel],
              [`+${added}`, added ? c.success : c.faint, 0, c.panel],
              [` -${removed} `, removed ? c.danger : c.faint, 0, c.panel],
            ],
            (box) =>
              box.add(
                new DiffRenderable(this.renderer, {
                  diff: change.patch,
                  // Two sides need room for two lines of code side by side, and one side reads better below that.
                  view: this.feedWidth >= 160 ? "split" : "unified",
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
              ),
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
      add("view-loading", `${w.view}:${height}`, [], (box) => this.centered(box, height, [loading()]), {
        title: loading,
        heading: false,
      });
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
        this.centered(box, height, [
          [
            [`${glyph.ring} `, c.faint],
            [title, c.text, bold],
          ],
          [[hint, c.muted]],
        ]),
      );
    }
    for (const id of existing) {
      this.cards.get(id)?.node.destroyRecursively();
      this.cards.delete(id);
    }
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
    return this.text(
      parts,
      c.text,
      first ? { onMouseUp: this.click(() => this.imageActions(first.uri)) } : {},
    );
  }
  /** Lines in the middle of the feed, which a view with nothing to show says. */
  private centered(box: BoxRenderable, height: number, lines: Part[][]): void {
    const frame = this.box({
      minHeight: Math.max(0, height - space.section * 2),
      justifyContent: "center",
      alignItems: "center",
      gap: space.stack,
    });
    for (const line of lines) frame.add(this.text(line, c.muted));
    box.add(frame);
  }
  /** The screen of a feed that has no turn yet: the logo of furb, what it is, where it works, where to start, and the
   * keys to know. */
  private welcome(box: BoxRenderable, height: number): void {
    const w = this.session;
    const frame = this.box({
      minHeight: Math.max(0, height - space.section * 2),
      justifyContent: "center",
      alignItems: "center",
    });
    const width = Math.min(72, this.feedWidth);
    const column = this.box({ width, alignItems: "center" });
    // The logo shades from the accent to the color of Python, one column at a time.
    const columns = Math.max(...logo.map((line) => line.length));
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
    column.add(
      this.whole(
        this.text(
          [
            [clip(shortenHome(w.workingDirectory), 40, "end"), c.faint],
            ["   ", c.faint],
            [w.actorChoice.model.replace(/^[^:]*:/, ""), c.faint],
          ],
          c.faint,
          { truncate: true },
        ),
        () => `${shortenHome(w.workingDirectory)}   ${w.actorChoice.model.replace(/^[^:]*:/, "")}`,
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
        backgroundColor: c.background,
        border: ["left"],
        borderColor: c.border,
        customBorderChars: { ...noBorder, vertical: glyph.bar },
        onMouseUp: this.click(() => this.insert(prompt)),
        onMouseOver() {
          this.backgroundColor = c.panel;
          this.borderColor = c.accent;
        },
        onMouseOut() {
          this.backgroundColor = c.background;
          this.borderColor = c.border;
        },
      });
      card.add(this.text([[label, c.text, bold]], c.text, { truncate: true }));
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
          { onMouseUp: this.click(run) },
        ),
      );
    column.add(keys);
    frame.add(column);
    box.add(frame);
  }

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
    const preview = this.box({ flexDirection: "row", paddingLeft: space.between });
    preview.add(this.text(`${glyph.branch} `, c.faint));
    preview.add(this.text(visible, color, { flexShrink: 1 }));
    box.add(preview);
  }
  private actPreview(act: ActRow): BlockOptions["preview"] {
    if (act.run)
      return act.run.reason && !cancelled(act)
        ? (box) => this.excerpt(box, act.run?.reason ?? "", false, c.danger)
        : undefined;
    if (failed(act)) {
      const fault = act.value as { is: string; args: unknown[] };
      return (box) =>
        this.excerpt(box, `${fault.is}: ${fault.args.map(display).join(", ")}`, false, c.danger);
    }
    // The part of an act shows what it likes under its heading, as a command shows the tail of what it prints while
    // it runs and folds to its heading once it is over.
    const shown = this.session.world.parts.view(act.kind)?.preview?.(act);
    if (shown) return (box) => this.excerpt(box, shown.text, shown.tail);
    // The answer of a prompt is markdown, whose marks of a heading, of emphasis, and of code the preview leaves out.
    if (act.kind === "prompt" && act.done && act.value !== null)
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
  private actState(act: ActRow): { word: string; mark: string; color: RGBA } {
    const w = this.session;
    const running = () => ({ word: `running ${this.progress(act.id)}`, mark: spin(), color: c.accent });
    const view = w.world.parts.view(act.kind);
    if (view?.standing)
      return act.done
        ? { word: "ended", mark: glyph.ring, color: c.faint }
        : { word: "", mark: glyph.dot, color: c.accent };
    // A rung that a pause holds waits for the wake, and says so, where it would otherwise seem to run.
    if (act.kind === "rung")
      return cancelled(act)
        ? { word: "cancelled", mark: glyph.cancelled, color: c.faint }
        : act.run?.status === "failed"
          ? this.retry(act)
            ? { word: `retried as ${this.retry(act)?.id}`, mark: glyph.failed, color: c.faint }
            : { word: "failed", mark: glyph.failed, color: c.danger }
          : act.run?.status === "done"
            ? { word: "", mark: glyph.done, color: c.success }
            : act.paused
              ? { word: "waits for resume", mark: glyph.held, color: c.warning }
              : running();
    if (act.done)
      return failed(act)
        ? { word: "failed", mark: glyph.failed, color: c.danger }
        : cancelled(act)
          ? { word: "cancelled", mark: glyph.cancelled, color: c.faint }
          : { word: "", mark: glyph.done, color: c.success };
    if (w.world.pending.has(act.id)) return { word: "pending", mark: glyph.ring, color: c.faint };
    if (w.world.prompts.has(act.id)) return { word: "needs input", mark: glyph.asks, color: c.warning };
    if (act.paused && !view?.runsPaused)
      return { word: "waits for resume", mark: glyph.held, color: c.warning };
    return running();
  }
  /** The rung that took the place of a rung of a model that failed: a later rung for the same prompt, which the model
   * wrote once it read the failure, so the failure no longer stands. */
  private retry(act: ActRow): ActRow | undefined {
    if (act.kind !== "rung" || act.by === "operator" || !failed(act)) return undefined;
    const acts = this.session.acts;
    return acts.slice(acts.indexOf(act) + 1).find((other) => other.kind === "rung" && other.by === act.by);
  }
  /** The heading of an act: its state, its kind, what it is about, and the word of its state. A rung is named by its
   * id, and says its first line while it is folded. */
  private actLabel(act: ActRow, closed = false, indent = 0, code?: string): Part[] {
    const { word, mark, color } = this.actState(act);
    const observation = act.kind === "prompt" && !this.session.isUserPrompt(act);
    const view = this.session.world.parts.view(act.kind);
    const subject =
      act.kind === "prompt"
        ? String(act.words[1]).replace(observation ? / done$/ : /$^/, "")
        : view?.subject
          ? view.subject(act)
          : act.kind === "wait"
            ? seconds(Number(act.words[0]))
            : act.kind === "rung"
              ? closed
                ? (String(code || this.session.program[act.id] || act.words[0] || "").split("\n")[0] ?? "")
                : ""
              : String(act.words[0] || "");
    const name = act.kind === "rung" ? act.id : observation ? (subject.split("\n")[0] ?? "") : act.kind;
    // An open rung says who wrote it: the model of the prompt that made it, or the operator.
    const author =
      act.kind === "rung" && !closed
        ? `  by ${act.by === "operator" ? "you" : this.model(String(act.words[2] ?? "")).name}`
        : observation
          ? `  ${this.sender(act)}`
          : "";
    const tail = word ? `  ${word}` : "";
    const reserve = indent + 2 + 2 + Bun.stringWidth(name) + 2 + Bun.stringWidth(tail) + 1;
    return [
      [`${mark} `, color],
      [name, act.kind === "rung" || !failed(act) ? c.text : c.danger, bold],
      [subject && !observation ? `  ${this.preview(subject, reserve)}` : "", c.muted],
      [author, c.faint],
      [tail, color],
    ];
  }
  /** Where a message of the operator stands, which its panel says at its right: the type of the answer that it asks
   * for, and a mark and a word only when a cancel or a failure ended it, or a pause holds it. */
  private promptState(act: ActRow): Part[] {
    const shape: Part = [String(act.words[0] ?? ""), c.faint];
    const state = act.done
      ? failed(act)
        ? { word: "failed", mark: glyph.failed, color: c.danger }
        : cancelled(act)
          ? { word: "cancelled", mark: glyph.cancelled, color: c.faint }
          : undefined
      : act.paused || this.session.world.pending.has(act.id)
        ? { word: "waits for resume", mark: glyph.held, color: c.warning }
        : undefined;
    return state ? [shape, [`   ${state.mark} ${state.word}`, state.color]] : [shape];
  }
  /** The model of an actor by its name alone, with no provider, and its effort. */
  private model(actor: string): { name: string; effort: string } {
    const { model, effort } = actorParts(
      actor || this.session.actor,
      this.session.roster.map(([name]) => name),
    );
    return { name: model.includes(":") ? model.slice(model.indexOf(":") + 1) : model, effort };
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
  /** A word of Python in a card: its numbered lines, and the reason it failed. */
  private word(box: BoxRenderable, word: string, reason?: string): void {
    // The code starts under the name of its rung, past the fold and the glyph of the heading.
    const inner = this.box({ gap: space.stack, paddingLeft: space.inset });
    // A rung whose word has not come yet shows no code.
    if (word) inner.add(this.numbered(word));
    if (reason) {
      const why = this.box({ flexDirection: "row", paddingLeft: space.between });
      why.add(this.text(`${glyph.branch} `, c.faint));
      // The parser names the word as <string> and says its line twice, which the reason leaves out.
      why.add(
        this.text(shortenHomes(reason.replace(/\s*\(<string>, line \d+\)$/, "")), c.danger, {
          flexShrink: 1,
        }),
      );
      inner.add(why);
    }
    box.add(inner);
  }
  private actDetails(box: BoxRenderable, act: ActRow): void {
    if (act.kind === "rung") {
      this.word(box, String(this.session.program[act.id] || act.words[0] || ""), act.run?.reason);
      return;
    }
    const details = this.box({ paddingLeft: space.between * 2, gap: space.stack });
    box.add(details);
    const view = this.session.world.parts.view(act.kind);
    if (view?.details) {
      this.partDetails(details, act);
      return;
    }
    const fields: Record<string, readonly string[]> = {
      prompt: ["shape", "message", "actor"],
      rung: ["word", "retells", "actor", "returns"],
      wait: ["seconds"],
    };
    details.add(this.reference(act.id, act.id));
    for (const [index, value] of act.words.entries()) {
      if (value === null || value === "") continue;
      details.add(
        this.text([
          [`${(view?.fields ?? fields[act.kind])?.[index] ?? `argument ${index + 1}`}: `, c.faint],
          [display(value), c.muted],
        ]),
      );
    }
    if (act.done && act.value !== null)
      details.add(
        this.text(display(act.value), failed(act) ? c.danger : c.text, { marginTop: space.section }),
      );
  }
  /** What the part of an act shows of it once its card opens: its streams of text, each named when it names one and
   * in the color of a failure when it is one, and under them its name and its notes. */
  private partDetails(details: BoxRenderable, act: ActRow): void {
    const read = (row: ActRow) => this.session.world.parts.view(row.kind)?.details?.(row);
    const { streams = [], notes = [] } = read(act) ?? {};
    const shown: TextRenderable[] = [];
    for (const stream of streams) {
      if (stream.name)
        details.add(
          this.text(stream.name, stream.failure ? c.danger : c.faint, {
            marginTop: shown.length ? space.section : 0,
          }),
        );
      // The line end that closes a stream opens no empty row.
      const node = this.text(stream.content.replace(/\n$/, ""), stream.failure ? c.danger : c.text);
      details.add(node);
      shown.push(node);
    }
    const meta = this.box({ flexDirection: "row", marginTop: shown.length ? space.section : 0 });
    meta.add(this.reference(act.id, act.id));
    const tones = { faint: c.faint, success: c.success, danger: c.danger };
    meta.add(
      this.text(
        notes.flatMap((note): Part[] => [
          [`   ${note.label ? `${note.label} ` : ""}`, c.faint],
          [note.text, tones[note.tone ?? "faint"]],
        ]),
        c.faint,
      ),
    );
    details.add(meta);
    // The row holds the tail of each long text of the act, and the card reads the whole of it once it opens.
    if (act.output !== undefined)
      void this.session.world.act(act.id).then((whole) => {
        const contents = (whole && read(whole)?.streams) ?? [];
        for (const [index, node] of shown.entries()) {
          const content = contents[index]?.content;
          if (!node.isDestroyed && content !== undefined) node.content = safeText(content.replace(/\n$/, ""));
        }
      }, this.report);
  }

  private numbered(word: string, findings: string[] = []): LineNumberRenderable {
    const lines = new LineNumberRenderable(this.renderer, {
      target: this.code(word),
      fg: c.faint,
      minWidth: 3,
      paddingRight: space.inset,
    });
    for (const finding of findings) {
      const line = Number(finding.match(/line (\d+)/)?.[1] ?? 0) - 1;
      if (line >= 0) {
        lines.setLineColor(line, { gutter: c.removed, content: c.removed });
        lines.setLineSign(line, { before: "!", beforeColor: c.danger });
      }
    }
    return lines;
  }

  private reference(label: string, value: string): TextRenderable {
    const node = this.text(label, c.link, { attributes: 8 });
    node.onMouseDown = () => {
      void this.follow(value).catch(this.report);
    };
    node.onMouseOver = (event) => {
      void this.referenceHover(value, event.x, event.y);
    };
    node.onMouseOut = () => {
      this.hover?.destroyRecursively();
      this.hover = undefined;
    };
    return node;
  }

  private async follow(value: string): Promise<void> {
    this.hover?.destroyRecursively();
    this.hover = undefined;
    const act = this.session.actOf(value);
    if (value.startsWith("furb-image://")) this.imageActions(value);
    else if (act?.kind === "chain") await this.session.select(act.id);
    else if (act && act.id === value && act.on === this.session.selected) this.go("feed", act.id);
    else this.showValue(value, await this.session.follow(value));
  }

  private async referenceHover(value: string, x: number, y: number): Promise<void> {
    try {
      const act = this.session.acts.find((act) => act.id === value);
      const detail = value.startsWith("furb-image://")
        ? "Image attachment. Click to open its actions."
        : act
          ? `${act.kind}  ${act.done ? display(act.value) : "pending"}`
          : await this.session.follow(value);
      if (this.closed || this.overlay) return;
      this.hover?.destroyRecursively();
      this.hover = this.box({
        position: "absolute",
        left: Math.max(1, Math.min(x, this.renderer.width - 60)),
        top: Math.max(1, Math.min(y + 1, this.renderer.height - 8)),
        width: Math.min(58, this.renderer.width - 4),
        maxHeight: 8,
        paddingX: space.between,
        paddingY: space.inset,
        backgroundColor: c.raised,
        zIndex: 30,
        onMouseDown: () => {
          void this.follow(value).catch(this.report);
        },
      });
      this.hover.add(this.text(value, c.link, { attributes: bold }));
      this.hover.add(this.text(detail.slice(0, 400), c.muted, { maxHeight: 4 }));
      this.root.add(this.hover);
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
    const node = new TextRenderable(this.renderer, {
      content: new StyledText(chunks),
      wrapMode: "word",
      flexShrink: 0,
    });
    const target = (x: number, y: number) => {
      const { line, column } = this.sourcePoint(node, text, x, y);
      const match = [...line.matchAll(/^#([\w@.]+)|furb-image:\/\/[\w.]+/g)].find(
        (match) =>
          (match[1] === undefined || this.session.actOf(match[1])) &&
          column >= Bun.stringWidth(line.slice(0, match.index)) &&
          column < Bun.stringWidth(line.slice(0, match.index + match[0].length)),
      );
      if (match) return { reference: true, value: match[1] ?? match[0] };
      const name = [...line.matchAll(/[\p{L}_][\p{L}\p{N}_]*/gu)].find(
        (match) =>
          column >= Bun.stringWidth(line.slice(0, match.index)) &&
          column < Bun.stringWidth(line.slice(0, match.index + match[0].length)),
      );
      return name ? { reference: false, value: name[0] } : undefined;
    };
    node.onMouseMove = (event) => {
      clearTimeout(this.hoverTimer);
      const value = target(event.x, event.y);
      this.hover?.destroyRecursively();
      this.hover = undefined;
      if (value)
        this.hoverTimer = setTimeout(() => {
          if (value.reference) void this.referenceHover(value.value, event.x, event.y);
          else void this.showHover(value.value, event.x, event.y);
        }, 220);
    };
    node.onMouseOut = () => {
      clearTimeout(this.hoverTimer);
      this.hover?.destroyRecursively();
      this.hover = undefined;
    };
    node.onMouseDown = (event) => {
      const value = target(event.x, event.y);
      if (value?.reference) void this.follow(value.value).catch(this.report);
      else if (value && (event.modifiers.ctrl || event.modifiers.alt)) this.inspect(value.value);
    };
    return node;
  }

  /** What a chain is doing, read off its own acts, and nothing for a chain at rest. */
  /** The state of a chain. A chain that the operator started, directly or by a word of their own, and that finished
   * its work while another chain was shown, is finished and not yet seen until the operator opens it. */
  private chainStatus(id: string): SessionStatus {
    const w = this.session;
    const acts = w.acts.filter((act) => act.on === id && !w.world.parts.hidden(act));
    const latest = acts.at(-1);
    const status: SessionStatus = acts.some((act) => w.world.prompts.has(act.id))
      ? "blocked"
      : acts.some((act) => w.world.pending.has(act.id))
        ? "paused"
        : acts.some(working)
          ? "working"
          : acts.some((act) => act.paused && !act.done)
            ? "paused"
            : latest && failed(latest)
              ? "error"
              : "idle";
    const before = w.phases.get(id);
    w.phases.set(id, status);
    const chain = w.acts.find((act) => act.id === id);
    const mine = chain?.by === "operator" || w.actOf(chain?.by ?? "")?.by === "operator";
    if (id === w.selected || status !== "idle") w.unread.delete(id);
    else if (before === "working" && mine) w.unread.add(id);
    return status === "idle" && w.unread.has(id) ? "done" : status;
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
    const added = w.world.parts.sidebar({ acts: w.acts, chain: w.selected });
    const width = w.preferences.sidebarWidth;
    if (
      !this.paneChanged(this.railSession, [
        this.theme,
        w.selected,
        w.spend,
        window,
        width,
        w.chains.map((chain) => [chain.id, w.labelOf(chain.id), this.chainStatus(chain.id)]),
        added,
        this.showResting,
      ])
    )
      return;
    this.clear(this.railSession);
    this.clear(this.railUsage);
    const inner = width - space.between * 2;
    // The chains fill the head of the sidebar, and the usage its foot.
    let target: BoxRenderable = this.railSession;
    const add = (parts: Part[], options: ConstructorParameters<typeof TextRenderable>[1] = {}) => {
      const node = this.text(parts, c.muted, { height: space.bar, truncate: true, ...options });
      target.add(node);
      return node;
    };
    // A row of a table: its name at the left, and its value at the right.
    const row = (name: string, value: string, color = c.text, note = "") => {
      const room = Math.max(1, inner - Bun.stringWidth(name + note + value));
      return add([[name, c.muted], [note, c.faint], [" ".repeat(room)], [value, color]]);
    };
    const section = (name: string, value = "", note = "", run?: () => void) => {
      const room = Math.max(1, inner - Bun.stringWidth(name + note + value));
      add([[name, c.text, bold], [note, c.faint], [" ".repeat(room)], [value, c.faint]], {
        marginTop: space.section,
        ...(run ? { onMouseUp: this.click(run) } : {}),
      });
    };
    section("Chains", "⌃B", "", () => this.chains());
    const first = this.railSession.getChildren()[0];
    if (first) first.marginTop = 0;
    // The root chain, the chain that is shown, and a chain that has something to say stand in the list, and the other
    // finished chains fold under a row of their own, which a click opens. The list takes six rows at most, and the rest
    // wait behind a button that lists them all.
    const states = new Map(w.chains.map((chain) => [chain.id, this.chainStatus(chain.id)]));
    const active = w.chains.filter(
      (chain) => chain.id === w.life.root || chain.id === w.selected || states.get(chain.id) !== "idle",
    );
    const resting = w.chains.filter((chain) => !active.includes(chain));
    const chainLine = (chain: ActRow) => {
      const selected = chain.id === w.selected;
      const status = states.get(chain.id) ?? "idle";
      const line = this.hoverable(
        this.box({
          flexDirection: "row",
          height: space.bar,
          marginX: -space.between,
          paddingX: space.inset,
          backgroundColor: selected ? c.selected : c.panel,
          onMouseUp: this.click(() => {
            void w.select(chain.id).catch(w.fail);
          }),
        }),
        selected ? c.selected : c.raised,
      );
      line.add(
        this.text(
          [
            [selected ? glyph.mark : " ", c.accent],
            [" "],
            [`${this.statusDot(status)} `, this.statusColor(status)],
            [
              w.labelOf(chain.id),
              selected ? c.text : status === "idle" ? c.faint : c.muted,
              selected ? bold : 0,
            ],
          ],
          c.muted,
          { truncate: true, flexShrink: 1 },
        ),
      );
      this.railSession.add(line);
    };
    const shown = active.length > 7 ? 6 : active.length;
    for (const chain of active.slice(0, shown)) chainLine(chain);
    if (active.length > shown)
      add(
        [
          ["   ", c.faint],
          [`${active.length - shown} more`, c.accent],
        ],
        { onMouseUp: this.click(() => this.chains()) },
      );
    if (resting.length) {
      // The row of the finished chains stands as a chain row does: its fold mark in the column of the bar of the
      // selected chain, and its name where the names of the chains start.
      const fold = this.hoverable(
        this.box({
          flexDirection: "row",
          height: space.bar,
          marginX: -space.between,
          paddingX: space.inset,
          backgroundColor: c.panel,
          onMouseUp: this.click(() => {
            this.showResting = !this.showResting;
            this.render();
          }),
        }),
        c.raised,
      );
      fold.add(
        this.text(
          [
            [this.showResting ? glyph.open : glyph.closed, c.faint],
            [" "],
            ["Finished", c.faint],
            [`  ${resting.length}`, c.faint],
          ],
          c.faint,
          { truncate: true, flexShrink: 1 },
        ),
      );
      this.railSession.add(fold);
      if (this.showResting) for (const chain of resting.slice(0, 12)) chainLine(chain);
    }
    target = this.railUsage;
    const spend = w.spend;
    const meter = added.findLast((part) => part.meter)?.meter;
    const known = window !== undefined && Number.isFinite(window);
    const tokens = spend.input + spend.output + spend.cacheRead + spend.cacheWrite;
    this.railUsage.visible = tokens > 0 || spend.dollars > 0 || added.length > 0;
    if (this.railUsage.visible) {
      section(
        "Context",
        known ? `${Number(((window ?? 0) * 100).toFixed(1))}%` : "",
        w.demo ? "  simulated" : "",
      );
      // The rule above the usage stands in for the space above a section.
      const head = this.railUsage.getChildren()[0];
      if (head) head.marginTop = 0;
      // The meter fills with the share of the window that the last answer used, and marks what a part marks on it, as
      // the ceiling of a grant. It stands empty until an answer tells the share, and a hover over it says the share
      // and what the part says of its mark.
      const part = known ? (window ?? 0) : 0;
      const used = Math.min(inner, Math.max(part > 0 ? 1 : 0, Math.round(part * inner)));
      const mark = meter === undefined ? -1 : Math.min(inner - 1, Math.round(meter.mark * inner));
      const tone = part >= 0.9 ? c.danger : part >= 0.7 ? c.warning : c.accent;
      const cells: Part[] = [];
      for (let at = 0; at < inner; at++)
        cells.push(at === mark ? ["╋", c.warning] : [glyph.meter, at < used ? tone : c.border]);
      const tip: Part[] = [
        [known ? `${Number((part * 100).toFixed(1))}%` : "No answer yet", c.text, bold],
        [known ? " of the context window" : "", c.muted],
        [meter?.tip ?? "", c.warning],
      ];
      add(cells, {
        onMouseOver: (event) => this.tip(tip, event.x, event.y),
        onMouseOut: () => {
          this.hover?.destroyRecursively();
          this.hover = undefined;
        },
      });
      // The input is the whole prompt of the last answer, its system prompt included, which the share of the window
      // measures. The output and the cache are what every answer of the chain wrote and read, each counted once.
      if (tokens > 0) {
        const input = w.context;
        if (input !== undefined) {
          const shown = row("Input", count(input));
          shown.onMouseOver = (event) =>
            this.tip(
              [
                ["The whole prompt of the last answer, system prompt included. ", c.text],
                [`${count(spend.input)} of the input of the chain was fresh.`, c.muted],
              ],
              event.x,
              event.y,
            );
          shown.onMouseOut = () => {
            this.hover?.destroyRecursively();
            this.hover = undefined;
          };
        }
        row("Output", count(spend.output));
        if (spend.cacheRead) row("Cache read", count(spend.cacheRead));
        if (spend.cacheWrite) row("Cache write", count(spend.cacheWrite));
      }
      row("Spent", dollars(spend.dollars));
      for (const one of added.flatMap((part) => part.rows ?? []))
        row(one.name, one.value, one.tone === "text" ? c.text : c.muted);
    }
  }
  /** The composer holds a text in place of the draft that the session shows, and an undo gives the draft back. */
  private insert(text: string): void {
    this.closeOverlay();
    if (this.draftKey !== this.session.draftKey) this.showDraft(this.session.draftKey);
    this.composer.replaceText(text);
    this.composer.focus();
  }
  private statusColor(status: SessionStatus): RGBA {
    return status === "blocked" || status === "paused"
      ? c.warning
      : status === "error"
        ? c.danger
        : status === "done"
          ? c.success
          : status === "working" || status === "opening"
            ? c.accent
            : c.faint;
  }
  /** The mark of a state, one shape for each: at work, waiting on the operator, paused, failed, finished and not yet
   * seen, and at rest. */
  private statusDot(status: SessionStatus): string {
    return {
      working: glyph.running,
      opening: glyph.running,
      blocked: glyph.asks,
      paused: glyph.held,
      error: glyph.failed,
      done: glyph.dot,
      idle: glyph.ring,
      saved: glyph.ring,
    }[status];
  }
  /** The workspaces in the sidebar, under the session: each folder with its sessions, which scroll on their own. */
  private renderWorkspaces(): void {
    const library = this.options.workspaces;
    this.railHeading.visible = Boolean(library);
    this.railSpaces.visible = Boolean(library);
    if (!library) return;
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
        [...this.showArchived],
        this.renaming && this.itemKey(this.renaming.item),
        this.menuItem && this.itemKey(this.menuItem),
      ])
    )
      return;
    this.clear(this.railHeading);
    this.clear(this.railSpaces);
    this.hover?.destroyRecursively();
    this.hover = undefined;
    const inner = width - space.between * 2;
    this.railHeading.add(this.text("Workspaces", c.text, { attributes: bold, flexGrow: 1 }));
    this.railHeading.add(
      this.text("⌃W", c.faint, { onMouseUp: this.click(() => void this.workspacePicker()) }),
    );
    // A row of the list spans the sidebar, so that the pointer and the selection show from edge to edge.
    const line = (parts: Part[], run: () => void, options: BoxOptions = {}, selected = false) => {
      const row = this.hoverable(
        this.box({
          flexDirection: "row",
          height: space.bar,
          paddingLeft: space.inset,
          paddingRight: space.between,
          backgroundColor: selected ? c.selected : c.panel,
          onMouseUp: this.click(run),
          ...options,
        }),
        selected ? c.selected : c.raised,
      );
      row.add(this.text(parts, c.muted, { truncate: true, flexGrow: 1, flexShrink: 1 }));
      this.railSpaces.add(row);
      return row;
    };
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
      const row = this.box({
        flexDirection: "row",
        height: space.bar,
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
      row.add(this.text(parts.lead, c.muted, { height: space.bar }));
      const state = parts.state && this.text([parts.state], c.muted, { height: space.bar });
      if (state && !parts.after) row.add(state);
      if (renaming) {
        const input = new InputRenderable(this.renderer, {
          flexGrow: 1,
          flexShrink: 1,
          value: renaming.value,
          backgroundColor: c.background,
          focusedBackgroundColor: c.background,
          textColor: c.text,
          focusedTextColor: c.text,
          cursorColor: c.accent,
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
      const name = this.text([parts.name], c.muted, {
        height: space.bar,
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
        this.text(mark, ground, {
          height: space.bar,
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
        this.hover?.destroyRecursively();
        this.hover = undefined;
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
        this.hover?.destroyRecursively();
        this.hover = undefined;
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
          state: quiet ? undefined : [this.statusDot(status), this.statusColor(status)],
          after: true,
        },
        () => [
          [`${this.statusDot(status)} `, this.statusColor(status)],
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
            this.text(clip(shortenHome(group.directory), inner - space.inset - 1, "end"), c.faint, {
              height: space.bar,
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
            state: [`${this.statusDot(entry.status)} `, this.statusColor(entry.status)],
            name: [entry.name, selected ? c.text : entry.archived ? c.faint : c.muted, selected ? bold : 0],
          },
          () => [
            [`${this.statusDot(entry.status)} `, this.statusColor(entry.status)],
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
      for (const entry of group.sessions.filter((entry) => !entry.archived)) sessionRow(entry);
      line([["   "], ["+ ", c.faint], ["New session", c.faint]], () => {
        void library.create(group).catch(this.report);
      });
      // The archived sessions fold under a row of their own, as the finished chains do, which a click opens.
      const archived = group.sessions.filter((entry) => entry.archived);
      if (archived.length) {
        const open = this.showArchived.has(group.directory);
        line(
          [
            [open ? glyph.open : glyph.closed, c.faint],
            ["  "],
            ["Archived", c.faint],
            [`  ${archived.length}`, c.faint],
          ],
          () => {
            if (open) this.showArchived.delete(group.directory);
            else this.showArchived.add(group.directory);
            this.render();
          },
        );
        if (open) for (const entry of archived) sessionRow(entry);
      }
    }
    line([[" "], ["+ ", c.faint], ["Add a workspace", c.faint]], () => this.insert("/workspace "), {
      marginTop: space.section,
    });
  }
  /** A tip that stands under the pointer and ends at its left, or above it where the rows under it cannot hold it,
   * inside the screen. It never covers the row of the pointer, or the pointer would leave the text that shows it. */
  private tip(parts: Part[], x: number, y: number): void {
    this.hover?.destroyRecursively();
    const size = Math.min(
      this.renderer.width - 2,
      Math.max(25, Bun.stringWidth(plain(parts)) + space.between),
    );
    const rows = Math.max(1, Math.ceil(Bun.stringWidth(plain(parts)) / Math.max(1, size - space.inset * 2)));
    this.hover = this.box({
      position: "absolute",
      left: Math.max(1, Math.min(x - size, this.renderer.width - size - 1)),
      top: y + 1 + rows <= this.renderer.height ? y + 1 : Math.max(0, y - rows),
      width: size,
      paddingX: space.inset,
      backgroundColor: c.raised,
      zIndex: 30,
    });
    this.hover.add(this.text(parts));
    this.root.add(this.hover);
  }
  /** Ask how to remove a session: archive it, which keeps its record under a row of its own, or move it to the trash.
   * An archived session offers to come back instead. */
  private removeSession(entry: SessionEntry): void {
    const library = this.options.workspaces;
    const group = library?.groupOf(entry);
    if (!library || !group) return;
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
          run: async () => {
            await library.delete(entry);
          },
        },
        { label: "Keep", detail: "Return without changes", run() {} },
      ],
      0,
      "",
      `The trash is ${shortenHome(join(group.directory, ".furb", "trash"))}, where you can get it back.`,
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
    if (keep && name && name !== renaming.item.name) this.options.workspaces?.rename(renaming.item, name);
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
    if (!library) return;
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
      0,
      "",
      `The folder is ${shortenHome(item.directory)}.`,
    );
  }
  /** The menu that a right click on the row of a session or a workspace opens, at the pointer. */
  private itemMenu(item: SessionEntry | Workspace, x: number, y: number): void {
    const library = this.options.workspaces;
    if (!library) return;
    const rename = { label: "Rename", detail: "", run: () => this.startRename(item) };
    const choices: Choice[] =
      "sessions" in item
        ? [
            { label: "New session", detail: "", run: () => library.create(item).then(() => {}) },
            rename,
            { label: item.collapsed ? "Unfold" : "Fold", detail: "", run: () => library.toggle(item) },
            { label: "Remove from the list", detail: "", run: () => library.remove(item) },
          ]
        : [
            ...(library.current === item
              ? []
              : [{ label: "Open", detail: "", run: () => library.select(item) }]),
            rename,
            item.archived
              ? { label: "Restore", detail: "", run: () => library.restore(item) }
              : { label: "Archive", detail: "", run: () => library.archive(item) },
            {
              label: "Move to trash",
              detail: "",
              color: c.danger,
              run: () => library.delete(item).then(() => {}),
            },
          ];
    this.openPalette(item.name, choices, 0, "", "", false, { x, y });
    this.menuItem = item;
    this.render();
  }
  /** The workspaces and their sessions in a palette, which opens once the workspaces are read again. */
  workspacePicker = (): Promise<void> => {
    const library = this.options.workspaces;
    if (!library) return Promise.resolve();
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
                detail: `${statusLabels[entry.status]}${library.current === entry ? "   current" : ""}`,
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
          0,
          "",
          "Open sessions keep running while you work in another one.",
        ),
      )
      .catch(this.report);
  };
  /** The commands that the view answers itself, by their text. */
  private async globalCommand(text: string): Promise<boolean> {
    const [name, ...words] = text.startsWith("/") ? text.slice(1).split(" ") : [];
    const argument = words.join(" ").trim();
    if (name === "extensions") {
      if (argument === "update") {
        const names = await this.session.world.fetchExtensions();
        this.session.notice = `Fetched ${names.join(", ")} again. A new session plays the change.`;
      } else if (argument) throw new Error("Use /extensions, or /extensions update.");
      else this.extensionsList();
      return true;
    }
    // A command with no argument that opens a picker, and /inspect, which opens the value it names.
    const pickers: Record<string, () => unknown> = {
      details: this.details,
      rewind: this.rewind,
      tree: this.sessionTree,
      queue: this.queuePicker,
      model: this.models,
      effort: this.effortPicker,
      shape: () => this.shapes(),
      theme: () => this.themes(),
    };
    if (name && !argument && Object.hasOwn(pickers, name)) {
      pickers[name]?.();
      return true;
    }
    if (name === "inspect" && argument) {
      await this.inspect(argument);
      return true;
    }
    if (text === "/exit") {
      await this.options.quit();
      return true;
    }
    if (text === "/editor") {
      await this.editDraft();
      return true;
    }
    if (text === "/files") {
      await this.filesPicker();
      return true;
    }
    if (text === "/new" && this.options.newSession) {
      await this.options.newSession();
      return true;
    }
    if (text === "/image" || text.startsWith("/image ")) {
      const path = text.slice(6).trim();
      if (path) await this.session.attachImage(path);
      else await clipboardImage((path) => this.session.attachImage(path));
      return true;
    }
    const library = this.options.workspaces;
    if (!library) return false;
    if (text === "/delete") {
      this.openPalette(
        "Delete a session",
        library.groups.flatMap((group) =>
          group.sessions.map((entry) => ({
            label: entry.name,
            detail: group.name,
            run: () =>
              this.openPalette(
                `Delete the session “${entry.name}”?`,
                [
                  { label: "Keep session", detail: "Return without changes", run() {} },
                  {
                    label: "Move to trash",
                    color: c.danger,
                    detail: "Stop this session and move its saved record to the trash",
                    run: async () => {
                      await library.delete(entry);
                    },
                  },
                ],
                0,
                "",
                `It goes to ${shortenHome(join(group.directory, ".furb", "trash"))}, where you can get it back.`,
              ),
          })),
        ),
      );
      return true;
    }
    if (text === "/sidebar") {
      library.toggle();
      return true;
    }
    if (text === "/workspace") {
      await this.workspacePicker();
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
    const content = imageContent(this.session.world.imageDirectory, uri);
    const file = imagePath(this.session.world.imageDirectory, uri).path;
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
      this.session.mode === "python" || Boolean(this.session.editing),
      this.session.directory || this.session.world.directory,
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
  /** The extensions that the session plays: where each stands, and the parts it has. A choice fetches them all again. */
  private extensionsList(): void {
    const world = this.session.world;
    this.openPalette("Extensions", [
      ...world.extensions.map((one) => ({
        label: one.name,
        detail: [
          one.builtin ? "builtin" : shortenHome(one.root ?? ""),
          [one.word ? "python" : "", one.world.ts || one.world.py ? "World" : "", one.tui ? "TUI" : ""]
            .filter(Boolean)
            .join(", "),
          one.requires.length ? `requires ${one.requires.join(", ")}` : "",
        ]
          .filter(Boolean)
          .join("   "),
        run: () => {},
      })),
      {
        label: "Update extensions",
        detail: "Fetch every extension again; a new session plays the change",
        command: "/extensions update",
        run: () => this.action("/extensions update"),
      },
    ]);
  }
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
      0,
      "",
      "Sent in turn after the current work. Choose one to edit or remove it.",
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
      0,
      "",
      `The page is ${shortenHome(path)}.`,
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
      const session = this.options.workspaces?.current?.session;
      if (session) session.notice = message;
    } else this.showValue("Could not complete action", message);
  };
  /** What the operator can do with an act that a right click chose. */
  private actActions(act: ActRow): void {
    const w = this.session;
    const message = w.isUserPrompt(act) && !asksOperator(act);
    this.openPalette(`${act.kind === "rung" ? "Rung" : title(act.kind)} ${act.id}`, [
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
      ...(act.kind === "prompt" && !asksOperator(act)
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
  /** Every action in one list: each with its label, the slash command that does it, what it does, and its keys. */
  palette(): void {
    const kitty = this.renderer.capabilities?.kitty_keyboard === true;
    const views_: Record<View, string> = {
      feed: "Messages, the Python each model wrote, and answers",
      transcript: "The exact text that the model reads",
      changes: "The diff of each file that the life wrote",
    };
    const shortcuts: Partial<Record<keyof typeof commands, string>> = {
      exit: "⌃Q",
      workspace: "⌃W",
      sidebar: "⌃\\",
      rewind: "Esc Esc",
      editor: "⌥E",
      files: "@",
      queue: "⌥Enter",
      image: "⌃V",
      details: "⌥D",
      chain: "⌃N",
      model: kitty ? "⌃M" : "⌥M",
      effort: "⇧Tab",
      theme: "⌃T",
      inspect: "⌃G",
      edit: "⌃L",
    };
    const choices: Choice[] = [];
    if (this.options.newSession)
      choices.push({
        label: "New session",
        detail: commands.new[2],
        command: "/new",
        run: this.options.newSession,
      });
    for (const [index, view] of views.entries())
      choices.push({
        label: `${viewLabels[view]} view`,
        detail: views_[view],
        command: "",
        keys: `${kitty ? "⌃" : "⌥"}${index + 1}`,
        run: () => this.showView(view),
      });
    for (const [name, command] of this.session.world.parts.commands)
      choices.push({
        label: command.label,
        detail: command.detail,
        command: `/${name}${command.argument ? ` ${command.argument}` : ""}`,
        keys: command.keys,
        run: this.command(name, command.argument ?? ""),
      });
    // The actions that have keys and no command stand beside the commands they go with.
    const beside: Record<string, Choice> = {
      chain: {
        label: "Switch chain",
        detail: "Go to any chain of this session",
        command: "",
        keys: "⌃B",
        run: () => this.chains(),
      },
      run: {
        label: "Python input",
        detail: "Write code with the same gate as the model",
        command: "",
        keys: "⌃R",
        run: () => this.toggleMode(),
      },
      files: {
        label: "Stash the input",
        detail: "Put the input aside, or bring it back",
        command: "",
        keys: "⌃S",
        run: () => this.stash(),
      },
      pause: {
        label: "Filter the view",
        detail: "Show only what holds a text",
        command: "",
        keys: "⌃F",
        run: () => this.openSearch(),
      },
      exit: {
        label: "Help",
        detail: "Keys, marks, and slash commands",
        command: "",
        keys: "F1",
        run: () => this.help(),
      },
    };
    for (const [name, [label, argument, detail]] of Object.entries(commands)) {
      if (name === "new") continue;
      const before = beside[name];
      if (before) choices.push(before);
      choices.push({
        label,
        detail,
        command: `/${name}${argument ? ` ${argument}` : ""}`,
        keys: shortcuts[name as keyof typeof commands],
        run: this.command(name, argument),
      });
    }
    this.openPalette("Commands", choices, 0, "", "", true);
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
    if (
      value &&
      (valued.has(value[1] ?? "") || this.session.world.parts.commands.get(value[1] ?? "")?.values)
    )
      return { kind: "value", text: value[2] ?? "", command: value[1] ?? "" };
    const at = before.match(/(?:^|\s)@([^\s"']*)$/);
    if (at && this.session.mode === "prompt" && !this.session.editing)
      return { kind: "@", text: at[1] ?? "" };
    return undefined;
  }
  /** The token before the cursor as it is now, replaced with the text of a suggestion. */
  private complete(suggestion: Suggestion): void {
    const token = this.token();
    if (token)
      this.replaceBefore(token.kind === "value" ? token.text : `${token.kind}${token.text}`, suggestion.text);
  }
  /** The paths of the project, which the suggestions read: known once the first read ends, which suggests again. */
  private projectPaths(fresh = false): string[] | undefined {
    const read = this.session.projectFiles(fresh);
    if (this.files?.read !== read) {
      const files: NonNullable<typeof this.files> = { read };
      this.files = files;
      read.then(
        (paths) => {
          files.paths = paths;
          if (this.files === files && !this.closed) this.suggest();
        },
        (error: unknown) => {
          files.error = error instanceof Error ? error.message : String(error);
          if (this.files === files && !this.closed) this.suggest();
        },
      );
    }
    return this.files.paths;
  }
  /** Whether the value of a command is a path of the project, which the suggestions wait for. */
  private pathCommand(name: string): boolean {
    return pathCommands.has(name) || Boolean(this.session.world.parts.commands.get(name)?.paths);
  }
  /** The values that the first argument of a command may take, each with what it means, or nothing for a command
   * whose argument is free text. A value of a command that takes more after it completes, and a value of any other
   * command runs the command when it is chosen. */
  private argumentValues(command: string): { value: string; detail: string; more?: boolean }[] | undefined {
    const w = this.session;
    const current = (yes: boolean) => (yes ? "   current" : "");
    const files = () => this.projectPaths() ?? [];
    switch (command) {
      case "model": {
        const roster = w.roster.filter(([name]) => name !== "operator");
        const short = (name: string) => (name.includes(":") ? name.slice(name.indexOf(":") + 1) : name);
        return roster.map(([name, , window]) => ({
          // A model goes by its name alone, unless two providers offer a model of that name.
          value: roster.filter(([other]) => short(other) === short(name)).length > 1 ? name : short(name),
          detail: `${count(window)} tokens of context${current(name === w.actorChoice.model)}`,
        }));
      }
      case "effort": {
        const { model, effort } = w.actorChoice;
        return (w.roster.find(([name]) => name === model)?.[1] ?? []).map((name) => ({
          value: name,
          detail: `${efforts[name] ?? ""}${current(name === effort)}`,
        }));
      }
      case "shape":
        return shapes.map((shape) => ({
          value: shape,
          detail: `The answer is a ${shape}${current(shape === w.shape)}`,
        }));
      case "theme":
        return (Object.keys(palettes) as ThemeName[]).map((name) => ({
          value: name,
          detail: `${themeLabels[name].join(", ")}${current(name === this.theme)}`,
        }));
      case "image":
        return files()
          .filter((path) => /\.(png|jpe?g|gif|webp)$/i.test(path))
          .map((path) => ({ value: path, detail: "" }));
      case "workspace":
        return (this.options.workspaces?.groups ?? []).map((group) => ({
          value: shortenHome(group.directory),
          detail: group.name,
        }));
      case "edit":
        return w.activity
          .filter((act) => w.isUserPrompt(act))
          .map((act) => ({
            value: act.id,
            detail: clip(String(act.words[1] ?? "").split("\n")[0] ?? "", 48),
          }));
      case "pause":
      case "wake":
        return w.chains.map((chain) => ({
          value: chain.id,
          detail: `${w.labelOf(chain.id)}${current(chain.id === w.selected)}`,
        }));
      case "cancel":
        return w.activity
          .filter((act) => working(act))
          .map((act) => ({
            value: act.id,
            detail: `${act.kind}  ${clip(String(act.words[act.kind === "prompt" ? 1 : 0] ?? ""), 40)}`,
          }));
      case "close":
        return w.activity
          .filter((act) => !act.done && act.kind !== "chain")
          .map((act) => ({ value: act.id, detail: act.kind, more: true }));
      case "extensions":
        return [{ value: "update", detail: "Fetch every extension again; a new session plays the change" }];
      default:
        return w.world.parts.commands.get(command)?.values?.(w.here(() => this.projectPaths()));
    }
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
      const names = [
        ...Object.entries(commands).map(([name, [, argument, detail]]) => ({ name, argument, detail })),
        ...[...this.session.world.parts.commands].map(([name, command]) => ({
          name,
          argument: command.argument ?? "",
          detail: command.detail,
        })),
      ];
      // The command that the token names whole comes first.
      this.suggestions = names
        .filter(({ name }) => name.startsWith(token.text))
        .sort((one, other) => Number(other.name === token.text) - Number(one.name === token.text))
        .map(({ name, argument, detail }) => ({
          label: `/${name}${argument ? ` ${argument}` : ""}`,
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
      this.suggestions = (this.argumentValues(command) ?? [])
        .filter(({ value }) => value.toLowerCase().includes(wanted))
        .sort(
          (one, other) =>
            Number(other.value.toLowerCase().startsWith(wanted)) -
            Number(one.value.toLowerCase().startsWith(wanted)),
        )
        .slice(0, 50)
        .map(({ value, detail, more }) => ({
          label: value,
          detail,
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
      this.suggestions = (this.projectPaths(started) ?? [])
        .filter((path) => path.toLowerCase().includes(wanted))
        .slice(0, 50)
        .map((path) => ({
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
    const state = !token
      ? ""
      : (token.kind === "@" || this.pathCommand(token.command ?? "")) && this.files?.error
        ? this.files.error
        : (token.kind === "@" || this.pathCommand(token.command ?? "")) && !this.files?.paths
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
      this.suggestionBox.add(
        this.inset(
          space.between,
          this.text(state, this.files?.error ? c.danger : c.muted, { height: space.bar }),
        ),
      );
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
      const row = this.box({
        flexDirection: "row",
        height: space.bar,
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
          flexShrink: 0,
          truncate: true,
          attributes: selected ? bold : 0,
        }),
      );
      if (one.detail) row.add(this.text(one.detail, c.muted, { truncate: true, flexShrink: 1 }));
      this.suggestionBox.add(row);
    }
    if (shown.length > rows)
      this.suggestionBox.add(
        this.inset(
          space.between + space.inset,
          this.text(`${shown.length - rows} more`, c.faint, { height: space.bar }),
        ),
      );
    this.renderStatus();
  }
  /** The mark of a choice that is the current one, and the room of that mark for the others. */
  private current(yes: boolean): Part {
    return yes ? [`${glyph.done} `, c.accent] : ["  "];
  }
  /** The models of the roster under their providers, with the one the chain uses marked. */
  models = (): void => {
    const roster = this.session.roster.filter(([name]) => name !== "operator");
    const current = this.session.actorChoice.model;
    this.openPalette(
      "Model",
      roster.map(([name, , window]) => {
        const [provider, model] = name.includes(":")
          ? [name.slice(0, name.indexOf(":")), name.slice(name.indexOf(":") + 1)]
          : ["", name];
        return {
          label: model,
          detail: `${count(window)} tokens of context`,
          mark: this.current(name === current),
          heading: provider || undefined,
          run: () => this.action(`/model ${name}`),
        };
      }),
      roster.findIndex(([name]) => name === current),
      "",
      "The chain sends its next prompt to this model.",
    );
  };
  effortPicker = (): void => {
    const { model, effort } = this.session.actorChoice;
    const offered = this.session.roster.find(([name]) => name === model)?.[1] ?? [];
    this.openPalette(
      `Effort of ${this.model(model).name}`,
      offered.map((name) => ({
        label: name,
        detail: efforts[name] ?? "",
        mark: this.current(name === effort),
        run: () => this.action(`/effort ${name}`),
      })),
      offered.indexOf(effort),
      "",
      "More effort thinks longer and costs more. ⇧Tab moves to the next.",
    );
  };
  details = (): void => {
    this.openPalette(
      "Details",
      [...this.cards]
        .filter(([, card]) => card.compact || card.collapsible)
        .map(([id, card]) => ({
          label: card.heading.plainText.replace(/^[▸▾] /, ""),
          detail: card.closed ? "Expand" : "Collapse",
          run: () => {
            this.folds.set(card.state, !card.closed);
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
          swatch: [palette.accent, palette.secondary, palette.success, palette.warning, palette.danger],
          run: () => {
            this.session.theme = name;
            this.render();
            this.session.save();
          },
        };
      }),
      names.indexOf(this.theme),
      "",
      "Every session shares this choice.",
    );
  }
  shapes(): void {
    this.openPalette(
      "Response shape",
      shapes.map((name) => ({
        label: name,
        detail: "The engine validates the result against this Python type",
        run: () => {
          this.session.shape = name;
          this.render();
        },
      })),
    );
  }
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
    this.composer.backgroundColor = c.panel;
    this.composer.focusedBackgroundColor = c.panel;
    this.composer.textColor = c.text;
    this.composer.focusedTextColor = c.text;
    this.composer.placeholderColor = c.muted;
    this.composer.cursorColor = c.accent;
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
    for (let line = 0; line < this.composer.lineCount; line++) this.composer.clearLineHighlights(line);
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
    const pending = this.session.world.pending;
    this.openPalette("Saved work is paused", [
      {
        label: "Resume saved work",
        detail: `${pending.size} unfinished ${pending.size === 1 ? "act starts" : "acts start"} again.`,
        run: async () => {
          await this.session.world.resume();
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
    const question = this.session.operatorPrompt;
    if (!question) return;
    if (question.shape === "bool") {
      this.openPalette("Answer yes or no", [
        {
          label: "Yes",
          detail: "",
          run: () => this.session.world.answer(question.id, "yes").then(() => {}),
        },
        {
          label: "No",
          detail: "",
          run: () => this.session.world.answer(question.id, "no").then(() => {}),
        },
      ]);
      this.showQuestionText(question.message);
      return;
    }
    this.openPalette(`Your answer, as ${question.shape}`, []);
    // The field of the dialog takes the answer, and shows its prompt from the start.
    const prompt = this.paletteInputRow?.getChildren()[0];
    if (prompt) prompt.visible = true;
    if (this.paletteInputRow) this.paletteInputRow.height = space.bar;
    this.showQuestionText(question.message);
    if (this.paletteInput) this.paletteInput.placeholder = `Type the answer, as ${question.shape}`;
    const error = this.text("Enter submits. Esc leaves the question open.", c.muted);
    this.paletteList?.add(error);
    const input = this.paletteInput;
    if (!input) return;
    input.removeAllListeners(InputRenderableEvents.INPUT);
    input.removeAllListeners(InputRenderableEvents.ENTER);
    input.on(InputRenderableEvents.ENTER, () => {
      void this.session.world
        .answer(question.id, input.value)
        .then(() => this.closeOverlay())
        .catch((failure) => {
          error.fg = c.danger;
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
    this.questionDocument = new ScrollBoxRenderable(this.renderer, {
      height: Math.max(1, Math.min(rows, 8, this.renderer.height - 18)),
      // A field that shows stands a row under the text, and a field that takes no row leaves its own space under it.
      marginBottom: this.paletteInputRow?.height ? space.section : 0,
      scrollX: false,
      scrollY: true,
      contentOptions: { paddingLeft: space.between, paddingRight: space.between },
    });
    this.questionDocument.add(this.markdown(message));
    this.overlay.insertBefore(this.questionDocument, this.paletteInputRow);
    this.overlay.maxHeight = this.paletteHeight;
  }
  private async showHover(name: string, x: number, y: number): Promise<void> {
    try {
      const inspected = await this.session.life.inspect(name, this.session.selected);
      if (this.closed || this.overlay) return;
      this.hover?.destroyRecursively();
      const width = Math.min(58, this.renderer.width - 4);
      this.hover = this.box({
        position: "absolute",
        left: Math.max(1, Math.min(x, this.renderer.width - width - 1)),
        top: Math.max(1, Math.min(y + 1, this.renderer.height - 9)),
        width,
        maxHeight: 9,
        paddingX: space.between,
        paddingY: space.inset,
        backgroundColor: c.raised,
        zIndex: 30,
        onMouseDown: () => this.inspect(name),
      });
      this.hover.add(
        this.text([
          [name, c.text, bold],
          [`: ${inspected.kind}`, c.secondary],
        ]),
      );
      this.hover.add(this.text(inspected.representation.slice(0, 280), c.muted, { maxHeight: 4 }));
      this.hover.add(
        this.text([
          ["Click", c.muted],
          [" opens it   ", c.faint],
          ["⌃G", c.muted],
          [" inspects a name", c.faint],
        ]),
      );
      this.root.add(this.hover);
    } catch {
      this.hover?.destroyRecursively();
      this.hover = undefined;
    }
  }
  /** The value a name holds in the module of the chain, shown once the life has read it. */
  inspect = (name: string): Promise<void> => {
    this.hover?.destroyRecursively();
    this.hover = undefined;
    return this.session.life
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
            const lines = (await this.session.world.source()).split("\n");
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
            const document = new ScrollBoxRenderable(this.renderer, {
              height: Math.min(24, this.renderer.height - 16),
              scrollX: false,
              scrollY: true,
            });
            document.add(
              new LineNumberRenderable(this.renderer, {
                target: this.code(lines.slice(at, next < 0 ? undefined : at + next + 1).join("\n")),
                fg: c.muted,
                lineNumberOffset: at,
                paddingRight: space.inset,
              }),
            );
            this.paletteList?.add(document);
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
          void this.session
            .follow(value)
            .then((text) => this.showValue(value, text))
            .catch(this.report);
        },
      });
    this.openPalette(label, choices);
    if (typeof value === "string" && (value.length > 100 || value.includes("\n"))) {
      const document = new ScrollBoxRenderable(this.renderer, {
        height: Math.max(3, Math.min(18, this.renderer.height - 18)),
        scrollX: false,
        scrollY: true,
      });
      document.add(this.text(value));
      this.paletteList?.add(document);
      this.questionDocument = document;
    }
  }
  async names(): Promise<void> {
    const w = this.session;
    const names = ((await w.life.held("modules", [w.selected], "keys")) as string[]).filter(
      (name) => !name.startsWith("_"),
    );
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
      0,
      "",
      "Enter reads the live value of the name, its type, and its fields.",
    );
  }
  private async completeNames(): Promise<void> {
    const names = (await this.session.life.held("modules", [this.session.selected], "keys")) as string[];
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
    const prompts = this.session.activity.filter((act) => act.kind === "prompt" && !asksOperator(act));
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
      Math.max(0, prompts.length - 1),
      "",
      "The program is the Python the model wrote for the prompt. An edit replays it.",
    );
  }
  /** The rewind tree in the feed, with the pointer on the last message of the chain: Enter then gives that message
   * back on a new branch that has not read it. A paused chain is resumed first. */
  rewind = (): void => {
    if (this.session.paused) {
      this.openPalette("Resume this chain before rewinding", [
        {
          label: "Resume chain",
          detail: "Then choose the point that the new branch starts from.",
          run: () => (this.session.world.pending.size ? this.resume() : this.action("/wake")),
        },
      ]);
      return;
    }
    const points = this.session.activity.filter((act) => this.isPoint(act));
    const last = points.findLast((act) => this.session.isUserPrompt(act)) ?? points.at(-1);
    this.openTree(last?.id ?? this.session.selected, "Rewind");
  };
  /** The tree of the session in the feed, with the pointer on the chain shown. */
  sessionTree = (): void => this.openTree(this.session.selected, "Session tree");
  /** Whether an act is a point that a branch can start from: an act that a turn of its chain tells, which its part
   * does not hide, and no rung that its chain wrote to retell its source or that the World played. */
  private isPoint(act: ActRow): boolean {
    return (
      !this.session.world.parts.hidden(act) &&
      (act.kind !== "rung" || (act.by !== "world" && this.session.actOf(act.by)?.kind !== "chain"))
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
      const status = this.chainStatus(act.id);
      return {
        parts: [
          [`${this.statusDot(status)} `, this.statusColor(status)],
          [w.labelOf(act.id), current ? c.text : c.muted, bold],
        ],
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
    const subject = asksOperator(act)
      ? String(act.words[1] ?? "")
      : act.kind === "rung"
        ? String(w.program[act.id] || act.words[0] || "")
        : act.kind === "prompt"
          ? String(act.words[1] ?? "")
          : act.kind === "wait"
            ? seconds(Number(act.words[0]))
            : String(act.words[0] ?? "");
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
          ? "Choose a point to branch from. The module and the files keep their state."
          : "Open a chain, or branch from a point. The module and the files keep their state.";
      // The note is cut at its end where the bar has no room for it beside the title and the key that leaves.
      const room =
        width - space.inset * 2 - Bun.stringWidth(tree.title) - space.between * 2 - Bun.stringWidth("Esc");
      this.treeBar.add(
        this.text(
          [
            [tree.title, c.text, bold],
            [`${" ".repeat(space.between)}${clip(note, Math.max(0, room))}`, c.muted],
          ],
          c.muted,
          { height: space.bar, flexGrow: 1, flexShrink: 1 },
        ),
      );
      this.treeBar.add(this.text("Esc", c.faint, { onMouseUp: this.click(() => this.closeTree()) }));
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
      const line = this.box({
        id: `tree-${row.id}`,
        flexDirection: "row",
        height: space.bar,
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
        onMouseOver() {
          if (!active) this.backgroundColor = c.panel;
        },
        onMouseOut() {
          if (!active) this.backgroundColor = c.background;
        },
      });
      const fold = row.parent ? `${row.folded ? glyph.closed : glyph.open} ` : "  ";
      const tag = row.tag ? `  ${row.tag}` : "";
      const room = Math.max(8, width - space.inset * 2 - Bun.stringWidth(row.lines + fold + tag));
      const label = plain(row.parts);
      line.add(
        this.text(
          [
            [row.lines, c.faint],
            [fold, c.faint],
            ...(Bun.stringWidth(label) > room ? this.clipParts(row.parts, room) : row.parts),
          ],
          c.text,
          { height: space.bar, flexGrow: 1, flexShrink: 1, truncate: true },
        ),
      );
      if (tag) line.add(this.text(tag, active ? c.muted : c.faint, { height: space.bar }));
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
          status: this.chainStatus(chain.id),
          run: () => w.select(chain.id),
        };
      }),
      Math.max(
        0,
        chains.findIndex((chain) => chain.id === w.selected),
      ),
      "",
      "Each chain has its own transcript and module. /tree shows the branches.",
    );
  }
  /** A dialog of choices, which a filter narrows. At a point, it is a menu as wide as its labels need, which stands
   * under the point, or over it where the rows under it cannot hold it, and leaves the screen behind it as it is. */
  openPalette(
    label: string,
    choices: Choice[],
    selected = 0,
    query = "",
    note = "",
    rich = false,
    at?: { x: number; y: number },
  ): void {
    // A dialog or a menu takes the keys, so a name that a row takes ends first, and is kept.
    this.endRename(true);
    this.closeOverlay();
    this.rich = rich;
    this.paletteStart = 0;
    this.composer.blur();
    this.hover?.destroyRecursively();
    this.hover = undefined;
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
    const heading = this.box({
      flexDirection: "row",
      height: space.bar,
      paddingLeft: space.between,
      paddingRight: space.inset,
    });
    heading.add(this.text(label, c.text, { attributes: bold, truncate: true, flexGrow: 1, flexShrink: 1 }));
    heading.add(this.text("Esc", c.faint, { onMouseUp: this.click(() => this.closeOverlay()) }));
    this.overlay.add(heading);
    this.paletteNote = note ? space.bar : 0;
    if (note)
      this.overlay.add(
        this.inset(
          space.between,
          this.text(clip(note, this.paletteWidth - 8), c.muted, { height: space.bar }),
        ),
      );
    // A short list shows its filter only once the operator types in it, and until then the filter takes no row.
    const quiet = choices.length <= 5 && !query;
    this.paletteInputRow = this.box({
      flexDirection: "row",
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
      backgroundColor: c.raised,
      focusedBackgroundColor: c.raised,
      textColor: c.text,
      focusedTextColor: c.text,
      placeholderColor: c.faint,
      cursorColor: c.accent,
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
    this.filtered = filter(query);
    this.selection = Math.max(0, selected);
    this.paletteInput.value = query;
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
      const title = first ? (choice.heading ? 1 : 0) : titled(index) ? 2 : 0;
      return (this.rich ? (choice.command ? 3 : 2) : 1) + title;
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
            this.text([[choice.heading ?? "", c.muted, bold]], c.muted, {
              height: space.bar,
              marginTop: offset ? space.section : 0,
            }),
          ),
        );
      const block = this.box({
        backgroundColor: selected ? c.selected : c.raised,
        marginTop: offset && this.rich ? gap : 0,
        onMouseDown: (event) => {
          if (event.button === 2 && choice.toggle) choice.toggle();
        },
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
        const title = this.box({ flexDirection: "row", height: space.bar });
        title.add(
          this.text([bar, [clip(choice.label, width - Bun.stringWidth(keys) - 2), c.text, bold]], c.text, {
            flexGrow: 1,
            flexShrink: 1,
            truncate: true,
          }),
        );
        if (keys) title.add(this.text(keys, selected ? c.muted : c.faint, { paddingRight: space.inset }));
        block.add(title);
        if (choice.command)
          block.add(
            this.text([bar, [clip(choice.command, width), selected ? c.accent : c.secondary]], c.text, {
              height: space.bar,
            }),
          );
        block.add(
          this.text([bar, [clip(choice.detail.replace(/\s*\n\s*/g, " "), width), c.muted]], c.muted, {
            height: space.bar,
          }),
        );
        this.paletteList.add(block);
        continue;
      }
      block.flexDirection = "row";
      block.height = space.bar;
      const fg = choice.color ?? c.text;
      block.add(this.text(`${selected ? glyph.pointer : " "} `, c.accent, { attributes: bold }));
      const mark: Part | undefined =
        choice.mark ??
        (choice.status ? [`${this.statusDot(choice.status)} `, this.statusColor(choice.status)] : undefined);
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
    const kitty = this.renderer.capabilities?.kitty_keyboard === true;
    const legend: [string, RGBA, string][] = [
      [`${spin(0)} Working`, c.accent, "A model or a command runs"],
      [`${glyph.running} Running`, c.accent, "A chain or a session has work in progress"],
      [`${glyph.asks} Input needed`, c.warning, "A question waits for your answer"],
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
            label: chords(key, kitty),
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
          Object.entries(commands).map(([name, [, argument, detail]]) => ({
            label: `/${name}${argument ? ` ${argument}` : ""}`,
            detail,
            heading: "Slash commands",
            run: () => {},
          })),
        ),
      0,
      "",
      "The chords that this terminal sends, what each mark means, and every command.",
    );
  }
  private key = (key: KeyEvent): void => {
    // While a row takes a new name, its input takes the keys, and Escape or Ctrl+C leaves the name as it was.
    if (this.renaming?.input?.focused) {
      if (key.name === "escape" || (key.ctrl && key.name === "c")) {
        key.preventDefault();
        this.endRename(false);
      } else if (key.ctrl && key.name === "q") {
        key.preventDefault();
        this.endRename(true);
        void this.options.quit();
      }
      return;
    }
    const plainKey = !key.ctrl && !key.meta && !key.shift;
    if (!this.overlay && this.tree && plainKey) {
      const row = this.treeRow();
      const steps: Record<string, number> = {
        up: -1,
        down: 1,
        pageup: -10,
        pagedown: 10,
        home: -1e9,
        end: 1e9,
      };
      if (key.name in steps) {
        key.preventDefault();
        this.moveTree(steps[key.name] ?? 0);
        return;
      }
      if (key.name === "right" && row?.parent) {
        key.preventDefault();
        if (row.folded) this.foldTree(row.id, false);
        else this.moveTree(1);
        return;
      }
      if (key.name === "left" && row) {
        key.preventDefault();
        if (row.parent && !row.folded) this.foldTree(row.id, true);
        else if (row.up) this.pointTree(row.up);
        return;
      }
      if (["return", "enter"].includes(key.name)) {
        key.preventDefault();
        void this.chooseTreeRow();
        return;
      }
      if (key.name === "escape") {
        key.preventDefault();
        this.closeTree();
        return;
      }
    }
    if (!this.overlay && this.suggestionBox.visible && !key.ctrl && !key.meta) {
      const chosen = this.suggestions[this.suggestionIndex];
      if (key.name === "up" || key.name === "down") {
        key.preventDefault();
        const count = this.suggestions.length;
        if (count)
          this.suggestionIndex = (this.suggestionIndex + (key.name === "up" ? count - 1 : 1)) % count;
        this.renderSuggestions();
        return;
      }
      if (key.name === "tab" && !key.shift && !key.ctrl && !key.super && chosen) {
        key.preventDefault();
        this.complete(chosen);
        return;
      }
      if (["return", "enter"].includes(key.name) && !key.shift && chosen) {
        key.preventDefault();
        if (chosen.submit) chosen.submit();
        else this.complete(chosen);
        return;
      }
      if (key.name === "escape") {
        key.preventDefault();
        this.dismissSuggestions();
        return;
      }
    }
    // As in Claude Code: Ctrl+D on an empty input asks once, and exits when it is pressed again while it asks.
    if (!this.overlay && key.ctrl && key.name === "d" && !this.composer.plainText) {
      key.preventDefault();
      if (this.session.notice === exitNotice) void this.options.quit();
      else this.session.notice = exitNotice;
      return;
    }
    if (!this.overlay && key.ctrl && key.name === "v") {
      key.preventDefault();
      void clipboardImage((path) => this.session.attachImage(path)).catch(this.report);
      return;
    }
    if (!this.overlay && key.ctrl && key.name === "s") {
      key.preventDefault();
      this.stash();
      return;
    }
    if (!this.overlay && key.meta && key.name === "e") {
      key.preventDefault();
      void this.editDraft().catch(this.report);
      return;
    }
    if (!this.overlay && key.meta && key.name === "d") {
      key.preventDefault();
      this.details();
      return;
    }
    if (!this.overlay && key.meta && ["return", "enter"].includes(key.name)) {
      key.preventDefault();
      // The queue holds messages to a model. Python input and a program under edit run when Enter sends them.
      if (this.session.mode === "python" || this.session.editing) {
        this.session.notice = "Only a message can wait in the queue. Enter runs this Python now.";
        return;
      }
      this.session.enqueue(this.composer.plainText);
      this.composer.setText("");
      return;
    }
    if (key.ctrl && key.name === "w" && !this.overlay) {
      key.preventDefault();
      void this.workspacePicker();
      return;
    }
    if (key.ctrl && key.name === "\\" && !this.overlay) {
      key.preventDefault();
      this.toggleSidebar();
      return;
    }
    // ⌃Tab rolls to the next chain and ⇧⌃Tab to the one before it. ⌘Tab does the same where the system and the
    // terminal pass it, which macOS does not, since it keeps ⌘Tab to switch applications.
    if (!this.overlay && key.name === "tab" && (key.ctrl || key.super)) {
      key.preventDefault();
      this.rollChain(key.shift ? -1 : 1);
      return;
    }
    if (!this.overlay && key.name === "tab" && key.shift) {
      key.preventDefault();
      this.effortPicker();
      return;
    }
    if (key.ctrl && ["pageup", "pagedown"].includes(key.name) && this.session.view === "changes") {
      key.preventDefault();
      this.changePage(key.name === "pageup" ? -1 : 1);
      return;
    }
    if (key.meta && key.ctrl && key.name === "left" && !this.overlay) {
      key.preventDefault();
      void this.back().catch(this.report);
      return;
    }
    // Up on an empty input takes back the last message queued on the chain, and Up and Down at the first and the last
    // line of the input walk the history of what it sent, as a shell does. Alt+Up and Alt+Down walk it from any line.
    if (
      !this.overlay &&
      this.composer.focused &&
      ["up", "down"].includes(key.name) &&
      !key.ctrl &&
      !key.shift
    ) {
      const up = key.name === "up";
      const queued = this.session.queued.findLast((entry) => entry.chain === this.session.selected);
      if (up && !key.meta && !this.composer.plainText && queued && this.historyIndex < 0) {
        key.preventDefault();
        this.session.removeQueued(queued.id);
        this.insert(queued.text);
        this.session.notice = "The queued message is back in the input. ⌥Enter queues it again.";
        return;
      }
      const cursor = this.composer.visualCursor.visualRow;
      const edge = up ? cursor === 0 : cursor >= this.composer.virtualLineCount - 1;
      const history = this.history();
      if ((key.meta || edge) && history.length && (up || this.historyIndex >= 0)) {
        key.preventDefault();
        if (this.historyIndex < 0) {
          this.historyDraft = this.composer.plainText;
          this.historyIndex = history.length;
        }
        this.historyIndex = Math.max(0, Math.min(history.length, this.historyIndex + (up ? -1 : 1)));
        const text = history[this.historyIndex] ?? this.historyDraft;
        if (this.historyIndex === history.length) this.historyIndex = -1;
        this.composer.replaceText(text);
        return;
      }
    }
    // Ctrl+J arrives as a line feed from a terminal with no kitty keyboard protocol.
    if (
      !this.overlay &&
      (this.session.mode === "python" || this.session.editing) &&
      ((key.shift && ["return", "enter"].includes(key.name)) ||
        (key.ctrl && key.name === "j") ||
        key.name === "linefeed")
    ) {
      key.preventDefault();
      const before = this.beforeCursor().split("\n").at(-1) ?? "";
      this.composer.insertText(
        `\n${before.match(/^\s*/)?.[0] ?? ""}${before.trimEnd().endsWith(":") ? "  " : ""}`,
      );
      return;
    }
    if (key.meta && ["[", "]", "p", "n"].includes(key.name) && !this.overlay) {
      key.preventDefault();
      this.jumpMessage(["]", "n"].includes(key.name) ? 1 : -1);
      return;
    }
    if (key.ctrl && key.name === "c") {
      key.preventDefault();
      if (this.overlay) this.closeOverlay();
      else if (this.tree) this.closeTree();
      else if (this.composer.plainText) this.composer.replaceText("");
      else this.action("/cancel");
      return;
    }
    if (key.ctrl && key.name === "q") {
      key.preventDefault();
      void this.options.quit();
      return;
    }
    if (key.name === "escape") {
      key.preventDefault();
      // One Escape closes what is open, or pauses the model at work. An Escape with nothing to do asks for a second
      // one, which opens the rewind tree.
      const open = Boolean(this.overlay || this.searchRow.visible || this.session.editing);
      const pausing =
        !open &&
        !this.session.paused &&
        this.session.activity.some((act) => act.kind === "prompt" && !act.done && !asksOperator(act));
      if (pausing) this.action("/pause");
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
      return;
    }
    if (this.overlay) {
      if (["left", "right"].includes(key.name) && this.filtered[this.selection]?.toggle) {
        key.preventDefault();
        this.filtered[this.selection]?.toggle?.();
        return;
      }
      if (this.questionDocument && ["pageup", "pagedown"].includes(key.name)) {
        key.preventDefault();
        this.questionDocument.scrollBy(key.name === "pageup" ? -8 : 8);
        return;
      }
      if (["up", "down", "pageup", "pagedown"].includes(key.name)) {
        key.preventDefault();
        const step = { up: -1, down: 1, pageup: -8, pagedown: 8 }[key.name as "up"] ?? 0;
        this.selection = Math.max(0, Math.min(this.filtered.length - 1, this.selection + step));
        this.renderChoices();
      }
      return;
    }
    // A chord with ⌃ needs the kitty keyboard protocol, and the same chord with ⌥ reaches every terminal.
    if ((key.ctrl || key.meta) && /^[1-3]$/.test(key.name)) {
      key.preventDefault();
      this.showView(views[Number(key.name) - 1] ?? "feed");
    } else if (key.name === "f1") {
      key.preventDefault();
      this.help();
    } else if (key.ctrl && key.name === "p") {
      key.preventDefault();
      this.palette();
    } else if (key.ctrl && key.name === "b") {
      key.preventDefault();
      this.chains();
    } else if ((key.ctrl || key.meta) && key.name === "m") {
      key.preventDefault();
      this.models();
    } else if (key.ctrl && key.name === "t") {
      key.preventDefault();
      this.themes();
    } else if (key.ctrl && key.name === "a" && this.session.operatorPrompt) {
      key.preventDefault();
      this.question();
    } else if (key.ctrl && key.name === "g") {
      key.preventDefault();
      void this.names().catch(this.report);
    } else if (key.ctrl && key.name === "l") {
      key.preventDefault();
      this.ladders();
    } else if (key.ctrl && key.name === "r") {
      key.preventDefault();
      this.toggleMode();
    } else if (key.ctrl && key.name === "y") {
      key.preventDefault();
      const selection = this.renderer.getSelection()?.getSelectedText();
      if (selection) this.renderer.copyToClipboardOSC52(selection);
    } else if (key.ctrl && key.name === "space") {
      key.preventDefault();
      void this.completeNames().catch(this.report);
    } else if (key.ctrl && key.name === "n") {
      key.preventDefault();
      this.insert("/chain ");
    } else if (key.ctrl && key.name === "o") {
      key.preventDefault();
      this.openSessions();
    } else if (key.ctrl && key.name === "f") {
      key.preventDefault();
      this.openSearch();
    } else if (key.name === "pageup" || key.name === "pagedown") {
      key.preventDefault();
      this.scroll.scrollBy((key.name === "pageup" ? -1 : 1) * Math.max(1, this.scroll.viewport.height - 2));
    }
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
  private toggleSidebar(): void {
    const library = this.options.workspaces;
    if (library) library.toggle();
    else {
      this.session.preferences.sidebar = !this.session.preferences.sidebar;
      this.session.preferences.save();
      this.render();
    }
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
  private openSessions(): void {
    void this.options
      .sessions?.()
      .then((choices) => this.openPalette("Sessions", choices))
      .catch(this.report);
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
  dispose(): void {
    clearInterval(this.tick);
    this.closed = true;
    this.keepDraft();
    this.session.scrolls[this.lastView] = this.place;
    this.renderer.off("frame", this.laidOut);
    this.session.folds = Object.fromEntries(this.folds);
    this.session.save();
    if (this.redraw) clearTimeout(this.redraw);
    if (this.hoverTimer) clearTimeout(this.hoverTimer);
    this.session.off("change", this.schedule);
    this.options.workspaces?.off("change", this.schedule);
    this.session.off("compose", this.compose);
    this.session.off("resume", this.resume);
    this.session.off("shared", this.shared);
    this.renderer.keyInput.off("keypress", this.key);
    this.renderer.off("resize", this.render);
    this.renderer.off("selection", this.copySelection);
    this.root.destroyRecursively();
    this.style.destroy();
  }
  private changePage(step: number): void {
    this.session.changePage = Math.max(
      0,
      Math.min(Math.ceil(this.session.world.changes / 20) - 1, this.session.changePage + step),
    );
    void this.session.refresh().catch(this.session.fail);
  }
}

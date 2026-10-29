export { sessionDetail } from "./app/shared.ts";

import {
  type BoxOptions,
  BoxRenderable,
  type CliRenderer,
  CodeRenderable,
  InputRenderable,
  InputRenderableEvents,
  type KeyEvent,
  type LineNumberRenderable,
  type MouseEvent,
  type Renderable,
  ScrollBoxRenderable,
  type SimpleHighlight,
  TextareaRenderable,
  TextRenderable,
} from "@opentui/core";
import type { Keymap } from "@opentui/keymap";
import { safeText, type Turn } from "furb";
import { type ChainThread, type LiveAct, type Loom, type ProgramLine, wants } from "furb/life";
import { type ActionsPart, actions, actionsBound } from "./app/actions.ts";
import { type BlocksPart, blocks } from "./app/blocks.ts";
import { type ChromePart, chrome } from "./app/chrome.ts";
import { type ComposerPart, composer, composerBound } from "./app/composer.ts";
import { type DeskPart, desk, deskBound } from "./app/desk.ts";
import { type DetailsPart, details } from "./app/details.ts";
import { type FeedPart, feed } from "./app/feed.ts";
import { GhostTextarea } from "./app/ghost.ts";
import { type InputPart, input, inputBound } from "./app/input.ts";
import { type ChainsPanel, type LoomsPart, looms } from "./app/looms.ts";
import { type PalettePart, palette } from "./app/palette.ts";
import { assemble, bindAll } from "./app/part.ts";
import { type QuestionsPart, questions, questionsBound } from "./app/questions.ts";
import { type RailHandler, type RailPart, rail } from "./app/rail.ts";
import { type SettingsPart, settings, settingsBound } from "./app/settings.ts";
import {
  type ChainRead,
  type Choice,
  inputColors,
  keymapOf,
  noBorder,
  type Suggestion,
  type TextShape,
  type ThreadList,
  type TreeRow,
  type Value,
} from "./app/shared.ts";
import { type SidebarPart, sidebar } from "./app/sidebar.ts";
import { type SpacesPart, spaces, spacesBound } from "./app/spaces.ts";
import { type StagePart, stage } from "./app/stage.ts";
import { type ThreadPart, thread } from "./app/thread.ts";
import { type TreePart, tree, treeBound } from "./app/tree.ts";
import { type ValuesPart, values, valuesBound } from "./app/values.ts";
import { type WelcomePart, welcome, welcomeBound } from "./app/welcome.ts";
import { graphemes, kibibytes } from "./format.ts";
import { Graph } from "./graph.ts";
import type { Action } from "./keys.ts";
import { LoomPane } from "./overview.ts";
import { loadParsers } from "./parsers.ts";
import { Parts } from "./parts.ts";
import { readingNote, readOnly, sameBuild } from "./reading.ts";
import type { ActRow, Scroll, Session, View } from "./session.ts";
import {
  theme as c,
  colorless,
  defaultTheme,
  frame,
  glyph,
  motion,
  setTheme,
  spacing as space,
  syntax,
  type ThemeName,
  threadTone,
} from "./theme.ts";
import { bold, type Part, styled } from "./ui.ts";
import type { SessionEntry, Workspace, Workspaces } from "./workspaces.ts";

export interface AppOptions {
  /** Close the window: each life detaches, and a life in a host of its own goes on, or each life ends when end is set. */
  quit(end?: boolean): void | Promise<void>;
  workspaces: Workspaces;
  /** Rings the bell of the terminal, which calls the operator to a question when the terminal can show no
   * notification. By default it writes BEL to the standard output. */
  bell?(): void;
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

const bounds = [
  ...composerBound,
  ...welcomeBound,
  ...deskBound,
  ...questionsBound,
  ...spacesBound,
  ...actionsBound,
  ...settingsBound,
  ...valuesBound,
  ...treeBound,
  ...inputBound,
] as const;

export class App {
  /** Binds the old arrow functions of the class, which the parts hold as methods, before a field reads one. */
  readonly bound = bindAll(this, bounds);

  readonly root: BoxRenderable;
  readonly composer: GhostTextarea;
  readonly scroll: ScrollBoxRenderable;
  style: ReturnType<typeof syntax>;
  theme: ThemeName = defaultTheme;
  draftKey = "";
  hover?: BoxRenderable;
  questionDocument?: ScrollBoxRenderable;
  hoverTimer?: ReturnType<typeof setTimeout>;
  editorVersion = 0;
  closed = false;
  /** The keys of the App, as layers of the keymap of its renderer, and what takes each layer away again. */
  readonly keymap: Keymap<Renderable, KeyEvent>;
  readonly layersOff: (() => void)[];
  /** The sidebar at the right: the session, its chains and its usage, then the workspaces, which scroll. */
  readonly rail: BoxRenderable;
  readonly railSession: BoxRenderable;
  readonly railHeading: BoxRenderable;
  readonly railSpaces: ScrollBoxRenderable;
  readonly railUsage: BoxRenderable;
  /** The program of each thread and each word of the operator, as it stood at a version of the acts and of the words
   * that its models write. */
  programs = new Map<string, { session: unknown; version: number; live: string; lines: ProgramLine[] }>();
  /** What each thread showed when it was last seen open, which tells a folded thread that new work came. */
  readonly seenThreads = new Map<string, { seen: string; folded: boolean }>();
  /** The rows that a model writes, by the step that it writes, which the tick draws again, each with the lines that
   * the card above it shows. */
  thinkingRows = new Map<
    string,
    {
      view: LineNumberRenderable;
      text: TextRenderable;
      display: number;
      room: number;
      said: string;
      wrote: number;
    }
  >();
  /** Whether the keys move in the sidebar, after ⌥T, and the row that they stand on: a workspace, a session, or a
   * thread, by the id that the row gets in railOrder. */
  railKeys = false;
  railAt?: string;
  /** The ids of the rows of the sidebar, in the order that it shows them, which the keys move through, and what the
   * keys do on each. */
  railOrder: string[] = [];
  readonly railHandlers = new Map<string, RailHandler>();
  /** The workspace that held the focus when the sidebar was last drawn: when it changes, the others fold again. */
  railFocus?: string;
  /** The row that the sidebar last scrolled to, so that the wheel scrolls free until the focus moves. */
  railWatched?: string;
  /** The next question that the key for the next question goes to. */
  railNext = 0;
  /** The threads of the session shown on every chain, read again only when its prompts or its questions change. */
  railThreadCache?: {
    session: unknown;
    version: number;
    owed: string;
    list: ChainThread[];
    chains: Map<string, ChainRead>;
  };
  /** The key of the rows of the threads of the sidebar, by the list that it came from and the minute. */
  railThreadKey?: { list: ChainThread[]; minute: number; text: string };
  /** The threads of the chain shown, read again only when the acts or the questions that wait change. */
  threadList: ThreadList = {
    session: undefined,
    chain: "",
    acts: undefined,
    owed: "",
    list: [],
    by: new Map(),
    of: new Map(),
    asked: new Map(),
  };
  /** The lists of the sidebar that the operator unfolded: the finished chains, under the key finished, and the
   * archived sessions of a workspace, under its folder. */
  readonly unfolded = new Set<string>();
  /** The session or the workspace that the operator renames in its row, and the name typed so far. */
  renaming?: {
    item: SessionEntry | Workspace;
    value: string;
    cursor?: number;
    input?: InputRenderable;
  };
  /** The session or the workspace whose menu is open, whose row stays lit under it. */
  menuItem?: SessionEntry | Workspace;
  /** The switch of the mode of the input, which the layout places. */
  readonly modeBox: BoxRenderable;
  /** Where the operator is, at the left of the top line, and the toggle of the views at its right. */
  readonly headline: BoxRenderable;
  readonly toggle: BoxRenderable;
  readonly status: TextRenderable;
  /** The keys that the footer offers for what the operator can do now, each a button. */
  readonly hints: BoxRenderable;
  /** The mode, the model, the effort, and the shape of the next message, under the input, each a button. */
  readonly meta: BoxRenderable;
  /** The rows of half blocks above and below the composer, and its bar, which the mode colors. */
  readonly composeEdges: BoxRenderable[];
  readonly composeBox: BoxRenderable;
  /** The values of a question of a choice, which the box of the prompt offers in place of its input. */
  readonly pickBox: BoxRenderable;
  /** The question that the list offers the values of, the row of its pointer, whether Other takes the input, and the
   * rows that a list of many values has ticked. */
  choice = { id: "", at: 0, other: false, ticked: [] as number[] };
  pickKey = "";
  readonly queueBox: BoxRenderable;
  readonly imageBox: BoxRenderable;
  /** The suggestions for a `/command` or an `@path` typed in the input, drawn above it while the input keeps focus. */
  readonly suggestionBox: BoxRenderable;
  suggestions: Suggestion[] = [];
  suggestionIndex = 0;
  /** The token whose suggestions Escape hid, which a change of the token shows again. */
  dismissed = "";
  lastToken = "";
  readonly search: InputRenderable;
  readonly searchRow: BoxRenderable;
  /** The bar above the feed while the rewind tree is open, which says what a choice does. */
  readonly treeBar: BoxRenderable;
  /** The bar of a life that the operator watches, pinned over the feed, so that it stays as the feed scrolls. */
  readonly watchBar: BoxRenderable;
  watchKey = "";
  /** Whether the feed shows the welcome, whose check of the setup takes the key r while the input is empty. */
  welcoming = false;
  /** The view of the life that shows over the feed: the loom or the graph, drawn as rows of text. */
  readonly stage: TextRenderable;
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
  readonly stepping = new Map<string, number>();
  /** What waits for the operator, over the input, and the key of what it shows. */
  readonly deskBox: BoxRenderable;
  deskKey = "";
  /** The parts of the extensions for the TUI, which add panels, rows, keys and commands. */
  readonly parts: Parts;
  /** The panels of the parts over the input, the key of what it shows, and what takes the keys of the parts away. */
  readonly dock: BoxRenderable;
  dockKey = "";
  partKeysOff?: () => void;
  /** Whether a part drew the rows of a kind at the last load. */
  partRows = false;
  /** The ask that the answer dialog shows, which Tab moves from. */
  answering?: string;
  /** The rewind tree, which the feed shows in place of its cards while it is open: its rows, and the row that the
   * pointer stands on. */
  tree?: { rows: TreeRow[]; selected: string; title: string; panel?: ChainsPanel };
  readonly treeFolds = new Map<string, boolean>();
  /** When the last Escape that had nothing to close was pressed. */
  escapedAt = 0;
  readonly paneKeys = new WeakMap<Renderable, string>();
  /** The node that the last press of a button reached. */
  pressed: Renderable | null = null;
  /** Where the last press was, so that a click still counts when the view drew its node again between the press and
   * the release. */
  pressedAt?: { x: number; y: number };
  readonly cards = new Map<
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
  scrollTarget?: Scroll | { card: string };
  overlay?: BoxRenderable;
  /** The veil behind the dialog that the overlay holds. */
  backdrop?: BoxRenderable;
  paletteInput?: InputRenderable;
  /** The row of the filter of the palette, with its prompt. */
  paletteInputRow?: BoxRenderable;
  paletteWidth = 80;
  /** The rows of the note under the title of the palette. */
  paletteNote = 0;
  paletteList?: BoxRenderable;
  filtered: Choice[] = [];
  selection = 0;
  /** The first choice that the list shows, and whether each choice takes rows for its command and its detail. */
  paletteStart = 0;
  rich = false;
  redraw?: ReturnType<typeof setTimeout>;
  lastView = "";
  /** Whether this run is the first one, whose welcome shows the check of the setup. */
  readonly firstRun: boolean;
  submitting = false;
  historyIndex = -1;
  historyDraft = "";
  /** The text typed when a walk of the history began, which a walk keeps to the entries that start with it, as fish
   * does. An empty prefix walks every entry. */
  historyPrefix = "";
  diagnosticsKey = "";
  /** What the footer shows, set again only when it changes. */
  statusKey = "";
  /** Whether the footer shows a spinner, which the tick turns. */
  statusMoves = false;
  /** The whole text of the footer, which a notice cut to its room shows in a tip. */
  statusWhole = "";
  readonly tick: ReturnType<typeof setInterval>;
  /** Whether the view serves a screen reader, as the preferences said when the view was made. */
  readonly reader: boolean;
  /** Whether the terminal has the focus, as it said last, or null while it said nothing. */
  focused: boolean | null = null;
  /** The questions to the operator that the view saw, so that it calls him once for each new one. */
  readonly called = new Set<string>();
  readonly navigation: {
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
    this.watchBar = this.box({ id: "watch-bar", visible: false, flexShrink: 0 });
    center.add(this.watchBar);
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
    this.composer = new GhostTextarea(renderer, {
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
        // A record open to read takes no text: its composer stays empty.
        if (this.session.host.reading && this.composer.plainText) {
          this.composer.setText("");
          return;
        }
        // The target of the message holds from its first character until it is sent or cleared.
        const w = this.session;
        if (!this.composer.plainText) w.pinned = undefined;
        else if (w.pinned === undefined) w.pinned = w.target ?? null;
        this.schedule();
        void this.highlightEditor();
        this.suggest();
        this.ghosting();
      },
      onCursorChange: () => {
        void this.highlightEditor();
        this.suggest();
        this.ghosting();
      },
      onSubmit: () => {
        void this.submit();
      },
    });
    this.pickBox = this.box({ id: "choices", visible: false });
    this.composeBox.add(this.pickBox);
    this.composeBox.add(this.composer);
    this.composer.tint = () => c.faint;
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
    this.railSession = this.box({ gap: space.stack });
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
      paddingX: space.inset,
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
      if (this.thinkingRows.size) this.tickThinking();
      if (this.tree?.panel?.live) this.renderChains();
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
   * opens to read: it names the build that made the record and this build, each on a line of its own, says what a new
   * life from the record carries, and offers to read the record, or to start a new life in its workspace. */
  readOrRenew(): void {
    const reading = this.session.host.reading;
    if (!reading) return;
    this.session.readingTold = true;
    const prompts = reading.carries?.prompts ?? 0;
    this.openPalette(
      sameBuild(reading) ? "This record does not replay" : "This record was made by another build",
      [
        { label: "Read it", detail: "See what it kept. No word runs again.", run: () => {} },
        {
          label: "Resume it",
          detail: prompts
            ? `Go on from its turns, ask ${prompts} prompt${prompts === 1 ? "" : "s"} again`
            : "Go on from its turns",
          run: () => this.startFrom(true),
        },
        {
          label: "Start a new life from it",
          detail: "A new session with its earlier work as text",
          run: () => this.startFrom(),
        },
      ],
      { note: readingNote(reading) },
    );
  }
  /** A new life in the workspace of a record open to read, whose first word carries the earlier work of the record as
   * text, or, with a resume, resumes the record: the new life goes on from its turns and asks its open prompts again. */
  startFrom(resume = false): void {
    const library = this.options.workspaces;
    void this.session.host
      .carry()
      .then((carry) => library.create(library.groupOf(), undefined, carry, resume))
      .then(() => {}, this.session.fail);
  }

  box(options: BoxOptions = {}): BoxRenderable {
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
  row(options: BoxOptions = {}): BoxRenderable {
    return this.box({ flexDirection: "row", height: space.bar, ...options });
  }
  /** A handler of the release of the left button that runs only for a click: a press and a release on one node that
   * selected no text between them. A drag over a button selects its text and does nothing else, and the release of
   * a press that opened a dialog does not close it. A node that the view drew again between the press and the
   * release is the same node when the pointer did not move. */
  click(run: (event: MouseEvent) => void): (event: MouseEvent) => void {
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
  hoverable<T extends BoxRenderable | TextRenderable>(node: T, color = c.raised): T {
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
  modeChanged = (mode: "dark" | "light"): void => {
    this.session.preferences.terminalMode = mode;
    this.render();
  };
  /** The terminal took the focus, as it says. */
  focusTaken = (): void => {
    this.focused = true;
  };
  /** The terminal lost the focus, as it says. */
  focusLost = (): void => {
    this.focused = false;
  };
  /** Each new question to the operator calls him while the terminal has no focus: a notification of the terminal, as
   * OSC 9 shows it, or the bell where the terminal shows none. A question that waited when the view was made calls
   * nobody. */
  call(asks: readonly { id: string; chain: string; label: string; message: string }[]): void {
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
  toggleAccess(which: "reader" | "motion"): void {
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
  accessPicker = (): void => {
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
  copySelection = (selection: { getSelectedText(): string } | null): void => {
    const text = selection?.getSelectedText() ?? "";
    if (this.closed || !text.trim()) return;
    this.renderer.copyToClipboardOSC52(text);
    const length = [...graphemes.segment(text)].length;
    this.session.notice = `Copied ${length} ${length === 1 ? "character" : "characters"}.`;
  };
  /** A text, which runs an action at a click when it has one. */
  text(content: string | Part[], fg = c.text, { run, ...options }: TextShape = {}): TextRenderable {
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
  line(content: string | Part[], fg = c.text, options: TextShape = {}): TextRenderable {
    return this.text(content, fg, { height: space.bar, ...options });
  }
  /** A text cut to its room shows the whole of it in a tip while the pointer is over it: the text that the terminal
   * cut, or the whole that a caller gives for a text that it cut itself. The tip takes the hover handlers of the text,
   * since OpenTUI keeps one of each and gives none back. */
  whole(node: TextRenderable, full?: () => string): TextRenderable {
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
  /** Whether a change waits for the next frame to draw the view again. */
  due = false;

  render = (): void => {
    if (this.closed) return;
    const w = this.session;
    this.parts.heard(w.written);
    motion.still = w.preferences.still;
    if (w.theme !== this.theme) this.applyTheme(w.theme);
    this.call(w.desk.asks);
    if (this.draftKey !== this.composeKey()) this.showDraft(this.composeKey());
    this.orphans();
    // The mode of a screen reader shows one column of text, and no sidebar beside it.
    this.rail.visible = this.sidebarShown;
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
    const reading = w.host.reading;
    const [mode, modeColor, placeholder] = reading
      ? ["Read only", c.faint, readOnly(reading)]
      : w.editing
        ? ["Edit program", c.warning, "Edit this prompt's Python program"]
        : pending
          ? [
              "Answer",
              c.warning,
              choosing
                ? `Your own words in place of ${wants(pending)}. Esc goes back to the values.`
                : `Your answer to ${pending.id}, as ${pending.shape}${pending.hint ? `: ${pending.hint}` : ""}`,
            ]
          : w.mode === "python"
            ? ["Python", c.secondary, "Write Python. The gate reads it before it runs."]
            : this.setupState().none
              ? [
                  "Prompt",
                  c.danger,
                  "No model answers yet. Fix the setup above, or watch a life, which needs none.",
                ]
              : w.target
                ? ["Prompt", threadTone(w.target), "Reply to the thread, or type / for commands"]
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
    const listed = choosing ? this.rows(choosing).length + 1 : 0;
    this.composeBox.height =
      (choosing && !this.choice.other ? listed : listed + typed) + space.section + space.bar;
    this.renderMeta(mode, modeColor);
    this.renderDock();
    this.renderDesk();
    this.renderStatus();
    this.renderContent();
    this.renderRail();
    this.suggest();
    this.ghosting();
    const diagnostics = JSON.stringify([w.rejectedWord, w.findings]);
    if (diagnostics !== this.diagnosticsKey) {
      this.diagnosticsKey = diagnostics;
      void this.highlightEditor();
    }
  };

  /** The cards of the feed in their order, read once while no card moves, since each read of them copies them. */
  feedOrder?: Renderable[];
  sourceGroups = new Map<string, Promise<(string | undefined)[]>>();
  /** The highlights that the parser gave each line of code last, by its filetype and its text, the newest last. */
  readonly paintedLines = new Map<string, SimpleHighlight[]>();

  readonly coloring = new Map<string, Promise<void>>();
  /** The loom that the threads of the sidebar read last, and what it was woven of, which a frame of the same life
   * reads again. */
  woven?: { of: unknown[]; loom: Loom };
  /** The acts of the life as they stood at the time cursor, which the loom and the graph read. */
  stageActs: readonly ActRow[] = [];
  /** The loom of the life at the time cursor, which the last draw of the stage read. */
  stageLoom: Loom = { rows: [], owed: [], folded: 0 };
  /** The desk over the input: one mark for each ask that waits for the operator, in the order that they came, then
   * the ask that he acts on, from which chain, and what it wants, the first line of its message, and its choices or
   * where its answer goes. A click on a mark points at its ask and shows its chain, and a click on the message opens
   * the ask whole. */
  /** The parts once they loaded: their keys again, and each card again, since a part may draw the row of its act. */
  /** The count of the loads of the parts, which a label that a part gives keys on. */
  partsLoaded = 0;
  /** The words that the turns of the chain shown tell of its rungs, by rung, which the program of the chain does not
   * hold for a word that the gate refused. */
  told: { turns: readonly Turn[]; at: number; rung?: ActRow; words: Map<string, string> } = {
    turns: [],
    at: 0,
    words: new Map(),
  };

  /** The lines of the programs whose strings the operator opened, as read from the folds of the session. */
  stringLines?: { folds: Record<string, boolean>; lines: Set<string> };

  /** What each act that a thread waits on does, and what each thread that ended came to, which hold while their act
   * and their state stay, so that a frame reads none of them again. */
  doing = new Map<string, string>();
  /** What each prompt that the list of the threads read is: a message to a thread, or a thread and its title. */
  readonly promptsRead = new WeakMap<LiveAct, { message: string | undefined; title: string }>();
  /** The values that the first argument of a command may take, for each command whose values the TUI knows. A value
   * of a command that takes more after it completes, and a value of any other command runs the command when it is
   * chosen. */
  readonly values: Partial<Record<string, () => Value[]>> = this.valuesTable();
  authenticationOpen = false;
  authenticationUrl = "";
  go(view: View, id?: string): void {
    this.navigation.push({
      chain: this.session.selected,
      view: this.session.view,
      search: this.session.search,
      place: this.place,
      mode: this.session.mode,
    });
    this.closeTree();
    this.session.show(view);
    const head = id ? this.reveal(id) : undefined;
    this.render();
    if (id) this.scrollAfterLayout({ card: head && !this.cards.has(id) ? head : id });
  }
  /** Open each fold of the feed that holds an act, so that a jump to it finds its card: the earlier steps of its
   * chain, the earlier steps of each prompt and rung over it, and the rung that made it. An act that a thread made
   * has no card of its own, so a jump to it opens the box of its thread and the windows of code in it, and gives the
   * thread whose card it goes to. */
  reveal(id: string): string | undefined {
    const w = this.session;
    const rows = new Map(w.acts.map((one) => [one.id, one]));
    let at = rows.get(id);
    if (!at || at.on !== w.selected) return undefined;
    w.folds[`prompts:${w.selected}`] = false;
    let head: string | undefined;
    const threads = this.chainThreadsBy(w.selected).by;
    for (let step = 0; at && step < 64; step++) {
      const maker = rows.get(at.by);
      if (!maker || maker.on !== w.selected) break;
      if (maker.kind === "prompt") {
        w.folds[`rungs:${maker.id}`] = false;
        if (threads.has(maker.id)) head = maker.id;
      }
      if (maker.kind === "rung") w.folds[at.kind === "prompt" ? `prompts:${maker.id}` : maker.id] = false;
      at = maker;
    }
    if (head) {
      w.folds[`thread:${head}`] = false;
      w.folds[`window:${head}`] = false;
    }
    return head;
  }
  async back(): Promise<void> {
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
  scrollAfterLayout(target?: Scroll | { card: string }): void {
    if (target === undefined) return;
    if (this.scrollTarget === undefined) this.renderer.once("frame", this.laidOut);
    this.scrollTarget = target;
  }
  laidOut = (): void => {
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
  scrollNow(place: Scroll): void {
    this.scroll.scrollTo(place === "end" ? this.scroll.scrollHeight : place);
  }
  /** Where the view stands, which is where it scrolls to once it is laid out: the end of a view that follows its end
   * while it stands there, and its offset otherwise. */
  get place(): Scroll {
    const target = this.scrollTarget;
    if (target !== undefined && typeof target !== "object") return target;
    const { scrollTop, scrollHeight, viewport, stickyScroll } = this.scroll;
    return stickyScroll && scrollTop >= scrollHeight - viewport.height ? "end" : scrollTop;
  }
  /** The chain a step away from the one shown, in the order of the chains, round from the last to the first. */
  rollChain(step: number): void {
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
  /** What each action of the table of keys does, with the key that ran it. */
  readonly actions: Record<Action, (key?: KeyEvent) => unknown> = this.actionsTable();
  /** When an action of the table acts, where it does not act at all times: a key that does not act goes on to the
   * input, and the footer does not offer it. */
  readonly when: Partial<Record<Action, () => boolean>> = this.whenTable();
}

export interface App
  extends BlocksPart,
    ComposerPart,
    ChromePart,
    FeedPart,
    WelcomePart,
    StagePart,
    DeskPart,
    QuestionsPart,
    DetailsPart,
    SidebarPart,
    RailPart,
    SpacesPart,
    ActionsPart,
    PalettePart,
    SettingsPart,
    ValuesPart,
    TreePart,
    ThreadPart,
    LoomsPart,
    InputPart {}
assemble(
  App,
  blocks,
  composer,
  chrome,
  feed,
  welcome,
  stage,
  desk,
  questions,
  details,
  sidebar,
  rail,
  spaces,
  actions,
  palette,
  settings,
  values,
  tree,
  thread,
  looms,
  input,
);

import { isAbsolute, relative } from "node:path";
import { display, type FileChange, safeText } from "@furb/engine";
import {
  BoxRenderable,
  type CliRenderer,
  CodeRenderable,
  type KeyEvent,
  type MouseEvent,
  type RGBA,
  ScrollBoxRenderable,
  TextareaRenderable,
  TextRenderable,
} from "@opentui/core";
import { createTwoFilesPatch } from "diff";
import { conversation } from "./conversation.ts";
import { shortenHome } from "./files.ts";
import { ago, clip, dollars, elapsed } from "./format.ts";
import { Graph, type Stage } from "./graph.ts";
import { hints, keyTable } from "./keys.ts";
import { type ActState, type Line, ladder, valueText } from "./ladder.ts";
import {
  asOf,
  type Cell,
  type Loom,
  momentsOf,
  owedAt,
  type Thread,
  type ThreadState,
  weave,
} from "./loom.ts";
import { loadParsers } from "./parsers.ts";
import { type ActRow, type Exit, type Session, type SessionStatus, statusLabels } from "./session.ts";
import { theme as c, defaultTheme, frame, setTheme, spin, syntax, type ThemeName } from "./theme.ts";
import { bold, logo, mix, type Part, plain, styled } from "./ui.ts";
import type { SessionEntry, Workspace, Workspaces } from "./workspaces.ts";

export interface AppOptions {
  quit(): void | Promise<void>;
  workspaces: Workspaces;
  /** The variant that the App shows first. */
  variant?: Variant;
}

/** The whole directions of the TUI over one model of a life: the loom of the chains in time, and the graph of the
 * programs that run, a debugger with levels of zoom. */
export const variants = ["loom", "graph"] as const;
export type Variant = (typeof variants)[number];

/** An App on the session that the library selects, which a new App replaces for each session that the library
 * selects later. It gives the App that shows now. */
export function follow(renderer: CliRenderer, options: AppOptions): () => App {
  const session = options.workspaces.current?.session;
  if (!session) throw new Error("The library has no session selected.");
  let app = new App(renderer, session, options);
  options.workspaces.on("select", (next: Session) => {
    if (app.session === next) return;
    const variant = app.variant;
    app.dispose();
    app = new App(renderer, next, { ...options, variant });
  });
  return () => app;
}

/** Where the keys go: the loom of the threads, the thread of the selected chain, the lens of one act, or the word
 * line. */
export type Focus = "loom" | "thread" | "lens" | "word" | "lives";
/** What the word line makes of its text: a prompt to a model, a word of python, a command, the answer of a prompt
 * that the operator owes, the ceiling of a grant, the value that closes an act, or the program of a door. */
export type Mode = "prompt" | "python" | "command" | "answer" | "grant" | "close" | "door";

/** The mark and the color of each state of a thread. */
const threadMarks: Record<ThreadState, [string, () => RGBA]> = {
  asks: ["◆", () => c.warning],
  failed: ["✗", () => c.danger],
  working: ["◉", () => c.accent],
  paused: ["◌", () => c.warning],
  done: ["✓", () => c.success],
  idle: ["○", () => c.faint],
};
/** The mark and the color of each state of a life in the list of lives. */
const lifeMarks: Record<SessionStatus, [string, () => RGBA]> = {
  working: ["◉", () => c.accent],
  opening: ["◉", () => c.accent],
  blocked: ["◆", () => c.warning],
  paused: ["◌", () => c.warning],
  error: ["✗", () => c.danger],
  done: ["●", () => c.success],
  idle: ["○", () => c.faint],
  saved: ["○", () => c.faint],
};
/** The mark and the color of each state of an act. */
const actMarks: Record<ActState, [string, () => RGBA]> = {
  running: ["", () => c.accent],
  stands: ["◇", () => c.muted],
  done: ["✓", () => c.success],
  failed: ["✗", () => c.danger],
  cancelled: ["⊘", () => c.faint],
  held: ["◌", () => c.warning],
  asks: ["◆", () => c.warning],
};
const toneColor = (cell: Cell): RGBA =>
  ({ work: c.accent, past: c.muted, asks: c.warning, failed: c.danger, none: c.faint })[cell.tone];
/** The ways to start that an empty thread offers, each a prompt that Enter puts in the word line. */
export const starters = [
  ["Explore a codebase", "Read the README and explain how this project works."],
  ["Make something better", "Find one useful improvement in this project and implement it."],
  ["Start with a plan", "Read the project and propose a small, testable plan."],
] as const;
/** What the word line says before its text, and what it says while it is empty, in each mode. */
const modeLabels: Record<Mode, string> = {
  prompt: "ask",
  python: "python",
  command: "/",
  answer: "answer",
  grant: "grant $",
  close: "close",
  door: "door",
};
export class App {
  readonly root: BoxRenderable;
  readonly word: TextareaRenderable;
  focus: Focus = "loom";
  mode: Mode = "prompt";
  /** The thread that the loom selects, and the line that the thread selects. */
  selectedThread = 0;
  selectedLine = 0;
  /** The act that the word line closes or answers, or the prompt whose door it edits. */
  target = "";
  /** The threads whose workers the loom folds into them. */
  readonly folds = new Set<string>();
  /** What the loom and the thread show now, as the last render made them. */
  loom: Loom = { threads: [], owed: [], folded: 0 };
  lines: Line[] = [];
  help = false;
  /** The row that the list of lives selects. */
  lifeIndex = 0;
  /** The act that the lens shows, and the whole of it, once the session gave it. */
  lens?: ActRow;
  private whole?: ActRow;
  /** The changes of the files, which the lens of a write reads its own from. */
  private changes: FileChange[] = [];
  /** The moment that the time cursor stands at, and nothing at the present. */
  cursor?: number;
  /** The variant that shows, and the graph, which the variant graph draws on the stage. */
  variant: Variant;
  readonly graph: Graph;
  /** The acts as they stood at the time cursor, which the last render read. */
  acts: readonly ActRow[] = [];
  /** The chains that a step lets run for one rung, with the count of their rungs that had ended when it began. */
  readonly stepping = new Map<string, number>();
  private readonly stage: TextRenderable;
  private readonly header: TextRenderable;
  private readonly ruler: TextRenderable;
  private readonly threads: TextRenderable;
  private readonly rule: TextRenderable;
  private readonly thread: TextRenderable;
  private readonly lensBox: ScrollBoxRenderable;
  private readonly lensHead: TextRenderable;
  private readonly lensCode: CodeRenderable;
  private readonly lensBody: TextRenderable;
  private readonly wordLabel: TextRenderable;
  private readonly wordMeta: TextRenderable;
  private readonly footer: TextRenderable;
  private readonly overlay: BoxRenderable;
  private readonly helpText: TextRenderable;
  private style: ReturnType<typeof syntax>;
  private theme: ThemeName = defaultTheme;
  private closed = false;
  private redraw?: ReturnType<typeof setTimeout>;
  private readonly tick: ReturnType<typeof setInterval>;
  /** The first line of the thread and the first thread of the loom that show, which follow the selection. */
  private threadTop = 0;
  private loomTop = 0;
  /** When the last ⌃D was pressed on an empty word line, which a second one within a moment makes a quit. */
  private quitAt = 0;
  /** Whether the loom has put its selection on the selected chain once. */
  private placed = false;
  /** The row of the thread that each line of the thread text starts at, and the line of each row. */
  private rowsOfLines: number[] = [];

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
      paddingX: 2,
      backgroundColor: c.background,
    });
    renderer.root.add(this.root);
    this.header = this.text({ id: "header", height: 1 });
    this.root.add(this.header);
    this.ruler = this.text({ id: "ruler", height: 1, marginTop: 1 });
    this.root.add(this.ruler);
    this.threads = this.text({
      id: "loom",
      onMouseUp: (event) => this.clickThread(event),
    });
    this.root.add(this.threads);
    this.rule = this.text({ id: "rule", height: 1, marginTop: 1 });
    this.root.add(this.rule);
    this.variant = options.variant ?? "loom";
    this.stage = this.text({
      id: "stage",
      flexGrow: 1,
      flexShrink: 1,
      minHeight: 0,
      visible: false,
      onMouseUp: (event) => {
        if (this.graph.click(event.y - this.stage.y)) this.render();
      },
    });
    this.root.add(this.stage);
    this.graph = new Graph(this.stageOf());
    this.thread = this.text({
      id: "thread",
      flexGrow: 1,
      flexShrink: 1,
      minHeight: 0,
      onMouseUp: (event) => this.clickLine(event),
    });
    this.root.add(this.thread);
    this.lensBox = new ScrollBoxRenderable(renderer, {
      id: "lens",
      flexGrow: 1,
      flexShrink: 1,
      minHeight: 0,
      visible: false,
      scrollX: false,
      contentOptions: { gap: 1 },
    });
    this.lensBox.verticalScrollBar.visible = false;
    this.lensBox.horizontalScrollBar.visible = false;
    this.lensHead = this.text({ id: "lens-head" });
    this.lensCode = new CodeRenderable(renderer, {
      id: "lens-code",
      content: "",
      filetype: "python",
      syntaxStyle: this.style,
      wrapMode: "word",
      drawUnstyledText: true,
    });
    this.lensBody = this.text({ id: "lens-body" });
    this.lensBox.add(this.lensHead);
    this.lensBox.add(this.lensCode);
    this.lensBox.add(this.lensBody);
    this.root.add(this.lensBox);
    const line = this.box({ flexDirection: "row", marginTop: 1, backgroundColor: c.panel, paddingX: 1 });
    this.wordLabel = this.text({ id: "word-label", height: 1, flexShrink: 0 });
    line.add(this.wordLabel);
    this.word = new TextareaRenderable(renderer, {
      id: "word",
      flexGrow: 1,
      minHeight: 1,
      maxHeight: 8,
      backgroundColor: c.panel,
      focusedBackgroundColor: c.panel,
      textColor: c.text,
      focusedTextColor: c.text,
      placeholderColor: c.faint,
      cursorColor: c.accent,
      keyBindings: [
        { name: "return", action: "submit" },
        { name: "return", shift: true, action: "newline" },
        { name: "j", ctrl: true, action: "newline" },
      ],
      onSubmit: () => void this.submit(),
      onContentChange: () => this.schedule(),
    });
    line.add(this.word);
    this.wordMeta = this.text({ id: "word-meta", height: 1, flexShrink: 0 });
    line.add(this.wordMeta);
    this.root.add(line);
    this.footer = this.text({ id: "footer", height: 1, marginTop: 1 });
    this.root.add(this.footer);
    this.overlay = this.box({
      id: "help",
      position: "absolute",
      left: 0,
      top: 1,
      right: 0,
      bottom: 0,
      visible: false,
      zIndex: 10,
      padding: 1,
      paddingX: 2,
      backgroundColor: c.raised,
      border: true,
      borderColor: c.border,
    });
    this.helpText = this.text({ id: "help-text" });
    this.overlay.add(this.helpText);
    renderer.root.add(this.overlay);
    // The draft that the word line held when the App on this session last ended comes back.
    const draft = session.drafts[session.draftKey];
    if (draft) this.word.setText(draft);
    session.on("change", this.schedule);
    session.on("compose", this.compose);
    options.workspaces.on("change", this.schedule);
    renderer.keyInput.on("keypress", this.press);
    renderer.on("resize", this.render);
    this.tick = setInterval(() => {
      if (
        this.loom.threads.some((thread) => thread.state === "working") ||
        this.lines.some(running) ||
        (this.variant === "graph" && this.loom.owed.length)
      )
        this.render();
    }, frame);
    void loadParsers().then(() => {
      if (this.closed) return;
      this.lensCode.filetype = undefined;
      this.lensCode.filetype = "python";
    }, session.fail);
    this.render();
  }

  private box(options: ConstructorParameters<typeof BoxRenderable>[1] = {}): BoxRenderable {
    return new BoxRenderable(this.renderer, { flexDirection: "column", flexShrink: 0, ...options });
  }
  private text(options: ConstructorParameters<typeof TextRenderable>[1] = {}): TextRenderable {
    return new TextRenderable(this.renderer, {
      content: "",
      fg: c.text,
      wrapMode: "none",
      flexShrink: 0,
      ...options,
    });
  }
  private schedule = (): void => {
    if (this.closed || this.redraw) return;
    this.redraw = setTimeout(() => {
      this.redraw = undefined;
      this.render();
    }, 30);
  };
  /** The columns of a thread of the loom that are not its cells: the mark, the label, what it waits on, and its
   * measure, which a narrow terminal leaves out. */
  get fixed(): number {
    return 4 + this.labelWidth + 1 + 2 + 12 + (this.wide ? 36 : 0);
  }
  private get wide(): boolean {
    return this.width >= 120;
  }
  private get labelWidth(): number {
    return this.width >= 100 ? 26 : 18;
  }
  /** The columns between the gutters. */
  private get width(): number {
    return Math.max(40, this.renderer.width - 4);
  }

  render = (): void => {
    if (this.closed) return;
    const w = this.session;
    if (w.theme !== this.theme) this.applyTheme(w.theme);
    const cellWidth = Math.max(8, this.width - this.fixed);
    // The time cursor shows the life as it stood at its moment, and the present shows it as it stands.
    const then = this.cursor;
    const acts = then === undefined ? w.acts : asOf(w.acts, then);
    this.acts = acts;
    this.steps();
    this.loom = weave({
      acts,
      prompts: then === undefined ? [...w.host.prompts.values()] : owedAt(acts),
      measures: w.measures,
      folds: this.folds,
      width: cellWidth,
      pending: then === undefined ? w.host.pending : new Map(),
      label: (id) => w.labelOf(id),
    });
    const owed = new Set(this.loom.owed.map((prompt) => prompt.id));
    const words = this.variant === "loom" ? this.words() : new Map<string, string>();
    // The graph reads the source of a chain, and the loom its ladder.
    this.lines =
      this.variant === "loom"
        ? ladder(
            w.selected,
            acts,
            (rung) => words.get(rung.id) ?? "",
            then === undefined ? w.host.streams : new Map(),
            owed,
          )
        : [];
    this.selectedThread = Math.min(this.selectedThread, this.loom.threads.length - 1);
    const selected = this.loom.threads.findIndex((thread) => thread.id === w.selected);
    if ((this.focus !== "loom" || !this.placed) && selected >= 0) {
      this.selectedThread = selected;
      this.placed = true;
    }
    this.selectedLine = Math.max(0, Math.min(this.selectedLine, (this.lines.length || starters.length) - 1));
    this.renderHeader();
    const graph = this.variant === "graph";
    this.ruler.visible = this.threads.visible = this.rule.visible = !graph;
    if (!graph) {
      this.renderLoom(cellWidth);
      this.renderRule();
    }
    this.lensBox.visible = this.focus === "lens" && Boolean(this.lens);
    this.thread.visible = !this.lensBox.visible && (!graph || this.focus === "lives");
    this.stage.visible = graph && !this.lensBox.visible && this.focus !== "lives";
    if (this.lensBox.visible) this.renderLens();
    else if (this.focus === "lives") this.renderLives();
    else if (graph) this.renderStage();
    else this.renderThread();
    this.renderWord();
    this.renderFooter();
    this.overlay.visible = this.help;
    if (this.help) this.renderHelp();
    this.renderer.requestRender();
  };

  /** The words of the rungs of the selected chain: what its caller wrote, what ran, and what its model wrote, the
   * words that the gate refused among them. */
  private words(): Map<string, string> {
    const w = this.session;
    const words = new Map<string, string>();
    for (const item of conversation(w.turns, w.acts))
      if (item.type === "python" && item.rung) words.set(item.rung.id, item.code);
    for (const [id, word] of Object.entries(w.program)) words.set(id, word);
    for (const act of w.acts)
      if (act.kind === "rung" && act.words[0]) words.set(act.id, String(act.words[0]));
    return words;
  }

  private renderHeader(): void {
    const w = this.session;
    const working = this.loom.threads.filter((thread) => thread.state === "working").length;
    const owed = this.loom.owed.length;
    const moments = momentsOf(w.acts);
    // The header says the state by shape and color alone, and shows a count only where it is not zero.
    const right: Part[] = [
      ...(this.cursor !== undefined
        ? ([[`◷ ${moments.indexOf(this.cursor) + 1} of ${moments.length}   `, c.secondary, bold]] as Part[])
        : []),
      ...(w.host.pending.size ? ([[`◌ ${w.host.pending.size}   `, c.warning]] as Part[]) : []),
      ...(owed ? ([[`◆ ${owed}   `, c.warning, bold]] as Part[]) : []),
      ...(working ? ([[`◉ ${working}   `, c.accent]] as Part[]) : []),
      [dollars(w.cost), c.muted],
    ];
    const left: Part[] = [
      ["furb", c.accent, bold],
      ["   ", c.faint],
      [w.sessionName, c.text],
      ["   ", c.faint],
      [clip(shortenHome(w.host.directory), Math.max(10, this.width - 80), "end"), c.faint],
    ];
    this.header.content = styled(justify(left, right, this.width));
  }

  private renderLoom(cellWidth: number): void {
    const { threads, folded } = this.loom;
    const room = Math.max(3, Math.floor((this.renderer.height - 12) * 0.45));
    if (this.selectedThread < this.loomTop) this.loomTop = this.selectedThread;
    if (this.selectedThread >= this.loomTop + room) this.loomTop = this.selectedThread - room + 1;
    this.loomTop = Math.max(0, Math.min(this.loomTop, threads.length - room));
    const shown = threads.slice(this.loomTop, this.loomTop + room);
    const cellStart = 4 + this.labelWidth + 1;
    const span = this.loom.threads[0]?.cells.length ?? 0;
    const blank = cellWidth - span;
    const ruler: Part[] = [[" ".repeat(cellStart), c.faint]];
    if (folded && span) {
      const at = blank + folded;
      ruler.push([" ".repeat(Math.max(0, at - 7)), c.faint], ["older ┊", c.faint]);
      ruler.push(
        [" ".repeat(Math.max(0, cellWidth - Math.max(at, 7) - 4)), c.faint],
        [this.edge, this.edgeColor],
      );
    } else ruler.push([" ".repeat(Math.max(0, cellWidth - 4)), c.faint], [this.edge, this.edgeColor]);
    ruler.push(["  "], ["waits on".padEnd(12), c.faint]);
    if (this.wide) ruler.push(["dollars".padStart(20), c.faint], ["     window", c.faint]);
    this.ruler.content = styled(ruler);
    this.threads.height = shown.length;
    const rows = shown.map((thread, index) =>
      this.threadRow(thread, this.loomTop + index === this.selectedThread, cellWidth),
    );
    this.threads.content = styled(rows.flatMap((row, index) => (index ? [["\n"] as Part, ...row] : row)));
  }

  /** What the right edge of the loom is: the present, or the moment of the time cursor. */
  private get edge(): string {
    return this.cursor === undefined ? " now" : "then";
  }
  private get edgeColor(): RGBA {
    return this.cursor === undefined ? c.faint : c.secondary;
  }
  /** One thread of the loom: its mark, its label in the tree, its cells, what it waits on, and its measure. */
  private threadRow(thread: Thread, selected: boolean, cellWidth: number): Part[] {
    const [mark, color] = threadMarks[thread.state];
    const frozen = this.cursor !== undefined;
    const live = thread.state === "working" && !frozen ? spin() : mark;
    const ground = selected ? (this.focus === "loom" ? c.selected : c.panel) : undefined;
    const tally = thread.folded
      ? ` ${Object.entries(thread.tally)
          .map(([state, n]) => `${n}${threadMarks[state as ThreadState][0]}`)
          .join(" ")}`
      : "";
    const fold = thread.workers ? (thread.folded ? "▸ " : "▾ ") : "";
    const owed = thread.id === "operator" ? this.loom.owed.length : 0;
    const label = clip(`${thread.lines}${fold}${thread.label}${tally}`, this.labelWidth).padEnd(
      this.labelWidth,
    );
    // The past fades: a cell takes more of its color the nearer it stands to the present.
    const count = thread.cells.length;
    const cells: Part[] = thread.cells.map((cell, index) =>
      index === count - 1 && thread.state === "working" && !frozen
        ? [spin(), c.accent]
        : [cell.mark, mix(c.background, toneColor(cell), 0.45 + (0.55 * (index + 1)) / count)],
    );
    const waits =
      thread.id === "operator"
        ? owed
          ? `${owed} owed`
          : ""
        : thread.state === "asks"
          ? "asks you"
          : thread.state === "paused"
            ? "paused"
            : (thread.waits ?? "");
    const measure = this.measureParts(thread);
    const parts: Part[] = [
      [selected ? "▎" : " ", c.accent],
      [" "],
      [live, color()],
      [" "],
      [label, selected ? c.text : thread.state === "idle" ? c.muted : c.text, selected ? bold : 0],
      [" "],
      [" ".repeat(Math.max(0, cellWidth - cells.length)), c.faint],
      ...cells,
      ["  "],
      [
        clip(waits, 12).padEnd(12),
        thread.state === "asks" || thread.state === "paused" ? c.warning : c.muted,
      ],
      ...(this.wide ? measure : []),
    ];
    return pad(parts, this.width, ground);
  }

  /** The measure of a thread: its dollars against the ceiling of its grant, and the share of the window that its last
   * answer read, as a bar of eight steps to a cell. */
  private measureParts(thread: Thread): Part[] {
    const measure = thread.measure;
    if (!measure) return [[" ".repeat(36), c.faint]];
    const money = measure.ceiling
      ? `${dollars(measure.spent)} of ${dollars(measure.ceiling)}`
      : measure.answers
        ? dollars(measure.dollars)
        : "";
    const full = measure.ceiling && measure.spent >= measure.ceiling;
    const share = measure.share ?? 0;
    const bar = gauge(share, 6);
    return [
      [money.padStart(20), full ? c.warning : c.muted],
      ["  ▕", c.faint],
      [bar, share > 0.5 ? c.warning : c.accent],
      ["▏", c.faint],
      [
        measure.share === undefined
          ? "     "
          : (share > 0 && share < 0.01 ? "<1%" : `${Math.round(share * 100)}%`).padStart(5),
        c.faint,
      ],
    ];
  }

  private renderRule(): void {
    const w = this.session;
    const chain = w.selected;
    const left: Part[] = [
      [w.labelOf(chain), c.text, bold],
      ["  ", c.faint],
      [chain, c.muted],
      ["  ", c.faint],
      [w.actor, c.muted],
      ["  ", c.faint],
      [clip(shortenHome(w.workingDirectory), Math.max(10, this.width - 80), "end"), c.faint],
      [" ", c.faint],
    ];
    const used = Bun.stringWidth(plain(left));
    this.rule.content = styled([...left, ["─".repeat(Math.max(0, this.width - used)), c.border]]);
  }

  private renderThread(): void {
    const lines = this.lines;
    const height = Math.max(1, this.thread.height || this.renderer.height - 14);
    if (!lines.length) {
      this.thread.content = styled(this.welcome());
      return;
    }
    // Each line takes as many rows as the text it holds, and the window follows the selected line.
    const rendered = lines.map((line, index) => this.lineParts(line, index === this.selectedLine));
    const rows = rendered.map((parts) => plain(parts).split("\n").length);
    const start = (index: number) => rows.slice(0, index).reduce((sum, n) => sum + n, 0);
    const at = start(this.selectedLine);
    if (at < this.threadTop) this.threadTop = at;
    const end = at + (rows[this.selectedLine] ?? 1);
    if (end > this.threadTop + height) this.threadTop = end - height;
    // A thread that grows at its end and whose last line is selected follows its end.
    this.threadTop = Math.max(0, Math.min(this.threadTop, start(lines.length) - height));
    const all = rendered.flatMap((parts, index) => (index ? [["\n"] as Part, ...parts] : parts));
    const text = splitRows(all);
    this.rowsOfLines = rows.flatMap((n, index) => Array.from({ length: n }, () => index));
    this.thread.content = styled(
      text
        .slice(this.threadTop, this.threadTop + height)
        .flatMap((row, index) => (index ? [["\n"] as Part, ...row] : row)),
    );
  }

  /** The empty thread: the name of furb, what it does, and the ways to start. */
  private welcome(): Part[] {
    const center = (text: string) =>
      " ".repeat(Math.max(0, Math.floor((this.width - Bun.stringWidth(text)) / 2)));
    const parts: Part[] = [["\n"]];
    for (const row of logo) parts.push([center(logo[1] ?? ""), c.faint], [row, c.accent], ["\n"]);
    const say = "A life is a tree of chains. Each act has a name, a state and a place in time.";
    parts.push(["\n"], [center(say), c.faint], [say, c.muted], ["\n\n"]);
    for (const [index, [title, prompt]] of starters.entries()) {
      const selected = this.focus === "thread" && index === this.selectedLine;
      const row = `${selected ? "▎ " : "  "}${title}   ${prompt}`;
      parts.push([center(row), c.faint], [selected ? "▎ " : "  ", c.accent], [title, c.text, bold], ["   "]);
      parts.push([prompt, c.muted], ["\n"]);
    }
    const keys = "i ask   : python   tab answer   ? keys";
    parts.push(["\n"], [center(keys), c.faint], [keys, c.faint]);
    return parts;
  }

  /** One line of the thread, as parts. A line of more than one row holds line breaks. */
  private lineParts(line: Line, selected: boolean): Part[] {
    const indent = "  ".repeat(line.depth);
    const ground = selected ? (this.focus === "thread" ? c.selected : c.panel) : undefined;
    const bar: Part = [selected ? "▎" : " ", c.accent];
    const mark = (state: ActState): Part => {
      const [glyph, color] = actMarks[state];
      return [state === "running" ? spin() : glyph, color()];
    };
    const width = this.width;
    if (line.type === "tail") {
      const rows = line.text
        .split("\n")
        .map((text) =>
          pad(
            [
              [" "],
              [`${indent}  ▏ `, c.faint],
              [clip(safeText(text), width - indent.length - 6), line.model ? c.secondary : c.muted],
            ],
            width,
            ground,
          ),
        );
      return rows.flatMap((row, index) => (index ? [["\n"] as Part, ...row] : row));
    }
    if (line.type === "value") {
      const text = clip(valueText(line.act.value).split("\n")[0] ?? "", width - indent.length - 14);
      return pad([bar, [`${indent}  closed  `, c.faint], [text, c.prose]], width, ground);
    }
    const act = line.act;
    if (line.type === "ear") {
      const first = line.rungs[0]?.id ?? "",
        end = line.rungs.at(-1)?.id ?? "";
      const name = line.rungs.length > 1 ? `${first}…${end.replace(/^rung/, "")}` : first;
      const chips: Part[] = line.acts.flatMap((chip): Part[] => [
        ["  "],
        [chip.label, c.faint],
        [" "],
        [actMarks[chip.state][0] || spin(), actMarks[chip.state][1]()],
      ]);
      const count = line.rungs.length;
      return pad(
        [
          bar,
          [indent],
          mark(line.state),
          [" "],
          [name, c.faint],
          ["  "],
          [`${count} ${count === 1 ? "word" : "words"} of ${act.by}`, c.muted],
          ...chips,
        ],
        width,
        ground,
      );
    }
    const head: Part[] = [bar, [indent], mark(line.state), [" "], [act.id, c.muted], ["  "]];
    if (line.type === "prompt") {
      const [shape, message, to] = act.words.map(String);
      const owed = line.state === "asks";
      const who = to === "operator" ? "asks you" : to ? `to ${to}` : "";
      const tail = `${shape}${who ? `  ${who}` : ""}`;
      const room = width - Bun.stringWidth(plain(head)) - tail.length - 4;
      const first = clip((message ?? "").split("\n")[0] ?? "", Math.max(8, room));
      return pad(
        [
          ...head,
          [first, owed ? c.warning : c.text, owed ? bold : 0],
          ["  "],
          [tail, owed ? c.warning : c.faint],
        ],
        width,
        ground,
      );
    }
    if (line.type === "rung") {
      const chips: Part[] = line.acts.flatMap((chip): Part[] => {
        const [glyph, color] = actMarks[chip.state];
        return [["  "], [chip.label, c.muted], [" "], [chip.state === "running" ? spin() : glyph, color()]];
      });
      const failure =
        line.state === "failed"
          ? `  ${(act.run?.reason ?? "").split("\n")[0] ?? ""}`
          : line.state === "cancelled"
            ? "  cancelled"
            : "";
      const more = line.more ? `  +${line.more}` : "";
      const fixed = Bun.stringWidth(plain([...head, ...chips])) + more.length + 4;
      const room = Math.max(10, width - fixed - Math.min(70, failure.length));
      const word = line.word || (line.state === "running" ? "the model writes its word" : "");
      return pad(
        [
          ...head,
          [clip(word, room), line.word ? c.text : c.faint],
          [more, c.faint],
          ...chips,
          [
            clip(failure, Math.max(10, width - fixed - Math.min(room, Bun.stringWidth(word)))),
            line.state === "failed" ? c.danger : c.faint,
          ],
        ],
        width,
        ground,
      );
    }
    return pad([...head, [clip(this.detail(act), width - 20), c.text]], width, ground);
  }

  /** What an act that no rung of the chain made says of itself on its line. */
  private detail(act: ActRow): string {
    const [first] = act.words;
    if (act.kind === "bash") {
      const exit = act.value as Exit | undefined;
      const took = act.done ? "" : `  ${elapsed(Date.now() - (this.session.started[act.id] ?? Date.now()))}`;
      return `${String(first ?? "").split("\n")[0]}${act.done && exit?.code !== undefined ? `  exited ${exit.code}` : took}`;
    }
    if (act.kind === "grant")
      return `ceiling ${first ? dollars(Number(first)) : ""}${act.words[1] ? ` share ${act.words[1]}` : ""}`;
    if (act.kind === "wait") return `${act.words[0] ?? ""} seconds`;
    return (
      act.words
        .map((one) => (typeof one === "string" ? one : JSON.stringify(one)))
        .join("  ")
        .split("\n")[0] ?? ""
    );
  }

  /** The rows of the list of lives: each workspace, then each life of it that is not archived, unless it folds. */
  lifeRows(): { group: Workspace; entry?: SessionEntry }[] {
    return this.options.workspaces.groups.flatMap((group) => [
      { group },
      ...(group.collapsed
        ? []
        : group.sessions.filter((entry) => !entry.archived).map((entry) => ({ group, entry }))),
    ]);
  }
  private renderLives(): void {
    const rows = this.lifeRows();
    this.lifeIndex = Math.max(0, Math.min(this.lifeIndex, rows.length - 1));
    const current = this.options.workspaces.current;
    const parts = rows.map(({ group, entry }, index): Part[] => {
      const selected = index === this.lifeIndex;
      const bar: Part = [selected ? "▎" : " ", c.accent];
      const ground = selected ? c.selected : undefined;
      if (!entry)
        return pad(
          [
            bar,
            [group.collapsed ? " ▸ " : " ▾ "],
            [group.name, c.text, bold],
            ["  "],
            [shortenHome(group.directory), c.faint],
          ],
          this.width,
          ground,
        );
      const [mark, color] = lifeMarks[entry.status];
      const detail = [
        statusLabels[entry.status],
        entry === current ? "on screen" : "",
        entry.modified ? ago(entry.modified) : "",
        dollars(entry.cost ?? entry.session?.cost ?? 0),
      ]
        .filter(Boolean)
        .join("   ");
      return pad(
        [
          bar,
          ["     "],
          [mark, color()],
          [" "],
          [clip(entry.name, 30).padEnd(32), entry === current ? c.text : c.prose],
          [detail, c.muted],
        ],
        this.width,
        ground,
      );
    });
    const title: Part[] = [
      ["Lives", c.text, bold],
      ["   Enter opens a life, n starts a new one in its workspace.", c.faint],
      ["\n\n"],
    ];
    this.thread.content = styled([
      ...title,
      ...parts.flatMap((row, index) => (index ? [["\n"] as Part, ...row] : row)),
    ]);
  }
  /** The list of lives, or the loom again. */
  lives(): void {
    this.focus = this.focus === "lives" ? "loom" : "lives";
    const rows = this.lifeRows();
    this.lifeIndex = Math.max(
      0,
      rows.findIndex((row) => row.entry === this.options.workspaces.current),
    );
    this.render();
  }
  /** Enter on a row of the list: a workspace folds or unfolds, and a life opens. */
  async openLife(): Promise<void> {
    const row = this.lifeRows()[this.lifeIndex];
    if (!row) return;
    if (!row.entry) {
      this.options.workspaces.toggle(row.group);
      return this.render();
    }
    if (row.entry === this.options.workspaces.current) return this.lives();
    await this.options.workspaces.select(row.entry);
  }
  /** A new life in the workspace of the selected row, which opens. */
  async newLife(): Promise<void> {
    const row = this.lifeRows()[this.lifeIndex];
    await this.options.workspaces.create(row?.group);
  }

  private renderLens(): void {
    const act = this.whole?.id === this.lens?.id ? this.whole : this.lens;
    if (!act) return;
    const words = this.words();
    const found = this.lines.find((line) => line.act.id === act.id && "state" in line);
    const state = actMarks[found && "state" in found ? found.state : "done"];
    const head: Part[] = [
      [act.id, c.text, bold],
      ["  "],
      [state[0] || spin(), state[1]()],
      ["  "],
      [`${act.kind} on ${this.session.labelOf(act.on)}, made by ${act.by}`, c.muted],
    ];
    this.lensHead.content = styled(head);
    let code = "";
    let body = "";
    if (act.kind === "rung") {
      code = words.get(act.id) ?? "";
      body = [act.run?.reason, this.session.host.streams.get(act.id)?.text].filter(Boolean).join("\n\n");
    } else if (act.kind === "bash") {
      code = String(act.words[0] ?? "");
      const exit = act.value as Exit | undefined;
      body = `${exit?.stdout?.content ?? ""}${exit?.stderr?.content ?? ""}`;
    } else if (act.kind === "prompt") {
      const [shape, message, to] = act.words.map(String);
      body = `${message}\n\nshape ${shape}${to ? `, to ${to}` : ""}${act.done ? `\n\nclosed with ${valueText(act.value)}` : ""}`;
    } else if (act.kind === "write" && this.changes.some((change) => change.act === act.id)) {
      // A write shows the change that it made to its file, line by line.
      const change = this.changes.findLast((one) => one.act === act.id);
      this.lensCode.visible = false;
      this.lensBody.content = styled(change ? patchParts(change, this.session.host.directory) : []);
      return;
    } else {
      const value = act.value as { is?: string; content?: string } | null;
      body = value?.is === "Text" ? String(value.content ?? "") : display(act.value);
      code = "";
    }
    this.lensCode.content = safeText(code);
    this.lensCode.visible = Boolean(code);
    this.lensBody.content = styled([[safeText(body), c.prose]]);
  }

  private renderWord(): void {
    const owed =
      this.mode === "answer" ? this.loom.owed.find((prompt) => prompt.id === this.target) : undefined;
    const label =
      this.mode === "answer"
        ? `answer ${this.target}`
        : this.mode === "close"
          ? `close ${this.target}`
          : modeLabels[this.mode];
    const color = {
      python: c.secondary,
      answer: c.warning,
      close: c.warning,
      grant: c.warning,
      door: c.secondary,
    }[this.mode as string];
    // The composer names its mode only when it does more than ask the model of the chain.
    this.wordLabel.content = styled(
      this.mode === "prompt" ? [["› ", c.faint]] : [[`${label}  `, color ?? c.accent, bold]],
    );
    this.word.placeholder =
      this.mode === "answer"
        ? `${clip(owed?.message.split("\n")[0] ?? "", 60)}  (${owed?.shape ?? ""})`
        : {
            prompt: `Ask ${this.session.labelOf(this.session.selected)}`,
            python: "A word of python, which runs as a rung on this chain",
            command: "A command, as model, theme or fork",
            answer: "",
            grant: "The ceiling in dollars",
            close: "The value, as JSON",
            door: "The program of the ladder",
          }[this.mode];
    const actor = this.session.actor.replace(/^[\w-]+:/, "");
    const shape = this.session.shape === "str" ? "" : `  ${this.session.shape}`;
    this.wordMeta.content = styled([[`  ${actor}${shape}`, c.faint]]);
  }

  private renderFooter(): void {
    const w = this.session;
    const note =
      w.error ||
      w.notice ||
      (w.host.pending.size ? "This life holds work from before. Press w to wake it." : "");
    // The footer keeps the keys out of sight: it offers the three of the focus that act most, and ? for the rest.
    const offered = this.variant === "graph" && this.focus === "loom" ? this.graph.hints : hints[this.focus];
    const keys: Part[] = [...offered.slice(0, 3), ["?", "keys"] as [string, string]].flatMap(
      ([key, action]): Part[] => [
        [key, c.muted],
        [` ${action}   `, c.faint],
      ],
    );
    this.footer.content = styled(
      justify(
        [[clip(note, Math.max(20, this.width - 60)), w.error ? c.danger : c.warning]],
        keys,
        this.width,
      ),
    );
  }

  private renderHelp(): void {
    const parts: Part[] = [["Keys", c.text, bold], ["\n\n"]];
    for (const [key, action] of keyTable) parts.push([key.padEnd(18), c.accent], [action, c.prose], ["\n"]);
    parts.push(["\n"], ["Marks  ", c.text, bold]);
    for (const [state, [mark, color]] of Object.entries(threadMarks))
      parts.push([`${mark} `, color()], [`${state}   `, c.muted]);
    parts.push(
      ["\n"],
      ["Cells  ", c.text, bold],
      ["▁ work runs   ▂▃▄▅ acts began or ended   older ┊ the past folds", c.muted],
    );
    this.helpText.content = styled(parts);
  }

  private applyTheme(name: ThemeName): void {
    setTheme(name);
    this.theme = name;
    this.style = syntax();
    this.lensCode.syntaxStyle = this.style;
    this.root.backgroundColor = c.background;
    this.renderer.setBackgroundColor(c.background);
  }

  /** The act or the chain that the verbs act on: the act of the selected line in the thread and the lens, and the
   * selected chain in the loom. */
  get selection(): string {
    if (this.variant === "graph" && this.focus === "loom") return this.graph.selection;
    if (this.focus === "loom") return this.loom.threads[this.selectedThread]?.id ?? this.session.selected;
    if (this.focus === "lens" && this.lens) return this.lens.id;
    return this.lines[this.selectedLine]?.act.id ?? this.session.selected;
  }

  private press = (key: KeyEvent): void => {
    if (this.closed) return;
    // A key that the App takes reaches no input, though the input that it focuses hears the keys after it.
    if (this.focus !== "word") key.preventDefault();
    void this.handle(key).catch((error) => this.session.fail(error));
  };
  private async handle(key: KeyEvent): Promise<void> {
    if (key.ctrl && key.name === "q") return void (await this.options.quit());
    if (key.ctrl && key.name === "o" && this.focus !== "word") return this.lives();
    if (this.focus === "lives") {
      if (key.name === "up" || key.name === "k") this.lifeIndex = Math.max(0, this.lifeIndex - 1);
      else if (key.name === "down" || key.name === "j") this.lifeIndex++;
      else if (key.name === "return" || key.name === "enter") return this.openLife();
      else if (key.name === "n") return this.newLife();
      else if (key.name === "escape") this.focus = "loom";
      return this.render();
    }
    if (this.focus === "word") {
      if (key.name === "escape") {
        key.preventDefault();
        this.leaveWord();
      } else if (key.ctrl && key.name === "d" && !this.word.plainText) {
        if (Date.now() - this.quitAt < 1500) await this.options.quit();
        this.quitAt = Date.now();
        this.session.notice = "Press ⌃D again to quit.";
      } else if (key.ctrl && key.name === "c") {
        key.preventDefault();
        this.word.setText("");
      }
      return;
    }
    if (this.help) {
      if (key.name === "escape" || key.name === "?" || key.sequence === "?") this.help = false;
      this.render();
      return;
    }
    const name = [":", "/", "?", "!", "[", "]", "{", "}"].includes(key.sequence) ? key.sequence : key.name;
    // The past is read only: a verb acts on the life as it stands, so the time cursor holds the verbs but a branch.
    if (
      this.cursor !== undefined &&
      ["space", "x", "c", "g", "e", "i", ":", "!", "tab", "n"].includes(name)
    ) {
      this.session.notice = "The loom shows the past. b branches here, and Esc returns to now.";
      return this.render();
    }
    if (name === "v") return this.switchVariant();
    if (name === "n") return this.step();
    // The graph takes the keys that move and zoom at each of its levels, and the lens of an act gives Esc back to it.
    if (
      this.variant === "graph" &&
      !key.ctrl &&
      !key.meta &&
      (this.focus === "loom" || (this.focus === "lens" && name === "escape")) &&
      (await this.graph.key(name))
    )
      return this.render();
    const moves: Record<string, number> = { up: -1, k: -1, down: 1, j: 1, pageup: -10, pagedown: 10 };
    if (name in moves && !key.ctrl && !key.meta) return this.move(moves[name] ?? 0);
    switch (name) {
      case "left":
      case "h":
        return this.fold(true);
      case "right":
      case "l":
        return this.fold(false);
      case "return":
      case "enter":
        return this.open();
      case "escape":
        return this.back();
      case "tab":
        return this.answerNext();
      case "i":
        return this.write("prompt");
      case ":":
        return this.write("python");
      case "/":
        return this.write("command");
      case "!":
        this.write("command");
        this.word.setText("bash ");
        this.word.gotoBufferEnd();
        return;
      case "?":
        this.help = !this.help;
        return this.render();
      case "[":
        return this.travel(-1);
      case "]":
        return this.travel(1);
      case "{":
        return this.travel(-10);
      case "}":
        return this.travel(10);
      case "space":
        return this.toggle();
      case "x":
        return this.cancel();
      case "c":
        if (this.focus === "loom" && this.variant === "loom") return;
        return this.write("close", this.selection);
      case "g":
        return this.write("grant");
      case "e":
        return this.door();
      case "b":
        return this.branch();
      case "w":
        if (this.session.host.pending.size) await this.session.host.resume();
        return this.session.refresh();
      case "d":
        if (key.ctrl) {
          if (Date.now() - this.quitAt < 1500) return void (await this.options.quit());
          this.quitAt = Date.now();
          this.session.notice = "Press ⌃D again to quit.";
        }
    }
  }

  /** The selection moves by a count of rows: the thread of the loom, the line of the thread, or the rows of the lens. */
  async move(by: number): Promise<void> {
    if (this.focus === "lens") {
      this.lensBox.scrollBy(by);
      return;
    }
    if (this.focus === "thread") {
      const count = this.lines.length || starters.length;
      this.selectedLine = Math.max(0, Math.min(count - 1, this.selectedLine + by));
      this.render();
      return;
    }
    this.selectedThread = Math.max(0, Math.min(this.loom.threads.length - 1, this.selectedThread + by));
    const thread = this.loom.threads[this.selectedThread];
    this.render();
    if (thread && thread.id !== "operator" && thread.id !== this.session.selected)
      await this.select(thread.id);
  }
  /** A chain selected, whose thread shows its last line. */
  async select(chain: string): Promise<void> {
    await this.session.select(chain);
    this.selectedLine = Number.MAX_SAFE_INTEGER;
    this.render();
  }
  /** The workers of the selected thread fold into it, or show again. */
  fold(folding: boolean): void {
    const thread = this.loom.threads[this.selectedThread];
    if (this.focus === "thread" && folding) {
      this.back();
      return;
    }
    if (this.focus !== "loom" || !thread?.workers) return;
    if (folding) this.folds.add(thread.id);
    else this.folds.delete(thread.id);
    this.render();
  }
  /** Enter: the loom opens its thread, a line of the thread opens its lens or its answer, and a way to start puts its
   * prompt in the word line. */
  async open(): Promise<void> {
    if (this.focus === "loom") {
      const thread = this.loom.threads[this.selectedThread];
      if (thread?.id === "operator") return this.answerNext();
      this.focus = "thread";
      this.render();
      return;
    }
    if (this.focus !== "thread") return;
    if (!this.lines.length) {
      const [, prompt] = starters[this.selectedLine] ?? starters[0];
      this.write("prompt");
      this.word.setText(prompt);
      return;
    }
    const line = this.lines[this.selectedLine];
    if (!line) return;
    if (line.type === "prompt" && line.state === "asks" && this.cursor === undefined)
      return this.write("answer", line.act.id);
    this.lens = line.act;
    this.focus = "lens";
    this.lensBox.scrollTo(0);
    this.render();
    await this.fetchWhole(line.act);
  }
  back(): void {
    if (this.focus === "lens") this.focus = "thread";
    else if (this.focus === "thread") this.focus = "loom";
    else if (this.cursor !== undefined) this.cursor = undefined;
    this.render();
  }
  /** The variant that comes next, in place of the one that shows. */
  switchVariant(): void {
    this.variant = variants[(variants.indexOf(this.variant) + 1) % variants.length] ?? "loom";
    if (this.focus === "thread" || this.focus === "lens") {
      this.focus = "loom";
      this.lens = undefined;
    }
    this.session.notice = `The ${this.variant} shows the life. v changes the variant.`;
    this.render();
  }
  /** A step: the selected chain runs until one more of its rungs ends, then a pause holds it again. */
  async step(): Promise<void> {
    const chain = this.session.selected;
    const ended = this.session.acts.filter(
      (act) => act.on === chain && act.kind === "rung" && act.done,
    ).length;
    this.stepping.set(chain, ended);
    if (this.loom.threads.find((thread) => thread.id === chain)?.state === "paused")
      await this.session.engine.wake(chain);
    this.session.notice = `${this.session.labelOf(chain)} runs one step, then pauses.`;
    await this.session.refresh();
  }
  /** Each chain that a step let run pauses again once a rung of it that the step let run ends. */
  private steps(): void {
    for (const [chain, ended] of this.stepping) {
      const now = this.session.acts.filter(
        (act) => act.on === chain && act.kind === "rung" && act.done,
      ).length;
      if (now <= ended) continue;
      this.stepping.delete(chain);
      void this.session.engine.pause(chain).then(() => {
        this.session.notice = `${this.session.labelOf(chain)} paused after one step.`;
        return this.session.refresh();
      }, this.session.fail);
    }
  }
  /** The stage of the graph: the life as the App reads it, and the controls that the graph asks for. */
  private stageOf(): Stage {
    const app = this;
    return {
      get acts() {
        return app.acts;
      },
      get loom() {
        return app.loom;
      },
      get owed() {
        return new Set(app.loom.owed.map((prompt) => prompt.id));
      },
      get frozen() {
        return app.cursor !== undefined;
      },
      get selected() {
        return app.session.selected;
      },
      words: () => app.words(),
      label: (chain) => app.session.labelOf(chain),
      streams: () => (app.cursor === undefined ? app.session.host.streams : new Map()),
      select: (chain) => app.select(chain),
      lens: (act) => app.showLens(act),
    };
  }
  /** The lens shows an act whole, or the graph again. */
  private showLens(act?: ActRow): void {
    if (!act) {
      this.lens = undefined;
      this.focus = "loom";
      return;
    }
    this.lens = act;
    this.focus = "lens";
    this.lensBox.scrollTo(0);
    void this.fetchWhole(act).catch(this.session.fail);
  }
  /** The act whole for the lens, once the session gave it, with the changes of the files for a write. */
  private async fetchWhole(act: ActRow): Promise<void> {
    const host = this.session.host;
    const [whole, changes] = await Promise.all([
      host.act(act.id),
      act.kind === "write" ? host.readChanges(0, host.changes) : Promise.resolve(this.changes),
    ]);
    if (whole && this.lens?.id === whole.id) {
      this.whole = whole;
      this.changes = changes;
      this.render();
    }
  }
  private renderStage(): void {
    const height = this.stage.height || Math.max(4, this.renderer.height - 8);
    const rows = this.graph.rows(this.width, height);
    this.stage.content = styled(rows.flatMap((row, index) => (index ? [["\n"] as Part, ...row] : row)));
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
  /** The oldest prompt that the operator owes, on any chain, which the word line answers. */
  async answerNext(): Promise<void> {
    const owed = this.loom.owed[0];
    if (!owed) {
      this.session.notice = "You owe no prompt.";
      return;
    }
    if (owed.chain !== this.session.selected) await this.select(owed.chain);
    this.selectedLine = Math.max(
      0,
      this.lines.findIndex((line) => line.act.id === owed.id),
    );
    this.write("answer", owed.id);
  }
  /** The word line takes the keys, in a mode, for an act. */
  write(mode: Mode, target = ""): void {
    this.mode = mode;
    this.target = target;
    this.focus = "word";
    this.word.focus();
    this.render();
  }
  private leaveWord(): void {
    this.word.blur();
    if (this.mode !== "prompt" && this.mode !== "python" && this.mode !== "command") this.word.setText("");
    this.session.editing = undefined;
    this.focus = this.variant === "loom" && this.lines.length ? "thread" : "loom";
    this.mode = "prompt";
    this.render();
  }
  private compose = (text: string): void => {
    this.mode = "door";
    this.target = this.session.editing ?? "";
    this.focus = "word";
    this.word.setText(text);
    this.word.focus();
    this.render();
  };

  /** What the word line sends, by its mode. */
  async submit(): Promise<void> {
    const text = this.word.plainText;
    if (!text.trim()) return;
    const w = this.session;
    const mode = this.mode;
    const target = this.target;
    this.word.setText("");
    // A mode that answers one thing leaves the word line as it sends, so the keys after it reach the loom.
    if (mode !== "prompt" && mode !== "python" && mode !== "door") this.leaveWord();
    try {
      if (mode === "prompt") {
        await w.wakeForInput();
        await w.ask(text);
      } else if (mode === "python") await w.submit(`/run ${text}`);
      else if (mode === "command" && /^\/?(exit|quit)$/.test(text.trim()))
        return void (await this.options.quit());
      else if (mode === "command" && /^\/?new$/.test(text.trim())) {
        this.leaveWord();
        return void (await this.options.workspaces.create());
      } else if (mode === "command") await w.submit(text.startsWith("/") ? text : `/${text}`);
      else if (mode === "answer") await w.host.answer(target, text);
      else if (mode === "grant") await w.command(`/grant ${text.trim().replace(/^\$/, "")}`);
      else if (mode === "close") await w.command(`/close ${target} ${text}`);
      else if (mode === "door") {
        await w.submit(text);
        this.leaveWord();
      }
      this.selectedLine = Number.MAX_SAFE_INTEGER;
      await w.refresh();
    } catch (error) {
      if (this.focus === "word" && !this.word.plainText) this.word.setText(text);
      w.fail(error);
    }
    this.render();
  }

  /** The selection paused, or woken when a pause holds it. */
  async toggle(): Promise<void> {
    const id = this.selection;
    if (id === "operator") return;
    const act = this.session.acts.find((one) => one.id === id);
    const held =
      act?.kind === "chain"
        ? this.loom.threads.find((thread) => thread.id === id)?.state === "paused"
        : act?.paused;
    if (held) await this.session.engine.wake(id);
    else await this.session.engine.pause(id);
    this.session.notice = held ? `${id} woke.` : `${id} paused. Work in flight ends on its own.`;
    await this.session.refresh();
  }
  /** The selected act cancelled, or the work of the selected chain. */
  async cancel(): Promise<void> {
    const id = this.selection;
    if (id === "operator") return;
    const act = this.session.acts.find((one) => one.id === id);
    if (act?.kind === "chain") await this.session.host.interrupt(id);
    else await this.session.engine.cancel(id);
    this.session.notice = `${id} cancelled.`;
    await this.session.refresh();
  }
  /** The door of the prompt of the selection, in the word line. */
  async door(): Promise<void> {
    const id = this.selection;
    const act = this.session.acts.find((one) => one.id === id);
    const prompt =
      act?.kind === "prompt"
        ? act
        : this.session.acts.find((one) => one.id === act?.by && one.kind === "prompt");
    if (!prompt) {
      this.session.notice = "Select a prompt, or a rung of one, to edit its door.";
      return;
    }
    await this.session.command(`/edit ${prompt.id}`);
  }
  /** A branch of the chain after the selected act, which the session selects. */
  async branch(): Promise<void> {
    const w = this.session;
    if (this.cursor !== undefined) {
      // A branch at the time cursor keeps the acts of the chain up to its moment.
      const at = this.cursor;
      const later = w.acts.filter((act) => act.on === w.selected && act.kind !== "chain" && act.began > at);
      this.cursor = undefined;
      await w.branch(
        `${w.label} then`,
        later.map((act) => act.id),
      );
      this.focus = "thread";
      this.selectedLine = Number.MAX_SAFE_INTEGER;
      w.notice =
        "The new branch reads the chain as it stood then. The module and the files keep their state.";
      this.render();
      return;
    }
    if (this.focus === "loom" && this.variant === "loom") return;
    const id = this.selection;
    await this.session.rewind(id);
    this.focus = "thread";
    this.selectedLine = Number.MAX_SAFE_INTEGER;
    this.render();
  }

  private clickThread(event: MouseEvent): void {
    const index = this.loomTop + (event.y - this.threads.y);
    if (index < 0 || index >= this.loom.threads.length) return;
    this.focus = "loom";
    void this.move(index - this.selectedThread).catch(this.session.fail);
  }
  private clickLine(event: MouseEvent): void {
    const index = this.rowsOfLines[this.threadTop + (event.y - this.thread.y)];
    if (index === undefined) return;
    const again = this.focus === "thread" && index === this.selectedLine;
    this.focus = "thread";
    this.selectedLine = index;
    if (again) void this.open().catch(this.session.fail);
    else this.render();
  }

  dispose(): void {
    if (this.closed) return;
    this.closed = true;
    // What the word line holds stays as the draft of the session, which its save keeps for the next App.
    const draft = this.mode === "prompt" || this.mode === "python" ? this.word.plainText : "";
    if (draft) this.session.drafts[this.session.draftKey] = draft;
    else delete this.session.drafts[this.session.draftKey];
    clearInterval(this.tick);
    clearTimeout(this.redraw);
    this.session.off("change", this.schedule);
    this.session.off("compose", this.compose);
    this.options.workspaces.off("change", this.schedule);
    this.renderer.keyInput.off("keypress", this.press);
    this.renderer.off("resize", this.render);
    this.root.destroyRecursively();
    this.overlay.destroyRecursively();
  }
}

/** Whether a line shows work that runs, whose spinner turns. */
const running = (line: Line) => line.type !== "tail" && line.type !== "value" && line.state === "running";

/** The change of a file, as the lines of its patch under the path of the file in the directory of the life: what it
 * added on the ground of an addition, what it removed on the ground of a removal, and the rest as it stands. */
function patchParts({ path, before, after }: FileChange, directory: string): Part[] {
  const near = relative(directory, path);
  const name = near.startsWith("..") || isAbsolute(near) ? shortenHome(path) : near;
  const patch = createTwoFilesPatch(path, path, before, after).split("\n").slice(4).filter(Boolean);
  return [
    [safeText(name), c.text, bold],
    ...patch.flatMap((line): Part[] => [
      ["\n"],
      [
        safeText(line),
        line.startsWith("+")
          ? c.success
          : line.startsWith("-")
            ? c.danger
            : line.startsWith("@@")
              ? c.faint
              : c.prose,
        0,
        line.startsWith("+") ? c.added : line.startsWith("-") ? c.removed : undefined,
      ],
    ]),
  ];
}

/** A bar of the share of a window, eight steps to a cell. */
export function gauge(share: number, cells: number): string {
  const eighths = Math.max(share > 0 ? 1 : 0, Math.round(Math.max(0, Math.min(1, share)) * cells * 8));
  const full = Math.floor(eighths / 8);
  const rest = eighths % 8;
  return `${"█".repeat(full)}${rest ? (["", "▏", "▎", "▍", "▌", "▋", "▊", "▉"][rest] ?? "") : ""}`.padEnd(
    cells,
  );
}

/** The parts of a row, filled with spaces to a width, all on a ground when it has one. */
function pad(parts: Part[], width: number, ground?: RGBA): Part[] {
  const used = Bun.stringWidth(plain(parts));
  const all: Part[] = [...parts, [" ".repeat(Math.max(0, width - used))]];
  return ground ? all.map(([text, fg, attributes, bg]) => [text, fg, attributes, bg ?? ground]) : all;
}

/** Two groups of parts on one row: the first at the left, the second at the right. */
function justify(left: Part[], right: Part[], width: number): Part[] {
  const used = Bun.stringWidth(plain(left)) + Bun.stringWidth(plain(right));
  return [...left, [" ".repeat(Math.max(1, width - used))], ...right];
}

/** The parts of a text cut into its rows, at each line break. */
function splitRows(parts: Part[]): Part[][] {
  const rows: Part[][] = [[]];
  for (const [text, ...rest] of parts) {
    const pieces = text.split("\n");
    for (const [index, piece] of pieces.entries()) {
      if (index) rows.push([]);
      if (piece) rows.at(-1)?.push([piece, ...rest] as Part);
    }
  }
  return rows;
}

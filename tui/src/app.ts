import { basename } from "node:path";
import { imageContent, imagePath, shapes } from "@furb/engine";
import { display, isTag, safeText } from "@furb/engine/world";
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
  type Renderable,
  type RGBA,
  ScrollBoxRenderable,
  StyledText,
  TextareaRenderable,
  type TextChunk,
  TextRenderable,
} from "@opentui/core";
import { clipboardImage } from "./clipboard.ts";
import { commands } from "./commands.ts";
import { externalEditor, openFile } from "./editor.ts";
import type { Extensions } from "./extensions.ts";
import { clip, count, dollars, graphemes, kibibytes, share } from "./format.ts";
import { chords, keys } from "./keys.ts";
import { loadParsers } from "./parsers.ts";
import {
  type ActRow,
  failed,
  type Scroll,
  type Session,
  type SessionStatus,
  statusLabels,
  type View,
  working,
} from "./session.ts";
import { publishShare } from "./share.ts";
import {
  theme as c,
  defaultTheme,
  palettes,
  setTheme,
  spacing as space,
  syntax,
  type ThemeName,
} from "./theme.ts";
import type { Workspaces } from "./workspaces.ts";

const exitNotice = "Press Ctrl+D again to exit.";
const views: View[] = ["conversation", "program", "activity", "facts", "transcript", "changes"];
const title = (text: string) => text.charAt(0).toUpperCase() + text.slice(1);
const short = (id: string) => id.replace(/^[^:]+:\/\//, "");
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
  title?: () => string;
}
interface Choice {
  label: string;
  detail: string;
  run(): void | Promise<void>;
  toggle?: () => void;
}
/** One suggestion for the token before the cursor: the text that Tab puts in its place, and what Enter does when it
 * does more than that. */
interface Suggestion {
  label: string;
  detail: string;
  text: string;
  submit?: () => void;
}

export interface AppOptions {
  quit(): void | Promise<void>;
  sessions?: () => Promise<Choice[]>;
  newSession?: () => Promise<void>;
  workspaces?: Workspaces;
  extensions?: Extensions;
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
  private readonly folds = new Map<string, boolean>();
  private readonly inspector: ScrollBoxRenderable;
  private readonly sidebar: ScrollBoxRenderable;
  private readonly sidebarSplitter: BoxRenderable;
  private readonly splitters: BoxRenderable[] = [];
  private readonly tabs: BoxRenderable;
  private readonly head: TextRenderable;
  private readonly status: TextRenderable;
  private readonly composeBox: BoxRenderable;
  private readonly promptBox: BoxRenderable;
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
  private readonly paneKeys = new WeakMap<Renderable, string>();
  private readonly cards = new Map<
    string,
    {
      node: BoxRenderable;
      heading: TextRenderable;
      /** The text of the heading, set again only when it changes. */
      label: string;
      title?: () => string;
      marker: string;
      key: string;
      compact: boolean;
      collapsible: boolean;
      state: string;
      closed: boolean;
    }
  >();
  /** Where the view scrolls to, or the card it brings into view, once the scroll box has laid out its cards. */
  private scrollTarget?: Scroll | { card: string };
  private overlay?: BoxRenderable;
  private paletteInput?: InputRenderable;
  private paletteList?: BoxRenderable;
  private filtered: Choice[] = [];
  private selection = 0;
  private redraw?: ReturnType<typeof setTimeout>;
  private lastView = "";
  private submitting = false;
  private historyIndex = -1;
  private historyDraft = "";
  private diagnosticsKey = "";
  private readonly tick: ReturnType<typeof setInterval>;
  private readonly navigation: {
    chain: string;
    view: View;
    search: string;
    place: Scroll;
    ladder?: string;
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
      flexDirection: "column",
      backgroundColor: c.background,
    });
    renderer.root.add(this.root);
    const header = this.box({
      height: space.bar,
      paddingX: space.inset,
      flexDirection: "row",
      alignItems: "center",
      backgroundColor: c.panel,
      gap: space.between,
    });
    header.add(this.text("furb", c.text, { attributes: 1 }));
    this.head = this.text("", c.muted, { flexGrow: 1 });
    header.add(this.head);
    const commands = this.text("Ctrl+P", c.muted, { onMouseDown: () => this.palette() });
    header.add(commands);
    this.root.add(header);

    const body = this.box({ flexGrow: 1, flexShrink: 1, flexDirection: "row", minHeight: 0 });
    this.root.add(body);
    this.sidebar = new ScrollBoxRenderable(renderer, {
      id: "workspaces",
      width: session.preferences.sidebarWidth,
      flexShrink: 0,
      backgroundColor: c.panel,
      scrollX: false,
      contentOptions: { paddingX: space.inset, gap: space.stack },
    });
    this.sidebar.verticalScrollBar.visible = false;
    this.sidebar.horizontalScrollBar.visible = false;
    body.add(this.sidebar);
    this.sidebarSplitter = this.box({
      width: space.inset,
      backgroundColor: c.border,
      onMouseDrag: (event) => {
        session.preferences.sidebarWidth = Math.max(22, Math.min(42, event.x));
        session.preferences.save();
        this.render();
      },
    });
    body.add(this.sidebarSplitter);
    const center = this.box({
      flexGrow: 1,
      flexShrink: 1,
      minHeight: 0,
      minWidth: 0,
      paddingX: space.inset,
      gap: space.stack,
    });
    body.add(center);
    this.tabs = this.box({
      height: space.bar,
      marginBottom: space.section,
      flexDirection: "row",
      gap: space.between,
    });
    center.add(this.tabs);
    this.search = new InputRenderable(renderer, {
      id: "search",
      placeholder: "Filter this view. Esc to return to the composer.",
      visible: false,
      backgroundColor: c.raised,
      textColor: c.text,
      placeholderColor: c.muted,
    });
    this.search.on(InputRenderableEvents.INPUT, (value: string) => {
      session.search = value;
      this.renderContent();
    });
    center.add(this.search);
    this.scroll = new ScrollBoxRenderable(renderer, {
      id: "timeline",
      flexGrow: 1,
      flexShrink: 1,
      minHeight: 0,
      scrollX: false,
      stickyScroll: true,
      stickyStart: "bottom",
      onSizeChange: this.schedule,
      contentOptions: { gap: space.stack, paddingBottom: space.stack },
      verticalScrollbarOptions: { visible: false },
      horizontalScrollbarOptions: { visible: false },
    });
    // The ScrollBar constructor resets manual visibility; set it after construction.
    this.scroll.verticalScrollBar.visible = false;
    this.scroll.horizontalScrollBar.visible = false;
    center.add(this.scroll);
    this.promptBox = this.box({
      id: "operator-prompt",
      height: space.bar,
      visible: false,
      backgroundColor: c.panel,
      onMouseDown: () => this.question(),
    });
    center.add(this.promptBox);
    this.queueBox = this.box({
      id: "queued-follow-ups",
      height: space.bar,
      visible: false,
      onMouseDown: () => this.queuePicker(),
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
    this.suggestionBox = this.box({ id: "suggestions", flexShrink: 0, visible: false });
    center.add(this.suggestionBox);
    this.composeBox = this.box({
      id: "composer-box",
      flexShrink: 0,
      padding: space.inset,
      border: ["left"],
      borderColor: c.border,
      backgroundColor: c.panel,
    });
    center.add(this.composeBox);
    this.composer = new TextareaRenderable(renderer, {
      id: "composer",
      flexGrow: 1,
      minHeight: 1,
      placeholder: "What would you like to build?",
      textColor: c.text,
      placeholderColor: c.muted,
      backgroundColor: c.panel,
      focusedBackgroundColor: c.panel,
      focusedTextColor: c.text,
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
    this.status = this.text("", c.muted, { height: space.bar, truncate: true });
    center.add(this.status);
    const rightSplitter = this.box({
      width: 1,
      backgroundColor: c.border,
      onMouseDrag: (event) => {
        session.panes.inspector = Math.max(24, Math.min(44, renderer.width - event.x));
        this.inspector.width = session.panes.inspector;
      },
      onMouseOver() {
        this.backgroundColor = c.accent;
      },
      onMouseOut() {
        this.backgroundColor = c.border;
      },
    });
    body.add(rightSplitter);
    this.splitters.push(rightSplitter);
    this.inspector = new ScrollBoxRenderable(renderer, {
      width: session.panes.inspector,
      flexShrink: 0,
      scrollX: false,
      scrollY: true,
      backgroundColor: c.panel,
      contentOptions: { paddingX: space.inset, gap: space.stack, minHeight: "100%" },
      verticalScrollbarOptions: { visible: false },
      horizontalScrollbarOptions: { visible: false },
    });
    this.inspector.verticalScrollBar.visible = false;
    this.inspector.horizontalScrollBar.visible = false;
    body.add(this.inspector);
    session.on("change", this.schedule);
    options.workspaces?.on("change", this.schedule);
    session.on("compose", this.compose);
    session.on("resume", this.resume);
    session.on("shared", this.shared);
    renderer.keyInput.on("keypress", this.key);
    renderer.on("resize", this.render);
    // A label that moves with time is read again, and no other part of the view is drawn again.
    this.tick = setInterval(() => {
      for (const card of this.cards.values())
        if (card.title) this.label(card, `${card.marker}${card.title()}`);
    }, 250);
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
    if (session.world.held.size) this.resume();
  }

  private box(options: BoxOptions = {}): BoxRenderable {
    return new BoxRenderable(this.renderer, { flexDirection: "column", flexShrink: 0, ...options });
  }
  private text(
    content: string,
    fg = c.text,
    options: ConstructorParameters<typeof TextRenderable>[1] = {},
  ): TextRenderable {
    return new TextRenderable(this.renderer, {
      content: safeText(content),
      fg,
      // A text truncates only on a line it does not wrap, so a text that truncates keeps one line with an ellipsis.
      wrapMode: options.truncate ? "none" : "word",
      flexShrink: 0,
      ...options,
    });
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
  }
  private compose = (text: string) => {
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
  async submit(): Promise<void> {
    if (this.submitting) return;
    this.submitting = true;
    const key = this.draftKey;
    const content = this.composer.plainText;
    const python = this.session.mode === "python" && !this.session.editing && !content.startsWith("/");
    // The text leaves its draft as it is sent, so what is typed while it is sent stays in the composer.
    this.composer.setText("");
    try {
      if (!(await this.globalCommand(content.trim())))
        await this.session.submit(python ? `/run ${content}` : content);
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
    this.sidebar.visible = Boolean(
      this.options.workspaces && w.preferences.sidebar && this.renderer.width >= 110,
    );
    this.sidebar.width = w.preferences.sidebarWidth;
    this.sidebarSplitter.visible = this.sidebar.visible;
    const available =
      this.renderer.width - (this.sidebar.visible ? w.preferences.sidebarWidth + space.inset : 0);
    this.inspector.visible = available >= 100;
    if (this.splitters[0]) this.splitters[0].visible = this.inspector.visible;
    const group = this.options.workspaces?.groupOf();
    this.head.content = `${group ? `${group.name} / ` : ""}${w.sessionName} / ${w.label}`;
    if (this.paneChanged(this.tabs, [w.view, this.theme, available])) {
      this.clear(this.tabs);
      for (const [index, view] of views.entries()) {
        const label =
          available < 140 ? ["Chat", "Code", "Acts", "Facts", "Transcript", "Diffs"][index] : title(view);
        this.tabs.add(
          this.text(`${index + 1} ${label}`, w.view === view ? c.accent : c.muted, {
            onMouseDown: () => w.show(view),
            attributes: w.view === view ? 1 : 0,
          }),
        );
      }
    }
    const pending = w.operatorPrompt;
    this.promptBox.visible = !!pending;
    const images = w.images[w.selected] ?? [];
    this.imageBox.visible = images.length > 0;
    if (this.paneChanged(this.imageBox, [images, this.theme])) {
      this.clear(this.imageBox);
      this.imageBox.add(
        this.text(
          `Images · ${images.length} · ${images
            .map((image) => image.name)
            .join(", ")
            .replace(/\s+/g, " ")}`,
          c.muted,
          {
            height: space.bar,
            truncate: true,
            flexShrink: 1,
            onMouseDown: () => {
              this.openPalette(
                "Image attachments",
                images.map((image) => ({
                  label: image.name,
                  detail: `${image.mimeType} · ${kibibytes(image.size)}`,
                  run: () => this.imageActions(image.uri),
                })),
              );
            },
          },
        ),
      );
    }
    this.queueBox.visible = w.queued.length > 0;
    if (this.paneChanged(this.queueBox, [w.queued, w.queueHeld, this.theme])) {
      this.clear(this.queueBox);
      this.queueBox.add(
        this.text(
          `${w.queueHeld ? "Queue held" : "Queued"} · ${w.queued.length} · ${w.queued[0]?.text.split("\n")[0] ?? ""}`,
          w.queueHeld ? c.warning : c.muted,
          { height: space.bar, truncate: true },
        ),
      );
    }
    if (this.paneChanged(this.promptBox, [pending, this.theme])) {
      this.clear(this.promptBox);
      if (pending)
        this.promptBox.add(
          this.text(`Reply (${pending.shape}): ${pending.message}`, c.warning, {
            height: space.bar,
            truncate: true,
          }),
        );
    }
    this.composer.placeholder = w.editing
      ? "Edit this prompt's Python program..."
      : pending
        ? `Your ${pending.shape} answer...`
        : w.mode === "python"
          ? "Write Python..."
          : "Ask anything, or type / for a command...";
    this.composeBox.height =
      space.inset * 2 +
      Math.min(6, Math.max(space.bar, this.composer.lineCount, this.composer.lineInfo.lineSources.length));
    const { model, effort } = w.actorChoice;
    const state = w.loading ? "Loading" : statusLabels[w.status(w.selected)];
    this.status.content = `${w.error ? state : w.notice || state} · model ${model} · effort ${effort}${pending ? "" : ` · ${w.mode === "python" || w.editing ? "Python" : `returns ${w.shape}`}`}${w.world.records.path ? ` · ${basename(w.world.records.path)}` : ""}`;
    this.renderContent();
    this.renderInspector();
    this.renderWorkspaces();
    this.renderSuggestions();
    const diagnostics = JSON.stringify([w.rejectedWord, w.findings]);
    if (diagnostics !== this.diagnosticsKey) {
      this.diagnosticsKey = diagnostics;
      void this.highlightEditor();
    }
  };

  /** The heading of a card set to a text, only when the text changed. */
  private label(card: { heading: TextRenderable; label: string }, text: string): void {
    if (card.label === text) return;
    card.label = text;
    card.heading.content = safeText(text);
  }
  private card(
    id: string,
    key: string,
    label: string,
    color: RGBA,
    body: (box: BoxRenderable) => void,
    index: number,
    options: BlockOptions = {},
  ): void {
    const rung = options.act?.kind === "rung" ? options.act : undefined;
    const state = rung?.id ?? id;
    const closed =
      this.folds.get(state) ??
      (rung
        ? this.session.preferences.autoCollapseRungs && rung.run?.status === "done"
        : Boolean(options.compact));
    key =
      options.compact && closed && !options.preview
        ? "closed"
        : `${key}:${closed}:${options.preview && closed ? this.scroll.width : ""}`;
    const marker = rung || options.compact || options.collapsible ? (closed ? "▸ " : "▾ ") : "";
    const heading = `${marker}${label}`;
    const visible = Boolean(label) && (options.compact || options.heading !== false || closed);
    const prior = this.cards.get(id);
    if (prior?.key === key) {
      this.label(prior, heading);
      Object.assign(prior, { marker, title: options.title, closed });
      if (prior.heading.fg !== color) prior.heading.fg = color;
      if (prior.heading.visible !== visible) prior.heading.visible = visible;
      const margin = options.separate ? space.section : space.stack;
      if (prior.node.marginTop !== margin) prior.node.marginTop = margin;
      if (this.scroll.getChildren()[index] !== prior.node) this.scroll.add(prior.node, index);
      return;
    }
    prior?.node.destroyRecursively();
    const box = this.box({
      id,
      gap: space.stack,
      flexShrink: 0,
      marginTop: options.separate ? space.section : space.stack,
      ...(options.prompt
        ? { border: ["left"], borderColor: c.accent, paddingX: space.inset, backgroundColor: c.panel }
        : {}),
    });
    const labelNode = this.text(heading, color, {
      height: space.bar,
      truncate: true,
      visible,
      onMouseDown: (event) => {
        if (event.button === 2 && options.act) {
          this.actActions(this.session.acts.find((act) => act.id === options.act?.id) ?? options.act);
          return;
        }
        this.folds.set(state, !closed);
        this.renderContent();
      },
    });
    box.add(labelNode);
    if (!closed) body(box);
    else if (!rung) options.preview?.(box);
    this.scroll.add(box, index);
    this.cards.set(id, {
      key,
      node: box,
      heading: labelNode,
      label: heading,
      marker,
      title: options.title,
      compact: options.compact ?? false,
      collapsible: Boolean(rung || options.collapsible),
      state,
      closed,
    });
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
  private markdown(content: string): MarkdownRenderable {
    return new MarkdownRenderable(this.renderer, {
      content: safeText(content),
      syntaxStyle: this.style,
      fg: c.text,
    });
  }

  renderContent(): void {
    const w = this.session;
    const view = `${w.selected}:${w.view}:${w.ladder ?? ""}`;
    if (this.lastView !== view) {
      if (this.lastView) w.scrolls[this.lastView] = this.place;
      this.clear(this.scroll);
      this.cards.clear();
      this.lastView = view;
      this.scroll.stickyScroll = w.view === "conversation";
      this.scrollNow(w.scrolls[view] ?? 0);
      this.scrollAfterLayout(w.scrolls[view]);
    }
    const existing = new Set(this.cards.keys());
    let order = 0,
      group = "";
    const add = (
      id: string,
      key: string,
      label: string,
      color: RGBA,
      body: (box: BoxRenderable) => void,
      options: BlockOptions = {},
    ) => {
      existing.delete(id);
      const heading = !options.group || group !== options.group;
      group = options.group ?? "";
      this.card(id, key, label, color, body, order, {
        ...options,
        heading: options.heading ?? heading,
        separate: order > 0 && (options.separate ?? heading),
      });
      order++;
    };
    const matches = (text: string) => !w.search || text.toLowerCase().includes(w.search.toLowerCase());
    // The label of an act that works moves with time, so the tick reads it again.
    const moving = (act?: ActRow) => (act && working(act) ? () => this.actSummary(act) : undefined);
    if (w.preferences.notice)
      add("preferences-notice", w.preferences.notice, "", c.warning, (box) =>
        box.add(
          this.text(w.preferences.notice, c.warning, {
            onMouseDown: () => {
              w.preferences.notice = "";
              this.renderContent();
            },
          }),
        ),
      );
    if (w.error && !(w.view === "program" && w.findings.length))
      add("view-error", w.error, "", c.danger, (box) => {
        box.add(this.text(w.error, c.danger));
        box.add(
          this.text("Refresh view", c.link, {
            onMouseDown: () => {
              w.error = "";
              void w.refresh().catch(w.fail);
            },
          }),
        );
      });
    let items = 0;
    if (w.view === "conversation") {
      const seen = new Set<string>();
      let rung: ActRow | undefined;
      for (const [index, turn] of w.turns.entries())
        for (const [part, content] of turn[1].entries()) {
          const id = `turn-${index}-${part}`;
          if (typeof content === "string") {
            if (!matches(content)) continue;
            items++;
            const wordAct = rung;
            if (wordAct) seen.add(wordAct.id);
            add(
              id,
              `${content}\n${wordAct?.run?.reason ?? ""}`,
              rung ? this.actSummary(rung) : "Python",
              rung ? this.actColor(rung) : c.muted,
              (box) => {
                box.add(this.numbered(content));
                if (wordAct?.run?.reason) box.add(this.text(wordAct.run.reason, c.danger));
              },
              {
                collapsible: true,
                act: rung,
                title: moving(rung),
              },
            );
            continue;
          }
          if (!isTag(content)) continue;
          const [name, attrs, body] = content;
          const fields = Object.fromEntries(attrs);
          const actId = String(fields.id ?? fields.over ?? "");
          const act = w.acts.find((act) => act.id === actId);
          if (act?.kind === "rung" && name === "opened") rung = act;
          if (act && ["raised", "refused"].includes(name) && failed(act) && seen.has(act.id)) continue;
          if (
            name === "ledger" ||
            (act &&
              (["chain", "grant"].includes(act.kind) ||
                (act.kind === "rung" &&
                  (act.by !== "operator" || !act.words[0]) &&
                  (name === "opened" || !failed(act) || seen.has(act.id)))) &&
              ["opened", "closed"].includes(name))
          )
            continue;
          if (!matches(JSON.stringify(content))) continue;
          if (act?.kind === "prompt" && name === "opened") {
            items++;
            add(
              id,
              JSON.stringify(content),
              w.isUserPrompt(act) ? "You" : "Observation",
              c.muted,
              (box) => box.add(this.markdown(String(fields.message ?? ""))),
              { group: w.isUserPrompt(act) ? "user" : "observation", prompt: w.isUserPrompt(act), act },
            );
          } else if (act?.kind === "prompt" && name === "closed") {
            items++;
            const parallel =
              turn[1].filter(
                (part) =>
                  isTag(part) &&
                  part[0] === "closed" &&
                  part[1].some(([key, value]) => key === "over" && String(value).startsWith("prompt://")),
              ).length > 1;
            add(
              id,
              JSON.stringify(content),
              parallel ? `Result · ${this.preview(String(act.words[1]).split("\n")[0] ?? "", 9)}` : "Result",
              c.muted,
              (box) => box.add(this.markdown(display(act.value ?? body))),
              { group: parallel ? `assistant:${act.id}` : "assistant", act },
            );
          } else if (act && ["opened", "closed"].includes(name)) {
            if (seen.has(act.id)) continue;
            seen.add(act.id);
            items++;
            add(
              act.id,
              JSON.stringify(act),
              this.actSummary(act),
              this.actColor(act),
              (box) => this.actDetails(box, act),
              { compact: true, group: "tools", preview: this.actPreview(act), act, title: moving(act) },
            );
          } else {
            items++;
            const detail = attrs
              .filter(([key]) => !["id", "over"].includes(key))
              .map(([key, value]) => `${key}: ${display(value)}`)
              .join(" · ");
            const summary = `${name}${detail ? ` · ${this.preview(detail, Bun.stringWidth(name) + 5)}` : actId ? ` · ${short(actId)}` : ""}`;
            add(
              id,
              JSON.stringify(content),
              summary,
              ["raised", "refused"].includes(name) ? c.danger : c.muted,
              (box) => {
                if (actId) box.add(this.reference(actId, actId));
                for (const [key, value] of attrs.filter(([key]) => !["id", "over"].includes(key)))
                  box.add(
                    typeof value === "string" && (key === "path" || value.includes("://"))
                      ? this.reference(`${key}: ${value}`, value)
                      : this.text(`${key}: ${display(value)}`, c.muted),
                  );
                if (body !== null && body !== "") this.renderBody(box, body);
              },
              {
                compact: true,
                group: "tools",
                act,
                ...(name === "refused"
                  ? {
                      preview: (box: BoxRenderable) =>
                        this.excerpt(box, display(body), false, false, c.danger),
                    }
                  : {}),
              },
            );
          }
        }
      for (const [id, stream] of w.world.streams) {
        if (stream.chain !== w.selected) continue;
        const act = w.acts.find((act) => act.id === id);
        items++;
        add(
          `stream-${id}`,
          stream.text + stream.thinking,
          act ? this.actSummary(act) : this.progress(id),
          c.muted,
          (box) => {
            if (stream.thinking) box.add(this.text(stream.thinking, c.muted));
            if (stream.text) box.add(this.code(stream.text));
          },
          { act, collapsible: true, title: () => (act ? this.actSummary(act) : this.progress(id)) },
        );
      }
      if (!items && !w.search && !w.loading && !w.error) {
        add("welcome", "welcome", "", c.muted, (box) => {
          for (const [label, prompt] of [
            ["Explore a codebase", "Read the README and explain how this project works."],
            ["Make something better", "Find one useful improvement in this project and implement it."],
            ["Start with a plan", "Read the project and propose a small, testable plan."],
          ])
            box.add(this.text(label ?? "", c.muted, { onMouseDown: () => this.insert(prompt ?? "") }));
        });
        items++;
      }
    } else if (w.view === "program") {
      const ladder = w.acts.find((act) => act.id === w.ladder);
      if (ladder) {
        items++;
        add(
          "prompt-repl",
          JSON.stringify(ladder),
          `Prompt ${short(ladder.id)} · ${ladder.done ? "closed" : "pending"}`,
          c.muted,
          (box) => box.add(this.markdown(String(ladder.words[1] ?? ""))),
        );
      }
      if (w.findings.length && (!w.ladder || w.repls[w.ladder]?.includes(w.rejectedAct))) {
        items++;
        add("findings", w.rejectedWord + w.findings.join("\n"), "Refused Python", c.danger, (box) => {
          box.add(this.numbered(w.rejectedWord, w.findings));
          for (const finding of w.findings) box.add(this.text(finding, c.danger));
        });
      }
      const words = Object.entries(w.program).filter(
        ([id]) => !w.ladder || short(id).startsWith(`${short(w.ladder)}.`) || w.repls[w.ladder]?.includes(id),
      );
      for (const [id, word] of words)
        if (matches(word)) {
          items++;
          const act = w.acts.find((act) => act.id === id);
          add(
            id,
            word,
            act ? this.actSummary(act) : `rung · ${short(id)} · done`,
            act ? this.actColor(act) : c.muted,
            (box) => box.add(this.numbered(word)),
            { collapsible: true, act, title: moving(act) },
          );
        }
    } else if (w.view === "activity") {
      for (const act of w.activity)
        if (matches(JSON.stringify(act))) {
          items++;
          add(
            act.id,
            JSON.stringify(act),
            this.actSummary(act),
            this.actColor(act),
            (box) => this.actDetails(box, act),
            { compact: true, preview: this.actPreview(act), act, title: moving(act) },
          );
        }
    } else if (w.view === "transcript") {
      w.turns.forEach((turn, index) => {
        const text = w.rendered[index] ?? "";
        if (!matches(text)) return;
        items++;
        add(
          `transcript-${index}`,
          text,
          `${turn[0]} · ${index + 1}`,
          c.muted,
          (box) => box.add(turn[0] === "assistant" ? this.code(text) : this.transcriptText(text)),
          { group: turn[0] },
        );
      });
    } else if (w.view === "changes") {
      if (w.world.changes > 20)
        add("change-pages", String(w.changePage), "", c.muted, (box) => {
          box.add(
            this.text(
              `Writes ${w.changePage * 20 + 1} to ${Math.min((w.changePage + 1) * 20, w.world.changes)} of ${w.world.changes}`,
              c.muted,
            ),
          );
          for (const [label, step] of [
            ["Previous page", -1],
            ["Next page", 1],
          ] as const)
            box.add(this.text(label, c.link, { onMouseDown: () => this.changePage(step) }));
        });
      for (const [index, change] of w.changes.entries())
        if (matches(change.path)) {
          items++;
          // A change never changes once it is written, so its position in the life keys its card.
          const position = String(w.changePage * 20 + index);
          add(`change-${position}`, position, change.path, c.muted, (box) =>
            box.add(
              new DiffRenderable(this.renderer, {
                diff: change.patch,
                view: this.renderer.width > 145 ? "split" : "unified",
                syntaxStyle: this.style,
                fg: c.text,
                showLineNumbers: true,
                lineNumberFg: c.muted,
                lineNumberBg: c.background,
                contextBg: c.background,
                addedBg: c.selected,
                removedBg: c.removed,
                addedSignColor: c.success,
                removedSignColor: c.danger,
                wrapMode: "word",
              }),
            ),
          );
        }
    } else {
      const facts = w.filteredFacts();
      for (const [index, fact] of facts.slice(-300).entries()) {
        items++;
        add(
          `fact-${index}`,
          JSON.stringify(fact),
          // A fact about no act, such as a keep, ends its heading at its kind.
          `${index + Math.max(0, facts.length - 300) + 1} · ${fact[0]}${fact[1] ? ` · ${fact[1]}` : ""}`,
          c.muted,
          (box) => {
            box.add(this.text(`by ${fact[2]}`, c.muted));
            box.add(this.text(JSON.stringify(fact.slice(3), null, 2)));
          },
          { compact: true, group: "facts" },
        );
      }
    }
    if (!items && w.loading && !w.error)
      add("view-loading", w.view, "", c.muted, (box) => box.add(this.text(`Loading ${w.view}...`, c.muted)));
    if (!items && !w.loading && !w.error) {
      const empty = {
        conversation: "No conversation yet.",
        program: "No accepted Python yet. Use /run to write a rung.",
        activity: "No acts yet.",
        facts: "No facts yet.",
        transcript: "No transcript yet.",
        changes: "No file changes yet.",
      };
      const message = w.search ? `No matching ${w.view} for “${w.search}”.` : empty[w.view];
      add("empty", message, "", c.muted, (box) => box.add(this.text(message, c.muted)));
    }
    for (const id of existing) {
      this.cards.get(id)?.node.destroyRecursively();
      this.cards.delete(id);
    }
  }

  private preview(text: string, reserve = 2): string {
    return clip(text.replace(/\s+/g, " "), Math.max(8, this.scroll.width - reserve));
  }
  private excerpt(box: BoxRenderable, content: string, python = false, tail = false, color = c.text): void {
    const lines = content.trimEnd().split("\n");
    const limit = python ? 2 : 3;
    const selected = tail ? lines.slice(-limit) : lines.slice(0, limit);
    if (lines.length > limit) {
      if (tail) selected[0] = `… ${selected[0]}`;
      else selected[selected.length - 1] += " …";
    }
    const visible = selected
      .map((line) => clip(line, Math.max(8, this.scroll.width - space.between)))
      .join("\n");
    const preview = this.box({ paddingLeft: space.between });
    preview.add(python ? this.code(visible) : this.text(visible, color));
    box.add(preview);
  }
  private actPreview(act: ActRow): BlockOptions["preview"] {
    if (act.run)
      return act.run.reason
        ? (box) => this.excerpt(box, act.run?.reason ?? "", false, false, c.danger)
        : undefined;
    if (failed(act)) {
      const fault = act.value as { is: string; args: unknown[] };
      return (box) =>
        this.excerpt(box, `${fault.is}: ${fault.args.map(display).join(", ")}`, false, false, c.danger);
    }
    if (act.kind === "bash" && act.value && typeof act.value === "object") {
      const exit = act.value as { stdout?: { content: string }; stderr?: { content: string } };
      const output = [exit.stdout?.content, exit.stderr?.content].filter(Boolean).join("\n");
      if (output) return (box) => this.excerpt(box, output, false, true);
    }
    if (act.kind === "prompt" && act.done && act.value !== null)
      return (box) => this.excerpt(box, display(act.value));
    return undefined;
  }
  private actColor(act: ActRow): RGBA {
    if (failed(act)) return c.danger;
    return this.session.world.prompts.has(act.id) ? c.warning : c.muted;
  }
  private actSummary(act: ActRow): string {
    const held = this.session.world.held.has(act.id);
    const fault = failed(act);
    const cancelled =
      act.value && typeof act.value === "object" && "is" in act.value && act.value.is === "CancelledError";
    const state =
      act.kind === "rung"
        ? (act.run?.status ?? "running")
        : act.kind === "grant"
          ? act.done
            ? "ended ceiling"
            : "active ceiling"
          : act.done
            ? fault
              ? "failed"
              : cancelled
                ? "cancelled"
                : "done"
            : held
              ? "held"
              : this.session.world.prompts.has(act.id)
                ? "needs input"
                : act.paused && act.kind !== "bash"
                  ? "paused"
                  : `running ${this.progress(act.id)}`;
    const words =
      act.kind === "prompt"
        ? String(act.words[1])
        : act.kind === "grant"
          ? [
              act.words[0] === null ? "" : dollars(Number(act.words[0])),
              act.words[1] === null ? "" : share(Number(act.words[1])),
            ]
              .filter(Boolean)
              .join(" · ")
          : act.kind === "wait"
            ? `${act.words[0]}s`
            : act.kind === "rung"
              ? short(act.id)
              : String(act.words[0] || "");
    const observation = act.kind === "prompt" && !this.session.isUserPrompt(act);
    const prefix = `${observation ? "observation" : act.kind} · `,
      suffix = ` · ${state}${act.kind === "rung" && state === "running" ? ` ${this.progress(act.id)}` : ""}`;
    return `${prefix}${this.preview(observation ? words.replace(/ done$/, "") : words, Bun.stringWidth(prefix + suffix) + 2)}${suffix}`;
  }
  private actDetails(box: BoxRenderable, act: ActRow): void {
    if (act.kind === "rung") {
      const word = String(this.session.program[act.id] || act.words[0] || "");
      if (word) box.add(this.numbered(word));
      if (act.run?.reason) box.add(this.text(act.run.reason, c.danger));
      return;
    }
    const fields: Record<string, string[]> = {
      prompt: ["shape", "message", "actor"],
      rung: ["word", "retells", "actor", "returns"],
      bash: ["command", "stdin open", "timeout (seconds)", "stdout show", "stderr show"],
      wait: ["seconds"],
      grant: ["dollar ceiling", "context ceiling"],
    };
    box.add(this.reference(act.id, act.id));
    for (const [index, value] of act.words.entries()) {
      if (value === null || value === "" || (act.kind === "rung" && index === 0)) continue;
      box.add(
        this.text(`${fields[act.kind]?.[index] ?? `argument ${index + 1}`}: ${display(value)}`, c.muted),
      );
    }
    if (act.kind === "rung" && (this.session.program[act.id] || act.words[0]))
      box.add(this.numbered(String(this.session.program[act.id] || act.words[0])));
    if (act.kind === "bash" && act.value && typeof act.value === "object") {
      type Exit = { stdout?: { content: string }; stderr?: { content: string }; code?: number };
      const exit = act.value as Exit;
      const shown: TextRenderable[] = [];
      for (const [name, color] of [
        ["stdout", c.text],
        ["stderr", c.danger],
      ] as const) {
        const content = exit[name]?.content;
        if (!content) continue;
        box.add(this.text(name, name === "stdout" ? c.muted : c.danger));
        const node = this.text(content, color);
        box.add(node);
        shown.push(node);
      }
      if (act.done) box.add(this.text(`exit: ${exit.code ?? "timeout"}`, c.muted));
      // The row holds the tail of what the command printed, and the card reads the whole of it once it opens.
      if (act.output !== undefined)
        void this.session.world.act(act.id).then((whole) => {
          const streams = whole?.value as Exit | undefined;
          const contents = [streams?.stdout?.content, streams?.stderr?.content].filter(Boolean) as string[];
          for (const [index, node] of shown.entries())
            if (!node.isDestroyed && contents[index] !== undefined) node.content = safeText(contents[index]);
        }, this.report);
    } else if (act.done && act.value !== null) box.add(this.text(display(act.value), this.actColor(act)));
  }

  private renderBody(box: BoxRenderable, body: unknown): void {
    if (!Array.isArray(body)) {
      box.add(this.text(display(body)));
      return;
    }
    for (const one of body) {
      if (isTag(one)) {
        box.add(this.text(title(one[0]), c.muted, { marginTop: space.section }));
        for (const [key, value] of one[1])
          box.add(
            typeof value === "string" && (key === "path" || value.includes("://"))
              ? this.reference(`${key}: ${value}`, value)
              : this.text(`${key}: ${display(value)}`, c.muted),
          );
        this.renderBody(box, one[2]);
      } else if (Array.isArray(one) && typeof one[0] === "number" && typeof one[1] === "string") {
        box.add(this.text(`${String(one[0]).padStart(4)}  ${one[1]}`));
      } else box.add(this.text(display(one)));
    }
  }

  private numbered(word: string, findings: string[] = []): LineNumberRenderable {
    const lines = new LineNumberRenderable(this.renderer, {
      target: this.code(word),
      fg: c.muted,
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
    if (value.startsWith("furb-image://")) this.imageActions(value);
    else if (value.startsWith("chain://")) await this.session.select(value);
    else if (value.startsWith("prompt://")) this.openLadder(value);
    else if (value.startsWith("rung://")) this.go("program", value);
    else if (value.includes("://") && !value.includes("/stdin")) this.go("activity", value);
    else
      this.showValue(
        value,
        (
          (await this.session.life.read(value, { is: "name", name: "HIDDEN" }, this.session.selected)) as {
            content: string;
          }
        ).content,
      );
  }

  private async referenceHover(value: string, x: number, y: number): Promise<void> {
    try {
      const act = this.session.acts.find((act) => act.id === value);
      const detail = value.startsWith("furb-image://")
        ? "Image attachment. Click to open its actions."
        : act
          ? `${act.kind} · ${act.done ? display(act.value) : "pending"}`
          : (
              (await this.session.life.read(
                value,
                { is: "name", name: "HIDDEN" },
                this.session.selected,
              )) as {
                content: string;
              }
            ).content;
      if (this.closed || this.overlay) return;
      this.hover?.destroyRecursively();
      this.hover = this.box({
        position: "absolute",
        left: Math.max(1, Math.min(x, this.renderer.width - 60)),
        top: Math.max(1, Math.min(y + 1, this.renderer.height - 8)),
        width: Math.min(58, this.renderer.width - 4),
        maxHeight: 7,
        padding: space.inset,
        border: true,
        borderColor: c.link,
        backgroundColor: c.raised,
        zIndex: 30,
        onMouseDown: () => {
          void this.follow(value).catch(this.report);
        },
      });
      this.hover.add(this.text(value, c.link));
      this.hover.add(this.text(detail.slice(0, 400), c.text, { maxHeight: 4 }));
      this.root.add(this.hover);
    } catch {
      /* A path can have disappeared since the turn was written. */
    }
  }

  private transcriptText(source: string): TextRenderable {
    const text = safeText(source);
    const chunks: TextChunk[] = [];
    let at = 0;
    for (const match of text.matchAll(/<\/?[\w-]+|\/?>|[\w-]+(?==)|"[^"\n]*"/g)) {
      if (match.index > at) chunks.push({ __isChunk: true, text: text.slice(at, match.index), fg: c.text });
      chunks.push({
        __isChunk: true,
        text: match[0],
        fg: match[0].startsWith("<")
          ? c.syntaxKeyword
          : match[0].startsWith('"')
            ? c.syntaxString
            : c.syntaxType,
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
      const match = [...line.matchAll(/[a-z][a-z-]*:\/\/[\w./-]+|path="([^"\n]+)"/g)].find(
        (match) =>
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

  private renderInspector(): void {
    const w = this.session;
    if (
      !this.paneChanged(this.inspector, [
        this.theme,
        w.selected,
        w.directory,
        w.usage,
        w.chains,
        w.activity.filter((act) => act.kind === "grant"),
      ])
    )
      return;
    this.clear(this.inspector);
    this.inspector.add(this.text("Chains", c.text, { attributes: 1 }));
    for (const chain of w.chains)
      this.inspector.add(
        this.text(w.labelOf(chain.id), chain.id === w.selected ? c.accent : c.muted, {
          height: space.bar,
          truncate: true,
          attributes: chain.id === w.selected ? 1 : 0,
          bg: chain.id === w.selected ? c.selected : c.panel,
          onMouseDown: () => {
            void w.select(chain.id).catch(w.fail);
          },
        }),
      );
    this.inspector.add(this.text("Directory", c.text, { attributes: 1, marginTop: space.section }));
    const directory = w.directory || w.world.directory;
    this.inspector.add(
      this.text(clip(directory, w.panes.inspector - space.inset * 2, "end"), c.muted, {
        height: space.bar,
        truncate: true,
        onMouseDown: () => this.showValue("Directory", directory),
      }),
    );
    const usage = w.usage;
    if (usage.some((amount) => amount > 0)) {
      const ledger = w.turns
        .flatMap((turn) => turn[1])
        .findLast((part) => isTag(part) && part[0] === "ledger");
      const filled =
        ledger && isTag(ledger) ? Number(ledger[1].find(([name]) => name === "filled")?.[1]) : undefined;
      this.inspector.add(
        this.text(w.demo ? "Simulated usage" : "Usage", c.text, { attributes: 1, marginTop: space.section }),
      );
      this.inspector.add(
        this.text(
          `${dollars(usage[4])}${filled !== undefined && Number.isFinite(filled) ? ` · ${share(filled)}` : ""}`,
        ),
      );
      this.inspector.add(this.text(`${count(usage[0])} in · ${count(usage[1])} out`, c.muted));
      if (usage[2]) this.inspector.add(this.text(`${count(usage[2])} cached`, c.muted));
    }
    const grant = w.activity.find((act) => act.kind === "grant" && !act.done);
    if (grant) {
      this.inspector.add(this.text("Ceiling", c.text, { attributes: 1, marginTop: space.section }));
      if (grant.words[0] !== null) this.inspector.add(this.text(dollars(Number(grant.words[0]))));
      if (grant.words[1] !== null) this.inspector.add(this.text(share(Number(grant.words[1]))));
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
            : c.muted;
  }
  private statusDot(status: SessionStatus): string {
    return status === "saved" || status === "idle" ? "○" : status === "paused" ? "◌" : "●";
  }
  private renderWorkspaces(): void {
    const library = this.options.workspaces;
    if (!library || !this.sidebar.visible) return;
    if (
      !this.paneChanged(this.sidebar, [
        this.theme,
        library.current?.path,
        this.session.preferences.sidebarWidth,
        library.notice,
        library.groups.map((group) => [
          group.name,
          group.collapsed,
          group.sessions.map((entry) => [entry.path, entry.name, entry.status, entry.error]),
        ]),
      ])
    )
      return;
    this.clear(this.sidebar);
    this.hover?.destroyRecursively();
    this.hover = undefined;
    const heading = this.box({ flexDirection: "row", height: space.bar, marginBottom: space.section });
    heading.add(this.text("Workspaces", c.text, { attributes: 1, flexGrow: 1 }));
    this.sidebar.add(heading);
    if (library.notice) this.sidebar.add(this.text(library.notice, c.warning));
    if (!library.groups.length)
      this.sidebar.add(this.text("No workspaces. Use /workspace to add a project.", c.muted));
    for (const group of library.groups) {
      const status = library.groupStatus(group);
      const row = this.box({
        flexDirection: "row",
        height: space.bar,
        onMouseDown: () => library.toggle(group),
        marginTop: group === library.groups[0] ? space.stack : space.section,
      });
      row.add(this.text(group.collapsed ? "▸ " : "▾ ", c.muted));
      row.add(this.text(`${this.statusDot(status)} `, this.statusColor(status)));
      row.add(
        this.text(group.name, c.text, {
          truncate: true,
          flexGrow: 1,
          flexShrink: 1,
          attributes: group === library.groupOf() ? 1 : 0,
        }),
      );
      this.sidebar.add(row);
      const path = this.box({ paddingLeft: space.between, height: space.bar });
      path.add(
        this.text(
          clip(
            group.directory,
            this.session.preferences.sidebarWidth - space.inset * 2 - space.between,
            "end",
          ),
          c.muted,
          { height: space.bar, truncate: true },
        ),
      );
      this.sidebar.add(path);
      if (group.collapsed) continue;
      for (const entry of group.sessions) {
        const selected = library.current === entry;
        const row = this.box({
          flexDirection: "row",
          height: space.bar,
          paddingLeft: space.between,
          backgroundColor: selected ? c.selected : c.panel,
          onMouseDown: () => {
            void library.select(entry).catch(this.report);
          },
          onMouseOver: (event) => {
            this.hover?.destroyRecursively();
            const hint = `${statusLabels[entry.status]} · ${entry.name}${entry.error ? ` · ${entry.error}` : ""}`;
            this.hover = this.box({
              position: "absolute",
              left: Math.min(event.x, this.renderer.width - 40),
              top: Math.min(event.y + 1, this.renderer.height - 2),
              width: Math.min(this.renderer.width - 2, Math.max(25, Bun.stringWidth(hint) + space.inset * 2)),
              paddingX: space.inset,
              backgroundColor: c.raised,
              zIndex: 30,
            });
            this.hover.add(this.text(hint, this.statusColor(entry.status)));
            this.root.add(this.hover);
          },
          onMouseOut: () => {
            this.hover?.destroyRecursively();
            this.hover = undefined;
          },
        });
        row.add(this.text(`${this.statusDot(entry.status)} `, this.statusColor(entry.status)));
        row.add(
          this.text(entry.name, selected ? c.text : c.muted, {
            truncate: true,
            flexGrow: 1,
            flexShrink: 1,
            attributes: selected ? 1 : 0,
          }),
        );
        this.sidebar.add(row);
      }
    }
  }
  /** The workspaces and their sessions in a palette, which opens once the workspaces are read again. */
  workspacePicker = (): Promise<void> => {
    const library = this.options.workspaces;
    if (!library) return Promise.resolve();
    return library
      .refresh()
      .then(() =>
        this.openPalette("Workspaces & sessions", [
          { label: "Add workspace", detail: "Open a project folder", run: () => this.insert("/workspace ") },
          ...library.groups.flatMap((group) => [
            {
              label: group.name,
              detail: group.directory,
              run: async () => {
                const first = group.sessions[0];
                if (first) await library.select(first);
                else await library.create(group);
              },
            },
            ...group.sessions.map((entry) => ({
              label: `  ${entry.name}`,
              detail: `${statusLabels[entry.status]} · ${group.name}`,
              run: () => library.select(entry),
            })),
          ]),
        ]),
      )
      .catch(this.report);
  };
  /** The commands that the view answers itself, by their text. */
  private async globalCommand(text: string): Promise<boolean> {
    const extensions = this.options.extensions;
    if (text.startsWith("/extension ") && extensions) {
      await extensions.load(this.session.path(text.slice(11).trim()));
      this.session.notice = "Extension loaded.";
      return true;
    }
    const [name, ...words] = text.startsWith("/") ? text.slice(1).split(" ") : [];
    const argument = words.join(" ").trim();
    if (name && extensions?.commands.has(name)) {
      await extensions.run(name, words.join(" "));
      return true;
    }
    // A command with no argument that opens a picker, and /inspect, which opens the value it names.
    const pickers: Record<string, () => unknown> = {
      details: this.details,
      rewind: this.rewind,
      tree: () => this.chainTree(),
      queue: this.queuePicker,
      model: this.models,
      effort: this.effortPicker,
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
              this.openPalette(`Delete ${entry.name}?`, [
                { label: "Keep session", detail: "Return without changes", run() {} },
                {
                  label: "Move to trash",
                  detail: "Stop this session and move its record and files to the workspace trash",
                  run: async () => {
                    await library.delete(entry);
                  },
                },
              ]),
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
        detail: `${content.mimeType} · ${kibibytes(Buffer.byteLength(content.data, "base64"))}`,
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
    if (!this.closed) {
      this.insert(value);
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
    this.openPalette("Queued follow-ups", [
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
        detail: `${session.labelOf(entry.chain)} · ${entry.actor}`,
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
    ]);
  };
  private shared = (path: string, markdown: string): void => {
    this.openPalette("Conversation ready to share", [
      {
        label: "Open HTML",
        detail: path,
        run: () => openFile(path),
      },
      {
        label: "Copy path",
        detail: path,
        run: () => {
          this.renderer.copyToClipboardOSC52(path);
          this.session.notice = "Export path copied.";
        },
      },
      {
        label: "Upload an unlisted GitHub gist",
        detail: "Anyone with the link can read this conversation and its images. Requires gh login.",
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
    ]);
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
  private actActions(act: ActRow): void {
    this.openPalette(`Act · ${act.kind}`, [
      {
        label: "Inspect activity",
        detail: act.id,
        run: () => {
          this.session.show("activity");
        },
      },
      ...(act.kind === "prompt"
        ? [
            {
              label: "Edit program",
              detail: "Edit and replay this prompt's Python",
              run: () => this.action(`/edit ${act.id}`),
            },
          ]
        : []),
    ]);
  }
  palette(): void {
    const choices: Choice[] = views.map((view) => ({
      label: `${title(view)} view`,
      detail: `Inspect ${view}`,
      run: () => this.session.show(view),
    }));
    for (const [name, command] of this.options.extensions?.commands ?? [])
      choices.push({ label: command.label, detail: command.description, run: () => this.action(`/${name}`) });
    if (this.options.newSession)
      choices.unshift({ label: "New session", detail: "Start a fresh life", run: this.options.newSession });
    choices.push(
      ...Object.entries(commands)
        .filter(([name]) => name !== "new")
        .map(([name, [label, argument, detail]]) => ({ label, detail, run: this.command(name, argument) })),
    );
    choices.push({
      label: "Inspect a name",
      detail: "Values from this chain's module",
      run: () => this.names(),
    });
    choices.push({
      label: "Prompt programs",
      detail: "Open a ladder, inspect it, or edit it",
      run: () => this.ladders(),
    });
    choices.push({
      label: "Python input",
      detail: "Write code with the same gate as the model",
      run: () => this.toggleMode(),
    });
    choices.push({
      label: "Response shape",
      detail: "Choose the type this prompt should return",
      run: () => this.shapes(),
    });
    choices.push({ label: "Switch chain", detail: "Go to any conversation", run: () => this.chains() });
    choices.push({ label: "Help", detail: "Keyboard and slash commands", run: () => this.help() });
    this.openPalette("Commands", choices);
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
  private token(): { kind: "/" | "@"; text: string } | undefined {
    const before = this.beforeCursor();
    const slash = before.match(/^\/([\w-]*)$/);
    if (slash) return { kind: "/", text: slash[1] ?? "" };
    const at = before.match(/(?:^|\s)@([^\s"']*)$/);
    if (at && this.session.mode === "prompt" && !this.session.editing)
      return { kind: "@", text: at[1] ?? "" };
    return undefined;
  }
  /** The token before the cursor as it is now, replaced with the text of a suggestion. */
  private complete(suggestion: Suggestion): void {
    const token = this.token();
    if (token) this.replaceBefore(`${token.kind}${token.text}`, suggestion.text);
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
        ...[...(this.options.extensions?.commands ?? [])].map(([name, command]) => ({
          name,
          argument: "",
          detail: command.description,
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
    } else {
      const read = this.session.projectFiles(started);
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
      const wanted = token.text.toLowerCase();
      this.suggestions = (this.files.paths ?? [])
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
      : token.kind === "@" && this.files?.error
        ? this.files.error
        : token.kind === "@" && !this.files?.paths
          ? "Finding project files..."
          : shown.length
            ? ""
            : token.kind === "/"
              ? "No command starts with that name."
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
      this.suggestionBox.add(this.text(state, this.files?.error ? c.danger : c.muted, { height: space.bar }));
    const rows = 8;
    const first = Math.max(0, Math.min(this.suggestionIndex - rows + 1, shown.length - rows));
    for (const [offset, one] of shown.slice(first, first + rows).entries()) {
      const index = first + offset;
      const selected = index === this.suggestionIndex;
      const row = this.box({
        flexDirection: "row",
        height: space.bar,
        gap: space.between,
        paddingX: space.inset,
        backgroundColor: selected ? c.selected : c.panel,
        onMouseDown: () => {
          this.suggestionIndex = index;
          this.complete(one);
        },
      });
      row.add(
        this.text(one.label, selected ? c.accent : c.text, { flexShrink: 0, attributes: selected ? 1 : 0 }),
      );
      if (one.detail) row.add(this.text(one.detail, c.muted, { truncate: true, flexShrink: 1 }));
      this.suggestionBox.add(row);
    }
  }
  models = (): void => {
    this.openPalette(
      "Model",
      this.session.roster
        .filter(([name]) => name !== "operator")
        .map(([name, , window]) => ({
          label: name,
          detail: `${count(window)} context${name === this.session.actorChoice.model ? " · selected" : ""}`,
          run: () => this.action(`/model ${name}`),
        })),
    );
  };
  effortPicker = (): void => {
    const { model, effort } = this.session.actorChoice;
    const offered = this.session.roster.find(([name]) => name === model)?.[1] ?? [];
    this.openPalette(
      `Effort · ${model}`,
      offered.map((name) => ({
        label: `${name}${name === effort ? " · current" : ""}`,
        detail: "",
        run: () => this.action(`/effort ${name}`),
      })),
      offered.indexOf(effort),
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
    this.openPalette(
      "Color theme",
      Object.keys(palettes).map((name) => ({
        label: name === "github" ? "GitHub Dark" : title(name),
        detail: name === "paper" ? "Light" : "Dark",
        run: () => {
          this.session.theme = name as ThemeName;
          this.render();
          this.session.save();
        },
      })),
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
    for (const [start, end, group] of result?.highlights ?? []) {
      const styleId =
        this.style.resolveStyleId(group) ?? this.style.resolveStyleId(group.split(".")[0] ?? "default");
      if (styleId !== null) this.composer.addHighlightByCharRange({ start, end, styleId });
    }
    if (content === this.session.rejectedWord || content === `/run ${this.session.rejectedWord}`) {
      const styleId = this.style.resolveStyleId("diagnostic");
      const lines = content.split("\n");
      for (const finding of this.session.findings) {
        const line = Number(finding.match(/line (\d+)/)?.[1] ?? 0) - 1;
        if (styleId !== null && line >= 0) {
          const start = lines.slice(0, line).reduce((size, line) => size + line.length + 1, 0);
          this.composer.addHighlightByCharRange({ start, end: start + (lines[line]?.length ?? 0), styleId });
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
          if (styleId !== null)
            for (const start of [at, index])
              this.composer.addHighlightByCharRange({ start, end: start + 1, styleId });
          break;
        }
      }
    }
  }
  private progress(id: string): string {
    const elapsed = Math.max(0, Math.floor((Date.now() - (this.session.started[id] ?? Date.now())) / 1000));
    return `${["◐", "◓", "◑", "◒"][Math.floor(Date.now() / 250) % 4]} ${elapsed}s since start`;
  }
  private resume = (): void => {
    const held = this.session.world.held;
    this.openPalette("Saved work is paused", [
      {
        label: "Resume saved work",
        detail: `${held.size} unfinished acts. Interrupted commands keep their output and report an interruption.`,
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
      this.openPalette("Operator question · bool", [
        {
          label: "Yes",
          detail: "Answer true",
          run: () => this.session.world.answer(question.id, "yes").then(() => {}),
        },
        {
          label: "No",
          detail: "Answer false",
          run: () => this.session.world.answer(question.id, "no").then(() => {}),
        },
      ]);
      this.showQuestionText(question.message);
      return;
    }
    this.openPalette(`Your answer · ${question.shape}`, []);
    this.showQuestionText(question.message);
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
    this.overlay.top = 1;
    this.questionDocument = new ScrollBoxRenderable(this.renderer, {
      height: Math.max(3, Math.min(8, this.renderer.height - 18)),
      scrollX: false,
      scrollY: true,
    });
    this.questionDocument.add(this.markdown(message));
    this.overlay.insertBefore(this.questionDocument, this.paletteInput);
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
        maxHeight: 8,
        padding: space.inset,
        border: true,
        borderStyle: "rounded",
        borderColor: c.text,
        backgroundColor: c.raised,
        zIndex: 30,
        onMouseDown: () => this.inspect(name),
      });
      this.hover.add(this.text(`${name}  ·  ${inspected.kind}`, c.text));
      this.hover.add(this.text(inspected.representation.slice(0, 280), c.text, { maxHeight: 4 }));
      this.hover.add(this.text("Click to expand  ·  Ctrl+G keyboard inspector", c.muted));
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
        this.showValue(`${name} · ${value.kind}`, value.value ?? value.representation, name);
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
            this.go("program", definition[0]);
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
            this.openPalette(`${name} · engine.py:${at + 1}`, [
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
          label: `${key}  ${typeof child === "object" && child !== null ? "▸" : ""}`,
          detail: display(child).replaceAll("\n", " ").slice(0, 110),
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
    if (typeof value === "string" && value.includes("://"))
      choices.push({
        label: "Follow this act or door",
        detail: value,
        run: () => {
          if (value.startsWith("chain://")) return this.session.select(value);
          void this.session.life
            .read(value, undefined, this.session.selected)
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
    const names = (await this.session.life.held("modules", [this.session.selected], "keys")) as string[];
    this.openPalette(
      "Inspect a name",
      names
        .filter((name) => !name.startsWith("_"))
        .map((name) => ({
          label: name,
          detail: "Read its live value, type, and fields",
          run: () => this.inspect(name),
        })),
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
  ladders(): void {
    this.openPalette(
      "Prompt programs",
      this.session.activity
        .filter((act) => act.kind === "prompt")
        .map((act) => ({
          label: `${act.done ? "✓" : "◌"} ${short(act.id)}`,
          detail: String(act.words[1]),
          run: () => this.openLadder(act.id),
        })),
    );
  }
  private openLadder(id: string): void {
    this.go("program");
    this.session.ladder = id;
    this.session.editing = undefined;
    this.session.mode = "python";
    this.render();
    this.session.notice = `${id}: run a rung below, or /edit ${id} to change its program.`;
  }
  rewind = (): void => {
    if (this.session.paused) {
      this.openPalette("Resume this chain before rewinding", [
        {
          label: "Resume chain",
          detail: "Then choose the last act the new chain will read.",
          run: () => (this.session.world.held.size ? this.resume() : this.action("/wake")),
        },
      ]);
      return;
    }
    const acts = this.session.activity.filter((act) => !["chain", "grant"].includes(act.kind));
    this.openPalette(
      "Rewind transcript · module and files stay current",
      acts.map((act, index) => ({
        label: `${"  ".repeat(Math.max(0, short(act.id).split(".").length - 2))}${act.kind} · ${short(act.id)}`,
        detail:
          String(
            act.kind === "prompt" ? act.words[1] : act.words[0] || this.session.program[act.id] || "",
          ).split("\n")[0] ?? "",
        run: async () => {
          if (this.session.paused) {
            this.rewind();
            return;
          }
          await this.session.branch(
            `${this.session.label} through ${short(act.id)}`,
            acts.slice(index + 1).map((later) => later.id),
          );
          this.session.notice =
            "The new chain reads the selected transcript prefix. Its module and files keep current state.";
        },
      })),
    );
  };
  private go(view: View, id?: string): void {
    this.navigation.push({
      chain: this.session.selected,
      view: this.session.view,
      search: this.session.search,
      place: this.place,
      ladder: this.session.ladder,
      mode: this.session.mode,
    });
    if (
      id &&
      this.session.ladder &&
      !short(id).startsWith(`${short(this.session.ladder)}.`) &&
      !this.session.repls[this.session.ladder]?.includes(id)
    )
      this.session.ladder = undefined;
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
    this.session.ladder = previous.ladder;
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
    if (typeof target === "object") this.scroll.scrollChildIntoView(target.card);
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
  chains(): void {
    this.openPalette(
      "Chains",
      this.session.chains.map((chain) => ({
        label: this.session.labelOf(chain.id),
        detail: chain.id,
        run: () => this.session.select(chain.id),
      })),
    );
  }
  private chainTree = (focus?: string): void => {
    const choices: (Choice & { id: string })[] = [];
    const chains = this.session.chains;
    const visit = (chain: ActRow, depth: number) => {
      const children = chains.filter((child) => child.words[1] === chain.id);
      const key = `chain:${chain.id}`;
      const closed = this.folds.get(key) ?? false;
      choices.push({
        id: chain.id,
        label: `${"  ".repeat(depth)}${children.length ? (closed ? "▸" : "▾") : " "} ${this.session.labelOf(chain.id)}`,
        detail: `${chain.id}${chain.id === this.session.selected ? " · current" : ""}`,
        run: () => this.session.select(chain.id),
        ...(children.length
          ? {
              toggle: () => {
                this.folds.set(key, !closed);
                this.chainTree(chain.id);
              },
            }
          : {}),
      });
      if (!closed) for (const child of children) visit(child, depth + 1);
    };
    for (const chain of chains.filter((chain) => !chains.some((parent) => parent.id === chain.words[1])))
      visit(chain, 0);
    this.openPalette(
      "Session tree · Left/Right folds, Enter opens",
      choices,
      choices.findIndex((choice) => choice.id === (focus ?? this.session.selected)),
    );
  };
  openPalette(label: string, choices: Choice[], selected = 0, query = ""): void {
    this.closeOverlay();
    this.composer.blur();
    const width = Math.min(76, this.renderer.width - 4);
    this.overlay = this.box({
      id: "palette",
      position: "absolute",
      left: Math.floor((this.renderer.width - width) / 2),
      top: 4,
      width,
      maxHeight: this.paletteHeight,
      padding: space.inset,
      gap: space.stack,
      border: true,
      borderStyle: "rounded",
      borderColor: c.accent,
      backgroundColor: c.raised,
      zIndex: 20,
    });
    this.root.add(this.overlay);
    this.overlay.add(this.text(label, c.text, { attributes: 1 }));
    this.paletteInput = new InputRenderable(this.renderer, {
      id: "palette-search",
      placeholder: "Type to filter...",
      backgroundColor: c.panel,
      textColor: c.text,
      placeholderColor: c.muted,
    });
    this.overlay.add(this.paletteInput);
    this.paletteList = this.box({ gap: space.stack });
    this.overlay.add(this.paletteList);
    const filter = (value: string) =>
      choices.filter((choice) =>
        `${choice.label} ${choice.detail}`.toLowerCase().includes(value.toLowerCase()),
      );
    this.filtered = filter(query);
    this.selection = Math.max(0, selected);
    this.paletteInput.value = query;
    this.paletteInput.on(InputRenderableEvents.INPUT, (value: string) => {
      this.filtered = filter(value);
      this.selection = 0;
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
      : Math.max(12, this.renderer.height - 8);
  }
  private renderChoices(): void {
    if (!this.paletteList) return;
    this.clear(this.paletteList);
    // The list has the rows of the palette but its border, its inset, its label, its input and the text it asks.
    const asked = this.questionDocument?.parent === this.overlay ? (this.questionDocument?.height ?? 0) : 0;
    const room = Math.max(2, this.paletteHeight - 2 - space.inset * 2 - 2 * space.bar - asked);
    const rows = (choice?: Choice) => (choice?.detail ? 2 : 1) * space.bar;
    // The window starts as far back as the rows let it, and ends at the selection when all before it do not fit.
    let start = this.selection + 1;
    for (let used = 0; start > 0 && used + rows(this.filtered[start - 1]) <= room; start--)
      used += rows(this.filtered[start - 1]);
    const shown: Choice[] = [];
    for (let used = 0, next = start; next < this.filtered.length; next++) {
      const choice = this.filtered[next];
      used += rows(choice);
      if (!choice || used > room) break;
      shown.push(choice);
    }
    for (const [index, choice] of shown.entries()) {
      const selected = index + start === this.selection;
      const row = this.box({
        backgroundColor: selected ? c.selected : c.raised,
        onMouseDown: (event) => {
          if (event.button === 2 && choice.toggle) {
            choice.toggle();
            return;
          }
          this.selection = index + start;
          this.choose();
        },
      });
      row.add(
        this.text(`${selected ? "▸" : " "} ${choice.label}`, selected ? c.accent : c.text, {
          height: space.bar,
          truncate: true,
          attributes: selected ? 1 : 0,
        }),
      );
      if (choice.detail)
        row.add(this.text(`  ${choice.detail}`, c.muted, { height: space.bar, truncate: true }));
      this.paletteList.add(row);
    }
    if (!this.filtered.length) this.paletteList.add(this.text("No matching actions.", c.muted));
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
    this.overlay?.destroyRecursively();
    this.overlay = undefined;
    this.paletteInput = undefined;
    this.paletteList = undefined;
    this.questionDocument = undefined;
    this.composer?.focus();
  }
  /** The keys that this terminal sends, the session dots, and the commands. */
  help(): void {
    const kitty = this.renderer.capabilities?.kitty_keyboard === true;
    this.openPalette(
      "Help & keyboard",
      keys
        .map((key) => ({ label: chords(key, kitty), detail: key.action, run: () => {} }))
        .concat({
          label: "Session dots",
          detail:
            "Blue: working. Yellow: input needed or paused. Green: unread result. Red: error. Open: ready or saved.",
          run: () => {},
        })
        .concat(
          Object.entries(commands).map(([name, [, argument, detail]]) => ({
            label: `/${name}${argument ? ` ${argument}` : ""}`,
            detail,
            run: () => {},
          })),
        ),
    );
  }
  private key = (key: KeyEvent): void => {
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
      if (key.name === "tab" && !key.shift && chosen) {
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
        const token = this.token();
        this.dismissed = token ? `${token.kind}${token.text}` : "";
        this.suggest();
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
      this.options.workspaces?.toggle();
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
    if (!this.overlay && key.meta && ["up", "down"].includes(key.name)) {
      key.preventDefault();
      const history = this.session.histories[this.draftKey] ?? [];
      if (this.historyIndex < 0) {
        this.historyDraft = this.composer.plainText;
        this.historyIndex = history.length;
      }
      this.historyIndex = Math.max(
        0,
        Math.min(history.length, this.historyIndex + (key.name === "up" ? -1 : 1)),
      );
      this.composer.replaceText(history[this.historyIndex] ?? this.historyDraft);
      return;
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
      const prompts = this.session.activity.filter((act) => act.kind === "prompt");
      const current = prompts.findIndex((act) => act.id === this.session.ladder);
      const step = ["]", "n"].includes(key.name) ? 1 : -1;
      const next = prompts[(current + step + prompts.length) % prompts.length];
      if (next) this.openLadder(next.id);
      return;
    }
    if (key.ctrl && key.name === "c") {
      key.preventDefault();
      if (this.overlay) this.closeOverlay();
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
      if (
        !this.overlay &&
        !this.search.visible &&
        !this.session.editing &&
        !this.session.paused &&
        this.session.activity.some((act) => act.kind === "prompt" && !act.done)
      )
        this.action("/pause");
      this.closeOverlay();
      this.search.visible = false;
      this.session.search = "";
      this.session.editing = undefined;
      this.render();
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
      if (["up", "down"].includes(key.name)) {
        key.preventDefault();
        this.selection = Math.max(
          0,
          Math.min(this.filtered.length - 1, this.selection + (key.name === "up" ? -1 : 1)),
        );
        this.renderChoices();
      }
      return;
    }
    // Each chord with Ctrl needs the kitty keyboard protocol, and the same chord with Alt reaches every terminal.
    if ((key.ctrl || key.meta) && /^[1-6]$/.test(key.name)) {
      key.preventDefault();
      this.session.show(views[Number(key.name) - 1] ?? "conversation");
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
      void this.options
        .sessions?.()
        .then((choices) => this.openPalette("Sessions", choices))
        .catch(this.report);
    } else if (key.ctrl && key.name === "f") {
      key.preventDefault();
      this.search.visible = true;
      this.search.focus();
    } else if (key.name === "pageup" || key.name === "pagedown") {
      key.preventDefault();
      this.scroll.scrollBy((key.name === "pageup" ? -1 : 1) * (this.renderer.height - 12));
    }
  };
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

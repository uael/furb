import { basename } from "node:path";
import { efforts, shapes } from "@furb/engine";
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
import { createTwoFilesPatch } from "diff";
import { commands } from "./commands.ts";
import { loadParsers } from "./parsers.ts";
import { theme as c, palettes, setTheme, syntax, type ThemeName } from "./theme.ts";
import type { ActRow, View, Workspace } from "./workspace.ts";

const views: View[] = ["conversation", "program", "activity", "facts", "transcript", "changes"];
const title = (text: string) => text.charAt(0).toUpperCase() + text.slice(1);
const short = (id: string) => id.replace(/^[^:]+:\/\//, "");
const count = (value: number) => (value > 999 ? `${(value / 1000).toFixed(1)}k` : String(value));
interface Choice {
  label: string;
  detail: string;
  run(): void | Promise<void>;
}

export interface AppOptions {
  quit(): void | Promise<void>;
  sessions?: () => Promise<Choice[]>;
  newSession?: () => Promise<void>;
}

export class App {
  readonly root: BoxRenderable;
  readonly composer: TextareaRenderable;
  readonly scroll: ScrollBoxRenderable;
  private style: ReturnType<typeof syntax>;
  private theme: ThemeName = "forest";
  private draftKey = "";
  private hover?: BoxRenderable;
  private questionDocument?: ScrollBoxRenderable;
  private hoverTimer?: ReturnType<typeof setTimeout>;
  private editorVersion = 0;
  private closed = false;
  private readonly collapsed = new Set<string>();
  private readonly sidebar: BoxRenderable;
  private readonly inspector: BoxRenderable;
  private readonly splitters: BoxRenderable[] = [];
  private readonly tabs: BoxRenderable;
  private readonly head: TextRenderable;
  private readonly hint: TextRenderable;
  private readonly status: TextRenderable;
  private readonly composeBox: BoxRenderable;
  private readonly promptBox: BoxRenderable;
  private readonly search: InputRenderable;
  private readonly paneKeys = new WeakMap<Renderable, string>();
  private readonly cards = new Map<string, { node: BoxRenderable; key: string }>();
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
    query: string;
    top: number;
    ladder?: string;
    mode: "prompt" | "python";
  }[] = [];

  constructor(
    readonly renderer: CliRenderer,
    readonly workspace: Workspace,
    readonly options: AppOptions,
  ) {
    setTheme(workspace.theme);
    for (const id of workspace.collapsed) this.collapsed.add(id);
    this.theme = workspace.theme;
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
      height: 3,
      paddingX: 2,
      flexDirection: "row",
      alignItems: "center",
      backgroundColor: c.panel,
      gap: 2,
    });
    header.add(this.text("▰  furb", c.accent, { width: 12 }));
    this.head = this.text("", c.muted, { flexGrow: 1 });
    header.add(this.head);
    const commands = this.text("⌘  Ctrl+P  commands", c.muted, { onMouseDown: () => this.palette() });
    header.add(commands);
    this.root.add(header);

    const body = this.box({ flexGrow: 1, flexShrink: 1, flexDirection: "row", minHeight: 0 });
    this.root.add(body);
    this.sidebar = new ScrollBoxRenderable(renderer, {
      width: workspace.panes.sidebar,
      flexShrink: 0,
      scrollX: false,
      scrollY: true,
      backgroundColor: c.panel,
      contentOptions: { padding: 1, gap: 1, minHeight: "100%" },
      verticalScrollbarOptions: { visible: false },
      horizontalScrollbarOptions: { visible: false },
    });
    body.add(this.sidebar);
    const leftSplitter = this.box({
      width: 1,
      backgroundColor: c.border,
      onMouseDrag: (event) => {
        workspace.panes.sidebar = Math.max(18, Math.min(44, event.x));
        this.sidebar.width = workspace.panes.sidebar;
      },
      onMouseOver() {
        this.backgroundColor = c.teal;
      },
      onMouseOut() {
        this.backgroundColor = c.border;
      },
    });
    body.add(leftSplitter);
    this.splitters.push(leftSplitter);
    const center = this.box({
      flexGrow: 1,
      flexShrink: 1,
      minHeight: 0,
      minWidth: 0,
      paddingX: 2,
      paddingTop: 1,
    });
    body.add(center);
    this.tabs = this.box({ height: 2, flexDirection: "row", gap: 2 });
    center.add(this.tabs);
    this.search = new InputRenderable(renderer, {
      id: "search",
      placeholder: "Filter this view. Esc to return to the composer.",
      visible: false,
      backgroundColor: c.raised,
      textColor: c.text,
      placeholderColor: c.faint,
    });
    this.search.on(InputRenderableEvents.INPUT, (value: string) => {
      workspace.query = value;
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
      contentOptions: { gap: 1, paddingBottom: 1 },
      verticalScrollbarOptions: { visible: false },
      horizontalScrollbarOptions: { visible: false },
    });
    center.add(this.scroll);
    this.promptBox = this.box({
      id: "operator-prompt",
      maxHeight: 8,
      visible: false,
      padding: 1,
      border: true,
      borderStyle: "rounded",
      borderColor: c.yellow,
      backgroundColor: c.raised,
      onMouseDown: () => this.question(),
    });
    center.add(this.promptBox);
    this.composeBox = this.box({
      id: "composer-box",
      border: true,
      borderStyle: "rounded",
      borderColor: c.border,
      paddingX: 1,
      paddingTop: 1,
      height: 6,
      flexShrink: 0,
      backgroundColor: c.panel,
      titleColor: c.muted,
    });
    center.add(this.composeBox);
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
      keyBindings: [
        { name: "return", action: "submit" },
        { name: "return", shift: true, action: "newline" },
        { name: "j", ctrl: true, action: "newline" },
      ],
      syntaxStyle: this.style,
      onContentChange: () => {
        void this.highlightEditor();
      },
      onCursorChange: () => {
        void this.highlightEditor();
      },
      onSubmit: () => {
        void this.submit();
      },
    });
    this.composeBox.add(this.composer);
    this.hint = this.text("", c.muted, { height: 1 });
    this.composeBox.add(this.hint);
    this.status = this.text("", c.faint, { height: 2, paddingTop: 1 });
    center.add(this.status);
    const rightSplitter = this.box({
      width: 1,
      backgroundColor: c.border,
      onMouseDrag: (event) => {
        workspace.panes.inspector = Math.max(24, Math.min(44, renderer.width - event.x));
        this.inspector.width = workspace.panes.inspector;
      },
      onMouseOver() {
        this.backgroundColor = c.teal;
      },
      onMouseOut() {
        this.backgroundColor = c.border;
      },
    });
    body.add(rightSplitter);
    this.splitters.push(rightSplitter);
    this.inspector = new ScrollBoxRenderable(renderer, {
      width: workspace.panes.inspector,
      flexShrink: 0,
      scrollX: false,
      scrollY: true,
      backgroundColor: c.panel,
      contentOptions: { padding: 1, gap: 0, minHeight: "100%" },
      verticalScrollbarOptions: { visible: false },
      horizontalScrollbarOptions: { visible: false },
    });
    body.add(this.inspector);
    this.root.add(
      this.text(
        "  F1 help   Ctrl+N chain   Ctrl+F search   Ctrl+M model   Ctrl+O sessions   Ctrl+Q quit",
        c.faint,
        { height: 1, bg: c.panel },
      ),
    );
    workspace.on("change", this.schedule);
    workspace.on("compose", this.compose);
    workspace.on("inspect", this.inspect);
    workspace.on("resume", this.resume);
    workspace.on("rewind", this.rewind);
    renderer.keyInput.on("keypress", this.key);
    renderer.on("resize", this.render);
    this.tick = setInterval(() => {
      if (
        !this.workspace.paused &&
        this.workspace.activity.some(
          (act) => !act.done && ["prompt", "bash", "wait", "rung"].includes(act.kind),
        )
      )
        this.schedule();
    }, 250);
    this.render();
    this.composer.focus();
    void loadParsers()
      .then(() => {
        if (!this.closed) {
          this.lastView = "";
          this.render();
          void this.highlightEditor();
        }
      })
      .catch(workspace.fail);
    if (workspace.world.held.size) this.resume();
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
      wrapMode: "word",
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
  private compose = (text: string) => {
    if (this.draftKey) this.workspace.drafts[this.draftKey] = this.composer.plainText;
    this.draftKey = `${this.workspace.selected}:${this.workspace.editing ?? this.workspace.ladder ?? this.workspace.mode}`;
    this.composer.setText(text);
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
  private async submit(): Promise<void> {
    if (this.submitting) return;
    this.submitting = true;
    const content = this.composer.plainText;
    try {
      if (content.trim() === "/new" && this.options.newSession) {
        await this.options.newSession();
        return;
      }
      await this.workspace.submit(
        this.workspace.mode === "python" && !this.workspace.editing && !content.startsWith("/")
          ? `/run ${content}`
          : content,
      );
      const history = this.workspace.histories[this.draftKey] ?? [];
      this.workspace.histories[this.draftKey] = history;
      if (content && history.at(-1) !== content) history.push(content);
      if (history.length > 200) history.shift();
      this.historyIndex = -1;
      if (!content.startsWith("/edit")) this.composer.setText("");
    } catch (error) {
      this.workspace.fail(error);
    } finally {
      this.submitting = false;
      this.render();
    }
  }

  render = (): void => {
    if (this.closed) return;
    const w = this.workspace;
    if (w.theme !== this.theme) this.applyTheme(w.theme);
    const draftKey = `${w.selected}:${w.editing ?? w.ladder ?? w.mode}`;
    if (this.draftKey !== draftKey) {
      if (this.draftKey) w.drafts[this.draftKey] = this.composer.plainText;
      this.draftKey = draftKey;
      this.composer.setText(w.drafts[draftKey] ?? "");
    }
    this.sidebar.visible = this.renderer.width >= 95;
    this.inspector.visible = this.renderer.width >= 132;
    if (this.splitters[0]) this.splitters[0].visible = this.sidebar.visible;
    if (this.splitters[1]) this.splitters[1].visible = this.inspector.visible;
    this.head.content = `${w.sessionName}  /  ${w.label}${w.demo ? "   DEMO" : ""}`;
    const sidebarState = [
      w.selected,
      w.sessionName,
      this.theme,
      w.chains.map((chain) => [
        chain.id,
        w.labelOf(chain.id),
        w.acts.filter((act) => act.on === chain.id && !act.done && ["prompt", "bash"].includes(act.kind))
          .length,
      ]),
      [...w.world.prompts.keys()],
    ];
    if (this.paneChanged(this.sidebar, sidebarState)) {
      this.clear(this.sidebar);
      this.sidebar.add(this.text("WORKSPACE", c.faint));
      this.sidebar.add(this.text(basename(w.world.directory), c.text));
      this.sidebar.add(this.text("CHAINS", c.faint, { marginTop: 1 }));
      for (const chain of w.chains) {
        const selected = chain.id === w.selected;
        const row = this.box({
          paddingX: 1,
          paddingY: 1,
          backgroundColor: selected ? c.selected : c.panel,
          onMouseDown: () => {
            void w.select(chain.id).catch(w.fail);
          },
        });
        const pending = w.acts.filter(
          (act) => act.on === chain.id && !act.done && ["prompt", "bash"].includes(act.kind),
        ).length;
        const needsInput = [...w.world.prompts.values()].some(
          (prompt) => w.acts.find((act) => act.id === prompt.id)?.on === chain.id,
        );
        row.add(
          this.text(
            `${needsInput ? "?" : selected ? "▸" : "·"} ${w.labelOf(chain.id)}`,
            needsInput ? c.yellow : selected ? c.accent : c.muted,
          ),
        );
        row.add(
          this.text(`  ${pending ? `${pending} active` : "ready"}  ·  ${short(chain.id)}`, c.faint, {
            height: 1,
            truncate: true,
          }),
        );
        this.sidebar.add(row);
      }
      this.sidebar.add(
        this.text("+ New chain", c.teal, { marginTop: 1, onMouseDown: () => this.insert("/chain ") }),
      );
      this.sidebar.add(this.text("⑂ Fork this chain", c.muted, { onMouseDown: () => this.insert("/fork ") }));
      this.sidebar.add(this.box({ flexGrow: 1 }));
      this.sidebar.add(this.text(w.world.records.path ? "●  Session saved" : "○  In memory", c.accent));
      this.sidebar.add(this.text("One life. Every step kept.", c.faint));
    }

    if (this.paneChanged(this.tabs, [w.view, this.theme, this.renderer.width])) {
      this.clear(this.tabs);
      for (const [index, view] of views.entries()) {
        const label =
          this.renderer.width < 132
            ? ["Chat", "Code", "Acts", "Facts", "Transcript", "Diffs"][index]
            : title(view);
        this.tabs.add(
          this.text(`${index + 1} ${label}`, w.view === view ? c.accent : c.faint, {
            onMouseDown: () => w.show(view),
            attributes: w.view === view ? 1 : 0,
          }),
        );
      }
    }
    const pending = w.operatorPrompt;
    this.promptBox.visible = !!pending;
    if (this.paneChanged(this.promptBox, [pending, this.theme])) {
      this.clear(this.promptBox);
      if (pending) {
        this.promptBox.add(this.text(`?  YOUR INPUT  ·  ${pending.shape}`, c.yellow));
        this.promptBox.add(this.text(pending.message, c.text, { maxHeight: 3, truncate: true }));
        this.promptBox.add(this.text("Reply below, click here, or press Ctrl+A.", c.muted));
      }
    }
    this.composer.placeholder = w.editing
      ? "Edit this prompt's Python program..."
      : pending
        ? `Your ${pending.shape} answer...`
        : w.mode === "python"
          ? "Write Python. Ctrl+Space completes a name. Ctrl+R returns to chat."
          : "Ask anything, or type / for a command...";
    this.composeBox.borderColor = w.editing || pending ? c.yellow : c.border;
    this.composeBox.title = w.editing
      ? " EDIT PROGRAM "
      : pending
        ? " REPLY TO OPERATOR PROMPT "
        : w.mode === "python"
          ? ` PYTHON${w.ladder ? ` · ${short(w.ladder)}` : ""} · same gate, same chain `
          : "";
    this.hint.content = `${w.actor}  → ${w.shape}  ${w.paused ? "◌ paused" : w.world.streams.size ? "● working" : "○ ready"}    Enter send  ·  Shift+Enter newline`;
    this.status.fg = w.error ? c.red : c.faint;
    this.status.content =
      w.error || `${w.notice}  ${w.world.records.path ? basename(w.world.records.path) : ""}`;
    this.renderContent();
    this.renderInspector();
    const diagnostics = JSON.stringify([w.rejectedWord, w.findings]);
    if (diagnostics !== this.diagnosticsKey) {
      this.diagnosticsKey = diagnostics;
      void this.highlightEditor();
    }
  };

  private card(
    id: string,
    key: string,
    label: string,
    color: RGBA,
    body: (box: BoxRenderable) => void,
    index: number,
  ): void {
    key += this.collapsed.has(id) ? ":collapsed" : ":open";
    const prior = this.cards.get(id);
    if (prior?.key === key) return;
    if (prior) {
      prior.node.destroyRecursively();
      this.cards.delete(id);
    }
    const box = this.box({
      id,
      paddingX: 1,
      paddingY: 1,
      border: ["left"],
      borderColor: color,
      backgroundColor: c.panel,
      flexShrink: 0,
    });
    box.add(
      this.text(`${this.collapsed.has(id) ? "▸" : "▾"} ${label}`, color, {
        marginBottom: this.collapsed.has(id) ? 0 : 1,
        onMouseDown: () => {
          if (this.collapsed.has(id)) this.collapsed.delete(id);
          else this.collapsed.add(id);
          this.renderContent();
        },
      }),
    );
    if (!this.collapsed.has(id)) body(box);
    this.scroll.add(box, index);
    this.cards.set(id, { key, node: box });
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
      const row = y - code.y;
      const source = code.getLineSources(row, 1)[0] ?? row;
      const line = content.split("\n")[source] ?? "";
      const column = x - code.x + (code.lineInfo.lineStartCols[row] ?? 0);
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
      if (name && (event.modifiers.ctrl || event.modifiers.alt || Reflect.get(event.modifiers, "meta")))
        this.inspect(name);
    };
    return code;
  }
  private markdown(content: string): MarkdownRenderable {
    return new MarkdownRenderable(this.renderer, {
      content: safeText(content),
      syntaxStyle: this.style,
      fg: c.text,
    });
  }

  renderContent(): void {
    const w = this.workspace;
    const view = `${w.selected}:${w.view}:${w.query}`;
    if (this.lastView !== view) {
      if (this.lastView) w.scrolls[this.lastView] = this.scroll.scrollTop;
      this.clear(this.scroll);
      this.cards.clear();
      this.lastView = view;
      this.scroll.stickyScroll = w.view === "conversation";
      this.scroll.scrollTo(w.scrolls[view] ?? 0);
    }
    const existing = new Set(this.cards.keys());
    let order = 0;
    const add = (id: string, key: string, label: string, color: RGBA, body: (box: BoxRenderable) => void) => {
      existing.delete(id);
      this.card(id, key, label, color, body, order++);
    };
    const matches = (text: string) => !w.query || text.toLowerCase().includes(w.query.toLowerCase());
    if (w.view === "conversation") {
      let items = 0;
      for (const [index, turn] of w.turns.entries()) {
        for (const [part, content] of turn[1].entries()) {
          const id = `turn-${index}-${part}`;
          if (typeof content === "string") {
            if (!matches(content)) continue;
            items++;
            add(id, content, `✦  ${w.actor.split("/")[0]}  ·  Python`, c.teal, (box) => {
              box.add(this.code(content));
              box.add(
                this.text("Inspect names: hover or Ctrl+click  ·  Ctrl+G opens the inspector", c.faint, {
                  marginTop: 1,
                }),
              );
            });
          } else if (isTag(content)) {
            const [name, attrs, body] = content;
            const attributes = Object.fromEntries(attrs);
            const act = String(attributes.id ?? attributes.over ?? "");
            if (
              (name === "opened" &&
                (act.startsWith("chain://") ||
                  act.startsWith("grant://") ||
                  (act.startsWith("rung://") && w.acts.find((one) => one.id === act)?.by !== "operator"))) ||
              (name === "closed" && act.startsWith("rung://") && body === null) ||
              name === "ledger"
            )
              continue;
            if (!matches(JSON.stringify(content))) continue;
            const prompt = act.startsWith("prompt://");
            const label =
              name === "opened" && prompt
                ? w.acts.find((one) => one.id === act)?.by === "operator"
                  ? "YOU"
                  : "OBSERVATION"
                : name === "closed" && prompt
                  ? "✦  RESULT"
                  : `${name.toUpperCase()}  ${act.split("://")[0] || ""}`;
            const color =
              name === "raised" || name === "refused"
                ? c.red
                : name === "closed" && prompt
                  ? c.accent
                  : name === "opened" && prompt
                    ? c.yellow
                    : c.muted;
            items++;
            add(id, JSON.stringify(content), label, color, (box) => {
              if (prompt && name === "opened") {
                box.add(this.markdown(String(attributes.message ?? "")));
                box.add(this.reference("Open prompt REPL", act));
                return;
              }
              if (act)
                box.add(
                  this.reference(`${act}${["raised", "refused"].includes(name) ? " · open word" : ""}`, act),
                );
              for (const [key, value] of attrs.filter(([key]) => !["id", "over"].includes(key))) {
                box.add(
                  typeof value === "string" && (key === "path" || value.includes("://"))
                    ? this.reference(`${key}: ${value}`, value)
                    : this.text(`${key}: ${display(value)}`, c.faint),
                );
              }
              if (body !== null && body !== "") {
                if (prompt && name === "closed")
                  box.add(this.markdown(display(w.acts.find((one) => one.id === act)?.value ?? body)));
                else if (name === "opened" && act.startsWith("rung://") && typeof body === "string")
                  box.add(this.numbered(body));
                else this.renderBody(box, body);
              }
            });
          }
        }
      }
      for (const [id, stream] of w.world.streams) {
        if (stream.chain !== w.selected) continue;
        items++;
        add(
          `stream-${id}`,
          stream.text + stream.thinking + this.progress(id),
          w.paused ? "◌  HELD RESPONSE" : `${this.progress(id)}  WORKING`,
          c.teal,
          (box) => {
            if (stream.thinking) box.add(this.text(stream.thinking, c.muted));
            if (stream.text) box.add(this.code(stream.text));
            else box.add(this.text("The model is thinking. You can keep exploring this life.", c.muted));
          },
        );
      }
      if (!items)
        add("welcome", "welcome", "", c.panel, (box) => {
          box.add(
            this.text(
              "       ▄▄▄▄  ▄   ▄  ▄▄▄   ▄▄▄▄\n       █▄▄   █   █  █  █  █▄▄█\n       █     ▀▄▄▄▀  █ ▀▄  █▄▄█",
              c.accent,
              { marginTop: 2, marginBottom: 2 },
            ),
          );
          box.add(this.text("A little space for ambitious work.", c.text, { marginBottom: 1 }));
          box.add(
            this.text(
              "Think in conversations. Work in chains.\nEvery step is yours to inspect, pause, and replay.",
              c.muted,
              { marginBottom: 2 },
            ),
          );
          for (const [label, prompt] of [
            ["Explore a codebase", "Read the README and explain how this project works."],
            ["Make something better", "Find one useful improvement in this project and implement it."],
            ["Start with a plan", "Read the project and propose a small, testable plan."],
          ]) {
            box.add(
              this.text(`↗  ${label}`, c.teal, {
                marginBottom: 1,
                onMouseDown: () => this.insert(prompt ?? ""),
              }),
            );
          }
          box.add(this.text("Ctrl+P opens every action. F1 shows the keys.", c.faint, { marginTop: 1 }));
        });
    } else if (w.view === "program") {
      const ladder = w.acts.find((act) => act.id === w.ladder);
      if (ladder)
        add(
          "prompt-repl",
          JSON.stringify(ladder) + w.paused,
          `${ladder.done ? "✓" : w.paused ? "◌" : "◐"} PROMPT REPL · ${short(ladder.id)}`,
          c.yellow,
          (box) => {
            box.add(this.markdown(String(ladder.words[1] ?? "")));
            box.add(
              this.text(
                `Result type: ${String(ladder.words[0])} · ${ladder.done ? "closed" : w.paused ? "paused" : "pending"}`,
                c.faint,
              ),
            );
            box.add(
              this.text("Edit this prompt's program", c.blue, {
                onMouseDown: () => this.action(`/edit ${ladder.id}`),
              }),
            );
            box.add(
              this.text("Show all programs", c.muted, {
                onMouseDown: () => {
                  w.ladder = undefined;
                  this.render();
                },
              }),
            );
          },
        );
      if (w.findings.length && (!w.ladder || w.repls[w.ladder]?.includes(w.rejectedAct)))
        add("findings", w.rejectedWord + w.findings.join("\n"), "GATE FINDINGS", c.red, (box) => {
          box.add(this.numbered(w.rejectedWord, w.findings));
          for (const finding of w.findings) box.add(this.text(`! ${finding}`, c.red));
        });
      const words = Object.entries(w.program).filter(
        ([id]) => !w.ladder || short(id).startsWith(`${short(w.ladder)}.`) || w.repls[w.ladder]?.includes(id),
      );
      for (const [id, word] of words) {
        if (matches(word))
          add(id, word, `λ  ${id}`, c.teal, (box) => {
            box.add(
              new LineNumberRenderable(this.renderer, {
                target: this.code(word),
                fg: c.faint,
                minWidth: 3,
                paddingRight: 1,
              }),
            );
          });
      }
      if (!words.length)
        add("empty", "program", "NO PROGRAM YET", c.faint, (box) =>
          box.add(this.text("Accepted Python words will appear here. Use /run to write a rung.")),
        );
    } else if (w.view === "activity") {
      for (const act of w.activity) {
        if (!matches(JSON.stringify(act))) continue;
        add(
          act.id,
          JSON.stringify(act) + (act.done ? "" : this.progress(act.id)),
          `${act.done ? "✓" : this.progress(act.id)}  ${act.kind.toUpperCase()}  ·  ${short(act.id)}`,
          act.done ? c.accent : c.yellow,
          (box) => {
            box.add(this.text(act.words.map(display).join("\n"), c.muted));
            if (act.kind === "bash" && act.value && typeof act.value === "object") {
              const exit = act.value as {
                stdout?: { content: string };
                stderr?: { content: string };
                code?: number;
              };
              if (exit.stdout?.content) {
                box.add(this.text("stdout", c.faint));
                box.add(this.text(exit.stdout.content));
              }
              if (exit.stderr?.content) {
                box.add(this.text("stderr", c.red));
                box.add(this.text(exit.stderr.content, c.red));
              }
              if (act.done) box.add(this.text(`exit ${exit.code ?? "timeout"}`, c.muted));
            } else if (act.done) box.add(this.text(display(act.value), c.text, { marginTop: 1 }));
            if (!act.done)
              box.add(
                this.text("Pause   Resume   Cancel", c.teal, {
                  marginTop: 1,
                  onMouseDown: () => this.actActions(act),
                }),
              );
          },
        );
      }
      if (!w.activity.length)
        add("empty", "activity", "NO ACTS YET", c.faint, (box) =>
          box.add(this.text("Prompts, commands, waits, and grants appear here as they run.")),
        );
    } else if (w.view === "transcript") {
      w.turns.forEach((turn, index) => {
        const text = w.rendered[index] ?? "";
        if (matches(text))
          add(
            `transcript-${index}`,
            text,
            `${turn[0].toUpperCase()}  ·  turn ${index + 1}`,
            turn[0] === "assistant" ? c.teal : c.yellow,
            (box) => box.add(turn[0] === "assistant" ? this.code(text) : this.transcriptText(text)),
          );
      });
    } else if (w.view === "changes") {
      if (w.world.changes.length > 20)
        add(
          "change-pages",
          String(w.changePage),
          `WRITES ${w.changePage * 20 + 1} TO ${Math.min((w.changePage + 1) * 20, w.world.changes.length)} OF ${w.world.changes.length}`,
          c.faint,
          (box) => {
            for (const [label, step] of [
              ["Previous page · Ctrl+PageUp", -1],
              ["Next page · Ctrl+PageDown", 1],
            ] as const)
              box.add(this.text(label, c.blue, { onMouseDown: () => this.changePage(step) }));
          },
        );
      for (const [index, change] of w.changes.entries()) {
        if (!matches(change.path)) continue;
        const diff = createTwoFilesPatch(change.path, change.path, change.before, change.after);
        add(`change-${index}`, diff, `±  ${change.path}`, c.teal, (box) =>
          box.add(
            new DiffRenderable(this.renderer, {
              diff,
              view: this.renderer.width > 145 ? "split" : "unified",
              syntaxStyle: this.style,
              fg: c.text,
              showLineNumbers: true,
              lineNumberFg: c.faint,
              lineNumberBg: c.panel,
              contextBg: c.panel,
              addedBg: c.selected,
              removedBg: c.removed,
              addedSignColor: c.accent,
              removedSignColor: c.red,
              wrapMode: "word",
            }),
          ),
        );
      }
      if (!w.world.changes.length)
        add("empty", "changes", "NO FILE CHANGES", c.faint, (box) =>
          box.add(this.text("Writes through the World appear here with their before and after lines.")),
        );
    } else {
      const facts = w.filteredFacts();
      for (const [index, fact] of facts.slice(-300).entries()) {
        const id = `fact-${index}`;
        add(
          id,
          JSON.stringify(fact),
          `${String(index + Math.max(0, facts.length - 300) + 1).padStart(4, "0")}  ${fact[0]}  ·  ${fact[1]}`,
          c.teal,
          (box) => {
            box.add(this.text(`by ${fact[2]}`, c.faint));
            box.add(this.text(JSON.stringify(fact.slice(3), null, 2), c.muted));
          },
        );
      }
    }
    for (const id of existing) {
      this.cards.get(id)?.node.destroyRecursively();
      this.cards.delete(id);
    }
  }

  private renderBody(box: BoxRenderable, body: unknown): void {
    if (!Array.isArray(body)) {
      box.add(this.text(display(body)));
      return;
    }
    for (const one of body) {
      if (isTag(one)) {
        box.add(this.text(one[0].toUpperCase(), c.teal, { marginTop: 1 }));
        for (const [key, value] of one[1])
          box.add(
            typeof value === "string" && (key === "path" || value.includes("://"))
              ? this.reference(`${key}: ${value}`, value)
              : this.text(`${key}: ${display(value)}`, c.faint),
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
      fg: c.faint,
      minWidth: 3,
      paddingRight: 1,
    });
    for (const finding of findings) {
      const line = Number(finding.match(/line (\d+)/)?.[1] ?? 0) - 1;
      if (line >= 0) {
        lines.setLineColor(line, { gutter: c.removed, content: c.removed });
        lines.setLineSign(line, { before: "!", beforeColor: c.red });
      }
    }
    return lines;
  }

  private reference(label: string, value: string): TextRenderable {
    const node = this.text(label, c.blue, { attributes: 8 });
    node.onMouseDown = () => {
      void this.follow(value).catch(this.workspace.fail);
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
    if (value.startsWith("chain://")) await this.workspace.select(value);
    else if (value.startsWith("prompt://")) this.openLadder(value);
    else if (value.startsWith("rung://")) this.go("program", value);
    else if (value.includes("://") && !value.includes("/stdin")) this.go("activity", value);
    else
      this.showValue(
        value,
        (
          (await this.workspace.life.read(
            value,
            { is: "name", name: "HIDDEN" },
            this.workspace.selected,
          )) as { content: string }
        ).content,
      );
  }

  private async referenceHover(value: string, x: number, y: number): Promise<void> {
    try {
      const act = this.workspace.acts.find((act) => act.id === value);
      const detail = act
        ? `${act.kind} · ${act.done ? display(act.value) : "pending"}`
        : (
            (await this.workspace.life.read(
              value,
              { is: "name", name: "HIDDEN" },
              this.workspace.selected,
            )) as { content: string }
          ).content;
      if (this.closed || this.overlay) return;
      this.hover?.destroyRecursively();
      this.hover = this.box({
        position: "absolute",
        left: Math.max(1, Math.min(x, this.renderer.width - 60)),
        top: Math.max(1, Math.min(y + 1, this.renderer.height - 8)),
        width: Math.min(58, this.renderer.width - 4),
        maxHeight: 7,
        padding: 1,
        border: true,
        borderColor: c.blue,
        backgroundColor: c.raised,
        zIndex: 30,
        onMouseDown: () => {
          void this.follow(value).catch(this.workspace.fail);
        },
      });
      this.hover.add(this.text(value, c.blue));
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
        fg: match[0].startsWith("<") ? c.teal : match[0].startsWith('"') ? c.accent : c.yellow,
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
      const row = y - node.y;
      const line = text.split("\n")[node.getLineSources(row, 1)[0] ?? row] ?? "";
      const column = x - node.x + (node.lineInfo.lineStartCols[row] ?? 0);
      const match = [...line.matchAll(/[a-z]+:\/\/[\w./-]+|path="([^"\n]+)"/g)].find(
        (match) =>
          column >= Bun.stringWidth(line.slice(0, match.index)) &&
          column < Bun.stringWidth(line.slice(0, match.index + match[0].length)),
      );
      return match?.[1] ?? match?.[0];
    };
    node.onMouseMove = (event) => {
      clearTimeout(this.hoverTimer);
      const value = target(event.x, event.y);
      this.hover?.destroyRecursively();
      this.hover = undefined;
      if (value)
        this.hoverTimer = setTimeout(() => {
          void this.referenceHover(value, event.x, event.y);
        }, 220);
    };
    node.onMouseOut = () => {
      clearTimeout(this.hoverTimer);
      this.hover?.destroyRecursively();
      this.hover = undefined;
    };
    node.onMouseDown = (event) => {
      const value = target(event.x, event.y);
      if (value) void this.follow(value).catch(this.workspace.fail);
    };
    return node;
  }

  private renderInspector(): void {
    const w = this.workspace;
    if (
      !this.paneChanged(this.inspector, [
        this.theme,
        w.selected,
        w.label,
        w.directory,
        w.paused,
        w.world.streams.size,
        w.usage,
        w.actor,
        this.renderer.height,
        w.activity.map((act) => [act.id, act.done]),
      ])
    )
      return;
    this.clear(this.inspector);
    this.inspector.add(this.text("THIS CHAIN", c.faint));
    this.inspector.add(this.text(w.label, c.accent));
    this.inspector.add(this.text(short(w.selected), c.faint));
    this.inspector.add(
      this.text(w.directory || w.world.directory, c.faint, { maxHeight: 2, truncate: true }),
    );
    this.inspector.add(
      this.text(
        w.paused ? "◌ Paused" : w.world.streams.size ? "● Working" : "✓ Ready",
        w.paused ? c.yellow : c.teal,
      ),
    );
    this.inspector.add(this.text("USAGE", c.faint, { marginTop: 1 }));
    const usage = w.usage;
    this.inspector.add(this.text(`$${usage[4].toFixed(4)}  total`, c.text));
    this.inspector.add(
      this.text(`${count(usage[0])} input    ${count(usage[1])} output\n${count(usage[2])} cached`, c.muted),
    );
    const last = [...w.turns].reverse().find((turn) => turn[2])?.[2];
    const share = (last?.[0] ?? 0) / w.world.route(w.actor).contextWindow;
    const filled = Math.min(20, Math.round(share * 20));
    this.inspector.add(
      this.text(
        `${"━".repeat(filled)}${"─".repeat(20 - filled)}\n${(share * 100).toFixed(1)}% of context`,
        c.accent,
      ),
    );
    const grant = w.activity.find((act) => act.kind === "grant" && !act.done);
    this.inspector.add(
      this.text(
        grant
          ? `Budget  ${grant.words[0] === null ? "context share" : `$${grant.words[0]}`}`
          : "No budget set",
        c.muted,
        { onMouseDown: () => this.insert("/grant ") },
      ),
    );
    this.inspector.add(this.text("RECENT ACTS", c.faint, { marginTop: 1 }));
    for (const act of w.activity.slice(this.renderer.height < 40 ? -2 : -4)) {
      const row = this.box({ onMouseDown: () => this.actActions(act) });
      row.add(this.text(`${act.done ? "✓" : "◌"} ${act.kind}`, act.done ? c.muted : c.yellow));
      row.add(
        this.text(String(w.program[act.id] || act.words[0] || short(act.id)).split("\n")[0] ?? "", c.faint, {
          height: 1,
          truncate: true,
        }),
      );
      this.inspector.add(row);
    }
    this.inspector.add(this.box({ flexGrow: 1 }));
    this.inspector.add(
      this.text(w.paused ? "▶  Resume chain" : "Ⅱ  Pause chain", c.teal, {
        onMouseDown: () => this.action(w.paused ? "/wake" : "/pause"),
      }),
    );
    this.inspector.add(this.text("×  Cancel work", c.muted, { onMouseDown: () => this.action("/cancel") }));
  }

  private insert(text: string): void {
    this.closeOverlay();
    this.composer.setText(text);
    this.composer.focus();
  }
  private action(command: string): void {
    this.closeOverlay();
    void this.workspace.submit(command).catch(this.workspace.fail);
  }
  private actActions(act: ActRow): void {
    this.openPalette(`Act · ${act.kind}`, [
      {
        label: "Inspect activity",
        detail: act.id,
        run: () => {
          this.workspace.show("activity");
        },
      },
      { label: "Pause", detail: "Hold delivery to this act", run: () => this.action(`/pause ${act.id}`) },
      { label: "Resume", detail: "Deliver held work", run: () => this.action(`/wake ${act.id}`) },
      { label: "Cancel", detail: "End this act and its work", run: () => this.action(`/cancel ${act.id}`) },
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
      run: () => this.workspace.show(view),
    }));
    if (this.options.newSession)
      choices.unshift({ label: "New session", detail: "Start a fresh life", run: this.options.newSession });
    choices.push(
      ...Object.entries(commands)
        .filter(([name]) => name !== "new")
        .map(([name, [label, argument, detail]]) => ({
          label,
          detail,
          run: () =>
            name === "rewind"
              ? this.rewind()
              : argument && !argument.startsWith("[")
                ? this.insert(`/${name} `)
                : this.action(`/${name}`),
        })),
    );
    choices.push({ label: "Choose model", detail: "Model and reasoning effort", run: () => this.models() });
    choices.push({ label: "Choose theme", detail: "Forest, paper, or midnight", run: () => this.themes() });
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
  models(): void {
    this.openPalette(
      "Model & effort",
      this.workspace.world.roster.flatMap((model) =>
        efforts.map((effort) => ({
          label: `${model}/${effort}`,
          detail: `${count(this.workspace.world.route(model).contextWindow)} context  ·  ${model.startsWith("claude-cli:") ? "Claude subscription" : "pi-ai"}`,
          run: () => this.action(`/model ${model}/${effort}`),
        })),
      ),
    );
  }
  themes(): void {
    this.openPalette(
      "Color theme",
      Object.keys(palettes).map((name) => ({
        label: title(name),
        detail: name === "paper" ? "A quiet light theme" : "A quiet dark theme",
        run: () => {
          this.workspace.theme = name as ThemeName;
          this.render();
          this.workspace.save();
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
          this.workspace.shape = name;
          this.render();
        },
      })),
    );
  }
  toggleMode(): void {
    this.workspace.mode = this.workspace.mode === "python" ? "prompt" : "python";
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
    this.composer.placeholderColor = c.faint;
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
    this.workspace.drafts[this.draftKey] = content;
    const version = ++this.editorVersion;
    for (let line = 0; line < this.composer.lineCount; line++) this.composer.clearLineHighlights(line);
    if (this.workspace.mode !== "python" && !this.workspace.editing && !content.startsWith("/run ")) return;
    const result = await getTreeSitterClient()
      .highlightOnce(content, "python")
      .catch((error) => {
        if (!this.closed) this.workspace.fail(error);
        return undefined;
      });
    if (version !== this.editorVersion || this.closed) return;
    for (const [start, end, group] of result?.highlights ?? []) {
      const styleId =
        this.style.resolveStyleId(group) ?? this.style.resolveStyleId(group.split(".")[0] ?? "default");
      if (styleId !== null) this.composer.addHighlightByCharRange({ start, end, styleId });
    }
    if (content === this.workspace.rejectedWord || content === `/run ${this.workspace.rejectedWord}`) {
      const styleId = this.style.resolveStyleId("diagnostic");
      const lines = content.split("\n");
      for (const finding of this.workspace.findings) {
        const line = Number(finding.match(/line (\d+)/)?.[1] ?? 0) - 1;
        if (styleId !== null && line >= 0) {
          const start = lines.slice(0, line).reduce((size, line) => size + line.length + 1, 0);
          this.composer.addHighlightByCharRange({ start, end: start + (lines[line]?.length ?? 0), styleId });
        }
      }
    }
    const cursor = this.composer.cursorOffset;
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
    const elapsed = Math.max(0, Math.floor((Date.now() - (this.workspace.started[id] ?? Date.now())) / 1000));
    return `${["◐", "◓", "◑", "◒"][Math.floor(Date.now() / 250) % 4]} ${elapsed}s`;
  }
  private resume = (): void => {
    const held = this.workspace.world.held;
    this.openPalette("Saved work is paused", [
      {
        label: "Resume saved work",
        detail: `${held.size} unfinished acts. Interrupted commands keep their output and report an interruption.`,
        run: async () => {
          await this.workspace.world.resume();
          await this.workspace.refresh();
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
    const question = this.workspace.operatorPrompt;
    if (!question) return;
    if (question.shape === "bool") {
      this.openPalette("Operator question · bool", [
        {
          label: "Yes",
          detail: "Answer true",
          run: () => this.workspace.world.answer(question.id, "yes").then(() => {}),
        },
        {
          label: "No",
          detail: "Answer false",
          run: () => this.workspace.world.answer(question.id, "no").then(() => {}),
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
      void this.workspace.world
        .answer(question.id, input.value)
        .then(() => this.closeOverlay())
        .catch((failure) => {
          error.fg = c.red;
          error.content = String(failure);
        });
    });
  }
  private showQuestionText(message: string): void {
    if (!this.overlay || !this.paletteInput) return;
    this.overlay.top = 1;
    this.overlay.maxHeight = this.renderer.height - 2;
    this.questionDocument = new ScrollBoxRenderable(this.renderer, {
      height: Math.max(3, Math.min(8, this.renderer.height - 18)),
      scrollX: false,
      scrollY: true,
    });
    this.questionDocument.add(this.markdown(message));
    this.overlay.insertBefore(this.questionDocument, this.paletteInput);
  }
  private async showHover(name: string, x: number, y: number): Promise<void> {
    try {
      const inspected = await this.workspace.life.inspect(name, this.workspace.selected);
      if (this.closed || this.overlay) return;
      this.hover?.destroyRecursively();
      const width = Math.min(58, this.renderer.width - 4);
      this.hover = this.box({
        position: "absolute",
        left: Math.max(1, Math.min(x, this.renderer.width - width - 1)),
        top: Math.max(1, Math.min(y + 1, this.renderer.height - 9)),
        width,
        maxHeight: 8,
        padding: 1,
        border: true,
        borderStyle: "rounded",
        borderColor: c.teal,
        backgroundColor: c.raised,
        zIndex: 30,
        onMouseDown: () => this.inspect(name),
      });
      this.hover.add(this.text(`${name}  ·  ${inspected.kind}`, c.teal));
      this.hover.add(this.text(inspected.representation.slice(0, 280), c.text, { maxHeight: 4 }));
      this.hover.add(this.text("Click to expand  ·  Ctrl+G keyboard inspector", c.faint));
      this.root.add(this.hover);
    } catch {
      this.hover?.destroyRecursively();
      this.hover = undefined;
    }
  }
  private inspect = (name: string): void => {
    this.hover?.destroyRecursively();
    this.hover = undefined;
    void this.workspace.life
      .inspect(name, this.workspace.selected)
      .then((value) => {
        this.showValue(`${name} · ${value.kind}`, value.value ?? value.representation, name);
      })
      .catch(this.workspace.fail);
  };
  private showValue(label: string, value: unknown, name?: string, back?: () => void): void {
    const choices: Choice[] = [];
    if (back) choices.push({ label: "← Back", detail: "Return to the parent value", run: back });
    if (name && /^[\p{L}_][\p{L}\p{N}_]*$/u.test(name)) {
      const definition = [...Object.entries(this.workspace.program)]
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
            const lines = (await this.workspace.world.source()).split("\n");
            const at = lines.findIndex((line) =>
              new RegExp(`^(?:(?:async )?def |class )?${name}\\b`).test(line),
            );
            if (at < 0) {
              this.workspace.notice = "This name has no definition in the engine source.";
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
                fg: c.faint,
                lineNumberOffset: at,
                paddingRight: 1,
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
          this.workspace.notice = "Value copied.";
        },
      });
    if (typeof value === "string" && value.includes("://"))
      choices.push({
        label: "Follow this act or door",
        detail: value,
        run: () => {
          if (value.startsWith("chain://")) return this.workspace.select(value);
          void this.workspace.life
            .read(value, undefined, this.workspace.selected)
            .then((text) => this.showValue(value, text))
            .catch(this.workspace.fail);
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
    const names = (await this.workspace.life.held("modules", [this.workspace.selected], "keys")) as string[];
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
    const names = (await this.workspace.life.held("modules", [this.workspace.selected], "keys")) as string[];
    const before = this.composer.plainText.slice(0, this.composer.cursorOffset);
    const prefix = before.match(/[\p{L}_][\p{L}\p{N}_]*$/u)?.[0] ?? "";
    this.openPalette(
      "Complete Python name",
      names
        .filter((name) => name.startsWith(prefix) && !name.startsWith("_"))
        .map((name) => ({
          label: name,
          detail: "Insert this name at the cursor",
          run: () => {
            for (const _ of prefix) this.composer.deleteCharBackward();
            this.composer.insertText(name);
            this.composer.focus();
          },
        })),
    );
  }
  ladders(): void {
    this.openPalette(
      "Prompt programs",
      this.workspace.activity
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
    this.workspace.ladder = id;
    this.workspace.editing = undefined;
    this.workspace.mode = "python";
    this.render();
    this.workspace.notice = `${id}: run a rung below, or /edit ${id} to change its program.`;
  }
  rewind = (): void => {
    const acts = this.workspace.activity.filter((act) => !["chain", "grant"].includes(act.kind));
    this.openPalette(
      "Rewind transcript · module and files stay current",
      acts.map((act, index) => ({
        label: `${"  ".repeat(Math.max(0, short(act.id).split(".").length - 2))}${act.kind} · ${short(act.id)}`,
        detail:
          String(
            act.kind === "prompt" ? act.words[1] : act.words[0] || this.workspace.program[act.id] || "",
          ).split("\n")[0] ?? "",
        run: async () => {
          const filter = await this.workspace.life.take(
            acts.slice(index + 1).map((later) => later.id),
            false,
          );
          const next = await this.workspace.life.chain(
            `${this.workspace.label} through ${short(act.id)}`,
            this.workspace.selected,
            filter,
          );
          await this.workspace.life.forget(filter.id);
          await this.workspace.select(next);
          this.workspace.notice =
            "The new chain reads the selected transcript prefix. Its module and files keep current state.";
        },
      })),
    );
  };
  private go(view: View, id?: string): void {
    this.navigation.push({
      chain: this.workspace.selected,
      view: this.workspace.view,
      query: this.workspace.query,
      top: this.scroll.scrollTop,
      ladder: this.workspace.ladder,
      mode: this.workspace.mode,
    });
    if (
      id &&
      this.workspace.ladder &&
      !short(id).startsWith(`${short(this.workspace.ladder)}.`) &&
      !this.workspace.repls[this.workspace.ladder]?.includes(id)
    )
      this.workspace.ladder = undefined;
    this.workspace.show(view);
    this.render();
    if (id) this.scroll.scrollChildIntoView(id);
  }
  private async back(): Promise<void> {
    const previous = this.navigation.pop();
    if (!previous) return;
    await this.workspace.select(previous.chain);
    this.workspace.show(previous.view);
    this.workspace.query = previous.query;
    this.workspace.ladder = previous.ladder;
    this.workspace.mode = previous.mode;
    this.render();
    this.scroll.scrollTo(previous.top);
  }
  chains(): void {
    this.openPalette(
      "Chains",
      this.workspace.chains.map((chain) => ({
        label: this.workspace.labelOf(chain.id),
        detail: chain.id,
        run: () => this.workspace.select(chain.id),
      })),
    );
  }
  openPalette(label: string, choices: Choice[]): void {
    this.closeOverlay();
    this.composer.blur();
    const width = Math.min(76, this.renderer.width - 4);
    this.overlay = this.box({
      id: "palette",
      position: "absolute",
      left: Math.floor((this.renderer.width - width) / 2),
      top: 4,
      width,
      maxHeight: Math.max(12, this.renderer.height - 8),
      padding: 1,
      gap: 1,
      border: true,
      borderStyle: "rounded",
      borderColor: c.accent,
      backgroundColor: c.raised,
      zIndex: 20,
    });
    this.root.add(this.overlay);
    this.overlay.add(this.text(`${label.toUpperCase()}   /   Esc to close`, c.accent));
    this.paletteInput = new InputRenderable(this.renderer, {
      id: "palette-search",
      placeholder: "Type to filter...",
      backgroundColor: c.panel,
      textColor: c.text,
      placeholderColor: c.faint,
    });
    this.overlay.add(this.paletteInput);
    this.paletteList = this.box({ gap: 1 });
    this.overlay.add(this.paletteList);
    this.filtered = choices;
    this.selection = 0;
    this.paletteInput.on(InputRenderableEvents.INPUT, (value: string) => {
      this.filtered = choices.filter((choice) =>
        `${choice.label} ${choice.detail}`.toLowerCase().includes(value.toLowerCase()),
      );
      this.selection = 0;
      this.renderChoices();
    });
    this.paletteInput.on(InputRenderableEvents.ENTER, () => this.choose());
    this.renderChoices();
    this.paletteInput.focus();
  }
  private renderChoices(): void {
    if (!this.paletteList) return;
    this.clear(this.paletteList);
    const max = Math.max(2, Math.floor((this.renderer.height - 17) / 3));
    const start = Math.max(0, this.selection - max + 1);
    for (const [index, choice] of this.filtered.slice(start, start + max).entries()) {
      const selected = index + start === this.selection;
      const row = this.box({
        paddingX: 1,
        backgroundColor: selected ? c.selected : c.raised,
        onMouseDown: () => {
          this.selection = index + start;
          this.choose();
        },
      });
      row.add(this.text(`${selected ? "▸" : " "} ${choice.label}`, selected ? c.accent : c.text));
      row.add(this.text(`  ${choice.detail}`, c.muted));
      this.paletteList.add(row);
    }
    if (!this.filtered.length) this.paletteList.add(this.text("No matching actions.", c.muted));
  }
  private choose(): void {
    const choice = this.filtered[this.selection];
    if (!choice) return;
    this.closeOverlay();
    Promise.resolve(choice.run()).catch(this.workspace.fail);
  }
  closeOverlay(): void {
    this.overlay?.destroyRecursively();
    this.overlay = undefined;
    this.paletteInput = undefined;
    this.paletteList = undefined;
    this.questionDocument = undefined;
    this.composer?.focus();
  }
  help(): void {
    this.openPalette(
      "Help & keyboard",
      [
        ["Enter / Shift+Enter", "Send a message / insert a new line"],
        ["Ctrl+1 through Ctrl+6", "Conversation / program / activity / facts / transcript / changes"],
        ["Ctrl+P", "Search all actions"],
        ["Ctrl+B", "Switch chains"],
        ["Ctrl+N / Ctrl+M / Ctrl+O", "New chain / choose model / saved sessions"],
        ["Ctrl+F / PageUp / PageDown", "Filter the current view / scroll"],
        ["Ctrl+R / Ctrl+Space / Tab", "Python input / complete a name / complete a slash command"],
        ["Ctrl+G / Ctrl+click a name", "Inspect a value and follow its definition"],
        ["Ctrl+L / Ctrl+A", "Prompt programs / answer an operator question"],
        ["Ctrl+T / Ctrl+Y", "Themes / copy the selected text"],
        ["Ctrl+C / Ctrl+Q", "Clear or cancel / save and quit"],
        ["Ctrl+Alt+Left", "Return from a definition jump"],
        ["Alt+[ / Alt+]", "Previous / next prompt REPL"],
        ["Alt+Up / Alt+Down", "Browse submitted input history"],
        ["Ctrl+PageUp / Ctrl+PageDown", "Previous / next page of file changes"],
      ]
        .map(([label, detail]) => ({ label: label ?? "", detail: detail ?? "", run: () => {} }))
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
    if (key.ctrl && ["pageup", "pagedown"].includes(key.name) && this.workspace.view === "changes") {
      key.preventDefault();
      this.changePage(key.name === "pageup" ? -1 : 1);
      return;
    }
    if (key.meta && key.ctrl && key.name === "left" && !this.overlay) {
      key.preventDefault();
      void this.back().catch(this.workspace.fail);
      return;
    }
    if (!this.overlay && key.meta && ["up", "down"].includes(key.name)) {
      key.preventDefault();
      const history = this.workspace.histories[this.draftKey] ?? [];
      if (this.historyIndex < 0) {
        this.historyDraft = this.composer.plainText;
        this.historyIndex = history.length;
      }
      this.historyIndex = Math.max(
        0,
        Math.min(history.length, this.historyIndex + (key.name === "up" ? -1 : 1)),
      );
      this.composer.setText(history[this.historyIndex] ?? this.historyDraft);
      return;
    }
    if (
      !this.overlay &&
      (this.workspace.mode === "python" || this.workspace.editing) &&
      ((key.shift && ["return", "enter"].includes(key.name)) || (key.ctrl && key.name === "j"))
    ) {
      key.preventDefault();
      const before = this.composer.plainText.slice(0, this.composer.cursorOffset).split("\n").at(-1) ?? "";
      this.composer.insertText(
        `\n${before.match(/^\s*/)?.[0] ?? ""}${before.trimEnd().endsWith(":") ? "  " : ""}`,
      );
      return;
    }
    if (key.meta && ["[", "]"].includes(key.name) && !this.overlay) {
      key.preventDefault();
      const prompts = this.workspace.activity.filter((act) => act.kind === "prompt");
      const current = prompts.findIndex((act) => act.id === this.workspace.ladder);
      const next = prompts[(current + (key.name === "]" ? 1 : -1) + prompts.length) % prompts.length];
      if (next) this.openLadder(next.id);
      return;
    }
    if (key.ctrl && key.name === "c") {
      key.preventDefault();
      if (this.overlay) this.closeOverlay();
      else if (this.composer.plainText) this.composer.setText("");
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
        !this.workspace.editing &&
        !this.workspace.paused &&
        this.workspace.activity.some((act) => act.kind === "prompt" && !act.done)
      )
        this.action("/pause");
      this.closeOverlay();
      this.search.visible = false;
      this.workspace.query = "";
      this.workspace.editing = undefined;
      this.render();
      return;
    }
    if (this.overlay) {
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
    if (key.ctrl && /^[1-6]$/.test(key.name)) {
      key.preventDefault();
      this.workspace.show(views[Number(key.name) - 1] ?? "conversation");
    } else if (key.name === "f1") {
      key.preventDefault();
      this.help();
    } else if (key.ctrl && key.name === "p") {
      key.preventDefault();
      this.palette();
    } else if (key.ctrl && key.name === "b") {
      key.preventDefault();
      this.chains();
    } else if (key.ctrl && key.name === "m") {
      key.preventDefault();
      this.models();
    } else if (key.ctrl && key.name === "t") {
      key.preventDefault();
      this.themes();
    } else if (key.ctrl && key.name === "a" && this.workspace.operatorPrompt) {
      key.preventDefault();
      this.question();
    } else if (key.ctrl && key.name === "g") {
      key.preventDefault();
      void this.names().catch(this.workspace.fail);
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
      void this.completeNames().catch(this.workspace.fail);
    } else if (key.name === "tab" && /^\/\w*$/.test(this.composer.plainText)) {
      key.preventDefault();
      const prefix = this.composer.plainText.slice(1);
      const command = Object.keys(commands).find((name) => name.startsWith(prefix));
      if (command) this.composer.setText(`/${command} `);
    } else if (key.ctrl && key.name === "n") {
      key.preventDefault();
      this.insert("/chain ");
    } else if (key.ctrl && key.name === "o") {
      key.preventDefault();
      void this.options
        .sessions?.()
        .then((choices) => this.openPalette("Sessions", choices))
        .catch(this.workspace.fail);
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
    this.workspace.drafts[this.draftKey] = this.composer.plainText;
    this.workspace.scrolls[this.lastView] = this.scroll.scrollTop;
    this.workspace.collapsed = [...this.collapsed];
    this.workspace.save();
    if (this.redraw) clearTimeout(this.redraw);
    if (this.hoverTimer) clearTimeout(this.hoverTimer);
    this.workspace.off("change", this.schedule);
    this.workspace.off("compose", this.compose);
    this.workspace.off("inspect", this.inspect);
    this.workspace.off("resume", this.resume);
    this.workspace.off("rewind", this.rewind);
    this.renderer.keyInput.off("keypress", this.key);
    this.renderer.off("resize", this.render);
    this.root.destroyRecursively();
    this.style.destroy();
  }
  private changePage(step: number): void {
    this.workspace.changePage = Math.max(
      0,
      Math.min(Math.ceil(this.workspace.world.changes.length / 20) - 1, this.workspace.changePage + step),
    );
    void this.workspace.refresh().catch(this.workspace.fail);
  }
}

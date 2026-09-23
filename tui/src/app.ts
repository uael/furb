import { basename } from "node:path";
import { actorParts, shapes } from "@furb/engine";
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
import {
  theme as c,
  defaultTheme,
  palettes,
  setTheme,
  spacing as space,
  syntax,
  type ThemeName,
} from "./theme.ts";
import type { ActRow, View, Workspace } from "./workspace.ts";

const views: View[] = ["conversation", "program", "activity", "facts", "transcript", "changes"];
const title = (text: string) => text.charAt(0).toUpperCase() + text.slice(1);
const short = (id: string) => id.replace(/^[^:]+:\/\//, "");
const count = (value: number) =>
  value >= 1_000_000
    ? `${Number((value / 1_000_000).toFixed(1))}M`
    : value >= 1000
      ? `${Number((value / 1000).toFixed(1))}k`
      : String(value);
interface BlockOptions {
  compact?: boolean;
  heading?: boolean;
  group?: string;
  act?: ActRow;
}
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
  private theme: ThemeName = defaultTheme;
  private draftKey = "";
  private hover?: BoxRenderable;
  private questionDocument?: ScrollBoxRenderable;
  private hoverTimer?: ReturnType<typeof setTimeout>;
  private editorVersion = 0;
  private closed = false;
  private readonly collapsed = new Set<string>();
  private readonly expanded = new Set<string>();
  private readonly inspector: BoxRenderable;
  private readonly splitters: BoxRenderable[] = [];
  private readonly tabs: BoxRenderable;
  private readonly head: TextRenderable;
  private readonly status: TextRenderable;
  private readonly composeBox: BoxRenderable;
  private readonly promptBox: BoxRenderable;
  private readonly search: InputRenderable;
  private readonly paneKeys = new WeakMap<Renderable, string>();
  private readonly cards = new Map<
    string,
    { node: BoxRenderable; heading: TextRenderable; key: string; compact: boolean }
  >();
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
    for (const id of workspace.expanded) this.expanded.add(id);
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
    const center = this.box({
      flexGrow: 1,
      flexShrink: 1,
      minHeight: 0,
      minWidth: 0,
      paddingX: space.inset,
      gap: space.stack,
    });
    body.add(center);
    this.tabs = this.box({ height: space.bar, flexDirection: "row", gap: space.between });
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
      contentOptions: { gap: space.stack, paddingBottom: space.stack },
      verticalScrollbarOptions: { visible: false },
      horizontalScrollbarOptions: { visible: false },
    });
    center.add(this.scroll);
    this.promptBox = this.box({
      id: "operator-prompt",
      height: space.bar,
      visible: false,
      backgroundColor: c.panel,
      onMouseDown: () => this.question(),
    });
    center.add(this.promptBox);
    this.composeBox = this.box({ id: "composer-box", height: 1, flexShrink: 0, backgroundColor: c.panel });
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
      },
      onCursorChange: () => {
        void this.highlightEditor();
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
        workspace.panes.inspector = Math.max(24, Math.min(44, renderer.width - event.x));
        this.inspector.width = workspace.panes.inspector;
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
      width: workspace.panes.inspector,
      flexShrink: 0,
      scrollX: false,
      scrollY: true,
      backgroundColor: c.panel,
      contentOptions: { paddingX: space.inset, gap: space.stack, minHeight: "100%" },
      verticalScrollbarOptions: { visible: false },
      horizontalScrollbarOptions: { visible: false },
    });
    body.add(this.inspector);
    workspace.on("change", this.schedule);
    workspace.on("compose", this.compose);
    workspace.on("inspect", this.inspect);
    workspace.on("resume", this.resume);
    workspace.on("rewind", this.rewind);
    workspace.on("models", this.models);
    workspace.on("efforts", this.effortPicker);
    workspace.on("details", this.details);
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
    this.inspector.visible = this.renderer.width >= 100;
    if (this.splitters[0]) this.splitters[0].visible = this.inspector.visible;
    this.head.content = `${w.sessionName} / ${w.label}`;
    if (this.paneChanged(this.tabs, [w.view, this.theme, this.renderer.width])) {
      this.clear(this.tabs);
      for (const [index, view] of views.entries()) {
        const label =
          this.renderer.width < 110
            ? ["Chat", "Code", "Acts", "Facts", "Transcript", "Diffs"][index]
            : title(view);
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
    if (this.paneChanged(this.promptBox, [pending, this.theme])) {
      this.clear(this.promptBox);
      if (pending)
        this.promptBox.add(
          this.text(`Reply (${pending.shape}): ${pending.message}`, c.warning, { height: 1, truncate: true }),
        );
    }
    this.composer.placeholder = w.editing
      ? "Edit this prompt's Python program..."
      : pending
        ? `Your ${pending.shape} answer...`
        : w.mode === "python"
          ? "Write Python..."
          : "Ask anything, or type / for a command...";
    this.composeBox.height = Math.min(
      6,
      Math.max(1, this.composer.lineCount, this.composer.lineInfo.lineSources.length),
    );
    const { model, effort } = actorParts(w.actor);
    const state = w.loading
      ? "loading"
      : w.error
        ? "error"
        : w.paused
          ? "paused"
          : pending
            ? "input needed"
            : w.activity.some((act) => !act.done && ["prompt", "rung", "bash", "wait"].includes(act.kind))
              ? "working"
              : "ready";
    this.status.content = `${state} · model ${model} · effort ${effort}${pending ? "" : ` · ${w.mode === "python" || w.editing ? "Python" : `returns ${w.shape}`}`}${w.world.records.path ? ` · ${basename(w.world.records.path)}` : ""}`;
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
    options: BlockOptions = {},
  ): void {
    const closed = options.compact ? !this.expanded.has(id) : this.collapsed.has(id);
    key = options.compact && closed ? "closed" : `${key}:${closed}`;
    const heading = `${options.compact ? (closed ? "▸ " : "▾ ") : ""}${label}`;
    const visible = Boolean(label) && (options.heading !== false || closed);
    const prior = this.cards.get(id);
    if (prior?.key === key) {
      prior.heading.content = heading;
      prior.heading.fg = color;
      prior.heading.visible = visible;
      if (this.scroll.getChildren()[index] !== prior.node) this.scroll.add(prior.node, index);
      return;
    }
    prior?.node.destroyRecursively();
    const box = this.box({ id, gap: space.stack, flexShrink: 0 });
    const labelNode = this.text(heading, color, {
      height: space.bar,
      truncate: true,
      visible,
      onMouseDown: (event) => {
        if (event.button === 2 && options.act) {
          this.actActions(this.workspace.acts.find((act) => act.id === options.act?.id) ?? options.act);
          return;
        }
        const held = options.compact ? this.expanded : this.collapsed;
        if (held.has(id)) held.delete(id);
        else held.add(id);
        this.renderContent();
      },
    });
    box.add(labelNode);
    if (!closed) body(box);
    this.scroll.add(box, index);
    this.cards.set(id, { key, node: box, heading: labelNode, compact: options.compact ?? false });
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
    const w = this.workspace;
    const view = `${w.selected}:${w.view}:${w.ladder ?? ""}`;
    if (this.lastView !== view) {
      if (this.lastView) w.scrolls[this.lastView] = this.scroll.scrollTop;
      this.clear(this.scroll);
      this.cards.clear();
      this.lastView = view;
      this.scroll.stickyScroll = w.view === "conversation";
      this.scroll.scrollTo(w.scrolls[view] ?? 0);
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
      this.card(id, key, label, color, body, order++, { ...options, heading: options.heading ?? heading });
    };
    const matches = (text: string) => !w.query || text.toLowerCase().includes(w.query.toLowerCase());
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
    if (w.loading)
      add("view-loading", w.view, "", c.muted, (box) => box.add(this.text(`Loading ${w.view}...`, c.muted)));
    let items = 0;
    if (w.view === "conversation") {
      const seen = new Set<string>();
      for (const [index, turn] of w.turns.entries())
        for (const [part, content] of turn[1].entries()) {
          const id = `turn-${index}-${part}`;
          if (typeof content === "string") {
            if (!matches(content)) continue;
            items++;
            add(
              id,
              content,
              `Python · ${this.preview(content, 11)}`,
              c.muted,
              (box) => box.add(this.numbered(content)),
              { compact: true },
            );
            continue;
          }
          if (!isTag(content)) continue;
          const [name, attrs, body] = content;
          const fields = Object.fromEntries(attrs);
          const actId = String(fields.id ?? fields.over ?? "");
          const act = w.acts.find((act) => act.id === actId);
          if (
            name === "ledger" ||
            (act &&
              (["chain", "grant"].includes(act.kind) ||
                (act.kind === "rung" && (act.by !== "operator" || !act.words[0]))) &&
              ["opened", "closed"].includes(name))
          )
            continue;
          if (!matches(JSON.stringify(content))) continue;
          if (act?.kind === "prompt" && name === "opened") {
            items++;
            add(
              id,
              JSON.stringify(content),
              act.by === "operator" ? "You" : "Observation",
              c.muted,
              (box) => box.add(this.markdown(String(fields.message ?? ""))),
              { group: act.by === "operator" ? "user" : "observation", act },
            );
          } else if (act?.kind === "prompt" && name === "closed") {
            items++;
            add(
              id,
              JSON.stringify(content),
              "Result",
              c.muted,
              (box) => box.add(this.markdown(display(act.value ?? body))),
              { group: "assistant", act },
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
              { compact: true, act },
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
              { compact: true, act },
            );
          }
        }
      for (const [id, stream] of w.world.streams) {
        if (stream.chain !== w.selected) continue;
        items++;
        add(
          `stream-${id}`,
          stream.text + stream.thinking,
          w.paused ? "Response held" : this.progress(id),
          c.muted,
          (box) => {
            if (stream.thinking) box.add(this.text(stream.thinking, c.muted));
            if (stream.text) box.add(this.code(stream.text));
          },
        );
      }
      if (!items && !w.query && !w.loading && !w.error) {
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
          add(id, word, `rung ${short(id)}`, c.muted, (box) => box.add(this.numbered(word)));
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
            { compact: true, act },
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
      if (w.world.changes.length > 20)
        add("change-pages", String(w.changePage), "", c.muted, (box) => {
          box.add(
            this.text(
              `Writes ${w.changePage * 20 + 1} to ${Math.min((w.changePage + 1) * 20, w.world.changes.length)} of ${w.world.changes.length}`,
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
          const diff = createTwoFilesPatch(change.path, change.path, change.before, change.after);
          add(`change-${index}`, diff, change.path, c.muted, (box) =>
            box.add(
              new DiffRenderable(this.renderer, {
                diff,
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
          `${index + Math.max(0, facts.length - 300) + 1} · ${fact[0]} · ${fact[1]}`,
          c.muted,
          (box) => {
            box.add(this.text(`by ${fact[2]}`, c.muted));
            box.add(this.text(JSON.stringify(fact.slice(3), null, 2)));
          },
          { compact: true },
        );
      }
    }
    if (!items && !w.loading && !w.error) {
      const empty = {
        conversation: "No conversation yet.",
        program: "No accepted Python yet. Use /run to write a rung.",
        activity: "No acts yet.",
        facts: "No facts yet.",
        transcript: "No transcript yet.",
        changes: "No file changes yet.",
      };
      const message = w.query ? `No matching ${w.view} for “${w.query}”.` : empty[w.view];
      add("empty", message, "", c.muted, (box) => box.add(this.text(message, c.muted)));
    }
    for (const id of existing) {
      this.cards.get(id)?.node.destroyRecursively();
      this.cards.delete(id);
    }
  }

  private preview(text: string, reserve = 2): string {
    const line = text.replace(/\s+/g, " ");
    const width = Math.max(8, this.scroll.width - reserve);
    if (Bun.stringWidth(line) <= width) return line;
    let result = "";
    for (const character of line) {
      if (Bun.stringWidth(result + character) >= width - 1) break;
      result += character;
    }
    return `${result}…`;
  }
  private failure(act: ActRow): boolean {
    return Boolean(
      act.value &&
        typeof act.value === "object" &&
        "is" in act.value &&
        "args" in act.value &&
        act.value.is !== "CancelledError",
    );
  }
  private actColor(act: ActRow): RGBA {
    if (this.failure(act)) return c.danger;
    return this.workspace.world.prompts.has(act.id) ? c.warning : c.muted;
  }
  private actSummary(act: ActRow): string {
    const held = this.workspace.world.held.has(act.id);
    const fault = this.failure(act);
    const cancelled =
      act.value && typeof act.value === "object" && "is" in act.value && act.value.is === "CancelledError";
    const parent = this.workspace.acts.find((candidate) => candidate.id === act.by);
    const answered = act.kind === "rung" && parent?.kind === "prompt" && parent.done && !this.failure(parent);
    const state =
      act.kind === "grant"
        ? act.done
          ? "ended ceiling"
          : "active ceiling"
        : act.done
          ? fault
            ? "failed"
            : cancelled && !answered
              ? "cancelled"
              : "done"
          : held
            ? "held"
            : this.workspace.world.prompts.has(act.id)
              ? "needs input"
              : this.workspace.paused && act.kind !== "bash"
                ? "paused"
                : `running ${this.progress(act.id)}`;
    const words =
      act.kind === "prompt"
        ? String(act.words[1])
        : act.kind === "grant"
          ? [
              act.words[0] === null ? "" : `$${act.words[0]}`,
              act.words[1] === null ? "" : `${Number(act.words[1]) * 100}% context`,
            ]
              .filter(Boolean)
              .join(" · ")
          : act.kind === "wait"
            ? `${act.words[0]}s`
            : String(this.workspace.program[act.id] || act.words[0] || "awaiting model");
    const prefix = `${act.kind} · `,
      suffix = ` · ${state}`;
    return `${prefix}${this.preview(words, Bun.stringWidth(prefix + suffix) + 2)}${suffix}`;
  }
  private actDetails(box: BoxRenderable, act: ActRow): void {
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
    if (act.kind === "rung" && (this.workspace.program[act.id] || act.words[0]))
      box.add(this.numbered(String(this.workspace.program[act.id] || act.words[0])));
    if (act.kind === "bash" && act.value && typeof act.value === "object") {
      const exit = act.value as { stdout?: { content: string }; stderr?: { content: string }; code?: number };
      if (exit.stdout?.content) {
        box.add(this.text("stdout", c.muted));
        box.add(this.text(exit.stdout.content));
      }
      if (exit.stderr?.content) {
        box.add(this.text("stderr", c.danger));
        box.add(this.text(exit.stderr.content, c.danger));
      }
      if (act.done) box.add(this.text(`exit: ${exit.code ?? "timeout"}`, c.muted));
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
        padding: space.inset,
        border: true,
        borderColor: c.link,
        backgroundColor: c.raised,
        zIndex: 30,
        onMouseDown: () => {
          void this.follow(value).catch(this.workspace.fail);
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
      const match = [...line.matchAll(/[a-z]+:\/\/[\w./-]+|path="([^"\n]+)"/g)].find(
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
      if (value?.reference) void this.follow(value.value).catch(this.workspace.fail);
      else if (value && (event.modifiers.ctrl || event.modifiers.alt)) this.inspect(value.value);
    };
    return node;
  }

  private renderInspector(): void {
    const w = this.workspace;
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
          height: 1,
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
    const width = w.panes.inspector - space.inset * 2;
    const path = Bun.stringWidth(directory) > width ? `…${directory.slice(1 - width)}` : directory;
    this.inspector.add(
      this.text(path, c.muted, {
        height: 1,
        truncate: true,
        onMouseDown: () => this.showValue("Directory", directory),
      }),
    );
    const usage = w.usage;
    if (usage.some((amount) => amount > 0)) {
      const ledger = w.turns
        .flatMap((turn) => turn[1])
        .findLast((part) => isTag(part) && part[0] === "ledger");
      const share =
        ledger && isTag(ledger) ? Number(ledger[1].find(([name]) => name === "filled")?.[1]) : undefined;
      this.inspector.add(
        this.text(w.demo ? "Simulated usage" : "Usage", c.text, { attributes: 1, marginTop: space.section }),
      );
      this.inspector.add(
        this.text(
          `$${usage[4].toFixed(4)}${share !== undefined && Number.isFinite(share) ? ` · ${(share * 100).toFixed(1)}% context` : ""}`,
        ),
      );
      this.inspector.add(this.text(`${count(usage[0])} in · ${count(usage[1])} out`, c.muted));
      if (usage[2]) this.inspector.add(this.text(`${count(usage[2])} cached`, c.muted));
    }
    const grant = w.activity.find((act) => act.kind === "grant" && !act.done);
    if (grant) {
      this.inspector.add(this.text("Ceiling", c.text, { attributes: 1, marginTop: space.section }));
      if (grant.words[0] !== null) this.inspector.add(this.text(`$${grant.words[0]}`));
      if (grant.words[1] !== null) this.inspector.add(this.text(`${Number(grant.words[1]) * 100}% context`));
    }
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
  models = (): void => {
    this.openPalette(
      "Model",
      this.workspace.roster
        .filter(([name]) => name !== "operator")
        .map(([name, , window]) => ({
          label: name,
          detail: `${count(window)} context${name === actorParts(this.workspace.actor).model ? " · selected" : ""}`,
          run: () => this.action(`/model ${name}`),
        })),
    );
  };
  effortPicker = (): void => {
    const { model, effort } = actorParts(this.workspace.actor);
    const offered = this.workspace.roster.find(([name]) => name === model)?.[1] ?? [];
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
        .filter(([, card]) => card.compact)
        .map(([id, card]) => ({
          label: card.heading.plainText.replace(/^[▸▾] /, ""),
          detail: this.expanded.has(id) ? "Collapse" : "Expand",
          run: () => {
            if (this.expanded.has(id)) this.expanded.delete(id);
            else this.expanded.add(id);
            this.renderContent();
            this.scroll.scrollChildIntoView(id);
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
    return `${["◐", "◓", "◑", "◒"][Math.floor(Date.now() / 250) % 4]} ${elapsed}s since start`;
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
          error.fg = c.danger;
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
          const source = this.workspace.selected;
          const omitted = acts.slice(index + 1).map((later) => JSON.stringify(later.id));
          const filter = `take(${omitted.length ? `${omitted.join(", ")}, ` : ""}inside=False)`;
          const word = `chain(${JSON.stringify(`${this.workspace.label} through ${short(act.id)}`)}, ${JSON.stringify(source)}, ${filter})`;
          const rung = await this.workspace.life.rung(word, { on: source });
          if (this.workspace.world.held.size) this.resume();
          else if (this.workspace.paused)
            this.workspace.notice = "Rewind is queued. Resume this chain to finish it.";
          await this.workspace.life.result(rung);
          await this.workspace.refresh();
          const next = this.workspace.chains.find((chain) => chain.by === rung);
          if (!next) throw new Error("The rewind word created no chain.");
          await this.workspace.select(next.id);
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
  openPalette(label: string, choices: Choice[], selected = 0): void {
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
    this.filtered = choices;
    this.selection = Math.max(0, selected);
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
    const max = Math.max(2, Math.floor((this.renderer.height - 12) / 2));
    const start = Math.max(0, this.selection - max + 1);
    for (const [index, choice] of this.filtered.slice(start, start + max).entries()) {
      const selected = index + start === this.selection;
      const row = this.box({
        backgroundColor: selected ? c.selected : c.raised,
        onMouseDown: () => {
          this.selection = index + start;
          this.choose();
        },
      });
      row.add(
        this.text(`${selected ? "▸" : " "} ${choice.label}`, selected ? c.accent : c.text, {
          height: 1,
          truncate: true,
          attributes: selected ? 1 : 0,
        }),
      );
      if (choice.detail) row.add(this.text(`  ${choice.detail}`, c.muted, { height: 1, truncate: true }));
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
        ["Ctrl+N / Ctrl+M / Shift+Tab / Ctrl+O", "New chain / model / effort / saved sessions"],
        ["Ctrl+F / PageUp / PageDown", "Filter the current view / scroll"],
        ["Ctrl+R / Ctrl+Space / Tab", "Python input / complete a name / complete a slash command"],
        ["Ctrl+G / Ctrl+click a name", "Inspect a value and follow its definition"],
        ["Click an act / /details", "Expand or collapse details"],
        ["Right-click an act", "Inspect its value or prompt program"],
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
    if (!this.overlay && key.name === "tab" && key.shift) {
      key.preventDefault();
      this.effortPicker();
      return;
    }
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
    this.workspace.expanded = [...this.expanded];
    this.workspace.save();
    if (this.redraw) clearTimeout(this.redraw);
    if (this.hoverTimer) clearTimeout(this.hoverTimer);
    this.workspace.off("change", this.schedule);
    this.workspace.off("compose", this.compose);
    this.workspace.off("inspect", this.inspect);
    this.workspace.off("resume", this.resume);
    this.workspace.off("rewind", this.rewind);
    this.workspace.off("models", this.models);
    this.workspace.off("efforts", this.effortPicker);
    this.workspace.off("details", this.details);
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

import {
  type BoxOptions,
  type BoxRenderable,
  CodeRenderable,
  getTreeSitterClient,
  infoStringToFiletype,
  MarkdownRenderable,
  type MouseEvent,
  type Renderable,
  type RenderNodeContext,
  type RGBA,
  type SimpleHighlight,
  StyledText,
  type SyntaxStyle,
  type TextRenderable,
  treeSitterToTextChunks,
} from "@opentui/core";
import { safeText, WORK } from "furb";
import { type Chip, cancelled, failed } from "furb/life";
import type { Token, Tokens } from "marked";
import { shortenHomes } from "../files.ts";
import { clip } from "../format.ts";
import { type ActRow, working } from "../session.ts";
import { theme as c, glyph, spacing as space, spin } from "../theme.ts";
import { type Part, plain, styled } from "../ui.ts";
import { part } from "./part.ts";
import { type BlockOptions, chipColors, noBorder, type State, stateOf } from "./shared.ts";

/** The lines of code whose colors the view keeps, and the widest line that it keeps. */
const PAINTED_LINES = 4096,
  PAINTED_WIDTH = 1000;

export const blocks = part({
  /** The columns of the feed: the screen but the sidebar that shows and the gutters. It is known before the feed is
   * laid out. */
  get feedWidth(): number {
    return this.renderer.width - (this.sidebarShown ? this.session.preferences.sidebarWidth : 0) - space.gutter * 2;
  },

  /** Whether the sidebar shows, from the preferences and the screen. The node of the rail follows it at the next
   * render, so a read of the node between a change and that render sees the old state. */
  get sidebarShown(): boolean {
    return this.session.preferences.sidebar && !this.reader && this.renderer.width >= 100;
  },

  /** A node that stands a number of columns in from the left, since a text leaves its own padding out when it draws. */
  inset(left: number, node: Renderable): BoxRenderable {
    const box = this.box({ paddingLeft: left, flexDirection: "row" });
    box.add(node);
    return box;
  },

  paneChanged(node: Renderable, state: unknown): boolean {
    const key = JSON.stringify(state);
    if (this.paneKeys.get(node) === key) return false;
    this.paneKeys.set(node, key);
    return true;
  },

  clear(node: Renderable): void {
    for (const child of [...node.getChildren()]) child.destroyRecursively();
  },

  /** The heading of a card set to a line, only when the line changed. A frame reads each card, so the line is held
   * as a flat list of its texts, the numbers of its colors and its attributes, which it compares with no copy. */
  label(card: { heading: TextRenderable; label: (string | number | undefined)[] }, parts: Part[]): void {
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
  },

  card(
    id: string,
    key: string,
    label: Part[],
    body: (box: BoxRenderable) => void,
    index: number,
    options: BlockOptions = {},
  ): void {
    const rung = options.act?.kind === "rung" ? options.act : undefined;
    const state = rung?.id ?? id;
    const closed = !options.open && this.folded(state, rung, Boolean(options.compact));
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
    const into = options.join && !this.reader ? this.joined(box, options.join, options.edge) : box;
    if (!closed) body(into);
    else if (!rung) options.preview?.(into);
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
  },

  /** The card at a place of the feed. */
  feedChild(index: number): Renderable | undefined {
    this.feedOrder ??= this.scroll.getChildren();
    return this.feedOrder[index];
  },

  /** A card moved to a place of the feed, or added there. */
  placeCard(node: Renderable, index: number): void {
    this.scroll.add(node, index);
    this.feedOrder = undefined;
  },

  /** The surface that the cards of one thread share: a panel, whose top edge the first card draws and whose bottom
   * edge the last one draws, so that the cards stand as one box. It gives the node that the body of the card fills. */
  joined(box: BoxRenderable, join: "only" | "first" | "middle" | "last", color?: RGBA): BoxRenderable {
    // The surface starts at the column where the bar of the input of the chain starts, so that the box of a thread and
    // the box of the input stand on one line. Its edges are half rows of the surface, and an edge holds the cap of the
    // line of a message of the operator that stands first or last in the box, in the color of that line.
    const edge = (side: "top" | "bottom") => {
      const half = this.box({
        height: space.bar,
        flexGrow: 1,
        border: [side === "top" ? "bottom" : "top"],
        borderColor: c.panel,
        customBorderChars: { ...noBorder, horizontal: side === "top" ? glyph.halfTop : glyph.halfBottom },
      });
      if (!color) return half;
      const cap = this.box({
        height: space.bar,
        border: ["left"],
        borderColor: color,
        customBorderChars: { ...noBorder, vertical: side === "top" ? glyph.barTop : glyph.barBottom },
      });
      cap.add(half);
      return cap;
    };
    if (join === "only" || join === "first") box.add(edge("top"));
    const inner = this.box({ paddingRight: space.between, backgroundColor: c.panel });
    box.add(inner);
    if (join === "only" || join === "last") box.add(edge("bottom"));
    return inner;
  },

  /** A panel in a box: a bar of a color at its left, and half a row of the panel above and below what it holds. */
  panel(box: BoxRenderable, color: RGBA, options: BoxOptions = {}): BoxRenderable {
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
  },

  /** Python, highlighted as it shows, or by colors, which give the highlights of the whole content in their place. */
  code(
    content: string,
    marks?: SimpleHighlight[],
    hover?: (event: MouseEvent) => boolean,
    colors?: () => Promise<SimpleHighlight[]>,
  ): CodeRenderable {
    const text = safeText(content);
    const code = new CodeRenderable(this.renderer, {
      content: text,
      filetype: "python",
      syntaxStyle: this.style,
      wrapMode: "word",
      drawUnstyledText: true,
      initialStyledText: this.painted(text, "python", this.style),
      onHighlight: async (highlights, context) => {
        const made = colors ? await colors() : [...highlights, ...(marks ?? [])];
        this.paint(context.content, "python", made);
        return made;
      },
    });
    return this.pointable(code, content, hover);
  },

  /** Code in the colors that the parser gave each of its lines last, and a line that it never colored in the color of
   * plain code. A node of code draws it until the parser colors the node, which takes a frame or more: the feed makes
   * its nodes again at each change, so code drawn plain would blink at each change. The node draws this text again
   * when its content changes, so a node that takes it keeps its content. */
  painted(content: string, filetype: string, style: SyntaxStyle): StyledText {
    const highlights: SimpleHighlight[] = [];
    let start = 0;
    for (const line of content.split("\n")) {
      for (const [from, to, ...rest] of this.paintedLines.get(`${filetype}\n${line}`) ?? [])
        highlights.push([start + from, start + to, ...rest] as SimpleHighlight);
      start += line.length + 1;
    }
    return new StyledText(treeSitterToTextChunks(content, highlights, style));
  },

  /** Keeps the highlights that the parser gave code, cut at each line, for the lines that came last. */
  paint(content: string, filetype: string, highlights: SimpleHighlight[]): void {
    const lines = content.split("\n");
    const starts: number[] = [];
    let offset = 0;
    for (const line of lines) {
      starts.push(offset);
      offset += line.length + 1;
    }
    const kept = lines.map((): SimpleHighlight[] => []);
    for (const [from, to, ...rest] of highlights) {
      let low = 0,
        high = lines.length - 1;
      while (low < high) {
        const middle = (low + high + 1) >> 1;
        if ((starts[middle] ?? 0) <= from) low = middle;
        else high = middle - 1;
      }
      for (let row = low; row < lines.length && (starts[row] ?? 0) < to; row++) {
        const begin = starts[row] ?? 0;
        const left = Math.max(from, begin) - begin,
          right = Math.min(to, begin + (lines[row]?.length ?? 0)) - begin;
        if (left < right) kept[row]?.push([left, right, ...rest] as SimpleHighlight);
      }
    }
    for (const [row, line] of lines.entries()) {
      if (line.length > PAINTED_WIDTH) continue;
      const key = `${filetype}\n${line}`;
      this.paintedLines.delete(key);
      this.paintedLines.set(key, kept[row] ?? []);
    }
    for (const key of this.paintedLines.keys()) {
      if (this.paintedLines.size <= PAINTED_LINES) break;
      this.paintedLines.delete(key);
    }
  },

  /** The group of each character of a program, as the parser of python highlights it, kept for the last programs. */
  groupsOf(source: string): Promise<(string | undefined)[]> {
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
  },

  /** Python under the pointer. A name shows what it holds in a card once the pointer rests on it, and a click with ⌃
   * or ⌥ opens it in the inspector. A header that names an act and an image attachment are references, which show
   * what they name in the card and open it at a click. */
  pointable<T extends CodeRenderable | TextRenderable>(
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
  },

  /** Markdown on a surface, whose code blocks stand out from it on the surface of a code block, or on the surface
   * that a code block takes in a panel. */
  markdown(content: string, fg = c.prose, surface = c.panel): MarkdownRenderable {
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
  },

  /** A code block: a surface of its own with an inset, its language as a label, the colors of its grammar, and its
   * lines as they are. A line too long for the block does not wrap, so that it keeps the indentation of each line. */
  codeBlock(
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
    const filetype = infoStringToFiletype(language);
    block.add(
      new CodeRenderable(this.renderer, {
        content: text,
        filetype,
        syntaxStyle: context.syntaxStyle,
        treeSitterClient: context.treeSitterClient,
        fg: c.text,
        wrapMode: "none",
        drawUnstyledText: true,
        width: "100%",
        ...(filetype
          ? {
              initialStyledText: this.painted(text, filetype, context.syntaxStyle),
              onHighlight: (highlights: SimpleHighlight[]) => {
                this.paint(text, filetype, highlights);
                return highlights;
              },
            }
          : {}),
      }),
    );
    return block;
  },

  /** Whether a card is folded: by the operator's click, or else as its kind starts. A rung that runs, or that failed
   * with no rung that took its place, stands open, and any other rung starts folded when the preference says so. */
  folded(state: string, rung?: ActRow, compact = false): boolean {
    return (
      this.session.folds[state] ??
      (rung
        ? this.session.preferences.foldThreads &&
          !working(rung) &&
          (!failed(rung) || Boolean(this.retry(rung)))
        : compact)
    );
  },

  preview(text: string, reserve = 2): string {
    return clip(text.replace(/\s+/g, " "), Math.max(8, this.feedWidth - reserve));
  },

  /** The first or the last lines of a text under a heading, joined to it by a branch. */
  excerpt(box: BoxRenderable, content: string, tail = false, color = c.muted): void {
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
  },

  /** A text joined by a branch to the heading above it. */
  branched(text: string, color: RGBA): BoxRenderable {
    const row = this.box({ flexDirection: "row", paddingLeft: space.between });
    row.add(this.text(`${glyph.branch} `, c.faint));
    row.add(this.text(text, color, { flexShrink: 1 }));
    return row;
  },

  /** What an act is doing: the word that says it, the glyph that shows it, and their color. A done act says
   * nothing, since its glyph says it. */
  actState(act: ActRow): State {
    const w = this.session;
    // At a moment of the past, an act that had not ended yet shows so, with no spinner and no time, since nothing runs
    // there.
    const running = () =>
      w.moment === undefined
        ? { word: `running ${this.progress(act.id)}`, mark: spin(), color: c.accent }
        : { word: "not done yet", mark: glyph.ring, color: c.faint };
    if (act.kind === "grant" && act.done) return { word: "ended", mark: glyph.ring, color: c.faint };
    // Only work moves: an act of another kind that lives, as a grant or the watcher of an extension, lives until
    // something ends it, as a chain does, and shows a dot.
    if (!act.done && !WORK.includes(act.kind)) return { word: "", mark: glyph.dot, color: c.accent };
    // In a record open to read, the work that had no end when the record ended has none to come: it neither runs nor
    // waits for a wake.
    if (act.unfinished && !act.done)
      return { word: "not finished when the record ends", mark: glyph.ring, color: c.faint };
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
  },

  /** The first lines of a text that take at most a count of rows of the feed, and how many lines are left out. */
  firstRows(text: string, rows: number): [string, number] {
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
  },

  /** The acts of a rung as chips, each in the color of its state, as many as the room holds. */
  chipParts(chips: readonly Chip[], room: number): Part[] {
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
  },

  /** Once each line of a word that the view asked to color has its colors. */
  async colorsDone(): Promise<void> {
    while (this.coloring.size) await Promise.all(this.coloring.values());
  },
});

export type BlocksPart = typeof blocks;

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
  type TextRenderable,
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

export const blocks = part({
  /** The columns of the feed: the screen but the sidebar that shows and the gutters. It is known before the feed is
   * laid out. */
  get feedWidth(): number {
    return (
      this.renderer.width - (this.rail.visible ? this.session.preferences.sidebarWidth : 0) - space.gutter * 2
    );
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

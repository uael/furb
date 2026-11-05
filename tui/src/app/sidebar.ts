import type { BoxOptions, BoxRenderable, Renderable, TextRenderable } from "@opentui/core";
import { unmarked } from "furb";
import type { ChainThread } from "furb/life";
import { count, dollars } from "../format.ts";
import { theme as c, glyph, spacing as space, threadTone } from "../theme.ts";
import { bold, type Part, plain } from "../ui.ts";
import { part } from "./part.ts";
import type { TextShape } from "./shared.ts";

export const sidebar = part({
  /** The context of the chain shown, in one short line for the footer: how full its window is, or the size of its
   * prompt, and what it spent. */
  contextLine(): string {
    const w = this.session;
    const window = w.filled;
    const known = window !== undefined && Number.isFinite(window);
    const share = known
      ? `${Math.round((window ?? 0) * 100)}%`
      : w.context === undefined
        ? "No answer yet"
        : count(w.context);
    return known || w.spend.dollars ? `${share} · ${dollars(w.spend.dollars)}` : share;
  },

  /** The context of the chain shown, in the corner under the sidebar: how full its window is and where a grant pauses it,
   * its tokens, and what it spent. */
  renderRailUsage(): void {
    const w = this.session;
    const window = w.filled;
    const grant = w.open.find((act) => act.kind === "grant");
    const width = w.preferences.sidebarWidth;
    if (
      !this.paneChanged(this.railUsage, [this.theme, w.spend, window, width, w.context, grant?.words, w.demo])
    )
      return;
    this.clear(this.railUsage);
    const inner = width - space.inset * 2;
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
    const spend = w.spend;
    const ceiling =
      grant?.words[1] === null || grant?.words[1] === undefined
        ? undefined
        : Number(unmarked(grant.words[1]));
    const known = window !== undefined && Number.isFinite(window);
    // The share of the window leads the header, with the size of the window when the answer tells it.
    const prompt = w.context;
    const size = known && prompt !== undefined && window ? count(Math.round(prompt / window)) : "";
    spread(
      ["Context", c.text, bold],
      w.demo ? "  simulated" : "",
      known
        ? [`${Number(((window ?? 0) * 100).toFixed(1))}%${size ? ` of ${size}` : ""}`, c.text]
        : [prompt === undefined ? "No answer yet" : count(prompt), prompt === undefined ? c.faint : c.text],
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
    // One line holds the numbers: the whole prompt of the last answer, what the cache gave it, and what the chain
    // spent against its ceiling. A hover over it names each number of the chain, each token counted once.
    const limit = grant?.words[0];
    const spent =
      limit === null || limit === undefined
        ? dollars(spend.dollars)
        : `${dollars(spend.dollars)}/${dollars(Number(unmarked(limit)))}`;
    // The numbers lose their words one by one before they lose their room, so that the line shows whole.
    const cached = spend.cacheRead ? ` · ${count(spend.cacheRead)} cached` : "";
    const forms =
      prompt === undefined
        ? [cached.slice(3)]
        : [`${count(prompt)} prompt${cached}`, `${count(prompt)}${cached}`, count(prompt)];
    const left = forms.find((one) => Bun.stringWidth(one + spent) + 1 <= inner) ?? forms.at(-1) ?? "";
    const numbers = add([
      [left, c.muted],
      [" ".repeat(Math.max(1, inner - Bun.stringWidth(left + spent)))],
      [spent, c.text],
    ]);
    this.tipped(numbers, [
      [`Prompt ${prompt === undefined ? "none" : count(prompt)}`, c.text, bold],
      [`  fresh input ${count(spend.input)}`, c.muted],
      [`  output ${count(spend.output)}`, c.muted],
      [`  cache read ${count(spend.cacheRead)}`, c.muted],
      [`  cache write ${count(spend.cacheWrite)}`, c.muted],
    ]);
  },

  /** Whether a thread stands folded to its header in the feed: as the operator left it, and else when it is resolved,
   * is not the last thread of its chain, and the preference folds what is over. */
  threadFolded(id: string): boolean {
    const w = this.session;
    const set = w.folds[`thread:${id}`];
    if (set !== undefined) return set;
    const own = w.threads;
    const thread = own.by.get(id);
    return Boolean(w.preferences.foldThreads && thread?.resolved && own.list.at(-1) !== thread);
  },

  /** A thread folds in the feed to its header line alone, or unfolds. */
  foldThread(id: string, folded?: boolean): void {
    const fold = folded ?? !this.threadFolded(id);
    const w = this.session;
    // A thread that folds leaves the eyes of the operator, so the record says up to where he read it.
    if (fold) w.leave(id);
    w.folds[`thread:${id}`] = fold;
    w.save();
    this.renderContent();
    this.render();
  },

  /** A thread folds or unfolds, and the feed then shows its header. */
  foldHere(id: string): void {
    this.foldThread(id);
    this.scrollAfterLayout({ card: id });
  },

  /** The cards of the feed in its order, each with the thread whose box it stands in, as the last layout placed them. */
  threadCards(): { node: Renderable; thread: string }[] {
    const by = new Map([...this.cards.values()].map((card) => [card.node as Renderable, card.thread]));
    return this.scroll.getChildren().flatMap((node) => {
      const thread = by.get(node);
      return thread ? [{ node, thread }] : [];
    });
  },

  /** The last card of the box of a thread in the feed, which ends the thread. */
  threadEnd(id: string): Renderable | undefined {
    return this.threadCards().findLast((one) => one.thread === id)?.node;
  },

  /** The thread whose box holds the middle row of the feed in view, or else the box of a thread in view nearest it. */
  threadInView(): string | undefined {
    const { viewport } = this.scroll;
    const middle = viewport.y + Math.floor(viewport.height / 2);
    let best: { thread: string; distance: number } | undefined;
    for (const { node, thread } of this.threadCards()) {
      const top = node.y;
      const bottom = node.y + node.height;
      if (bottom <= viewport.y || top >= viewport.y + viewport.height) continue;
      const distance = middle < top ? top - middle : middle >= bottom ? middle - bottom + 1 : 0;
      if (!best || distance < best.distance) best = { thread, distance };
    }
    return best?.thread;
  },

  /** A thread resolves: its open prompt closes, and the model that works for it stops. */
  resolveThread(thread: ChainThread): void {
    const open = [thread.act, ...thread.messages].findLast((act) => !act.done);
    if (open) void this.session.resolve(open.id).catch(this.report);
  },

  /** A thread unfolds, and the input takes the keys and sends its next messages to the thread, with the color of the
   * thread on its line. A message to a thread that is over opens it again. */
  replyTo(id: string): void {
    const w = this.session;
    if (w.host.reading) return;
    w.folds[`thread:${id}`] = false;
    w.save();
    w.aim(id);
    this.railAt = id;
    this.railKeys = false;
    this.renderContent();
    this.scrollAfterLayout({ card: `${id}-reply` });
    this.composer.focus();
    this.render();
  },

  /** The toggle of a thread on its header in the feed: Collapse, or Expand for a thread that stands folded. It is the
   * one control of the thread in the feed, and a fold from it shows the header of the thread. */
  threadToggle(id: string): TextRenderable {
    const folded = this.threadFolded(id);
    const toggle = this.link(
      [
        [`${folded ? glyph.closed : glyph.open} `, c.faint],
        [folded ? "Expand" : "Collapse", c.faint],
      ],
      () => this.foldHere(id),
    );
    // The toggle takes its own row alone: in a row beside a message of many rows it would stretch to the height of the
    // message, and a click in its column on any row of the message would fold the thread.
    toggle.marginLeft = space.between;
    toggle.alignSelf = "flex-start";
    return toggle;
  },

  /** The thread whose box holds the top row of the feed while its header stands above the feed. */
  pinnedThread(): string | undefined {
    const cards = this.threadCards();
    const top = this.scroll.viewport.y;
    const at = cards.findLastIndex((one) => one.node.y <= top);
    const card = cards[at];
    const head = card && this.cards.get(card.thread)?.node;
    if (!card || !head || head.y >= top) return undefined;
    // The top row stands in the card, or in the gap between two cards of the same box.
    const holds = card.node.y + card.node.height > top || cards[at + 1]?.thread === card.thread;
    return holds ? card.thread : undefined;
  },

  /** The header of the thread whose box holds the top row of the feed, pinned at the top of the feed while the header
   * itself stands above it: on one row, who sent the message, the title of the thread in bold, its state and its
   * toggle, with the line and the inset of the header, on the raised surface that parts it from the body under it. It
   * reads the layout of the frame that was drawn, and draws again only when what it shows changed. */
  renderPin(): void {
    const w = this.session;
    const id = this.pinnedThread();
    const thread = id ? w.threads.by.get(id) : undefined;
    const card = id ? this.cards.get(id) : undefined;
    const act = id ? w.actOf(id) : undefined;
    const folded = Boolean(id && this.threadFolded(id));
    const head = act && this.threadHead(act, folded);
    const user = Boolean(act && w.isUserPrompt(act));
    const key =
      id && thread && card && head
        ? `${id}\n${thread.title}\n${head.sent}\n${plain(head.state)}\n${user}\n${folded}\n${card.indent}\n${this.feedWidth}\n${this.theme}`
        : "";
    if (key === this.pinKey) return;
    // A press on the toggle of the pin holds no more once the pin shows another thread: the toggle that the release
    // finds at the same place, with the same text, is a control that the operator never pressed.
    if (key.split("\n", 1)[0] !== this.pinKey.split("\n", 1)[0]) {
      let node = this.pressed;
      while (node && node !== this.pin) node = node.parent;
      if (node) this.pressed = null;
    }
    this.pinKey = key;
    this.clear(this.pin);
    this.pin.visible = Boolean(key);
    this.renderer.requestRender();
    if (!id || !thread || !card || !head) return;
    const row = this.box({ backgroundColor: c.background, paddingLeft: card.indent });
    const surface = this.box({ paddingRight: space.between, backgroundColor: c.raised });
    row.add(surface);
    this.pin.add(row);
    const line = this.row();
    line.add(
      this.text(
        [
          ...(user
            ? [["You  ", threadTone(id)] as Part]
            : head.sent
              ? [[`${head.sent}  `, c.faint] as Part]
              : []),
          [thread.title, c.text, bold],
        ],
        c.text,
        { truncate: true, flexGrow: 1, flexShrink: 1 },
      ),
    );
    line.add(this.text(head.state, c.faint, { marginLeft: space.between }));
    line.add(this.threadToggle(id));
    this.barred(surface, head.bar, 0, { top: false, bottom: false }).add(line);
  },

  /** A row of the sidebar, whose click runs, which spans the sidebar so that the pointer and the selection show from
   * edge to edge. It lights while the pointer is on it, and stays lit while it is selected. */
  railRow(parts: Part[], run: () => void, options: BoxOptions, selected = false): BoxRenderable {
    const row = this.hoverable(
      this.row({ backgroundColor: selected ? c.selected : c.panel, onMouseUp: this.click(run), ...options }),
      selected ? c.selected : c.raised,
    );
    row.add(this.text(parts, c.muted, { truncate: true, flexGrow: 1, flexShrink: 1 }));
    return row;
  },

  /** A row of the sidebar that folds a list under it, with its label and the count of the list, which a click folds
   * or unfolds. */
  foldRow(key: string, label: string, count: number, options: BoxOptions): BoxRenderable {
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
  },

  /** A node that shows a tip while the pointer is over it. */
  tipped<T extends Renderable>(node: T, parts: Part[]): T {
    node.onMouseOver = (event) => this.tip(parts, event.x, event.y);
    node.onMouseOut = () => this.unhover();
    return node;
  },
});

export type SidebarPart = typeof sidebar;

export const sidebarBound = ["renderPin"] as const;

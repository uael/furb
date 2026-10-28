import { basename } from "node:path";
import type { BoxOptions, BoxRenderable, RGBA } from "@opentui/core";
import { type Ask, type ChainThread, chainThreads, gist } from "furb/life";
import { checkoutOf } from "../checkout.ts";
import { clock } from "../clock.ts";
import { shortenHome, shortenHomes } from "../files.ts";
import { clip, dollars } from "../format.ts";
import { isRailStyle, type RailStyle } from "../preferences.ts";
import { statusLabels } from "../session.ts";
import { theme as c, glyph, spacing as space, threadTone } from "../theme.ts";
import { bold, type Part, plain } from "../ui.ts";
import type { SessionEntry, Workspace } from "../workspaces.ts";
import { part } from "./part.ts";
import { type ChainRead, statusMark, threadMarks, wrapped } from "./shared.ts";

/** What waits in a part of the library, counted where the operator should look. */
export interface Attention {
  asks: number;
  working: number;
  held: number;
  failed: number;
  unread: number;
}

/** What the keys of the sidebar do on a row. */
export interface RailHandler {
  enter(): void;
  /** The right arrow: it opens a workspace to peek at its sessions, and else does what Enter does. */
  right?(): void;
  /** The left arrow: it folds a workspace that was opened to peek, and else gives the keys back to the input. */
  left?(): void;
  space?(): void;
  thread?: ChainThread;
}

/** A question that waits in a session of the library. */
export interface Waiting {
  group: Workspace;
  entry: SessionEntry;
  ask: Ask;
}

const LEFT = space.inset;
const RIGHT = space.inset;
/** How many workspaces the switcher of the cards shows at once. */
const SWITCHER = 8;
/** How many questions the inbox of the sidebar holds before it counts the rest. */
const INBOX = 3;

/** The number in the name of an act, which orders the acts as they came. */
const numberOf = (id: string) => Number(id.replace(/\D+/g, "")) || 0;

export const rail = part({
  /** The shape of the sidebar: the flag FURB_SIDEBAR, else the preference. */
  get railStyle(): RailStyle {
    const named = process.env.FURB_SIDEBAR;
    return isRailStyle(named) ? named : this.session.preferences.sidebarStyle;
  },

  /** The sidebar takes another shape, which the preferences keep. */
  setRailStyle(name: string): void {
    const w = this.session;
    if (!isRailStyle(name)) {
      w.notice = `The sidebar takes the shapes outline, cards and inbox, and not ${name}.`;
      return;
    }
    w.preferences.sidebarStyle = name;
    w.preferences.save();
    w.notice = `The sidebar is ${name}.`;
    this.render();
  },

  /** What waits in some sessions: each question, each session at work, held, failed, or finished and not seen. */
  attentionOf(entries: readonly SessionEntry[]): Attention {
    const out: Attention = { asks: 0, working: 0, held: 0, failed: 0, unread: 0 };
    for (const entry of entries) {
      if (entry.archived) continue;
      const asks = entry.session ? entry.session.desk.asks.filter((ask) => !ask.notice).length : 0;
      if (asks) out.asks += asks;
      else if (entry.status === "blocked") out.asks += 1;
      if (entry.status === "working" || entry.status === "opening") out.working += 1;
      else if (entry.status === "paused") out.held += 1;
      else if (entry.status === "error") out.failed += 1;
      else if (entry.status === "done") out.unread += 1;
    }
    return out;
  },

  /** The marks of what needs a look, in the order of their weight: a question first. */
  badges(a: Attention): Part[] {
    const parts: Part[] = [];
    const add = (n: number, mark: string, tone: RGBA, attributes = 0) => {
      if (!n) return;
      if (parts.length) parts.push(["  "]);
      parts.push([`${mark} ${n}`, tone, attributes]);
    };
    add(a.asks, glyph.asks, c.warning, bold);
    add(a.failed, glyph.failed, c.danger);
    add(a.held, glyph.held, c.warning);
    add(a.working, glyph.running, c.accent);
    add(a.unread, glyph.dot, c.success);
    return parts;
  },

  /** The words that a hover over the marks of a row says. */
  badgeTip(a: Attention): Part[] {
    const said = [
      a.asks ? `${a.asks} ${a.asks === 1 ? "question waits" : "questions wait"} for you` : "",
      a.failed ? `${a.failed} failed` : "",
      a.held ? `${a.held} paused` : "",
      a.working ? `${a.working} at work` : "",
      a.unread ? `${a.unread} finished and not seen` : "",
    ].filter(Boolean);
    return [[said.join("  ·  "), c.text]];
  },

  /** Each question that waits in an open session of the library, the session shown first. */
  railAsks(): Waiting[] {
    const lib = this.options.workspaces;
    const all: Waiting[] = [];
    for (const group of lib.groups)
      for (const entry of group.sessions) {
        if (entry.archived || !entry.session) continue;
        for (const ask of entry.session.desk.asks) if (!ask.notice) all.push({ group, entry, ask });
      }
    return [
      ...all.filter((one) => one.entry === lib.current),
      ...all.filter((one) => one.entry !== lib.current),
    ];
  },

  /** The threads of the session shown, on every chain of the life, in the order they began. A thread is a prompt, and
   * the chain that it stands on is a detail of it. */
  railThreads(): ChainThread[] {
    const w = this.session;
    const prompts = new Set(w.host.prompts.keys());
    const owed = [...prompts].join(" ");
    const kept = this.railThreadCache;
    if (kept && kept.session === w && kept.version === w.version && kept.owed === owed) return kept.list;
    const index = w.ladderIndex;
    const board = w.boardIndex;
    const list: ChainThread[] = [];
    // A chain that no act touched since the last read keeps its threads, so a new act costs one chain and not all.
    const reads = kept?.session === w && kept.owed === owed ? kept.chains : new Map<string, ChainRead>();
    const chains = new Map<string, ChainRead>();
    for (const [chain, acts] of w.activityByChain) {
      if (!acts.length) continue;
      const stamp = board.stamp(chain);
      const read = reads.get(chain);
      // The chain shown has its threads read already, for the feed, at this version.
      const threads =
        read && read.stamp === stamp
          ? read.list
          : chain === w.selected
            ? this.chainThreadsBy(chain).list
            : chainThreads(acts, index.ids, prompts, this.promptsRead);
      chains.set(chain, { stamp, list: threads });
      list.push(...threads);
    }
    list.sort((a, b) => numberOf(a.act.id) - numberOf(b.act.id));
    this.railThreadCache = { session: w, version: w.version, owed, list, chains };
    return list;
  },

  /** What the rows of the threads need to be drawn again, kept while the list of the threads and the minute stand: a
   * frame of a life with many threads then reads each thread once for each new act, and not once for each frame. */
  threadsKey(threads: ChainThread[]): string {
    const minute = Math.floor(clock() / 60000);
    const kept = this.railThreadKey;
    if (kept && kept.list === threads && kept.minute === minute) return kept.text;
    const text = JSON.stringify(
      threads.map((one) => [
        one.act.id,
        one.state,
        one.resolved,
        one.title,
        one.messages.length,
        plain(this.threadMeta(one)),
      ]),
    );
    this.railThreadKey = { list: threads, minute, text };
    return text;
  },

  /** The thread that the feed shows now: the one chosen, else the last that is open on the chain shown. */
  activeThread(): ChainThread | undefined {
    const w = this.session;
    const all = this.railThreads();
    const chosen = w.thread ? all.find((one) => one.act.id === w.thread) : undefined;
    const own = all.filter((one) => one.act.on === w.selected);
    const found = chosen ?? own.findLast((one) => !one.resolved) ?? own.at(-1);
    // The input sends its next message to the thread that is lit.
    w.lit = found?.act.id;
    return found;
  },

  /** The state of a thread in a word and its color. */
  stateWord(thread: ChainThread): Part {
    if (thread.state === "asks") return ["asks you", c.warning, bold];
    if (thread.state === "held") return ["paused", c.warning];
    if (thread.state === "failed") return ["failed", c.danger];
    if (thread.state === "cancelled") return ["cancelled", c.faint];
    if (thread.resolved) return ["resolved", c.faint];
    return ["working", c.accent];
  },

  /** What a row of a thread says at its right: why it needs a look, else how long it has worked. */
  threadMeta(thread: ChainThread): Part[] {
    if (thread.state !== "running" && thread.state !== "done") return [this.stateWord(thread)];
    if (thread.resolved) return [];
    const began = this.session.started[thread.act.id];
    const minutes = began === undefined ? 0 : Math.floor((clock() - began) / 60000);
    return minutes < 1 ? [] : [[minutes < 60 ? `${minutes}m` : `${Math.floor(minutes / 60)}h`, c.faint]];
  },

  /** A row of the sidebar in two parts, its head, which the room cuts and a hover shows whole, and its tail, which
   * stays. */
  railSplit(
    head: Part[],
    tail: Part[],
    run: () => void,
    options: BoxOptions = {},
    selected = false,
  ): BoxRenderable {
    const row = this.hoverable(
      this.row({ backgroundColor: selected ? c.selected : c.panel, onMouseUp: this.click(run), ...options }),
      selected ? c.selected : c.raised,
    );
    row.add(this.text(head, c.muted, { truncate: true, flexGrow: 1, flexShrink: 1 }));
    if (tail.length) row.add(this.text(tail, c.muted, { marginLeft: space.between, flexShrink: 0 }));
    return row;
  },

  /** The thread chosen in the sidebar: the feed shows it on its chain, scrolled to it, and the input of its box takes
   * the reply. */
  chooseThread(thread: ChainThread): void {
    const w = this.session;
    const id = thread.act.id;
    this.railAt = `th:${id}`;
    void (async () => {
      const head = w.runOf(thread.act.on).at(-1) ?? thread.act.on;
      if (head !== w.selected) await w.select(thread.act.on);
      w.thread = id;
      if (this.threadFolded(id)) {
        w.folds[`thread:${id}`] = false;
        w.save();
      }
      if (this.railKeys) this.railSteer(false);
      this.go("feed", id);
    })().catch(this.report);
  },

  /** A workspace takes the focus: the session of it that needs the operator, else one that lives, else its newest. */
  async focusWorkspace(group: Workspace): Promise<void> {
    const lib = this.options.workspaces;
    if (group === lib.groupOf()) return;
    const kept = group.sessions.filter((entry) => !entry.archived);
    const asks = (entry: SessionEntry) => Boolean(entry.session?.desk.asks.some((ask) => !ask.notice));
    const pick = kept.find(asks) ?? kept.find((entry) => entry.session) ?? kept[0];
    if (pick) await lib.select(pick);
    else await lib.create(group);
  },

  /** A workspace that does not hold the focus opens to show its sessions, or folds again. */
  peekWorkspace(group: Workspace): void {
    const key = `ws:${group.directory}`;
    if (!this.unfolded.delete(key)) this.unfolded.add(key);
    this.render();
  },

  /** The session of a question opens, and the question takes the input. */
  async goToAsk({ entry, ask }: Waiting): Promise<void> {
    const lib = this.options.workspaces;
    if (lib.current !== entry) await lib.select(entry);
    await entry.session?.point(ask.id);
  },

  /** The next question that waits for the operator in any workspace, one after the other. */
  async nextAttention(): Promise<void> {
    const all = this.railAsks();
    if (!all.length) {
      this.session.notice = "Nothing waits for you.";
      return;
    }
    const target = all[this.railNext % all.length];
    this.railNext += 1;
    if (target) await this.goToAsk(target);
  },

  renderRail(): void {
    if (!this.rail.visible) return;
    this.renderRailTop();
    this.renderWorkspaces();
    this.renderRailUsage();
  },

  /** The top of the sidebar: one line that says whether something waits for you, anywhere, and what a click goes to.
   * The shape inbox lists the questions under it, and the shape cards puts the switcher of the workspaces there. */
  renderRailTop(): void {
    const w = this.session;
    const lib = this.options.workspaces;
    const style = this.railStyle;
    const width = this.rail.width || w.preferences.sidebarWidth;
    const total = this.attentionOf(lib.groups.flatMap((group) => group.sessions));
    const waiting = this.railAsks();
    const key = JSON.stringify([
      this.theme,
      width,
      style,
      total,
      style === "inbox"
        ? waiting.map(({ entry, ask }) => [entry.path, ask.id, ask.label, ask.message, ask.choices])
        : 0,
      style === "cards"
        ? [
            lib.groups.map((group) => [group.name, group.directory, this.attentionOf(group.sessions)]),
            lib.notice,
          ]
        : 0,
      this.choice,
      lib.current?.path,
      lib.groupOf()?.directory,
    ]);
    if (!this.paneChanged(this.railSession, key)) return;
    this.clear(this.railSession);
    const room = width - LEFT - RIGHT;
    const edge: BoxOptions = { paddingLeft: LEFT, paddingRight: RIGHT };
    if (style === "cards") {
      // A question that waits shows on top, wherever it waits; the rest of the marks stand on the rows of the switcher.
      if (total.asks)
        this.railSession.add(
          this.railSplit(
            [
              [`${glyph.asks} ${total.asks} `, c.warning, bold],
              [total.asks === 1 ? "needs you" : "need you", c.text, bold],
            ],
            [["⌥A", c.faint]],
            () => void this.nextAttention().catch(this.report),
            edge,
          ),
        );
      this.renderSwitcher(room, edge);
      return;
    }
    const head: Part[] = total.asks
      ? [
          [`${glyph.asks} ${total.asks} `, c.warning, bold],
          [total.asks === 1 ? "needs you" : "need you", c.text, bold],
        ]
      : [
          [`${glyph.done} `, c.success],
          ["Nothing waits for you", c.muted],
        ];
    const tail: Part[] = [
      ...(total.working ? [[`${glyph.running} ${total.working} at work`, c.accent] as Part] : []),
      ...(total.asks ? [["  ⌥A", c.faint] as Part] : []),
    ];
    this.railSession.add(
      this.railSplit(head, tail, () => void this.nextAttention().catch(this.report), edge),
    );
    if (style !== "inbox" || !waiting.length) return;
    for (const one of waiting.slice(0, INBOX)) this.renderInboxItem(one, room);
    if (waiting.length > INBOX)
      this.railSession.add(
        this.railSplit(
          [[`${waiting.length - INBOX} more`, c.faint]],
          [["⌥A", c.faint]],
          () => void this.nextAttention().catch(this.report),
          edge,
        ),
      );
    this.railSession.add(this.box({ height: space.section }));
  },

  /** A question of the inbox: where it waits, its first rows, and the chips that answer it. Another session than the
   * one shown offers to go there. */
  renderInboxItem(one: Waiting, room: number): void {
    const lib = this.options.workspaces;
    const { group, entry, ask } = one;
    const mine = entry === lib.current;
    const label = (entry.session?.labelOf(ask.chain) ?? ask.label).split("/").at(-1) ?? "";
    const where = mine ? [] : [[`${group.name} › ${entry.name}`, c.faint] as Part];
    const edge: BoxOptions = { paddingLeft: LEFT, paddingRight: RIGHT, marginTop: space.section };
    this.railSession.add(
      this.railSplit(
        [
          [`${glyph.asks} `, c.warning],
          [label, c.text, bold],
        ],
        where,
        () => void this.goToAsk(one).catch(this.report),
        edge,
      ),
    );
    for (const text of wrapped(shortenHomes(gist(ask.message)), room - 2, 2)) {
      const row = this.row({ paddingLeft: LEFT + 2 });
      row.add(this.line([[text, c.text]], c.muted, { truncate: true, flexShrink: 1 }));
      this.railSession.add(row);
    }
    const controls = this.row({ gap: 1, paddingLeft: LEFT + 2, flexWrap: "wrap" });
    const chip = (text: string, run: () => void) =>
      controls.add(this.text([[` ${text} `, c.text, 0, c.raised]], c.text, { run }));
    const session = entry.session;
    if (!mine || !session) chip("Go there", () => void this.goToAsk(one).catch(this.report));
    else if (ask.choices.length) {
      for (const [at, value] of ask.choices.entries())
        chip(`${at + 1} ${value}`, () => void session.answer(ask, value).catch(this.report));
      chip(`${ask.choices.length + 1} Other`, () => {
        this.choice = { id: ask.id, at: ask.choices.length, other: true, ticked: [] };
        void session.point(ask.id).catch(this.report);
      });
    } else chip(`Answer as ${ask.shape}`, () => void session.point(ask.id).catch(this.report));
    this.railSession.add(controls);
  },

  /** The switcher of the shape cards: one row for each workspace, the one shown lit, each with what needs a look in it.
   * It shows the rows around the workspace shown when they are many. */
  renderSwitcher(room: number, edge: BoxOptions): void {
    const lib = this.options.workspaces;
    const current = lib.groupOf();
    const heading = this.row({ paddingLeft: LEFT, paddingRight: RIGHT });
    heading.add(this.text("Workspaces", c.text, { attributes: bold, flexGrow: 1 }));
    heading.add(this.text("⌃W", c.faint, { run: () => void this.workspacePicker() }));
    this.railSession.add(heading);
    const groups = lib.groups;
    const at = current ? Math.max(0, groups.indexOf(current)) : 0;
    const from = Math.max(0, Math.min(groups.length - SWITCHER, at - Math.floor(SWITCHER / 2)));
    if (from > 0)
      this.railSession.add(
        this.railSplit([[`↑ ${from} more`, c.faint]], [], () => this.workspacePicker(), edge),
      );
    for (const group of groups.slice(from, from + SWITCHER)) {
      const mine = group === current;
      const attention = this.attentionOf(group.sessions);
      const id = `ws:${group.directory}`;
      this.railOrder.push(id);
      this.railHandlers.set(id, { enter: () => void this.focusWorkspace(group).catch(this.report) });
      const row = this.railSplit(
        [[mine ? glyph.mark : " ", c.accent], [" "], [group.name, mine ? c.text : c.muted, mine ? bold : 0]],
        this.badges(attention),
        () => void this.focusWorkspace(group).catch(this.report),
        edge,
        mine || (this.railKeys && this.railAt === id),
      );
      this.railSession.add(row);
    }
    if (groups.length > from + SWITCHER)
      this.railSession.add(
        this.railSplit(
          [[`↓ ${groups.length - from - SWITCHER} more`, c.faint]],
          [],
          () => this.workspacePicker(),
          edge,
        ),
      );
    this.railSession.add(
      this.railSplit(
        [
          ["+ ", c.faint],
          ["Add a workspace", c.faint],
        ],
        [],
        () => this.insert("/workspace "),
        edge,
      ),
    );
    void room;
    this.railSession.add(this.box({ height: space.section }));
  },

  /** The workspaces and the threads in the scroll of the sidebar. The workspace shown stands open, with its sessions
   * and, under the session shown, its threads, and the others fold to one row with the marks of what needs a look. */
  renderWorkspaces(): void {
    const lib = this.options.workspaces;
    const w = this.session;
    const style = this.railStyle;
    const width = this.rail.width || w.preferences.sidebarWidth;
    const current = lib.groupOf();
    // Only the workspace shown stands open, and the others fold again once the focus moves.
    if (this.railFocus !== current?.directory) {
      this.railFocus = current?.directory;
      for (const key of [...this.unfolded]) if (key.startsWith("ws:")) this.unfolded.delete(key);
    }
    const threads = this.railThreads();
    const active = this.activeThread();
    const cursor = this.railKeys ? this.railAt : undefined;
    const askCount = (entry: SessionEntry) =>
      entry.session ? entry.session.desk.asks.filter((ask) => !ask.notice).length : 0;
    const key = JSON.stringify([
      this.theme,
      width,
      style,
      lib.notice,
      lib.current?.path,
      lib.groups.map((group) => [
        group.name,
        group.directory,
        group.collapsed,
        group.sessions.map((entry) => [
          entry.path,
          entry.name,
          entry.status,
          entry.error,
          entry.archived,
          askCount(entry),
          entry === lib.current ? Math.round((entry.cost ?? 0) * 100) : 0,
        ]),
      ]),
      this.threadsKey(threads),
      active?.act.id,
      [...this.unfolded],
      this.renaming && this.itemKey(this.renaming.item),
      this.menuItem && this.itemKey(this.menuItem),
      cursor,
    ]);
    this.railHeading.visible = style !== "cards";
    if (!this.paneChanged(this.railSpaces, key)) return;
    this.clear(this.railHeading);
    this.clear(this.railSpaces);
    this.unhover();
    if (style !== "cards") {
      this.railHeading.add(this.text("Workspaces", c.text, { attributes: bold, flexGrow: 1 }));
      this.railHeading.add(this.text("⌃W", c.faint, { run: () => void this.workspacePicker() }));
    }
    if (style !== "cards") {
      this.railOrder = [];
      this.railHandlers.clear();
    }
    const order = (id: string, handler: RailHandler) => {
      this.railOrder.push(id);
      this.railHandlers.set(id, handler);
    };
    // The line of the scroll that each row takes, so that the row that the keys or the feed lead to shows.
    const target =
      cursor ?? (active ? `th:${active.act.id}` : lib.current ? `se:${lib.current.path}` : undefined);
    let at = 0;
    let found: number | undefined;
    const step = (lines: number, id?: string, margin = 0) => {
      at += margin;
      if (id && id === target) found = at;
      at += lines;
    };
    const padding: BoxOptions = { paddingLeft: LEFT, paddingRight: RIGHT };
    const plainRow = (parts: Part[], run: () => void, options: BoxOptions = {}, selected = false) =>
      this.railSpaces.add(this.railSplit(parts, [], run, { ...padding, ...options }, selected));
    if (lib.notice) this.railSpaces.add(this.inset(space.between, this.text(lib.notice, c.warning)));
    if (!lib.groups.length)
      this.railSpaces.add(
        this.inset(space.between, this.text("No workspaces yet. Add a project folder below.", c.muted)),
      );

    // The controls of the thread shown, as the controls of the header of its box in the feed: a mark and a word each.
    const links = (thread: ChainThread) => {
      const id = thread.act.id;
      const row = this.row({ gap: 3, flexWrap: "wrap" });
      const link = (mark: string, label: string, run: () => void, tone = c.faint) =>
        row.add(
          this.link(
            [
              [`${mark} `, tone],
              [label, c.muted],
            ],
            run,
          ),
        );
      const folded = this.threadFolded(id);
      link(folded ? glyph.closed : glyph.open, folded ? "Expand" : "Collapse", () => this.foldThread(id));
      link(glyph.reply, "Reply", () => this.replyTo(id));
      if (!thread.resolved) link("✓", "Resolve", () => this.resolveThread(thread));
      const ask = thread.asks[0];
      if (ask) link(glyph.asks, "Answer", () => void w.point(ask).catch(this.report), c.warning);
      return row;
    };
    const controls = (thread: ChainThread, indent: number) => {
      const row = links(thread);
      row.paddingLeft = indent;
      this.railSpaces.add(row);
      step(1);
    };

    // A thread of the session shown, on one row.
    const threadRow = (thread: ChainThread, indent: number) => {
      const id = `th:${thread.act.id}`;
      const lit = active === thread;
      const [mark, tone] = threadMarks(thread);
      order(id, {
        enter: () => this.chooseThread(thread),
        space: () => this.foldThread(thread.act.id),
        thread,
      });
      const title = shortenHomes(thread.title) || thread.act.id;
      const row = this.railSplit(
        [
          [glyph.mark, threadTone(thread.act.id)],
          [" "],
          [`${mark} `, tone],
          [title, lit ? c.text : thread.resolved ? c.faint : c.muted, lit ? bold : 0],
        ],
        this.threadMeta(thread),
        () => this.chooseThread(thread),
        { paddingLeft: indent, paddingRight: RIGHT },
        lit || cursor === id,
      );
      this.railSpaces.add(row);
      step(1, id);
      if (lit) controls(thread, indent + 3);
    };

    // A thread of the session shown, as a card of its own.
    const card = (thread: ChainThread) => {
      const id = `th:${thread.act.id}`;
      const lit = active === thread || cursor === id;
      const [mark, tone] = threadMarks(thread);
      order(id, {
        enter: () => this.chooseThread(thread),
        space: () => this.foldThread(thread.act.id),
        thread,
      });
      const bar: Part = [glyph.mark, threadTone(thread.act.id)];
      const box = this.hoverable(
        // The surface of the card starts at its bar, so no color of the card stands left of it.
        this.box({
          marginLeft: LEFT,
          paddingRight: RIGHT,
          marginTop: space.section,
          backgroundColor: lit ? c.selected : c.panel,
          onMouseUp: this.click(() => this.chooseThread(thread)),
        }),
        lit ? c.selected : c.raised,
      );
      const shown = active === thread;
      const words = wrapped(shortenHomes(thread.title) || thread.act.id, width - LEFT - RIGHT - 4, 2);
      for (const [row, text] of words.entries())
        box.add(
          this.line(
            [
              bar,
              [" "],
              [row ? "  " : `${mark} `, tone],
              [text, shown ? c.text : thread.resolved ? c.faint : c.muted, shown ? bold : 0],
            ],
            c.muted,
            { truncate: true },
          ),
        );
      const state = this.stateWord(thread);
      const chain = thread.act.on === w.engine.root ? "" : (w.labelOf(thread.act.on).split("/").at(-1) ?? "");
      const since = plain(this.threadMeta(thread).filter((one) => one[0] !== state[0]));
      const notes = [
        thread.messages.length ? `${thread.messages.length + 1} msgs` : "",
        since,
        chain ? `in ${chain}` : "",
      ].filter(Boolean);
      box.add(
        this.line(
          [bar, ["   "], state, [notes.length ? `  ·  ${notes.join("  ·  ")}` : "", c.faint]],
          c.muted,
          { truncate: true },
        ),
      );
      // The controls of the thread shown stand in its card, on a last row with its bar.
      if (shown) {
        const row = this.row({});
        row.add(this.line([bar, ["   "]], c.muted));
        row.add(links(thread));
        box.add(row);
      }
      this.railSpaces.add(box);
      step(words.length + 1 + (shown ? 1 : 0), id, space.section);
    };

    // The sessions of a workspace that are worth a row: the one shown and each that lives, then the newest others.
    const sessionsOf = (group: Workspace) => {
      const kept = group.sessions.filter((entry) => !entry.archived);
      const live = (entry: SessionEntry) =>
        lib.current === entry ||
        ["working", "opening", "blocked", "paused", "error", "done"].includes(entry.status);
      const resting = kept.filter((entry) => !live(entry));
      const room = Math.max(0, 3 - (kept.length - resting.length));
      const shown = new Set([...kept.filter(live), ...resting.slice(0, room)]);
      return {
        shown: kept.filter((entry) => shown.has(entry)),
        earlier: kept.filter((entry) => !shown.has(entry)),
      };
    };

    const sessionRow = (group: Workspace, entry: SessionEntry) => {
      const mine = lib.current === entry;
      const id = `se:${entry.path}`;
      order(id, { enter: () => void lib.select(entry).catch(this.report) });
      const checkout = checkoutOf(entry.path, group.directory);
      const label = checkout === group.directory ? entry.name : `${entry.name}  ${basename(checkout)}`;
      const asks = askCount(entry);
      const tail: Part[] = mine
        ? [[dollars(entry.cost ?? 0), c.faint]]
        : asks
          ? [[`${glyph.asks} ${asks}`, c.warning, bold]]
          : [];
      this.itemRow(
        this.railSpaces,
        entry,
        {
          lead: [[" "], [mine ? glyph.mark : " ", c.accent], [" "]],
          state: statusMark(entry.status),
          name: [label, mine ? c.text : entry.archived ? c.faint : c.muted, mine ? bold : 0],
          tail,
        },
        () => [
          statusMark(entry.status),
          [statusLabels[entry.status], c.text, bold],
          [entry.error ? `  ${entry.error}` : "", c.danger],
        ],
        () => void lib.select(entry).catch(this.report),
        {},
        cursor === id,
      );
      step(1, id);
    };

    // The folds of a workspace that hold its older and its archived sessions.
    const folds = (group: Workspace, earlier: SessionEntry[]) => {
      if (earlier.length) {
        this.railSpaces.add(
          this.foldRow(`${group.directory}\nearlier`, "  Earlier", earlier.length, padding),
        );
        step(1);
        if (this.unfolded.has(`${group.directory}\nearlier`))
          for (const entry of earlier) sessionRow(group, entry);
      }
      const id = `new:${group.directory}`;
      order(id, { enter: () => void lib.create(group).catch(this.report) });
      plainRow(
        [["   "], ["+ ", c.faint], ["New session", c.faint]],
        () => void lib.create(group).catch(this.report),
        {},
        cursor === id,
      );
      step(1, id);
      const archived = group.sessions.filter((entry) => entry.archived);
      if (archived.length) {
        this.railSpaces.add(this.foldRow(group.directory, "  Archived", archived.length, padding));
        step(1);
        if (this.unfolded.has(group.directory)) for (const entry of archived) sessionRow(group, entry);
      }
    };

    // The threads that a session holds: those that live, and the resolved ones folded under a row of their own.
    const threadBlock = (entry: SessionEntry) => {
      if (!threads.length) {
        plainRow([["     No thread yet", c.faint]], () => this.composer.focus());
        step(1);
        return;
      }
      const shown = threads.filter((one) => !one.resolved || one === active);
      const done = threads.filter((one) => !shown.includes(one));
      for (const one of shown) threadRow(one, LEFT + 4);
      if (!done.length) return;
      const fold = `resolved:${entry.path}`;
      this.railSpaces.add(this.foldRow(fold, "      Resolved", done.length, padding));
      step(1);
      if (this.unfolded.has(fold)) for (const one of done) threadRow(one, LEFT + 4);
    };

    if (style === "cards") {
      const group = current;
      if (group) {
        const { shown, earlier } = sessionsOf(group);
        // The switcher on top holds the workspace shown, so its folder stands here without a second row for its name.
        this.railSpaces.add(
          this.inset(
            LEFT + 2,
            this.line(clip(shortenHome(group.directory), width - LEFT - RIGHT - 3, "end"), c.faint, {
              truncate: true,
            }),
          ),
        );
        step(1);
        for (const entry of shown) sessionRow(group, entry);
        folds(group, earlier);
        const open = threads.filter((one) => !one.resolved || one === active);
        const done = threads.filter((one) => !open.includes(one));
        const heading = this.row({ paddingLeft: LEFT, paddingRight: RIGHT, marginTop: space.section });
        heading.add(this.text("Threads", c.text, { attributes: bold, flexGrow: 1 }));
        heading.add(this.text(this.badges(this.threadAttention(threads)), c.muted));
        this.railSpaces.add(heading);
        step(1, undefined, space.section);
        if (!threads.length) {
          plainRow([["No thread yet", c.faint]], () => this.composer.focus());
          step(1);
        }
        for (const one of open) card(one);
        if (done.length) {
          const fold = `resolved:${lib.current?.path}`;
          this.railSpaces.add(
            this.foldRow(fold, " Resolved", done.length, { ...padding, marginTop: space.section }),
          );
          step(1, undefined, space.section);
          if (this.unfolded.has(fold)) for (const one of done) card(one);
        }
      }
    } else {
      for (const [index, group] of lib.groups.entries()) {
        const mine = group === current;
        // The workspace shown folds to its row when the operator asks, and the others open to peek.
        const open = mine ? !group.collapsed : this.unfolded.has(`ws:${group.directory}`);
        const id = `ws:${group.directory}`;
        const attention = this.attentionOf(group.sessions);
        const status = lib.groupStatus(group);
        const focus = mine
          ? () => lib.toggle(group)
          : () => void this.focusWorkspace(group).catch(this.report);
        order(id, {
          enter: focus,
          right: mine
            ? () => {
                if (!open) lib.toggle(group);
              }
            : () => (open ? focus() : this.peekWorkspace(group)),
          left: mine
            ? () => (open ? lib.toggle(group) : this.railSteer(false))
            : () => (open ? this.peekWorkspace(group) : this.railSteer(false)),
          space: mine ? () => lib.toggle(group) : () => this.peekWorkspace(group),
        });
        const margin = index ? space.section : 0;
        this.itemRow(
          this.railSpaces,
          group,
          {
            lead: [
              [mine ? glyph.mark : " ", c.accent],
              [`${open ? glyph.open : glyph.closed} `, c.faint],
            ],
            name: [group.name, mine ? c.text : c.muted, bold],
            after: true,
            tail: this.badges(attention),
            tailTip: () => this.badgeTip(attention),
            ...(mine ? {} : { peek: () => this.peekWorkspace(group) }),
          },
          () => [statusMark(status), [statusLabels[status], c.text, bold]],
          focus,
          { marginTop: margin },
          cursor === id,
        );
        step(1, id, margin);
        if (!open) continue;
        const { shown, earlier } = sessionsOf(group);
        for (const entry of shown) {
          sessionRow(group, entry);
          if (entry === lib.current) threadBlock(entry);
        }
        folds(group, earlier);
      }
    }
    // The shape cards holds the row that adds a workspace in its switcher, and the others hold it here.
    if (style !== "cards") {
      plainRow([[" "], ["+ ", c.faint], ["Add a workspace", c.faint]], () => this.insert("/workspace "), {
        marginTop: space.section,
      });
      step(1, undefined, space.section);
    }
    if (found !== undefined && target !== this.railWatched) {
      const line = found;
      this.railWatched = target;
      setTimeout(() => {
        if (this.closed) return;
        const box = this.railSpaces;
        const view = box.viewport.height;
        if (line < box.scrollTop) box.scrollTo(Math.max(0, line - 1));
        else if (line + 2 > box.scrollTop + view) box.scrollTo(line + 2 - view);
      }, 0);
    }
  },

  /** What the threads of the session shown say, counted as the marks of a row. */
  threadAttention(threads: readonly ChainThread[]): Attention {
    return {
      asks: threads.filter((one) => one.state === "asks").length,
      working: threads.filter((one) => !one.resolved && one.state !== "asks").length,
      held: 0,
      failed: threads.filter((one) => one.state === "failed").length,
      unread: 0,
    };
  },

  /** The row that the keys of the sidebar stand on. */
  railHandler(): RailHandler | undefined {
    return this.railAt ? this.railHandlers.get(this.railAt) : undefined;
  },

  /** The thread under the keys of the sidebar. */
  railThread(): ChainThread | undefined {
    return this.railHandler()?.thread;
  },

  /** The keys move in the sidebar, or go back to the input. */
  railSteer(want?: boolean): void {
    const on = want ?? !this.railKeys;
    this.railKeys = on;
    if (on) {
      if (this.steering) this.steer(false);
      this.composer.blur();
      if (!this.railAt || !this.railHandlers.has(this.railAt)) {
        const active = this.activeThread();
        this.railAt = active ? `th:${active.act.id}` : `se:${this.options.workspaces.current?.path}`;
      }
    } else this.composer.focus();
    this.render();
  },

  /** A step of the keys of the sidebar through the rows that it shows. */
  moveRail(by: number): void {
    const ids = this.railOrder;
    if (!ids.length) return;
    const at = ids.indexOf(this.railAt ?? "");
    const next = Math.max(0, Math.min(ids.length - 1, at < 0 ? 0 : at + by));
    this.railAt = ids[next];
    this.render();
  },
});

export type RailPart = typeof rail;

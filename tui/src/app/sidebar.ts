import type { BoxOptions, BoxRenderable, Renderable, TextRenderable } from "@opentui/core";
import { unmarked } from "furb";
import { type ChainThread, chainThreads } from "furb/life";
import { count, dollars } from "../format.ts";
import type { ActRow } from "../session.ts";
import { theme as c, glyph, spacing as space } from "../theme.ts";
import { bold, type Part, plain } from "../ui.ts";
import { part } from "./part.ts";
import type { ThreadList } from "./shared.ts";

export const sidebar = part({
  /** The context of the chain shown, at the foot of the sidebar: how full its window is and where a grant pauses it,
   * its tokens, and what it spent. */
  renderRailUsage(): void {
    const w = this.session;
    const window = w.filled;
    const grant = w.open.find((act) => act.kind === "grant");
    const width = this.rail.width || w.preferences.sidebarWidth;
    if (
      !this.paneChanged(this.railUsage, [this.theme, w.spend, window, width, w.context, grant?.words, w.demo])
    )
      return;
    this.clear(this.railUsage);
    const inner = width - space.inset * 2;
    const spend = w.spend;
    const ceiling =
      grant?.words[1] === null || grant?.words[1] === undefined
        ? undefined
        : Number(unmarked(grant.words[1]));
    const known = window !== undefined && Number.isFinite(window);
    const tokens = spend.input + spend.output + spend.cacheRead + spend.cacheWrite;
    // A chain with a source reads the prompt of its origin before it answers once, so its context shows from its birth.
    this.railUsage.visible = tokens > 0 || spend.dollars > 0 || Boolean(grant) || w.context !== undefined;
    if (!this.railUsage.visible) return;
    // One row holds all of it: the name, a meter of the share of the window that the last answer used, the share and
    // what the chain spent against its ceiling. A hover over the row gives each number, and where a grant pauses.
    const prompt = w.context;
    const part = known ? (window ?? 0) : 0;
    const share = `${Number((part * 100).toFixed(1))}%`;
    const size = known && prompt !== undefined && window ? count(Math.round(prompt / window)) : "";
    const limit = grant?.words[0];
    const spent =
      limit === null || limit === undefined
        ? dollars(spend.dollars)
        : `${dollars(spend.dollars)}/${dollars(Number(unmarked(limit)))}`;
    const shares: Part[] = known
      ? [[size ? `${share} of ${size}` : share, c.text]]
      : [[prompt === undefined ? "No answer yet" : count(prompt), prompt === undefined ? c.faint : c.text]];
    const name = "Context ";
    // The share loses its size before the meter loses its room, and the meter goes before the spent text is cut.
    const forms = known && size ? [`${share} of ${size}`, share] : [shares[0]?.[0] ?? ""];
    const meterRoom = (said: string) => inner - Bun.stringWidth(name + said + spent) - 4;
    const said = forms.find((one) => meterRoom(one) >= 6) ?? forms.at(-1) ?? "";
    // A window that no answer gave is unknown, so no meter stands for it: an empty meter would say that nothing is used.
    const cells = known ? Math.max(0, Math.min(16, meterRoom(said))) : 0;
    const used = Math.min(cells, Math.max(part > 0 ? 1 : 0, Math.round(part * cells)));
    const mark = ceiling === undefined ? -1 : Math.min(cells - 1, Math.round(ceiling * cells));
    const tone = part >= 0.9 ? c.danger : part >= 0.7 ? c.warning : c.accent;
    const meter: Part[] = [];
    for (let at = 0; at < cells; at++)
      meter.push(at === mark ? ["╋", c.warning] : [glyph.meter, at < used ? tone : c.border]);
    const row: Part[] = [
      ["Context", c.text, bold],
      [" "],
      ...(cells ? [...meter, [" "] as Part] : []),
      [said, shares[0]?.[1] ?? c.text],
    ];
    const gap = Math.max(1, inner - Bun.stringWidth(plain(row) + spent));
    const line = this.line([...row, [" ".repeat(gap)], [spent, c.text]], c.muted, { truncate: true });
    this.railUsage.add(line);
    this.tipped(line, [
      [known ? `${share} of the context window` : "No answer yet", c.text, bold],
      [ceiling === undefined ? "" : `, pauses at ${Number((ceiling * 100).toFixed(1))}%`, c.warning],
      [`  prompt ${prompt === undefined ? "none" : count(prompt)}`, c.muted],
      [`  fresh input ${count(spend.input)}`, c.muted],
      [`  output ${count(spend.output)}`, c.muted],
      [`  cache read ${count(spend.cacheRead)}`, c.muted],
      [`  cache write ${count(spend.cacheWrite)}`, c.muted],
      [w.demo ? "  simulated" : "", c.faint],
    ]);
  },

  /** The threads of the chain shown, and each by its id. */
  chainThreadList(chain: string): ChainThread[] {
    return this.chainThreadsBy(chain).list;
  },

  chainThreadsBy(chain: string): ThreadList {
    const w = this.session;
    if (w.selected !== chain)
      return {
        session: w,
        chain,
        acts: undefined,
        owed: "",
        list: [],
        by: new Map(),
        of: new Map(),
        asked: new Map(),
      };
    const owed = [...w.host.prompts.keys()].join(" ");
    // A list read from another session or another chain is no list of this one, even when its version is the same.
    const kept = this.threadList;
    if (
      kept.session !== w ||
      kept.chain !== chain ||
      kept.acts !== w.threadStamp(chain) ||
      kept.owed !== owed
    ) {
      const index = w.ladderIndex;
      const list = chainThreads(index.on, index.ids, new Set(w.host.prompts.keys()), this.promptsRead);
      this.threadList = {
        session: w,
        chain,
        acts: w.threadStamp(chain),
        owed,
        list,
        by: new Map(list.map((one) => [one.act.id, one])),
        of: new Map(
          list.flatMap((one) => one.messages.map((message): [string, ChainThread] => [message.id, one])),
        ),
        asked: new Map(list.flatMap((one) => one.asked.map((ask): [string, ChainThread] => [ask.id, one]))),
      };
    }
    return this.threadList;
  },

  /** Whether a thread stands folded to its header in the feed: as the operator left it, and else when it is resolved,
   * is not the last thread of its chain, and the preference folds what is over. */
  threadFolded(id: string): boolean {
    const w = this.session;
    const set = w.folds[`thread:${id}`];
    if (set !== undefined) return set;
    const own = this.chainThreadsBy(w.selected);
    const thread = own.by.get(id);
    return Boolean(w.preferences.foldThreads && thread?.resolved && own.list.at(-1) !== thread);
  },

  /** A thread folds in the feed to its header line alone, or unfolds. */
  foldThread(id: string, folded?: boolean): void {
    const fold = folded ?? !this.threadFolded(id);
    const w = this.session;
    w.folds[`thread:${id}`] = fold;
    w.save();
    this.renderContent();
    this.render();
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

  /** The controls of a thread on the header of the thread in the feed: collapse it, and resolve it or open it again.
   * A message of a thread and a prompt that heads no thread have none. */
  threadControls(act: ActRow): BoxRenderable | undefined {
    const thread = this.chainThreadsBy(this.session.selected).by.get(act.id);
    if (!thread) return undefined;
    const id = thread.act.id;
    const folded = this.threadFolded(id);
    const row = this.row({ gap: 3, paddingLeft: space.between });
    row.add(
      this.link(
        [
          [`${folded ? glyph.closed : glyph.open} `, c.faint],
          [folded ? "Expand" : "Collapse", c.faint],
        ],
        () => this.foldThread(id, !folded),
      ),
    );
    if (!thread.resolved)
      row.add(
        this.link(
          [
            [`${glyph.done} `, c.faint],
            ["Resolve", c.faint],
          ],
          () => this.resolveThread(thread),
        ),
      );
    return row;
  },

  /** The columns that the controls of a thread take: the words of its links, the gaps between them, and its margin. */
  threadControlsWidth(controls: BoxRenderable): number {
    const links = controls.getChildren().map((one) => Bun.stringWidth((one as TextRenderable).plainText));
    return links.reduce((sum, one) => sum + one, 0) + 3 * Math.max(0, links.length - 1) + space.between;
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

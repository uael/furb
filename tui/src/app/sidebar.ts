import type { BoxOptions, BoxRenderable, Renderable, RGBA } from "@opentui/core";
import { type Ask, type ChainThread, chainThreads, doingOf, gist, nowOf } from "furb/life";
import { shortenHomes } from "../files.ts";
import { clip, count, dollars } from "../format.ts";
import type { ActRow } from "../session.ts";
import { theme as c, glyph, spacing as space, threadTone } from "../theme.ts";
import { bold, type Part, plain } from "../ui.ts";
import { part } from "./part.ts";
import { chainMarks, type TextShape, type ThreadList, threadMarks, wrapped } from "./shared.ts";

export const sidebar = part({
  renderRail(): void {
    if (!this.rail.visible) return;
    this.renderRailSession();
    this.renderWorkspaces();
    this.renderRailUsage();
  },

  /** The sidebar of the session: each question that waits for the operator, which a click answers in place, then the
   * threads of the life as a tree of their chains, each with what it does now, and the finished ones folded at the
   * foot. */
  renderRailSession(): void {
    const w = this.session;
    // The layout gives the sidebar its width, which the preferences only ask for.
    const width = this.rail.width || w.preferences.sidebarWidth;
    const inner = width - space.between * 2;
    if (this.railChain && w.chains.some((chain) => chain.id === this.railChain)) {
      this.renderThreadList(this.railChain, width, inner);
      return;
    }
    this.railChain = undefined;
    const loom = this.lifeLoom();
    // A run of compactions into new chains stands as one row, which the last chain of the run heads.
    const rows = loom.rows.filter((row) => row.id !== "operator" && w.listed(row.id));
    const asks = w.desk.asks.filter((ask) => !ask.notice);
    const find = (id: string) => w.actOf(id);
    const remember = (key: string, make: () => string): string => {
      const known = this.doing.get(key);
      if (known !== undefined) return known;
      if (this.doing.size > 4000) this.doing.clear();
      const made = make();
      this.doing.set(key, made);
      return made;
    };
    const label = (chain: string) => w.labelOf(chain).split("/").at(-1) ?? "";
    // A thread that works says what its newest act does and since when, in minutes, so that a row changes at most once
    // a minute while nothing needs the operator.
    const said = (row: (typeof rows)[number]): [string, RGBA, string] => {
      if (row.state === "asks") return ["asks you", c.warning, ""];
      if (row.state === "paused") return ["paused", c.warning, ""];
      // A thread that ended says what it came to: the first line of its answer, or why it failed.
      if (row.state === "failed" || row.state === "done") {
        const text = remember(`${row.state} ${row.id} ${row.measure?.answers ?? 0}`, () => {
          return nowOf(row, loom, w.acts, (rung) => w.program[rung.id] ?? "").text;
        });
        return row.state === "failed"
          ? [`failed${text ? `: ${text}` : ""}`, c.danger, ""]
          : [text || "done", c.faint, ""];
      }
      if (row.state !== "working") return ["", c.faint, ""];
      if (!row.waits) return ["writes a step", c.muted, ""];
      const waits = row.waits;
      const text = remember(`act ${waits}`, () => {
        const act = find(waits);
        return act ? doingOf(act, find, label) : "writes a step";
      });
      const began = w.started[waits];
      const minutes = began === undefined ? 0 : Math.floor((Date.now() - began) / 60000);
      const since = minutes < 1 ? "" : minutes < 60 ? `${minutes}m` : `${Math.floor(minutes / 60)}h`;
      return [text, minutes >= 15 ? c.warning : c.muted, since];
    };
    // A thread stands in the tree while it or a thread under it needs a look: it works, asks, failed or is paused. The
    // root and the thread shown stand always, and the others fold under Finished at the foot.
    const looks = (row: (typeof rows)[number]) => !["done", "idle"].includes(row.state);
    const shownRow = w.runOf(w.selected).at(-1) ?? w.selected;
    // A run stands at the place of its first chain, which the tree does not list.
    const loomed = new Map(loom.rows.map((one) => [one.id, one]));
    const placed = (row: (typeof rows)[number]) => loomed.get(w.runOf(row.id)[0] ?? row.id) ?? row;
    const depth = (row: (typeof rows)[number]) => placed(row).lines.length / 2;
    const standing = rows.filter((row, at) => {
      if (row.id === w.engine.root || row.id === shownRow || looks(row)) return true;
      for (const below of rows.slice(at + 1)) {
        if (depth(below) <= depth(row)) return false;
        if (looks(below)) return true;
      }
      return false;
    });
    const resting = rows.filter((row) => !standing.includes(row));
    const lines = standing.map((row) => ({ row, said: said(row) }));
    this.railRows = [...standing, ...(this.unfolded.has("finished") ? resting.slice(0, 12) : [])].map(
      (row) => row.id,
    );
    const key = JSON.stringify([
      this.theme,
      width,
      w.selected,
      w.host.compactions.length,
      asks.map((ask) => [ask.id, ask.label, ask.message, ask.choices]),
      lines.map(({ row, said: [text, , since] }) => [row.id, row.label, row.state, row.lines, text, since]),
      resting.map((row) => [row.id, row.label]),
      this.unfolded.has("finished"),
      this.choice,
      this.railKeys && this.railAt,
    ]);
    if (key === this.railKey) return;
    this.railKey = key;
    this.clear(this.railSession);
    const add = (parts: Part[], options: TextShape = {}) => {
      const node = this.line(parts, c.muted, { truncate: true, ...options });
      this.railSession.add(node);
      return node;
    };
    const heading = (name: string, count: Part[]) => {
      const room = Math.max(1, inner - Bun.stringWidth(name + plain(count)));
      add([[name, c.text, bold], [" ".repeat(room)], ...count]);
    };
    this.renderNeeds(asks, inner);
    const working = rows.filter((row) => row.state === "working").length;
    heading("Chains", [
      [working ? `${glyph.running} ${working}` : "", c.accent],
      [working && resting.length ? "  " : ""],
      [resting.length ? `${glyph.done} ${resting.length}` : "", c.faint],
    ]);
    // A row of a thread spans the sidebar past its padding, with the tone of its thread as a bar at its left edge. The
    // threads of the first level stand with no guide, and the lines of the tree show from the second level.
    const edge = { marginX: -space.between, paddingRight: space.inset };
    const named = [...standing, ...(this.unfolded.has("finished") ? resting.slice(0, 12) : [])];
    const name = Math.min(
      18,
      Math.floor(width * 0.38),
      Math.max(8, ...named.map((row) => Bun.stringWidth(label(row.id)) + 2)),
    );
    const threadRow = (row: (typeof rows)[number], [said, color, since]: [string, RGBA, string]) => {
      let text = said;
      const selected = row.id === shownRow;
      const [mark, tone] = chainMarks[row.state];
      const guide = depth(row) ? `  ${placed(row).lines.slice(2)}` : "";
      // A chain that compacted says how many times, in place or into a new chain of its run.
      const chains = w.runOf(row.id);
      const cuts = w.host.compactions.filter((one) => chains.includes(one.chain)).length;
      if (cuts) text = `${cuts > 1 ? `compacted ${cuts}×` : "compacted"}  ${text}`;
      const title = clip(label(row.id), Math.max(4, name - Bun.stringWidth(guide)));
      // The row spans the sidebar, so its room is the width of the sidebar less its bar, its padding at the right, and
      // the parts before and after the words.
      const room = Math.max(
        0,
        width - 1 - Bun.stringWidth(guide) - 2 - name - 1 - (since ? since.length + 1 : 0) - space.inset,
      );
      this.railSession.add(
        this.railRow(
          [
            [glyph.mark, threadTone(row.id)],
            [guide, c.faint],
            [`${mark()} `, tone()],
            [
              title.padEnd(Math.max(4, name - Bun.stringWidth(guide))),
              selected ? c.text : c.muted,
              selected ? bold : 0,
            ],
            [" "],
            [clip(text, room).padEnd(room), color],
            [since ? ` ${since}` : "", c.faint],
          ],
          () => this.openThreads(row.id),
          edge,
          selected || this.cursorOn(row.id),
        ),
      );
    };
    for (const { row, said: told } of lines) threadRow(row, told);
    if (resting.length) {
      this.railSession.add(
        this.foldRow("finished", " Finished", resting.length, { ...edge, paddingLeft: 1 }),
      );
      if (this.unfolded.has("finished")) for (const row of resting.slice(0, 12)) threadRow(row, said(row));
    }
  },

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
    const inner = width - space.between * 2;
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
    const row = (name: string, value: string, color = c.text) => spread([name, c.muted], "", [value, color]);
    const section = (name: string, value = "", note = "") =>
      spread([name, c.text, bold], note, [value, c.faint]);
    const spend = w.spend;
    const ceiling =
      grant?.words[1] === null || grant?.words[1] === undefined ? undefined : Number(grant.words[1]);
    const known = window !== undefined && Number.isFinite(window);
    const tokens = spend.input + spend.output + spend.cacheRead + spend.cacheWrite;
    // A chain with a source reads the prompt of its origin before it answers once, so its context shows from its birth.
    this.railUsage.visible = tokens > 0 || spend.dollars > 0 || Boolean(grant) || w.context !== undefined;
    if (this.railUsage.visible) {
      section(
        "Context",
        known ? `${Number(((window ?? 0) * 100).toFixed(1))}%` : "",
        w.demo ? "  simulated" : "",
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
      // The last prompt is the whole prompt of the last answer, its system prompt and the reads of the cache included,
      // which the share of the window measures. The fresh input, the output and the cache are what every answer of the
      // chain read past the cache, wrote, and read and wrote in the cache, each token counted once.
      const prompt = w.context;
      if (prompt !== undefined)
        this.tipped(row("Last prompt", count(prompt)), [
          ["The whole prompt of the last answer, its system prompt and the cache included.", c.text],
        ]);
      if (tokens > 0) {
        this.tipped(row("Fresh input", count(spend.input)), [
          ["The input of the answers of the chain that the cache did not hold.", c.text],
        ]);
        row("Output", count(spend.output));
        if (spend.cacheRead) row("Cache read", count(spend.cacheRead));
        if (spend.cacheWrite) row("Cache write", count(spend.cacheWrite));
      }
      row("Spent", dollars(spend.dollars));
      if (grant && grant.words[0] !== null) row("Ceiling", dollars(Number(grant.words[0])), c.muted);
    }
  },

  /** Each question that waits for the operator, on top of the sidebar over the tree and over the list of threads, which
   * a click answers in place: a question never hides. */
  renderNeeds(asks: readonly Ask[], inner: number): void {
    const w = this.session;
    const add = (parts: Part[], options: TextShape = {}) => {
      const node = this.line(parts, c.muted, { truncate: true, ...options });
      this.railSession.add(node);
      return node;
    };
    const heading = (name: string, count: Part[]) => {
      const room = Math.max(1, inner - Bun.stringWidth(name + plain(count)));
      add([[name, c.text, bold], [" ".repeat(room)], ...count]);
    };
    const label = (chain: string) => w.labelOf(chain).split("/").at(-1) ?? "";
    if (asks.length) {
      heading("Needs you", [[`${asks.length}`, c.warning, bold]]);
      for (const ask of asks) {
        const parent = w.labelOf(ask.chain).split("/").slice(0, -1).join("/");
        add(
          [
            [`${glyph.asks} `, c.warning],
            [label(ask.chain), c.text, bold],
            [parent ? `  in ${parent}` : "", c.faint],
          ],
          { marginTop: space.section, run: () => void w.point(ask.id).catch(this.report) },
        );
        for (const text of wrapped(shortenHomes(gist(ask.message)), inner - 2, 2))
          add([["  "], [text, c.text]], { run: () => this.question(ask) });
        const controls = this.row({ gap: 1, paddingLeft: 2, flexWrap: "wrap" });
        const chip = (text: string, run: () => void) =>
          controls.add(this.text([[` ${text} `, c.text, 0, c.raised]], c.text, { run }));
        if (ask.choices.length) {
          for (const [at, value] of ask.choices.entries())
            chip(`${at + 1} ${value}`, () => void w.answer(ask, value).catch(this.report));
          chip(`${ask.choices.length + 1} Other`, () => {
            this.choice = { id: ask.id, at: ask.choices.length, other: true, ticked: [] };
            void w.point(ask.id).catch(this.report);
          });
        } else chip(`Answer as ${ask.shape}`, () => void w.point(ask.id).catch(this.report));
        this.railSession.add(controls);
      }
      add([], { marginTop: space.section });
    }
  },

  /** The threads of the chain shown, and each by its id. */
  chainThreadList(chain: string): ChainThread[] {
    return this.chainThreadsBy(chain).list;
  },

  chainThreadsBy(chain: string): ThreadList {
    const w = this.session;
    if (w.selected !== chain) return { acts: undefined, owed: "", list: [], by: new Map(), of: new Map() };
    const owed = [...w.host.prompts.keys()].join(" ");
    if (this.threadList.acts !== w.version || this.threadList.owed !== owed) {
      const index = w.ladderIndex;
      const list = chainThreads(index.on, index.ids, new Set(w.host.prompts.keys()), this.promptsRead);
      this.threadList = {
        acts: w.version,
        owed,
        list,
        by: new Map(list.map((one) => [one.act.id, one])),
        of: new Map(
          list.flatMap((one) => one.messages.map((message): [string, ChainThread] => [message.id, one])),
        ),
      };
    }
    return this.threadList;
  },

  /** How many threads the list of a chain shows at once. */
  get railRoom(): number {
    return Math.max(5, Math.floor(this.renderer.height / 2) - 5);
  },

  /** Whether the keys of the sidebar stand on a row. */
  cursorOn(id: string): boolean {
    return this.railKeys && this.railAt === id;
  },

  /** The threads of a chain in the sidebar, in place of the tree of the chains: a row that goes back to the tree, then
   * a row for each prompt of the chain, in the color of its thread, with its state and the first line of its message.
   * The thread chosen, or under the keys, shows its controls under its row. The list scrolls by the wheel and by the
   * keys. */
  renderThreadList(chain: string, width: number, inner: number): void {
    const w = this.session;
    const list = this.chainThreadList(chain);
    const room = this.railRoom;
    this.railFrom = Math.max(0, Math.min(Math.max(0, list.length - room), this.railFrom));
    const from = this.railFrom;
    const questions = w.desk.asks.filter((ask) => !ask.notice);
    const key = JSON.stringify([
      "threads",
      questions.map((ask) => [ask.id, ask.label, ask.message, ask.choices]),
      this.choice,
      this.theme,
      width,
      chain,
      w.labelOf(chain),
      w.thread,
      this.railKeys && this.railAt,
      from,
      room,
      list.map((one) => [one.act.id, one.state, one.resolved, one.title, this.threadFolded(one.act.id)]),
    ]);
    if (key === this.railKey) return;
    this.railKey = key;
    this.clear(this.railSession);
    this.railSession.onMouseScroll = (event) => {
      const direction = event.scroll?.direction;
      if (!this.railChain || (direction !== "up" && direction !== "down")) return;
      this.railFrom = Math.max(0, this.railFrom + (direction === "up" ? -3 : 3));
      this.render();
    };
    this.renderNeeds(questions, inner);
    const edge = { marginX: -space.between, paddingRight: space.inset };
    const line = (parts: Part[], run?: () => void) =>
      this.railSession.add(this.line(parts, c.muted, { truncate: true, ...(run ? { run } : {}) }));
    this.railSession.add(
      this.railRow(
        [
          ["← ", c.accent],
          ["Chains", c.muted],
        ],
        () => this.closeThreads(),
        { ...edge, paddingLeft: 1 },
      ),
    );
    let asks = 0;
    let running = 0;
    let resolved = 0;
    for (const one of list)
      if (one.state === "asks") asks++;
      else if (one.resolved) resolved++;
      else running++;
    const tally: Part[] = [
      [asks ? `${glyph.asks} ${asks}  ` : "", c.warning],
      [running ? `${glyph.running} ${running}  ` : "", c.accent],
      [resolved ? `${glyph.done} ${resolved}` : "", c.faint],
    ];
    const name = clip(w.labelOf(chain), Math.max(4, inner - Bun.stringWidth(plain(tally)) - 1));
    line([
      [name, c.text, bold],
      [" ".repeat(Math.max(1, inner - Bun.stringWidth(name + plain(tally))))],
      ...tally,
    ]);
    if (!list.length) line([["No thread on this chain yet.", c.faint]]);
    if (from > 0)
      line([[`↑ ${from} earlier`, c.faint]], () => {
        this.railFrom = Math.max(0, from - room);
        this.render();
      });
    for (const one of list.slice(from, from + room)) {
      const id = one.act.id;
      const chosen = w.thread === id;
      const folded = this.threadFolded(id);
      const [mark, tone] = threadMarks(one);
      const text = clip(shortenHomes(one.title) || id, Math.max(4, width - 6 - space.inset));
      this.railSession.add(
        this.railRow(
          [
            [glyph.mark, threadTone(id)],
            [`${mark} `, tone],
            [folded ? `${glyph.closed} ` : "", c.faint],
            [text, chosen ? c.text : one.resolved ? c.faint : c.muted, chosen ? bold : 0],
          ],
          () => this.chooseThread(id),
          edge,
          chosen || this.cursorOn(id),
        ),
      );
      if (!chosen && !this.cursorOn(id)) continue;
      const controls = this.row({ gap: 1, paddingLeft: 3, flexWrap: "wrap" });
      const chip = (label: string, run: () => void) =>
        controls.add(this.text([[` ${label} `, c.text, 0, c.raised]], c.text, { run }));
      chip(folded ? "Expand" : "Collapse", () => this.foldThread(id));
      if (one.resolved) chip("Open again", () => this.reopenThread(id));
      else chip("Resolve", () => this.resolveThread(one));
      const ask = one.asks[0];
      if (ask) chip("Answer", () => void w.point(ask).catch(this.report));
      this.railSession.add(controls);
    }
    const after = list.length - from - room;
    if (after > 0)
      line([[`↓ ${after} later`, c.faint]], () => {
        this.railFrom = from + room;
        this.render();
      });
  },

  /** A chain chosen in the tree: the feed shows it, and the sidebar lists its threads in place of the tree. */
  openThreads(chain: string): void {
    this.railChain = chain;
    this.railFrom = Number.MAX_SAFE_INTEGER;
    void this.session.select(chain).catch(this.report);
    const list = this.chainThreadList(chain);
    this.railAt = (list.findLast((one) => !one.resolved) ?? list.at(-1))?.act.id;
    this.render();
  },

  /** The tree of the chains again, in place of the list of the threads of a chain, with the keys on that chain. */
  closeThreads(): void {
    if (this.railChain) this.railAt = this.railChain;
    this.railChain = undefined;
    this.session.thread = undefined;
    this.render();
  },

  /** A thread chosen in the list: the feed unfolds it and scrolls to it, and the input sends each message to it. */
  chooseThread(id: string): void {
    const w = this.session;
    this.railAt = id;
    w.thread = id;
    if (this.threadFolded(id)) {
      w.folds[`thread:${id}`] = false;
      w.save();
    }
    if (this.railKeys) this.railSteer(false);
    this.go("feed", id);
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

  /** A thread opens again: the input takes the message that goes to it. */
  reopenThread(id: string): void {
    const w = this.session;
    if (w.host.reading) return;
    w.thread = id;
    this.railAt = id;
    w.notice = `Your next message goes to ${id} and opens it again.`;
    if (this.railKeys) this.railSteer(false);
    else this.composer.focus();
    this.render();
  },

  /** The keys move in the sidebar, or go back to the input. */
  railSteer(want?: boolean): void {
    const on = want ?? !this.railKeys;
    this.railKeys = on;
    if (on) {
      if (this.steering) this.steer(false);
      this.composer.blur();
      if (!this.railAt)
        this.railAt = this.railChain
          ? (this.session.thread ?? this.chainThreadList(this.railChain).at(-1)?.act.id)
          : this.session.selected;
    } else this.composer.focus();
    this.render();
  },

  /** A step of the keys of the sidebar, in the tree or in the list, which scrolls the list so that the row shows. */
  moveRail(by: number): void {
    const ids = this.railChain
      ? this.chainThreadList(this.railChain).map((one) => one.act.id)
      : this.railRows;
    if (!ids.length) return;
    const at = ids.indexOf(this.railAt ?? "");
    const next = Math.max(0, Math.min(ids.length - 1, at < 0 ? ids.length - 1 : at + by));
    this.railAt = ids[next];
    if (this.railChain) {
      if (next < this.railFrom) this.railFrom = next;
      else if (next >= this.railFrom + this.railRoom) this.railFrom = next - this.railRoom + 1;
    }
    this.render();
  },

  /** Enter or → in the sidebar: a chain lists its threads, and a thread is chosen. */
  enterRail(): void {
    const at = this.railAt;
    if (!at) return;
    if (this.railChain) this.chooseThread(at);
    else this.openThreads(at);
  },

  /** ← or Esc in the sidebar: the list goes back to the tree, and the tree gives the keys back to the input. */
  leaveRail(): void {
    if (this.railChain) this.closeThreads();
    else this.railSteer(false);
  },

  /** The thread under the keys of the list. */
  railThread(): ChainThread | undefined {
    return this.railChain ? this.chainThreadsBy(this.railChain).by.get(this.railAt ?? "") : undefined;
  },

  /** The controls of a thread on the header of the thread in the feed: collapse it, and resolve it or open it again.
   * A message of a thread and a prompt that heads no thread have none. */
  threadControls(act: ActRow): BoxRenderable | undefined {
    const thread = this.chainThreadsBy(this.session.selected).by.get(act.id);
    if (!thread) return undefined;
    const id = thread.act.id;
    const row = this.row({ gap: 3, paddingLeft: space.between });
    row.add(
      this.link(
        [
          [`${glyph.open} `, c.faint],
          ["Collapse", c.faint],
        ],
        () => this.foldThread(id, true),
      ),
    );
    if (thread?.resolved)
      row.add(
        this.link(
          [
            [`${glyph.ring} `, c.faint],
            ["Open again", c.faint],
          ],
          () => this.reopenThread(id),
        ),
      );
    else if (thread)
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

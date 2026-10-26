import { type Loom, momentsOf, owedAt, weave } from "furb/life";
import { conversation } from "../conversation.ts";
import type { Stage } from "../graph.ts";
import { actorOf, joined, type LifeView, LoomPane, lifeViews } from "../overview.ts";
import { styled } from "../ui.ts";
import { part } from "./part.ts";

export const stage = part({
  /** The loom of the chains of the life as it stands, each chain under the label that the operator gave it last. */
  lifeLoom(): Loom {
    const w = this.session;
    const of = [
      w.version,
      [...w.host.prompts.keys()].join(" "),
      w.host.measures,
      [...w.host.pending.keys()].join(" "),
    ];
    const same = this.woven?.of.every((one, at) => one === of[at]) ? this.woven.loom : undefined;
    const loom =
      same ??
      weave({
        acts: w.acts,
        prompts: [...w.host.prompts.values()],
        measures: w.host.measures,
        folds: new Set(),
        width: 0,
        pending: w.host.pending,
        label: (id) => id,
        index: w.boardIndex,
      });
    this.woven = { of, loom };
    // A chain that the operator names again keeps its loom, so each label is read again.
    return {
      ...loom,
      rows: loom.rows.map((row) => (row.id === "operator" ? row : { ...row, label: w.labelOf(row.id) })),
    };
  },

  /** The view of the life that shows: main, the loom, or the graph. */
  /** The view of the life: the one that the preferences keep, or main in the mode of a screen reader,
   * which reads text in lines and no picture of the life in time. */
  get lifeView(): LifeView {
    return this.reader ? "main" : this.session.preferences.view;
  },

  /** The view of the life switched to the one named, or to the next one, which the preferences keep. */
  switchView(view?: LifeView): void {
    const w = this.session;
    const next = view ?? lifeViews[(lifeViews.indexOf(this.lifeView) + 1) % lifeViews.length] ?? "main";
    w.preferences.view = next;
    w.preferences.save();
    if (next === "main") {
      this.cursor = undefined;
      if (this.steering) this.steer(false);
    }
    w.notice =
      next === "main"
        ? "Main shows the threads of the life beside the feed. ⌥V switches the view."
        : `The ${next} shows the life. ⌥V switches the view, and ⌥G moves in it.`;
    this.render();
  },

  /** The actor of a chain, as its last answer, its rungs or its prompts name it. */
  actorOf(chain: string): string {
    const w = this.session;
    return actorOf(chain, w.actsOn(chain), w.host.measures, chain === w.selected ? w.actor : w.host.actor);
  },

  weaveStage(width: number): Loom {
    const w = this.session;
    const then = this.cursor;
    const acts = w.at(w.acts);
    this.stageActs = acts;
    this.stageLoom = weave({
      acts,
      prompts: then === undefined ? [...w.host.prompts.values()] : owedAt(acts),
      measures: w.host.measures,
      folds: this.loom.folds,
      width,
      pending: then === undefined ? w.host.pending : new Map(),
      label: (id) => w.labelOf(id),
    });
    return this.stageLoom;
  },

  /** Whether the stage moves with time: a chain works, or the graph shows a message that flows. */
  moves(): boolean {
    return (
      this.stageLoom.rows.some((row) => row.state === "working") ||
      (this.lifeView === "graph" && this.stageLoom.owed.length > 0)
    );
  },

  /** The loom or the graph over the feed. The graph takes the whole height of the
   * feed for one program and its words, and gives it back to the feed to show one act. */
  renderStage(): void {
    const w = this.session;
    const view = this.lifeView;
    const shows =
      view !== "main" && w.view === "feed" && !this.tree && !(view === "graph" && this.graph.level === "act");
    const whole = shows && view === "graph" && ["program", "word"].includes(this.graph.level);
    if (this.stage.visible !== shows) this.stage.visible = shows;
    if (this.scroll.visible === whole) this.scroll.visible = !whole;
    this.steps();
    if (!shows) return;
    const width = this.feedWidth;
    const body = Math.max(8, this.renderer.height - 14);
    if (view === "loom") {
      const loom = this.weaveStage(LoomPane.cellWidth(width));
      const rows = this.loom.draw({
        loom,
        width,
        height: Math.min(loom.rows.length + 1, Math.max(4, Math.floor(body * 0.45))),
        selected: w.selected,
        frozen: this.cursor !== undefined,
        steering: this.steering,
        actor: (id) => this.actorOf(id),
      });
      this.stage.flexGrow = 0;
      this.stage.height = rows.length;
      this.stage.content = styled(joined(rows));
      return;
    }
    this.weaveStage(Math.max(8, width - 60));
    if (whole) {
      this.stage.flexGrow = 1;
      this.stage.flexShrink = 1;
      this.stage.minHeight = 0;
    } else {
      this.stage.flexGrow = 0;
      this.stage.flexShrink = 0;
    }
    const height = whole ? Math.max(6, this.renderer.height - 12) : Math.max(8, Math.floor(body * 0.55));
    this.stage.height = height;
    this.stage.content = styled(joined(this.graph.rows(width, height)));
  },

  /** A click on a row of the stage: the loom selects its chain, or answers for the row of the operator, and the graph
   * selects what the row shows, and zooms in on a second click. */
  clickStage(row: number): void {
    if (this.lifeView === "graph") {
      if (this.graph.click(row)) this.render();
      return;
    }
    const chain = this.loom.click(row);
    if (chain === "operator") this.question();
    else if (chain) void this.selectAt(chain).catch(this.report);
  },

  /** What the graph reads of the App: the life at the time cursor, and the controls that it asks for. */
  stageOf(): Stage {
    const app = this;
    return {
      get acts() {
        return app.stageActs;
      },
      get loom() {
        return app.stageLoom;
      },
      get owed() {
        return new Set(app.stageLoom.owed.map((prompt) => prompt.id));
      },
      get frozen() {
        return app.cursor !== undefined;
      },
      get selected() {
        return app.session.selected;
      },
      words: () => app.words(),
      label: (chain) => app.session.labelOf(chain),
      actor: (chain) => app.actorOf(chain),
      streams: () => (app.cursor === undefined ? app.session.host.streams : new Map()),
      select: (chain) => app.selectAt(chain),
      // The lens of the graph is the card of the act in the feed, which shows it whole.
      lens: (act) => {
        if (!act) return app.render();
        const w = app.session;
        void (act.on === w.selected ? Promise.resolve() : w.select(act.on))
          .then(() => app.go("feed", act.id))
          .catch(app.report);
      },
    };
  },

  /** The words of the rungs of the life: what the caller of each wrote, what ran, and what its model wrote, the words
   * that the gate refused among them. */
  words(): Map<string, string> {
    const w = this.session;
    const words = new Map<string, string>();
    for (const item of conversation(w.turns, (one) => w.actOf(one)))
      if (item.type === "python" && item.rung) words.set(item.rung.id, item.code);
    for (const [id, word] of Object.entries(w.program)) words.set(id, word);
    for (const act of w.acts)
      if (act.kind === "rung" && act.words[0]) words.set(act.id, String(act.words[0]));
    return words;
  },

  /** The keys go to the loom or the graph, or back to the input. */
  steer(want?: boolean): void {
    const on = want ?? !this.steering;
    const w = this.session;
    if (on && this.lifeView === "main") {
      w.notice = "⌥V shows the loom or the graph, which ⌥G moves in.";
      return;
    }
    this.steering = on;
    if (on) {
      this.closeTree();
      this.composer.blur();
    } else this.composer.focus();
    this.render();
  },

  /** The selection of the loom moves by a count of chains, and the session shows the chain that it reaches. */
  moveLoom(by: number): void {
    const rows = this.stageLoom.rows.filter((row) => row.id !== "operator");
    const at = rows.findIndex((row) => row.id === this.session.selected);
    const next = rows[Math.max(0, Math.min(rows.length - 1, at + by))];
    if (next && next.id !== this.session.selected) void this.selectAt(next.id).catch(this.report);
  },

  /** A chain shown at the moment of the cursor in time, which the loom and the graph keep from chain to chain. */
  async selectAt(chain: string): Promise<void> {
    const at = this.cursor;
    await this.session.select(chain);
    // A move of the cursor while the chain opened stands.
    if (at !== undefined && this.cursor === undefined) this.cursor = at;
    this.render();
  },

  /** The workers of the selected chain fold into its row of the loom, or show again. */
  foldLoom(folding: boolean): void {
    const row = this.stageLoom.rows.find((one) => one.id === this.session.selected);
    if (!row?.workers) return;
    if (folding) this.loom.folds.add(row.id);
    else this.loom.folds.delete(row.id);
    this.render();
  },

  /** The time cursor moves back or forward by a count of moments, and past the last moment it stands at the present
   * again. */
  travel(by: number): void {
    const moments = momentsOf(this.session.acts);
    if (!moments.length) return;
    const from = this.cursor === undefined ? moments.length - 1 : moments.indexOf(this.cursor);
    const to = Math.max(0, from + by);
    this.cursor = to >= moments.length - 1 ? undefined : moments[to];
    this.render();
  },

  /** Whether a verb that changes the life can act: none acts in the past. */
  present(): boolean {
    if (this.cursor === undefined) return true;
    this.session.notice =
      "The cursor stands in the past, where nothing changes. Esc returns to now, and b branches.";
    this.render();
    return false;
  },

  /** The selected chain paused, or woken when a pause holds it. */
  holdChain(): void {
    if (!this.present()) return;
    const w = this.session;
    const row = this.stageLoom.rows.find((one) => one.id === w.selected);
    this.action(`/${row?.state === "paused" ? "wake" : "pause"} ${w.selected}`);
  },

  /** The work of the selected chain cancelled: the act that the graph selects, or what the chain waits on. */
  cancelChain(): void {
    if (!this.present()) return;
    const w = this.session;
    const target =
      this.lifeView === "graph" && this.graph.level !== "threads" && this.graph.level !== "map"
        ? this.graph.selection
        : this.stageLoom.rows.find((one) => one.id === w.selected)?.waits;
    if (target) this.action(`/cancel ${target}`);
    else w.notice = `${w.label} waits on nothing to cancel.`;
  },

  /** A step: the selected chain runs until one more of its rungs ends, then a pause holds it again. */
  async step(): Promise<void> {
    if (!this.present()) return;
    const w = this.session;
    const chain = w.selected;
    this.stepping.set(chain, this.rungsDone(chain));
    if (this.stageLoom.rows.find((row) => row.id === chain)?.state === "paused")
      this.action(`/wake ${chain}`);
    w.notice = `${w.labelOf(chain)} runs one step, then pauses.`;
    this.render();
  },

  rungsDone(chain: string): number {
    return this.session.acts.filter((act) => act.on === chain && act.kind === "rung" && act.done).length;
  },

  /** Each chain that a step let run pauses again once a rung of it that the step let run ends. */
  steps(): void {
    for (const [chain, ended] of this.stepping) {
      if (this.rungsDone(chain) <= ended) continue;
      this.stepping.delete(chain);
      this.action(`/pause ${chain}`);
      this.session.notice = `${this.session.labelOf(chain)} paused after one step.`;
    }
  },

  /** A branch of the selected chain at the time cursor, as the feed branches at its moment. */
  branchAtCursor(): void {
    const w = this.session;
    if (this.cursor === undefined) {
      w.notice = "[ moves the cursor back in time, and b branches the chain there.";
      this.render();
      return;
    }
    void w.branchHere().catch(this.report);
  },

  /** Escape on the stage: the graph zooms out, then the cursor returns to now, then the keys go back to the input. */
  async leaveStage(): Promise<void> {
    if (this.lifeView === "graph" && (await this.graph.key("escape"))) this.render();
    else if (this.cursor !== undefined) this.travel(Number.MAX_SAFE_INTEGER);
    else this.steer(false);
  },

  /** Enter on the stage: the graph zooms in, and the loom answers for the row of the operator or gives the keys to the
   * input on the selected chain. */
  async enterStage(): Promise<void> {
    if (this.lifeView === "graph") {
      await this.graph.key("return");
      this.render();
    } else this.steer(false);
  },

  /** An arrow on the stage: the graph moves its selection, and the loom moves between chains or folds their workers. */
  async arrowStage(name: string): Promise<void> {
    if (this.lifeView === "graph") {
      await this.graph.key(name);
      this.render();
    } else if (name === "up" || name === "down") this.moveLoom(name === "up" ? -1 : 1);
    else if (name === "pageup" || name === "pagedown") this.moveLoom(name === "pageup" ? -8 : 8);
    else this.foldLoom(name === "left");
  },
});

export type StagePart = typeof stage;

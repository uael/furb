import type { BoxRenderable, KeyEvent } from "@opentui/core";
import { asksOperator, type ChainThread } from "furb/life";
import { clipboardImage } from "../clipboard.ts";
import { type Action, bindings, shown } from "../keys.ts";
import { commentStart, type Hunk, lines, scopeLabels, scopes } from "../review.ts";
import { type View, views } from "../session.ts";
import { theme as c, spacing as space } from "../theme.ts";
import { bold, underline } from "../ui.ts";
import { part } from "./part.ts";
import { exitNotice, type Hint, type KeyCommand, rewindNotice, steps, twice } from "./shared.ts";

export const input = part({
  /** The layers of the keys of the App, from the top: a layer that holds takes a key before the layers under it, a
   * command that does not hold lets its key go on, and a key that no layer takes goes to the part that has the focus.
   * The footer offers the keys of the top layer that offers any. It gives what takes each layer away again. */
  layers(): (() => void)[] {
    const offs: (() => void)[] = [];
    // Each layer that comes later stands under the ones before it.
    const layer = (enabled: () => boolean, group: string, keys: KeyCommand[]) =>
      offs.push(
        this.keymap.registerLayer({
          priority: -offs.length,
          enabled,
          commands: keys.map(({ run, when }, index) => ({
            name: `${offs.length}-${index}`,
            run: ({ event }) => run(event) !== false,
            ...(when ? { enabled: when } : {}),
          })),
          bindings: keys.flatMap(({ on, hint }, index) =>
            [on].flat().map((key, first) => ({
              key,
              cmd: `${offs.length}-${index}`,
              ...(hint && !first ? { hint: [...hint, group] } : {}),
            })),
          ),
        }),
      );
    const free = () => !this.renaming?.input?.focused;
    const bare = () => free() && !this.overlay;
    const row = () => this.treeRow();
    const chosen = () => Boolean(this.suggestions[this.suggestionIndex]);
    // The keys of the table, which act while no dialog is open, or at all times.
    const table = (always: boolean): KeyCommand[] =>
      bindings.flatMap(({ key, binding }) => {
        const { run, on = [], hint } = binding;
        if (!run || Boolean(binding.always) !== always) return [];
        const offered: Hint | undefined = hint ? [() => shown(key, binding, this.kitty), hint] : undefined;
        return { on, run: (event) => void this.actions[run](event), when: this.when[run], hint: offered };
      });
    // While a row takes a new name, Escape or Ctrl+C leaves the name as it was, and Ctrl+Q keeps it and quits.
    layer(() => !free(), "rename", [
      { on: ["escape", "ctrl+c"], run: () => this.endRename(false) },
      {
        on: "ctrl+q",
        run: () => {
          this.endRename(true);
          void this.options.quit();
        },
      },
    ]);
    // While a question of a choice is aimed, its list takes the arrows, Enter and the numbers, and Escape gives the
    // list back the keys that Other gave the input.
    const choosing = () => bare() && !this.tree && Boolean(this.picking());
    layer(() => choosing() && !this.choice.other, "choices", [
      { on: "up", run: () => this.stepPick(-1), hint: ["↑↓", "choose"] },
      { on: "down", run: () => this.stepPick(1) },
      { on: "space", run: () => void this.takePick(), when: () => this.onTick(), hint: ["Space", "tick"] },
      {
        on: ["return", "enter"],
        run: () => void (this.onTick() ? this.sendTicked() : this.takePick()),
        hint: [
          "Enter",
          () =>
            this.picking()?.options.length && this.onTick()
              ? `send ${this.choice.ticked.length || "none"}`
              : this.picking()?.options.length
                ? "write your words"
                : "answer",
        ],
      },
      ...Array.from(
        { length: 9 },
        (_, at): KeyCommand => ({
          on: String(at + 1),
          run: () => void this.takePick(at),
          when: () => at <= this.pickCount(),
        }),
      ),
    ]);
    layer(() => choosing() && this.choice.other, "other", [
      {
        on: "escape",
        run: () => {
          this.choice = { ...this.choice, other: false };
          this.render();
        },
        hint: ["Esc", "back to the values"],
      },
      // Up from an empty field goes back to the values, as the list itself does at its first row.
      {
        on: "up",
        run: () => {
          this.choice = { ...this.choice, other: false };
          this.render();
        },
        when: () => !this.composer.plainText,
      },
    ]);
    layer(() => bare() && Boolean(this.tree), "tree", [
      ...Object.entries(steps).map(
        ([on, step]): KeyCommand => ({
          on,
          run: () => this.moveTree(step),
          hint: on === "up" ? ["↑↓", "move"] : undefined,
        }),
      ),
      { on: "right", run: () => this.unfoldTree(), when: () => Boolean(row()?.parent), hint: ["←→", "fold"] },
      { on: "left", run: () => this.foldTreeUp(), when: () => Boolean(row()) },
      {
        on: ["return", "enter"],
        run: () => void this.chooseTreeRow(),
        hint: ["Enter", () => row()?.hint ?? "choose"],
      },
      { on: "escape", run: () => this.closeTree(), hint: ["Esc", "back"] },
    ]);
    layer(() => bare() && this.suggestionBox.visible, "suggestions", [
      { on: "up", run: () => this.stepSuggestion(-1), hint: ["↑↓", "choose"] },
      { on: "down", run: () => this.stepSuggestion(1) },
      { on: "tab", run: () => this.takeSuggestion(false), when: chosen, hint: ["Tab", "complete"] },
      { on: ["return", "enter"], run: () => this.takeSuggestion(true), when: chosen },
      { on: "escape", run: () => this.dismissSuggestions(), hint: ["Esc", "hide"] },
    ]);
    layer(() => bare() && Boolean(this.session.editing), "edit", [
      { on: ["return", "enter"], run: () => void this.submit(), hint: ["Enter", "run"] },
      { on: "escape", run: this.escape, hint: ["Esc", "leave the program"] },
    ]);
    // While the welcome shows the check of the setup and the input is empty, r checks it again.
    layer(() => bare() && this.welcoming && this.setupState().shown && !this.composer.plainText, "setup", [
      { on: "r", run: this.checkAgain, hint: ["r", "check again"] },
    ]);
    // While the operator watches a life and the input is empty, [ and ] move the moment of the feed, b starts a
    // branch there, and Escape comes back to now.
    const watched = () => bare() && this.session.watching && !this.composer.plainText;
    const past = () => this.session.moment !== undefined;
    layer(watched, "watch", [
      { on: "[", run: () => this.session.travel(-1), hint: ["[ ]", "move in time"] },
      { on: "]", run: () => this.session.travel(1) },
      {
        on: "b",
        run: () => void this.session.branchHere().catch(this.report),
        when: past,
        hint: ["b", "branch here"],
      },
      {
        on: "escape",
        run: () => {
          this.session.moment = undefined;
          this.renderContent();
        },
        when: past,
        hint: ["Esc", "now"],
      },
    ]);
    // While the keys move on the loom or the graph, the arrows, Enter and Escape move there, and a letter acts on the
    // selected chain.
    const stageKey = (on: string | string[], run: () => unknown, hint?: Hint): KeyCommand => ({
      on,
      run: () => void Promise.resolve(run()).catch(this.report),
      hint,
    });
    layer(() => bare() && this.steering && this.lifeView !== "main", "stage", [
      stageKey("up", () => this.arrowStage("up"), ["↑↓←→", "move"]),
      ...["down", "left", "right", "pageup", "pagedown"].map((on) => stageKey(on, () => this.arrowStage(on))),
      stageKey(["return", "enter"], () => this.enterStage(), [
        "Enter",
        () => (this.lifeView === "graph" ? "zoom in" : "input"),
      ]),
      stageKey("escape", () => this.leaveStage(), [
        "Esc",
        () => (this.lifeView === "graph" && this.graph.level !== "threads" ? "zoom out" : "back"),
      ]),
      stageKey("[", () => this.travel(-1), ["[ ]", "time"]),
      stageKey("]", () => this.travel(1)),
      stageKey("{", () => this.travel(-10)),
      stageKey("}", () => this.travel(10)),
      stageKey("space", () => this.holdChain(), ["Space", "pause"]),
      stageKey("x", () => this.cancelChain(), ["x", "cancel"]),
      stageKey("n", () => this.step(), ["n", "step"]),
      stageKey("b", () => this.branchAtCursor(), ["b", "branch"]),
      stageKey("tab", () => this.question(), ["Tab", "answer"]),
    ]);
    // While the keys move in the sidebar, the arrows move them, Enter opens a chain or chooses a thread, Escape goes
    // back, and a letter acts on the thread under them.
    const onThread = (run: (thread: ChainThread) => void) => () => {
      const thread = this.railThread();
      if (thread) run(thread);
    };
    layer(() => bare() && this.railKeys, "rail", [
      { on: "up", run: () => this.moveRail(-1), hint: ["↑↓", "move"] },
      { on: "down", run: () => this.moveRail(1) },
      {
        on: ["return", "enter", "right"],
        run: () => this.enterRail(),
        hint: ["Enter", () => (this.railChain ? "choose" : "threads")],
      },
      {
        on: ["escape", "left"],
        run: () => this.leaveRail(),
        hint: ["Esc", () => (this.railChain ? "chains" : "input")],
      },
      {
        on: "space",
        run: onThread((thread) => this.foldThread(thread.act.id)),
        when: () => Boolean(this.railThread()),
        hint: ["Space", "collapse"],
      },
      {
        on: "r",
        run: onThread((thread) => this.resolveThread(thread)),
        when: () => this.railThread()?.resolved === false,
        hint: ["r", "resolve"],
      },
      {
        on: "o",
        run: onThread((thread) => this.reopenThread(thread.act.id)),
        when: () => this.railThread()?.resolved === true,
        hint: ["o", "open again"],
      },
    ]);
    layer(bare, "", [
      {
        on: ["right", "ctrl+f", "meta+right", "meta+f"],
        run: this.acceptGhost,
        when: () => this.composer.focused && Boolean(this.composer.ghost),
      },
      // As in Claude Code, Ctrl+D acts on an empty input.
      { on: "ctrl+d", run: () => this.exit(), when: () => !this.composer.plainText },
      ...table(false),
      {
        on: ["up", "down", "meta+up", "meta+down"],
        run: this.walkHistory,
        when: () => this.composer.focused,
      },
      // Ctrl+J arrives as a line feed from a terminal with no kitty keyboard protocol.
      {
        on: ["shift+return", "shift+enter", "ctrl+j", "linefeed"],
        run: this.newline,
        when: () => this.writesPython,
      },
    ]);
    layer(free, "", [
      { on: ["tab", "shift+tab"], run: this.moveAsk, when: () => Boolean(this.overlay && this.answering) },
      ...table(true),
      { on: "ctrl+c", run: () => this.cancel() },
      { on: "escape", run: this.escape },
      {
        on: ["pageup", "pagedown"],
        run: this.pageQuestion,
        when: () => Boolean(this.overlay && this.questionDocument),
      },
      ...["up", "down", "pageup", "pagedown"].map(
        (on): KeyCommand => ({
          on,
          run: () => this.stepChoice(steps[on] ?? 0),
          when: () => Boolean(this.overlay),
        }),
      ),
    ]);
    return offs;
  },

  /** Whether this terminal sends the chords of the kitty keyboard protocol. */
  get kitty(): boolean {
    return this.renderer.capabilities?.kitty_keyboard === true;
  },

  /** The suggestion a step away from the one chosen, round from the last to the first. */
  stepSuggestion(step: number): void {
    const count = this.suggestions.length;
    if (count) this.suggestionIndex = (this.suggestionIndex + step + count) % count;
    this.renderSuggestions();
  },

  /** The suggestion chosen, completed in the input, or sent when Enter takes one that sends itself. */
  takeSuggestion(enter: boolean): void {
    const chosen = this.suggestions[this.suggestionIndex];
    if (enter && chosen?.submit) chosen.submit();
    else if (chosen) this.complete(chosen);
  },

  /** The text that a question asks, scrolled by a page. */
  pageQuestion(key?: KeyEvent) {
    return this.questionDocument?.scrollBy(key?.name === "pageup" ? -8 : 8);
  },

  /** The choice of the dialog a step away from the one chosen, within the list. */
  stepChoice(step: number): void {
    this.selection = Math.max(0, Math.min(this.filtered.length - 1, this.selection + step));
    this.renderChoices();
  },

  /** The row of the rewind tree unfolded, or the pointer on its first child when it is open. */
  unfoldTree(): void {
    const row = this.treeRow();
    if (row?.folded) this.foldTree(row.id, false);
    else this.moveTree(1);
  },

  /** The row of the rewind tree folded, or the pointer on the row above it when it is folded. */
  foldTreeUp(): void {
    const row = this.treeRow();
    if (row?.parent && !row.folded) this.foldTree(row.id, true);
    else if (row?.up) this.pointTree(row.up);
  },

  /** Ctrl+D on an empty input asks once, and exits when it is pressed again while it asks. */
  exit(): void {
    if (this.session.notice === exitNotice) void this.options.quit();
    else this.session.notice = exitNotice;
  },

  /** Whether the input holds Python: Python input, or a program under edit. */
  get writesPython(): boolean {
    return this.session.mode === "python" || Boolean(this.session.editing);
  },

  /** A new line in the input, as deep as the line before it, and deeper after a colon. */
  newline(): void {
    const before = this.beforeCursor().split("\n").at(-1) ?? "";
    this.composer.insertText(
      `\n${before.match(/^\s*/)?.[0] ?? ""}${before.trimEnd().endsWith(":") ? "  " : ""}`,
    );
  },

  /** Ctrl+C closes a dialog or the rewind tree, or clears the input, or cancels the last act that the operator made on
   * the chain, which takes back a message of his. */
  cancel(): void {
    if (this.overlay) this.closeOverlay();
    else if (this.tree) this.closeTree();
    else if (this.composer.plainText) this.composer.replaceText("");
    else this.action("/cancel");
  },

  /** Right and Ctrl+F at the end of the line take the grey rest of it, and Alt+Right and Alt+F take its next word, as
   * fish does. */
  acceptGhost(key?: KeyEvent): void {
    const word = key?.meta ? /^\s*\S+/.exec(this.composer.ghost)?.[0] : undefined;
    this.composer.insertText(word ?? this.composer.ghost);
  },

  /** Up on an empty input takes back the last message queued on the chain, and Up and Down at the first and the last
   * line of the input walk the history of what it sent, as a shell does: the entries of this chain first, then those
   * of the other chains, and with text typed, only the entries that start with it, as fish does. Alt+Up and Alt+Down
   * walk it from any line. It gives false when it does nothing, and the input moves its cursor. */
  walkHistory(key?: KeyEvent): boolean {
    const up = key?.name === "up";
    const anywhere = key?.meta;
    const queued = this.session.queued.findLast((entry) => entry.chain === this.session.selected);
    if (up && !anywhere && !this.composer.plainText && queued && this.historyIndex < 0) {
      this.session.removeQueued(queued.id);
      this.insert(queued.text);
      this.session.notice = "The queued message is back in the input. ⌥Enter queues it again.";
      return true;
    }
    const cursor = this.composer.visualCursor.visualRow;
    const edge = up ? cursor === 0 : cursor >= this.composer.virtualLineCount - 1;
    const prefix = this.historyIndex < 0 ? this.composer.plainText : this.historyPrefix;
    const history = this.history().filter(
      (entry) => entry.startsWith(prefix) && (this.historyIndex >= 0 || entry !== prefix),
    );
    if (!(anywhere || edge) || !history.length || !(up || this.historyIndex >= 0)) return false;
    if (this.historyIndex < 0) {
      this.historyDraft = this.composer.plainText;
      this.historyPrefix = prefix;
      this.historyIndex = history.length;
    }
    this.historyIndex = Math.max(0, Math.min(history.length, this.historyIndex + (up ? -1 : 1)));
    const text = history[this.historyIndex] ?? this.historyDraft;
    if (this.historyIndex === history.length) {
      this.historyIndex = -1;
      this.historyPrefix = "";
    }
    this.composer.replaceText(text);
    return true;
  },

  /** Escape closes what is open, or pauses the model at work. An Escape with nothing to do asks for a second one,
   * which opens the rewind tree. */
  escape(): void {
    const open = Boolean(this.overlay || this.searchRow.visible || this.session.editing);
    if (!open && this.pausing()) this.action("/pause");
    else if (!open) {
      const now = Date.now();
      if (now - this.escapedAt <= twice) {
        this.escapedAt = 0;
        if (this.session.notice === rewindNotice) this.session.notice = "";
        this.rewind();
        return;
      }
      this.escapedAt = now;
      this.session.notice = rewindNotice;
      // The notice lasts as long as a second Escape rewinds.
      setTimeout(() => {
        if (!this.closed && this.session.notice === rewindNotice) this.session.notice = "";
      }, twice).unref();
    }
    this.closeOverlay();
    this.closeSearch();
    this.leaveEdit();
  },

  /** Whether Escape pauses the chain: a model is at work on it, and no pause holds it. */
  pausing(): boolean {
    const w = this.session;
    return !w.host.reading && !w.paused && w.open.some((act) => act.kind === "prompt" && !asksOperator(act));
  },

  actionsTable(): Record<Action, (key?: KeyEvent) => unknown> {
    return {
      queue: () => {
        // The queue holds messages to a model. Python input and a program under edit run when Enter sends them.
        if (this.writesPython) {
          this.session.notice = "Only a message can wait in the queue. Enter runs this Python now.";
          return;
        }
        this.session.enqueue(this.composer.plainText);
        this.composer.setText("");
      },
      stash: () => this.stash(),
      view: (key) => this.showView(views[Number(key?.name) - 1] ?? "feed"),
      // ⌘Tab rolls the chains too where the system and the terminal pass it, which macOS does not, since it keeps ⌘Tab
      // to switch applications.
      roll: (key) => this.rollChain(key?.shift ? -1 : 1),
      palette: () => this.palette(),
      chains: () => this.chains(),
      newChain: () => this.insert("/chain "),
      models: () => this.models(),
      effort: () => this.effortPicker(),
      themes: () => this.themes(),
      workspaces: () => void this.workspacePicker(),
      sidebar: () => this.options.workspaces.toggle(),
      details: () => this.details(),
      editor: () => void this.editDraft().catch(this.report),
      image: () => void clipboardImage((path) => this.session.attachImage(path)).catch(this.report),
      search: () => this.openSearch(),
      scroll: (key) =>
        this.scroll.scrollBy(
          (key?.name === "pageup" ? -1 : 1) * Math.max(1, this.scroll.viewport.height - 2),
        ),
      python: () => this.toggleMode(),
      complete: () => void this.completeNames().catch(this.report),
      ladders: () => this.ladders(),
      answer: () => this.question(),
      names: () => void this.names().catch(this.report),
      copy: () => {
        const selection = this.renderer.getSelection()?.getSelectedText();
        if (selection) this.renderer.copyToClipboardOSC52(selection);
      },
      back: () => void this.back().catch(this.report),
      jump: (key) => this.jumpMessage(["]", "n"].includes(key?.name ?? "") ? 1 : -1),
      scope: (key) => this.session.review(key?.name === "pageup" ? -1 : 1),
      pause: this.escape,
      takeBack: () => this.cancel(),
      help: () => this.help(),
      quit: () => void this.options.quit(),
      lifeView: () => this.switchView(),
      steer: () => this.steer(),
      rail: () => this.railSteer(),
      repair: () => void this.session.repairJournal().catch(this.session.fail),
      renew: () => void this.session.renewJournal().catch(this.session.fail),
    };
  },

  whenTable(): Partial<Record<Action, () => boolean>> {
    return {
      answer: () => this.session.desk.asks.length > 0,
      scope: () => this.session.view === "changes",
      repair: () => this.session.journalDamaged && !this.session.host.reading,
      renew: () => this.session.journalDamaged && !this.session.host.reading,
      pause: () => !this.searchRow.visible && this.pausing(),
      takeBack: () => {
        const last = this.session.lastOwn;
        return (
          !this.composer.plainText &&
          !this.searchRow.visible &&
          Boolean(last && this.session.isUserPrompt(last) && last.words[2] !== "operator")
        );
      },
    };
  },

  /** The view shown, with the rewind tree closed. */
  showView(view: View): void {
    this.closeTree();
    this.session.show(view);
  },

  openSearch(): void {
    this.closeTree();
    this.searchRow.visible = true;
    this.search.focus();
  },

  closeSearch(): void {
    if (!this.searchRow.visible && !this.session.search) return;
    this.searchRow.visible = false;
    this.search.value = "";
    this.session.search = "";
    this.composer.focus();
    this.render();
  },

  dismissSuggestions(): void {
    const token = this.token();
    this.dismissed = token ? `${token.kind}${token.text}` : "";
    this.suggest();
  },

  leaveEdit(): void {
    if (!this.session.editing) return;
    this.session.editing = undefined;
    this.render();
  },

  /** The input put aside, or brought back: the input and the text put aside for its draft trade places. */
  stash(): void {
    const w = this.session;
    const key = this.draftKey;
    const text = this.composer.plainText;
    const kept = w.stashes[key] ?? "";
    if (!text && !kept) {
      w.notice = "⌃S puts the input aside. The input is empty.";
      return;
    }
    if (text) w.stashes[key] = text;
    else delete w.stashes[key];
    this.composer.setText(kept);
    this.composer.cursorOffset = kept.length;
    w.notice = text
      ? kept
        ? "The input and the text put aside traded places."
        : "The input is put aside. ⌃S brings it back."
      : "The text put aside is back.";
    w.save();
    this.render();
  },

  /** The feed scrolled to the message of the operator before or after the top of the view. */
  jumpMessage(step: number): void {
    const w = this.session;
    if (this.tree || w.view !== "feed") return;
    const top = this.scroll.viewport.y;
    const messages = this.scroll
      .getChildren()
      .filter((node) => {
        const act = w.acts.find((one) => one.id === this.cards.get(node.id)?.act);
        return act && w.isUserPrompt(act) && !asksOperator(act);
      })
      .map((node) => node.y - top);
    const offset = step > 0 ? messages.find((at) => at > 0) : messages.findLast((at) => at < 0);
    if (offset !== undefined) this.scroll.scrollBy(offset);
  },

  /** The App ends once, by its own quit or as its renderer ends. It stops each timer and each frame to come first,
   * so that nothing draws on a buffer that is gone, and it keeps the draft while the input still holds it. */
  dispose(): void {
    if (this.closed) return;
    this.closed = true;
    clearInterval(this.tick);
    clearTimeout(this.redraw);
    clearTimeout(this.hoverTimer);
    this.redraw = this.hoverTimer = undefined;
    this.renderer.off("frame", this.laidOut);
    this.renderer.off("destroy", this.dispose);
    if (!this.composer.isDestroyed) this.keepDraft();
    this.session.scrolls[this.lastView] = this.place;
    this.session.save();
    this.session.off("change", this.schedule);
    this.options.workspaces.off("change", this.schedule);
    this.session.off("compose", this.compose);
    this.session.off("resume", this.resume);
    this.session.off("shared", this.shared);
    this.session.off("confirm", this.confirm);
    this.session.off("authentication", this.authentication);
    this.session.off("codexModels", this.codexModels);
    for (const off of this.layersOff) off();
    this.partKeysOff?.();
    this.parts.dispose();
    this.renderer.off("resize", this.render);
    this.renderer.off("selection", this.copySelection);
    this.renderer.off("theme_mode", this.modeChanged);
    this.renderer.removeFrameCallback(this.drawDue);
    this.renderer.off("focus", this.focusTaken);
    this.renderer.off("blur", this.focusLost);
    this.root.destroyRecursively();
    this.style.destroy();
  },

  /** The scopes of the review of the changes: the one shown in bold, and each other one a link to it. */
  scopes(): BoxRenderable {
    const w = this.session;
    const row = this.row({ gap: space.between, flexShrink: 0 });
    for (const scope of scopes)
      row.add(
        scope === w.scope
          ? this.text([[scopeLabels[scope], c.text, bold | underline]])
          : this.link(scopeLabels[scope], () => w.review(scope)),
      );
    row.add(this.text("⌃PageUp / ⌃PageDown", c.faint));
    return row;
  },

  /** The feed scrolled so that the card of an id stands at its top. */
  jumpTo(id: string): void {
    const node = this.scroll.getChildren().find((one) => one.id === id);
    if (node) this.scroll.scrollBy(node.y - this.scroll.viewport.y);
  },

  /** A comment on one line of a hunk: the operator picks the line, and the input holds the start of a message that
   * names it, which Enter sends to the chain as a prompt. */
  commentLine(path: string, hunk: Hunk): void {
    const shown = lines(hunk);
    this.openPalette(
      `Comment a line of ${path}`,
      shown.map((line) => ({
        label: `${line.sign} ${line.text}`.trimEnd(),
        detail: `${line.sign === "-" ? "removed " : ""}line ${line.number}`,
        run: () => {
          const start = commentStart(path, line);
          this.insert(start);
          this.composer.cursorOffset = start.length;
        },
      })),
      {
        selected: Math.max(
          0,
          shown.findIndex((line) => line.sign !== " "),
        ),
      },
    );
  },
});

export type InputPart = typeof input;

export const inputBound = [
  "pageQuestion",
  "newline",
  "acceptGhost",
  "walkHistory",
  "escape",
  "dispose",
] as const;

import { shapes } from "furb";
import { takesNothing } from "furb/life";
import { slashes } from "../commands.ts";
import { emojiNamed, emojis } from "../emoji.ts";
import { shortenHome } from "../files.ts";
import { clip, count, dollars, graphemes, modelName, share } from "../format.ts";
import { modeOf } from "../history.ts";
import { working } from "../session.ts";
import { theme as c, glyph, palettes, spacing as space, type ThemeChoice, themeLabels } from "../theme.ts";
import { bold, type Part } from "../ui.ts";
import { part } from "./part.ts";
import { efforts, pathCommands, type Suggestion, terminalDetail, type Value } from "./shared.ts";

export const composer = part({
  /** The text of the composer kept as the draft it shows. An empty draft is none, so that a program to edit
   * opens from its door. */
  keepDraft(): void {
    const text = this.composer.plainText;
    if (text) this.session.drafts[this.draftKey] = text;
    else delete this.session.drafts[this.draftKey];
  },

  /** The key of the draft that the input shows: the draft of the chain, or the draft of the question that the input
   * answers, so that the words written for a question never go to another one. */
  composeKey(): string {
    const ask = this.session.operatorPrompt;
    const key = this.session.draftKey;
    return ask && !ask.notice && !this.session.editing ? `${key}#${ask.id}` : key;
  },

  /** The words that were written for a question that closed meanwhile go to the history of the input, where the
   * operator finds them again with the up key, and no later question takes them. */
  orphans(): void {
    const w = this.session;
    const open = new Set(w.desk.asks.map((ask) => ask.id));
    for (const [key, text] of Object.entries(w.drafts)) {
      const at = key.lastIndexOf("#");
      if (at < 0 || key === this.draftKey || open.has(key.slice(at + 1))) continue;
      delete w.drafts[key];
      if (!text.trim()) continue;
      w.remember(key.slice(0, at), text);
      w.notice = `${key.slice(at + 1)} closed. ↑ brings your words back.`;
    }
  },

  /** The composer shows the draft of a key, and the draft it showed stays under its own key. */
  showDraft(key: string, shown?: string): void {
    const text = shown ?? this.session.drafts[key] ?? "";
    if (this.draftKey) this.keepDraft();
    this.draftKey = key;
    this.composer.setText(text);
    this.historyIndex = -1;
    this.historyPrefix = "";
  },

  compose(text: string) {
    this.closeTree();
    this.showDraft(this.composeKey(), text);
    this.composer.focus();
  },

  /** A change of the session draws the view again once the changes of 35 ms are in, at the start of the next frame
   * of the renderer, so that the work of the view and the frame that shows it share the time of one frame. */
  schedule() {
    if (this.closed) return;
    if (!this.redraw)
      this.redraw = setTimeout(() => {
        this.redraw = undefined;
        this.due = true;
        this.renderer.requestRender();
      }, 35);
  },

  async drawDue(): Promise<void> {
    if (!this.due || this.closed) return;
    this.due = false;
    this.render();
  },

  /** What the input sent in this mode, the oldest first: the lines of the other sessions of the workspace and of its
   * worktrees, then those of the other chains of this session, then those of this chain, so that the session now open
   * comes first in a walk and in a suggestion. A line that came before comes only once, at its latest place. A chain
   * that holds messages the input did not send, as a record that opens with no saved input, walks those messages
   * instead of the lines of its session. */
  history(): string[] {
    const w = this.session;
    const mode = modeOf(w.draftKey);
    const own = w.histories[w.draftKey] ?? [];
    const others = Object.entries(w.histories)
      .filter(([key]) => key !== w.draftKey && key.endsWith(mode))
      .flatMap(([, entries]) => entries);
    const sent = [...others, ...own];
    const session =
      !sent.length && w.mode === "prompt" && !w.editing
        ? w.activity
            .filter((act) => w.isUserPrompt(act))
            .map((act) => String(act.words[1] ?? ""))
            .filter(Boolean)
        : sent;
    const seen = new Set<string>();
    return [...w.workspaceHistory(mode), ...session]
      .reverse()
      .filter((entry) => !seen.has(entry) && seen.add(entry))
      .reverse();
  },

  /** Sets the grey rest of the line again: what the latest entry of the history that starts with the text adds to it.
   * It is empty while a walk of the history runs, while the suggestions of a token show, and unless the cursor ends
   * the text. */
  ghosting(): void {
    const text = this.composer.plainText;
    let rest = "";
    if (
      text &&
      this.historyIndex < 0 &&
      !this.suggestionBox.visible &&
      !this.session.host.reading &&
      this.beforeCursor() === text
    ) {
      const entry = this.history().findLast((one) => one.length > text.length && one.startsWith(text));
      rest = entry?.slice(text.length) ?? "";
    }
    if (rest === this.composer.ghost) return;
    this.composer.ghost = rest;
    this.composer.requestRender();
  },

  async submit(): Promise<void> {
    if (this.submitting || this.session.host.reading) return;
    this.submitting = true;
    const key = this.draftKey;
    const base = this.session.draftKey;
    const content = this.composer.plainText;
    const python = this.session.mode === "python" && !this.session.editing && !content.startsWith("/");
    // The text leaves its draft as it is sent, so what is typed while it is sent stays in the composer.
    this.composer.setText("");
    try {
      const declined = this.picking();
      const ask = this.session.operatorPrompt;
      if (declined && this.choice.other && !content.startsWith("/")) {
        if (content.trim()) await this.session.decline(declined, content);
        else this.session.notice = "Write your own words, or press Esc to go back to the values.";
      } else if (!content.trim() && ask && !ask.notice && !declined && takesNothing(ask.shape)) {
        await this.session.answer(ask, "");
      } else if (!(await this.globalCommand(content.trim()))) {
        const woke = !content.startsWith("/") && (await this.session.wakeForInput());
        await this.session.submit(python ? `/run ${content}` : content);
        if (woke) this.session.notice = "This message woke the paused chain.";
      }
      this.session.remember(base, content);
      this.historyIndex = -1;
      this.historyPrefix = "";
    } catch (error) {
      // A text that was not sent goes back to its draft, unless that draft holds new text by now.
      if (this.draftKey === key && !this.composer.plainText) this.composer.setText(content);
      else if (this.draftKey !== key && !this.session.drafts[key]) this.session.drafts[key] = content;
      this.report(error);
    } finally {
      this.submitting = false;
      this.render();
    }
  },

  /** The composer holds a text in place of the draft that the session shows, and an undo gives the draft back. */
  insert(text: string): void {
    this.closeOverlay();
    if (this.draftKey !== this.composeKey()) this.showDraft(this.composeKey());
    this.composer.replaceText(text);
    this.composer.focus();
  },

  /** How a command runs when it is picked from a list: one that needs its argument waits for it in the input. */
  command(name: string, argument: string): () => void {
    return () => (argument && !argument.startsWith("[") ? this.insert(`/${name} `) : this.action(`/${name}`));
  },

  /** The text before the cursor. The offset of the cursor counts cells of the terminal, not units of the text. */
  beforeCursor(): string {
    return this.composer.getTextRange(0, this.composer.cursorOffset);
  },

  /** The text before the cursor that ends with `old` replaced. The composer deletes one grapheme at a time. */
  replaceBefore(old: string, replacement: string): void {
    for (const _ of graphemes.segment(old)) this.composer.deleteCharBackward();
    this.composer.insertText(replacement);
  },

  /** The token the cursor ends: a `/command` that opens the input, or an `@path` or a `:shortcode` that starts a word
   * of a prompt. */
  token(): { kind: "/" | "@" | ":" | "value"; text: string; command?: string } | undefined {
    const before = this.beforeCursor();
    const slash = before.match(/^\/([\w-]*)$/);
    if (slash) return { kind: "/", text: slash[1] ?? "" };
    // The first word after a command whose values are known is a value, which the suggestions complete.
    const value = before.match(/^\/([\w-]+) (\S*)$/);
    if (value && Object.hasOwn(this.values, value[1] ?? ""))
      return { kind: "value", text: value[2] ?? "", command: value[1] ?? "" };
    const at = before.match(/(?:^|\s)@([^\s"']*)$/);
    if (at && this.session.mode === "prompt" && !this.session.editing)
      return { kind: "@", text: at[1] ?? "" };
    const colon = before.match(/(?:^|\s):([\w+-]*)$/);
    if (colon && this.session.mode === "prompt" && !this.session.editing)
      return { kind: ":", text: colon[1] ?? "" };
    return undefined;
  },

  /** The token before the cursor as it is now, replaced with the text of a suggestion. */
  complete(suggestion: Suggestion): void {
    const token = this.token();
    if (token)
      this.replaceBefore(token.kind === "value" ? token.text : `${token.kind}${token.text}`, suggestion.text);
  },

  /** The paths of the project, which the suggestions read, as a pick of them gives them: none until the read of the
   * session ends, which draws the suggestions again. An `@` that starts a word reads them afresh. */
  paths(pick = (paths: string[]) => paths, fresh = false): Value[] {
    void this.session.projectFiles(fresh);
    return pick(this.session.files?.paths ?? []).map((path) => ({ value: path, detail: "" }));
  },

  valuesTable(): Partial<Record<string, () => Value[]>> {
    return {
      model: () => {
        const w = this.session;
        const roster = w.roster.filter(([name]) => name !== "operator");
        // The models of the catalog that the roster does not hold follow it, and one of them joins the roster when it is
        // chosen.
        const added = w.catalog.filter(([name]) => !roster.some(([held]) => held === name));
        return [...roster, ...added].map(([name, , window]) => {
          const { provider, id } = modelName(name);
          return {
            // A catalog can hold the same id under a provider that the roster does not offer.
            value: name,
            detail: `${count(window)} tokens of context`,
            current: name === w.actorChoice.model,
            label: id,
            heading: provider || undefined,
          };
        });
      },
      effort: () => {
        const { model, effort } = this.session.actorChoice;
        return (this.session.roster.find(([name]) => name === model)?.[1] ?? []).map((name) => ({
          value: name,
          detail: efforts[name] ?? "",
          current: name === effort,
        }));
      },
      shape: () =>
        shapes().map((shape) => ({
          value: shape,
          detail: `The answer is a ${shape}`,
          current: shape === this.session.shape,
        })),
      theme: () =>
        (["terminal", ...Object.keys(palettes)] as ThemeChoice[]).map((name) => ({
          value: name,
          detail: name === "terminal" ? terminalDetail : themeLabels[name].join(", "),
          current: name === this.session.preferences.theme,
        })),
      gutter: () =>
        (["left", "right"] as const).map((side) => ({
          value: side,
          detail:
            side === "left"
              ? "The numbers stand inside the block of code"
              : "The numbers stand right of the block",
          current: side === this.session.preferences.gutter,
        })),
      access: () =>
        (["reader", "motion"] as const).map((name) => ({
          value: name,
          detail: name === "reader" ? "Screen reader mode, on or off" : "Reduced motion, on or off",
          current:
            name === "reader"
              ? this.session.preferences.screenReader
              : this.session.preferences.reducedMotion,
        })),
      read: () => this.paths(),
      image: () => this.paths((paths) => paths.filter((path) => /\.(png|jpe?g|gif|webp)$/i.test(path))),
      cd: () =>
        this.paths((paths) =>
          [
            ...new Set(
              paths.flatMap((path) => (path.includes("/") ? [path.slice(0, path.lastIndexOf("/"))] : [])),
            ),
          ].sort(),
        ),
      workspace: () =>
        this.options.workspaces.groups.map((group) => ({
          value: shortenHome(group.directory),
          detail: group.name,
        })),
      edit: () =>
        this.session.activity
          .filter((act) => this.session.editable(act))
          .map((act) => ({
            value: act.id,
            detail: clip(String(act.words[1] ?? "").split("\n")[0] ?? "", 48),
          })),
      pause: () => this.chainValues(),
      wake: () => this.chainValues(),
      cancel: () =>
        this.session.open
          .filter((act) => working(act))
          .map((act) => ({
            value: act.id,
            detail: `${act.kind}  ${clip(String(act.words[act.kind === "prompt" ? 1 : 0] ?? ""), 40)}`,
          })),
      feed: () =>
        this.session.open
          .filter((act) => act.kind === "bash" && act.words[1] === null)
          .map((act) => ({ value: act.id, detail: clip(String(act.words[0] ?? ""), 48), more: true })),
      close: () => this.session.open.map((act) => ({ value: act.id, detail: act.kind, more: true })),
      grant: () =>
        ["1", "2", "5", "10", "20"].map((value) => ({
          value,
          detail: `Pause this chain at ${dollars(Number(value))}`,
        })),
      context: () =>
        ["0.5", "0.7", "0.8", "0.9"].map((value) => ({
          value,
          detail: `Pause this chain at ${share(Number(value))}`,
        })),
    };
  },

  /** The chains of the session as the values of a command, which pause and wake take. */
  chainValues(): Value[] {
    const w = this.session;
    return w.chains.map((chain) => ({
      value: chain.id,
      detail: w.labelOf(chain.id),
      current: chain.id === w.selected,
    }));
  },

  suggest(): void {
    // A whole `:shortcode:` of a prompt that names an emoji becomes that emoji.
    const whole = this.beforeCursor().match(/(?:^|\s)(:([\w+-]+):)$/);
    const named =
      whole && this.session.mode === "prompt" && !this.session.editing && emojiNamed(whole[2] ?? "");
    if (whole && named) {
      this.replaceBefore(whole[1] ?? "", named);
      return;
    }
    const token = this.token();
    const key = token ? `${token.kind}${token.text}` : "";
    const started = key === "@" && this.lastToken !== "@";
    // A new filter chooses anew from its first suggestion, and a move of the cursor keeps the choice.
    if (key !== this.lastToken) this.suggestionIndex = 0;
    this.lastToken = key;
    if (key !== this.dismissed) this.dismissed = "";
    if (!token || this.overlay || this.dismissed) {
      this.suggestions = [];
      this.renderSuggestions();
      return;
    }
    if (token.kind === "/") {
      // The command that the token names whole comes first.
      this.suggestions = slashes
        .filter(({ name }) => name.startsWith(token.text))
        .sort((one, other) => Number(other.name === token.text) - Number(one.name === token.text))
        .map(({ name, argument, detail, usage }) => ({
          label: usage,
          detail,
          text: `/${name} `,
          submit: () => {
            this.composer.setText("");
            this.command(name, argument)();
          },
        }));
    } else if (token.kind === "value") {
      const command = token.command ?? "";
      const wanted = token.text.toLowerCase();
      // A value that starts with what is typed comes first, then a value that holds it.
      this.suggestions = (this.values[command]?.() ?? [])
        .filter(({ value }) => value.toLowerCase().includes(wanted))
        .sort(
          (one, other) =>
            Number(other.value.toLowerCase().startsWith(wanted)) -
            Number(one.value.toLowerCase().startsWith(wanted)),
        )
        .slice(0, 50)
        .map(({ value, detail, current, more }) => ({
          label: value,
          detail: `${detail}${current ? "   current" : ""}`,
          text: `${value} `,
          ...(more
            ? {}
            : {
                submit: () => {
                  this.composer.setText(`/${command} ${value}`);
                  void this.submit();
                },
              }),
        }));
    } else if (token.kind === ":") {
      this.suggestions = emojis(token.text)
        .slice(0, 50)
        .map(({ emoji, name, description }) => ({
          label: `${emoji} :${name}:`,
          detail: description,
          text: emoji,
        }));
    } else {
      const wanted = token.text.toLowerCase();
      this.suggestions = this.paths(
        (paths) => paths.filter((path) => path.toLowerCase().includes(wanted)),
        started,
      )
        .slice(0, 50)
        .map(({ value: path }) => ({
          label: `@${path}`,
          detail: "",
          text: `@${/\s/.test(path) ? JSON.stringify(path) : path} `,
        }));
    }
    this.suggestionIndex = Math.min(this.suggestionIndex, Math.max(0, this.suggestions.length - 1));
    this.renderSuggestions();
  },

  renderSuggestions(): void {
    const token = this.overlay || this.dismissed ? undefined : this.token();
    const shown = token ? this.suggestions : [];
    const files = this.session.files;
    // An emoji list that matches nothing closes.
    const state =
      !token || token.kind === ":"
        ? ""
        : (token.kind === "@" || pathCommands.has(token.command ?? "")) && files?.error
          ? files.error
          : (token.kind === "@" || pathCommands.has(token.command ?? "")) && !files?.paths
            ? "Finding project files..."
            : shown.length
              ? ""
              : token.kind === "/"
                ? "No command starts with that name."
                : token.kind === "value"
                  ? "No known value matches. Enter sends what is typed."
                  : "No project file matches.";
    if (
      !this.paneChanged(this.suggestionBox, [
        this.theme,
        state,
        this.suggestionIndex,
        shown.map((one) => one.label),
      ])
    )
      return;
    this.clear(this.suggestionBox);
    this.suggestionBox.visible = Boolean(state || shown.length);
    if (state)
      this.suggestionBox.add(this.inset(space.between, this.line(state, files?.error ? c.danger : c.muted)));
    const rows = 8;
    const first = Math.max(0, Math.min(this.suggestionIndex - rows + 1, shown.length - rows));
    const visible = shown.slice(first, first + rows);
    // The details stand in one column after the longest label, and a long label pushes none of them.
    const column = Math.min(
      36,
      Math.max(0, ...visible.map((one) => Bun.stringWidth(one.label))) + space.between,
    );
    for (const [offset, one] of visible.entries()) {
      const index = first + offset;
      const selected = index === this.suggestionIndex;
      const row = this.row({
        paddingX: space.inset,
        backgroundColor: selected ? c.selected : c.raised,
        onMouseUp: this.click(() => {
          this.suggestionIndex = index;
          this.complete(one);
        }),
        onMouseOver: () => {
          if (this.suggestionIndex === index) return;
          this.suggestionIndex = index;
          this.renderSuggestions();
        },
      });
      row.add(this.text(selected ? `${glyph.pointer} ` : "  ", c.accent));
      row.add(
        this.text(one.label, selected ? c.accent : c.text, {
          width: one.detail ? column : undefined,
          truncate: true,
          attributes: selected ? bold : 0,
        }),
      );
      if (one.detail) row.add(this.text(one.detail, c.muted, { truncate: true, flexShrink: 1 }));
      this.suggestionBox.add(row);
    }
    if (shown.length > rows)
      this.suggestionBox.add(
        this.inset(space.between + space.inset, this.line(`${shown.length - rows} more`, c.faint)),
      );
    this.renderStatus();
  },

  /** The mark of a choice that is the current one, and the room of that mark for the others. */
  current(yes: boolean): Part {
    return yes ? [`${glyph.done} `, c.accent] : ["  "];
  },

  /** The values of a command in a picker, with the current one marked and selected, which runs the command on the
   * value that is chosen. */
  pick(command: string, label: string, note: string, only?: (value: string) => boolean): void {
    const values = (this.values[command]?.() ?? []).filter((one) => !only || only(one.value));
    this.openPalette(
      label,
      values.map((one) => ({
        label: one.label ?? one.value,
        detail: one.detail,
        heading: one.heading,
        mark: this.current(Boolean(one.current)),
        run: () => this.action(`/${command} ${one.value}`),
      })),
      { selected: values.findIndex((one) => one.current), note },
    );
  },
});

export type ComposerPart = typeof composer;

export const composerBound = ["compose", "schedule", "drawDue", "suggest"] as const;

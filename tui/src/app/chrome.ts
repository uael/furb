import type { BoxRenderable, RGBA, TextRenderable } from "@opentui/core";
import { gist } from "furb/life";
import { shortenHome, shortenHomes } from "../files.ts";
import { clip, count, modelName, retryLine } from "../format.ts";
import { readOnly } from "../reading.ts";
import { statusLabels, views } from "../session.ts";
import { theme as c, glyph, spacing as space, spin, threadTone } from "../theme.ts";
import { bold, type Part, plain, styled } from "../ui.ts";
import { part } from "./part.ts";
import { exitNotice, type Hint, rewindNotice, statusMark, viewLabels } from "./shared.ts";

export const chrome = part({
  /** A switch between a few choices, which the views and the mode of the input share. A track in a color of its own
   * holds a segment for each choice, two columns of space at each side of its label. The chosen segment is filled
   * with the color of its choice from edge to edge, the segment under the pointer lights in place, and a click chooses
   * a segment. The chord that moves the switch stands after the track. */
  switcher(
    box: BoxRenderable,
    choices: { label: string; badge?: string; color: RGBA; run?: () => void }[],
    chosen: number,
    chord: string,
    track: RGBA,
  ): void {
    for (const [index, choice] of choices.entries()) {
      const active = index === chosen;
      const fill = active ? choice.color : track;
      const parts = (lit: boolean): Part[] => [
        [`  ${choice.label}`, active ? c.background : lit ? c.text : c.muted, active ? bold : 0, fill],
        [choice.badge ? ` ${choice.badge}` : "", active ? c.background : lit ? c.text : c.faint, 0, fill],
        ["  ", c.text, 0, fill],
      ];
      // The pointer recolors the segment that it is over, and builds no node, so that a press and its release land on
      // the same segment.
      const segment: TextRenderable = this.line(parts(false), c.text, {
        run: choice.run,
        ...(choice.run && !active
          ? {
              onMouseOver: () => {
                segment.content = styled(parts(true));
              },
              onMouseOut: () => {
                segment.content = styled(parts(false));
              },
            }
          : {}),
      });
      box.add(segment);
    }
    if (chord) box.add(this.line(`  ${chord}`, c.faint));
  },

  /** Without journal entries, a read cannot say that the writes had no net change. */
  get unrecordedChanges(): boolean {
    const w = this.session;
    return !!w.host.reading && w.host.changes === 0 && w.scope !== "branch";
  },

  /** The top line: the session and the chain at its left, each a button, and the toggle of the views at its right. */
  renderTop(): void {
    const w = this.session;
    const unknown = w.view === "changes" && (w.journalDamaged || w.error || this.unrecordedChanges);
    const changes = unknown ? "?" : w.host.changes;
    const shown = this.tree ? "feed" : w.view;
    const directory = shortenHome(w.workingDirectory);
    // The top line spends its room in this order: the session and the chain, the switch of the views, the key of the
    // switch, then the directory. A part that finds no room is left out, and a session name that is still too long is
    // cut at its end.
    const chord = `${this.kitty ? "⌃" : "⌥"}1-3`;
    const toggle = views.reduce(
      (sum, view) =>
        sum +
        this.viewLabel(view).length +
        4 +
        (view === "changes" && changes ? String(changes).length + 1 : 0),
      0,
    );
    const head = Bun.stringWidth(w.sessionName) + 3 + Bun.stringWidth(w.label);
    const room = this.feedWidth - space.between;
    const hint = head + toggle + chord.length + 2 <= room;
    const folder = head + 3 + Bun.stringWidth(directory) + toggle + (hint ? chord.length + 2 : 0) <= room;
    const name = clip(w.sessionName, Math.max(8, room - toggle - 3 - Bun.stringWidth(w.label)));
    if (this.paneChanged(this.toggle, [shown, changes, this.kitty, hint, this.theme])) {
      this.clear(this.toggle);
      this.switcher(
        this.toggle,
        views.map((view) => ({
          label: this.viewLabel(view),
          badge: view === "changes" && changes ? String(changes) : "",
          color: c.accent,
          run: () => this.showView(view),
        })),
        views.indexOf(shown),
        // F1 lists the chord where the top line has no room for it.
        hint ? chord : "",
        c.panel,
      );
    }
    if (this.paneChanged(this.headline, [name, w.label, folder && directory, this.theme])) {
      this.clear(this.headline);
      this.headline.add(
        this.line([[name, c.text, bold]], c.text, { run: () => void this.workspacePicker() }),
      );
      this.headline.add(this.line(` ${glyph.crumb} `, c.faint));
      this.headline.add(this.line(w.label, c.muted, { run: () => this.chains() }));
      if (folder)
        this.headline.add(
          this.line(`   ${directory}`, c.faint, {
            truncate: true,
            flexShrink: 1,
            run: () => this.showValue("Directory", w.workingDirectory),
          }),
        );
    }
  },

  /** Where the input goes, under it: its mode, its chain, and for a prompt the model, the effort, and the type of its
   * answer. Each part is a button that changes it. */
  renderMeta(mode: string, modeColor: RGBA): void {
    const w = this.session;
    const pending = w.operatorPrompt;
    const { model, effort } = w.actorChoice;
    const { provider, id } = modelName(model);
    // With no model that answers, the actor falls to the operator, which the line names as no model, with no effort.
    const none = id === "operator";
    const name = none ? "no model" : id;
    const stash = w.stashes[w.draftKey];
    const reading = Boolean(w.host.reading);
    const prompt = w.mode === "prompt" && !w.editing && !pending && !reading;
    // A message to a thread names the thread where the chain stands otherwise, in the color of the thread.
    const target = prompt ? w.target : undefined;
    const title = target
      ? clip(shortenHomes(gist(this.railThreads().find((one) => one.act.id === target)?.title ?? target)), 20)
      : "";
    const where = target ? `to ${glyph.reply} ${title}` : `on ${w.label}`;
    // The line spans the input but its bar and its padding.
    const room = this.feedWidth - 3;
    if (
      !this.paneChanged(this.meta, [
        mode,
        w.label,
        name,
        provider,
        effort,
        w.shape,
        pending?.id,
        pending?.shape,
        stash,
        prompt,
        reading,
        target,
        title,
        room,
        this.theme,
      ])
    )
      return;
    this.clear(this.meta);
    // A narrow input leaves out, in this order, the provider, the type of the answer, the effort, and the chord of the
    // switch, and the stash shows less of what waits in it.
    const edited = Boolean(w.editing || pending || reading);
    const shown = {
      chord: !edited,
      provider: prompt && Boolean(provider) && !none,
      shape: prompt,
      effort: prompt && !none,
    };
    const gaps = 3;
    const width = () =>
      (edited ? Bun.stringWidth(mode) + 4 : "Prompt".length + "Python".length + 8) +
      gaps +
      (shown.chord ? 4 : 0) +
      Bun.stringWidth(where) +
      (target ? gaps + "+ New thread".length : 0) +
      (pending ? 2 * gaps + Bun.stringWidth(`answers ${pending.id}returns ${pending.shape}`) : 0) +
      (prompt ? gaps + Bun.stringWidth(name) : 0) +
      (shown.provider ? provider.length + 1 : 0) +
      (shown.effort ? gaps + Bun.stringWidth(`${effort} effort`) : 0) +
      (shown.shape ? gaps + Bun.stringWidth(`returns ${w.shape}`) : 0) +
      // The stash takes a gap, its word, its quotes and their space, and its key, and its preview takes the rest.
      (stash ? gaps + "stashed “”  ⌃S".length : 0);
    for (const part of ["provider", "shape", "effort", "chord"] as const)
      if (width() > room) shown[part] = false;
    const gap = () => this.meta.add(this.text("   ", c.faint));
    const button = (parts: Part[], run?: () => void) => this.meta.add(this.line(parts, c.muted, { run }));
    // The mode of the input is a switch between a prompt and Python, and an answer or a program that the input edits
    // is a switch of one segment, which only its key leaves.
    this.clear(this.modeBox);
    if (edited) this.switcher(this.modeBox, [{ label: mode, color: modeColor }], 0, "", c.raised);
    else
      this.switcher(
        this.modeBox,
        [
          { label: "Prompt", color: c.accent, run: () => w.mode === "prompt" || this.toggleMode() },
          { label: "Python", color: c.secondary, run: () => w.mode === "python" || this.toggleMode() },
        ],
        w.mode === "python" ? 1 : 0,
        shown.chord ? "⌃R" : "",
        c.raised,
      );
    if (target) {
      const tone = threadTone(target);
      button(
        [
          ["to ", c.faint],
          [`${glyph.reply} `, tone],
          [title, tone],
        ],
        () => this.replyTo(target),
      );
    } else
      button(
        [
          ["on ", c.faint],
          [w.label, c.muted],
        ],
        () => this.chains(),
      );
    if (pending) {
      gap();
      button(
        [
          ["answers ", c.faint],
          [pending.id, c.muted],
        ],
        () => this.question(pending),
      );
      gap();
      button([
        ["returns ", c.faint],
        [pending.shape, c.muted],
      ]);
    } else if (prompt) {
      gap();
      button(
        [
          [name, none ? c.danger : c.text],
          [shown.provider ? ` ${provider}` : "", c.faint],
        ],
        () => this.models(),
      );
      if (shown.effort) {
        gap();
        button(
          [
            [effort, c.muted],
            [" effort", c.faint],
          ],
          () => this.effortPicker(),
        );
      }
      if (shown.shape) {
        gap();
        button(
          [
            ["returns ", c.faint],
            [w.shape, c.muted],
          ],
          this.shapes,
        );
      }
    }
    if (stash) {
      this.meta.add(this.box({ flexGrow: 1 }));
      // The stash shows the start of what waits in it, as much as the line has room for, and the key that brings it
      // back.
      const first = stash.split("\n")[0] ?? "";
      const preview = Math.min(28, room - width());
      const more = first.length < stash.length && Bun.stringWidth(first) <= preview ? " …" : "";
      button(
        [
          ["stashed ", c.faint],
          [preview >= 6 ? `“${clip(first, preview)}${more}”  ` : "", c.muted],
          ["⌃S", c.faint],
        ],
        () => this.stash(),
      );
    }
    // A message that goes to a thread offers the way back to a new thread, at the right end of the line, in the blue of
    // the line of the input in a new thread.
    if (target) {
      if (!stash) this.meta.add(this.box({ flexGrow: 1 }));
      else gap();
      button([["+ New thread", c.accent]], () => {
        w.aim();
        this.render();
      });
    }
  },

  /** The state of the session and the keys that act on it, in the footer. A notice stands after the state until it
   * ends, and each key is a button that does what it says. */
  /** The tokens of the chain shown, which the tip of the footer says whole: the whole prompt of its last answer, and
   * what every answer read past the cache, wrote, and read and wrote in the cache, each token counted once. */
  tokens(): string {
    const { spend, context } = this.session;
    return [
      context === undefined ? "" : `last prompt ${count(context)}`,
      spend.input ? `fresh input ${count(spend.input)}` : "",
      spend.output ? `output ${count(spend.output)}` : "",
      spend.cacheRead ? `cache read ${count(spend.cacheRead)}` : "",
      spend.cacheWrite ? `cache write ${count(spend.cacheWrite)}` : "",
    ]
      .filter(Boolean)
      .join(", ");
  },

  renderStatus(): void {
    const w = this.session;
    // A refresh follows each fact, so the footer says Loading only while the chain has nothing to show yet.
    const loading = w.loading && !w.turns.length;
    const status = loading ? "opening" : w.status(w.selected);
    // A call of a model that waits to be made again counts its wait down in the footer, on the selected chain first.
    const retries = [...w.host.streams.values()].filter((stream) => stream.retry);
    const retry = loading
      ? undefined
      : (retries.find((stream) => stream.chain === w.selected) ?? retries[0])?.retry;
    const moving = status === "working" || status === "opening" || Boolean(retry);
    this.statusMoves = moving;
    const [mark, color] = retry ? [`${glyph.held} `, c.warning] : statusMark(status);
    const label = loading
      ? "Loading"
      : retry
        ? retryLine(retry)
        : w.host.reading
          ? readOnly(w.host.reading)
          : w.host.lost
            ? "The life is not open"
            : status === "error"
              ? w.damagedJournal
                ? "The journal of file changes is damaged"
                : w.error
                  ? "The view could not load"
                  : "The last act failed"
              : statusLabels[status];
    // The footer offers the keys of the top layer that offers any, each of which does what its key does, and under
    // the layer of the table, the wake of a paused chain.
    type Offer = readonly [chord: string, action: string, run: () => void];
    const text = (value: string | (() => string)) => (typeof value === "function" ? value() : value);
    const offered = this.keymap
      .getActiveKeys({ includeMetadata: true })
      .flatMap(({ bindingAttrs, command }) => {
        const hint = bindingAttrs?.hint as readonly [...Hint, group: string] | undefined;
        return hint && typeof command === "string" ? [{ hint, command }] : [];
      });
    const group = offered[0]?.hint[2];
    const keys: readonly Offer[] = [
      ...(w.host.reading ? ([["/new", "Start a new life", () => this.startFrom()]] as readonly Offer[]) : []),
      ...(group === "" && w.paused
        ? ([["/wake", "wake the chain", () => this.action("/wake")]] as const)
        : []),
      ...offered
        .filter(({ hint }) => hint[2] === group)
        .map(
          ({ hint: [chord, action], command }): Offer => [
            text(chord),
            text(action),
            () => void this.keymap.dispatchCommand(command),
          ],
        ),
    ];
    const notice = w.error ? "" : w.notice;
    // The state stays in the footer, and a notice stands after it, cut at its end where the footer has no room for it
    // beside the keys.
    const state: Part[] = [
      [moving && !retry ? `${spin()} ` : mark, color],
      [label, moving ? c.text : c.muted],
    ];
    // The keys that have no room beside the whole state leave the footer from the last one, so the state is never cut.
    const shown = [...keys];
    const buttonsOf = (list: typeof keys) =>
      list.map(([chord, action], index): Part[] => [
        [index ? "   " : "", c.faint],
        [chord, c.muted],
        [` ${action}`, c.faint],
      ]);
    let buttons = buttonsOf(shown);
    const stateWidth = Bun.stringWidth(plain(state)) + space.between * 2;
    while (shown.length && Bun.stringWidth(plain(buttons.flat())) + stateWidth > this.feedWidth) {
      shown.pop();
      buttons = buttonsOf(shown);
    }
    const hints = buttons.flat();
    const room =
      this.feedWidth - Bun.stringWidth(plain(hints)) - Bun.stringWidth(plain(state)) - space.between * 2;
    const tokens = this.tokens();
    this.statusWhole = [plain(state), notice, tokens && `Tokens: ${tokens}.`].filter(Boolean).join("   ");
    if (notice && room >= 8)
      state.push(
        ["   ", c.text],
        [clip(notice, room - 3), [exitNotice, rewindNotice].includes(notice) ? c.warning : c.text],
      );
    const key = JSON.stringify([plain(state), plain(hints), color, this.theme]);
    if (key === this.statusKey) return;
    this.statusKey = key;
    this.status.content = styled(state);
    this.clear(this.hints);
    for (const [index, [, , run]] of shown.entries())
      this.hints.add(this.line(buttons[index] ?? [], c.faint, { run }));
  },
});

export type ChromePart = typeof chrome;

import type { RGBA } from "@opentui/core";
import { gist, wants } from "furb/life";
import { shortenHomes } from "../files.ts";
import { clip } from "../format.ts";
import { type Line as PartLine, type Tone as PartTone, plainError, type Row, type Seen } from "../parts.ts";
import type { ActRow } from "../session.ts";
import { theme as c, glyph, spacing as space } from "../theme.ts";
import { bold, type Part } from "../ui.ts";
import { part } from "./part.ts";

export const desk = part({
  partsChanged(): void {
    this.partsLoaded++;
    if (this.closed) return;
    this.partKeysOff?.();
    const keys = this.parts.loaded.flatMap((part) => part.keys);
    // A key of a part stands under every key of the TUI, and one with no modifier acts on an empty input alone.
    this.partKeysOff = keys.length
      ? this.keymap.registerLayer({
          priority: -1000,
          enabled: () => !this.renaming?.input?.focused && !this.overlay,
          commands: keys.map(({ on, run }, index) => ({
            name: `part-${index}`,
            run: () => {
              this.partRun(() => run(this.seen()));
              return true;
            },
            enabled: () => /^(ctrl|meta|alt|super)\+/.test(on) || !this.composer.plainText,
          })),
          bindings: keys.map(({ on }, index) => ({ key: on, cmd: `part-${index}` })),
        })
      : undefined;
    // The cards are made again only when a part draws the rows of a kind, before this load or after it.
    const rows = this.parts.loaded.some((part) => part.rows.size);
    if (rows || this.partRows) {
      this.clear(this.scroll);
      this.cards.clear();
      this.paneKeys.delete(this.scroll);
    }
    this.partRows = rows;
    this.dockKey = "";
    this.schedule();
  },

  /** Runs what a part does, and shows in the footer the error of one that fails. */
  partRun(run: () => unknown): void {
    const failed = (error: unknown) => {
      this.session.notice = `A part of an extension failed: ${plainError(error)}`;
      this.schedule();
    };
    try {
      const got = run();
      if (got instanceof Promise) got.catch(failed);
    } catch (error) {
      failed(error);
    }
  },

  /** What a component of a part reads of the life, and what it can do in it. */
  seen(): Seen {
    const w = this.session;
    return {
      chain: w.selected,
      acts: w.acts,
      compactions: w.host.compactions,
      of: (kind) => w.acts.filter((act) => act.kind === kind),
      run: (word) => w.command(`/run ${word}`),
      say: (text) => {
        w.notice = text;
        this.schedule();
      },
    };
  },

  /** The runs of a line of a part, each in the color of its tone. */
  partLine(line: PartLine): Part[] {
    const tones: Record<PartTone, RGBA> = {
      text: c.text,
      muted: c.muted,
      faint: c.faint,
      accent: c.accent,
      success: c.success,
      warning: c.warning,
      danger: c.danger,
    };
    if (typeof line === "string") return [[line, c.text]];
    return line.map(([text, tone]) => [String(text), tones[tone ?? "text"] ?? c.text]);
  },

  /** The row that a part draws for an act of its kind, with its error as its label when it throws. */
  partRow(act: ActRow): Row | undefined {
    const draw = this.parts.row(act.kind);
    if (!draw) return undefined;
    try {
      return draw(act, this.seen()) ?? {};
    } catch (error) {
      return { label: `its row failed: ${plainError(error)}` };
    }
  },

  /** The panels of the parts over the input, while the feed shows: each panel of a part that loaded, and the error of
   * a part that did not load or of a panel that throws, in plain words in its own panel. A panel shows twelve lines at
   * most. */
  renderDock(): void {
    const w = this.session;
    const shown: { title: string; lines: readonly PartLine[]; error?: string }[] = [];
    if (w.view === "feed" && !this.tree) {
      const seen = this.seen();
      for (const part of this.parts.loaded) {
        if (part.error) shown.push({ title: part.name, lines: [], error: part.error });
        for (const panel of part.panels)
          try {
            const lines = panel.draw(seen);
            if (!Array.isArray(lines)) throw new Error("Its panel gives no list of lines.");
            shown.push({ title: panel.name, lines });
          } catch (error) {
            shown.push({ title: panel.name, lines: [], error: plainError(error) });
          }
      }
    }
    const visible = shown.length > 0;
    if (this.dock.visible !== visible) this.dock.visible = visible;
    if (!visible) {
      this.dockKey = "";
      return;
    }
    const key = JSON.stringify([shown, this.feedWidth, this.theme]);
    if (key === this.dockKey) return;
    this.dockKey = key;
    this.dock.backgroundColor = c.panel;
    this.clear(this.dock);
    for (const one of shown) {
      const box = this.box();
      const mark: Part[] = one.error ? [[`${glyph.failed} `, c.danger]] : [];
      box.add(this.line([...mark, [one.title, c.text, bold]], c.text, { truncate: true }));
      if (one.error) box.add(this.text(one.error, c.danger));
      const lines = one.lines.slice(0, 12);
      for (const line of lines) box.add(this.line(this.partLine(line), c.text, { truncate: true }));
      if (one.lines.length > lines.length)
        box.add(this.line(`${one.lines.length - lines.length} more lines`, c.faint));
      this.dock.add(box);
    }
  },

  renderDesk(): void {
    const w = this.session;
    const { asks, questions } = w.desk;
    const ask = w.aimed;
    const visible = asks.length > 0 && !this.tree;
    if (this.deskBox.visible !== visible) this.deskBox.visible = visible;
    if (!visible) {
      this.deskKey = "";
      return;
    }
    const key = JSON.stringify([asks, ask?.id, w.selected, this.feedWidth, this.theme]);
    if (key === this.deskKey) return;
    this.deskKey = key;
    this.deskBox.backgroundColor = c.panel;
    this.clear(this.deskBox);
    const room = this.feedWidth - space.inset * 2;
    const head = this.row();
    const marks = this.row({ gap: 1 });
    head.add(marks);
    for (const one of asks)
      marks.add(
        this.text(glyph.asks, one.id === ask?.id ? c.warning : c.faint, {
          run: () => void w.point(one.id).catch(this.report),
        }),
      );
    const count = `${questions} ${questions === 1 ? "question" : "questions"}`;
    if (!ask) {
      head.add(
        this.line(
          [
            ["  ", c.faint],
            [`${count} ${asks.length === 1 ? "waits" : "wait"} on other chains`, c.text, bold],
            ["  ⌃A or a mark goes to the first", c.faint],
          ],
          c.text,
          { truncate: true, flexShrink: 1, run: () => this.question() },
        ),
      );
      this.deskBox.add(head);
      return;
    }
    const place = `${asks.findIndex((one) => one.id === ask.id) + 1} of ${asks.length}`;
    const wanted = `wants ${wants(ask)}`;
    head.add(
      this.line(
        [
          ["  ", c.faint],
          ["Question", c.text, bold],
          [` ${place}`, c.muted],
          ["  from ", c.faint],
          [ask.label, c.text],
          [`  ${ask.id}  `, c.faint],
          [wanted, c.warning],
        ],
        c.text,
        { truncate: true, flexShrink: 1 },
      ),
    );
    this.deskBox.add(head);
    const lines = ask.message.split("\n").filter((line) => line.trim()).length;
    const more = lines > 1 ? `  +${lines - 1} ${lines === 2 ? "line" : "lines"}` : "";
    this.deskBox.add(
      this.line(
        [
          [clip(shortenHomes(gist(ask.message)), room - Bun.stringWidth(more)), c.text],
          [more, c.faint],
        ],
        c.text,
        { truncate: true, run: () => this.question(ask) },
      ),
    );
    const act = this.row({ gap: space.between });
    if (ask.choices.length && ask.id === this.picking()?.id)
      act.add(
        this.text("Choose below with ↑↓ and Enter, or its number, or Other for your own words", c.faint),
      );
    else if (ask.choices.length) {
      for (const choice of ask.choices)
        act.add(
          this.line([[choice, c.warning, bold]], c.warning, {
            run: () => void w.answer(ask, choice).catch(this.report),
          }),
        );
      act.add(this.text("or type it below", c.faint));
    } else
      act.add(
        this.line(
          [
            ["Enter sends the input to ", c.faint],
            [ask.id, c.muted],
            [` as ${ask.shape}`, c.faint],
            [asks.length > 1 ? "   ⌃A opens each in turn" : "   ⌃A opens it", c.faint],
          ],
          c.faint,
          { truncate: true, flexShrink: 1 },
        ),
      );
    this.deskBox.add(act);
  },
});

export type DeskPart = typeof desk;

export const deskBound = ["partsChanged"] as const;

import type { BoxRenderable, RGBA } from "@opentui/core";
import type { ProgramLine } from "furb/life";
import { theme as c, glyph, spacing as space } from "../theme.ts";
import { part } from "./part.ts";
import { noBorder } from "./shared.ts";

/** How many rows a window of code shows until a click opens it: the first, the five after it, and the last. */
export const WINDOW = 7;
/** The most lines that an open window holds, and the rows of it that show before it scrolls. */
export const OPENED = 400;
export const SCROLL = 24;

export const thread = part({
  /** The lines of a run of code as its window shows them: all of them while they fit, else seven rows. The first row is
   * the first line, or a ... once the window rolled. The line that runs stands second to last, and the last row is a
   * ... while more lines come after it, else the row where the model writes. A window that was opened shows the last
   * lines of the run, without the ... rows. */
  rolling(
    given: readonly ProgramLine[],
    key: string,
    writing: boolean,
  ): { lines: ProgramLine[]; opened: boolean } {
    // A jump to an act of the thread opens each window of the thread, which the key of the thread alone holds.
    const folds = this.session.folds;
    const opened = folds[`window:${key}`] === false || folds[`window:${key.split(":")[0]}`] === false;
    const n = given.length;
    if (opened) return { lines: given.slice(-OPENED), opened };
    if (n + (writing ? 1 : 0) <= WINDOW) return { lines: [...given], opened };
    const base = given[0];
    if (!base) return { lines: [], opened };
    const gap = (at: ProgramLine | undefined, count: number): ProgramLine => ({
      kind: "earlier",
      step: (at ?? base).step,
      count,
      text: "...",
    });
    const runs = given.findLastIndex((line) => line.kind === "code" && line.runs);
    const anchor = runs < 0 ? n - 1 : runs;
    let body = WINDOW - 1;
    let start = Math.max(0, anchor - (body - 1));
    const more = start + body < n;
    if (!more && !writing) {
      body = WINDOW;
      start = Math.max(0, n - body);
    }
    const shown = given.slice(start, start + body);
    const lines = [...shown];
    if (start > 0) lines[0] = gap(shown[0], start + 1);
    if (more) lines.push(gap(shown.at(-1), n - start - body));
    return { lines, opened };
  },

  /** A window of code opens to its lines, or rolls up again. */
  openWindow(key: string): void {
    const w = this.session;
    if (w.folds[`window:${key}`] === false) delete w.folds[`window:${key}`];
    else w.folds[`window:${key}`] = false;
    w.save();
    this.renderContent();
  },

  /** A part of the box of a thread. A message of the operator has a line of the color of the thread at its left, which
   * starts half a row above it and ends half a row under it, as the line of a panel does, so that it stands out of the
   * rest. The half row lies in the row that parts the message from the part above or under it, which the message
   * holds, or in the edge of the box where the message stands first or last, so the line adds no row. Any other part has
   * no line, and its text starts at the column of the text of a message. A part that a blank row parts from the one
   * above it holds that row. */
  barred(box: BoxRenderable, color?: RGBA, gap = 0, ends = { top: true, bottom: true }): BoxRenderable {
    if (!color) {
      const plain = this.box({ paddingLeft: space.inset + 1, ...(gap ? { paddingTop: gap } : {}) });
      box.add(plain);
      return plain;
    }
    const cap = (side: "top" | "bottom") =>
      this.box({
        height: space.bar,
        border: ["left"],
        borderColor: color,
        customBorderChars: { ...noBorder, vertical: side === "top" ? glyph.barTop : glyph.barBottom },
      });
    const part = this.box({
      border: ["left"],
      borderColor: color,
      customBorderChars: { ...noBorder, vertical: glyph.bar },
      paddingLeft: space.inset,
    });
    if (ends.top) box.add(cap("top"));
    box.add(part);
    if (ends.bottom) box.add(cap("bottom"));
    return part;
  },

  /** Whether a thread that stands folded has new work since it was last seen open. A thread that stands open is seen. */
  unread(id: string, seen: string, folded: boolean): boolean {
    const known = this.seenThreads.get(id);
    // The work that came before the thread folded was seen, whoever folded it.
    if (!folded || !known?.folded) {
      this.seenThreads.set(id, { seen, folded });
      return false;
    }
    return known.seen !== seen;
  },

  /** The row at the end of a thread, a button that makes the thread the target of the input: the input then sends its
   * messages to it, and a message to a thread that is over opens it again. While the thread is the target, the row
   * says so in the color of the thread, the color of the line of the input. */
  replyRow(box: BoxRenderable, id: string, over: boolean, tone: RGBA): void {
    const aimed = this.session.target === id;
    const label = aimed ? "Replying here" : over ? "Reply to open it again" : "Reply";
    // The mark stands in the column of the line of a message, so the word starts where the text of the input starts.
    const row = this.row({ backgroundColor: c.panel, marginLeft: -(space.inset + 1) });
    row.add(
      this.link(
        [
          [`${glyph.reply} `, aimed ? tone : c.faint],
          [label, aimed ? tone : c.muted],
        ],
        () => this.replyTo(id),
      ),
    );
    box.add(row);
  },
});

export type ThreadPart = typeof thread;

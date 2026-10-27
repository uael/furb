import { type BoxRenderable, InputRenderable, InputRenderableEvents, type RGBA } from "@opentui/core";
import type { ProgramLine } from "furb/life";
import { theme as c, glyph, spacing as space } from "../theme.ts";
import { part } from "./part.ts";
import { inputColors, noBorder } from "./shared.ts";

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

  /** A part of the box of a thread, with a line at its left: the color of the thread for a message of the operator, and
   * a quiet color for the rest, so that one line runs the height of the box. A part that a blank row parts from the one
   * above it holds that row, and the line goes through it. */
  barred(box: BoxRenderable, color: RGBA, gap = 0): BoxRenderable {
    const bar = this.box({
      border: ["left"],
      borderColor: color,
      customBorderChars: { ...noBorder, vertical: glyph.bar },
      paddingLeft: space.inset,
      ...(gap ? { paddingTop: gap } : {}),
    });
    box.add(bar);
    return bar;
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

  /** The input at the end of a thread: a message sent here goes to the thread, and opens it again when it is over. */
  replyBox(box: BoxRenderable, id: string, over: boolean): void {
    const w = this.session;
    const draft = `reply:${id}`;
    const input = new InputRenderable(this.renderer, {
      flexGrow: 1,
      flexShrink: 1,
      value: w.drafts[draft] ?? "",
      placeholder: over ? "Reply to open this thread again" : "Reply to this thread",
      ...inputColors(c.panel),
    });
    this.replyInputs.set(id, input);
    input.on(InputRenderableEvents.INPUT, (value: string) => {
      if (value) w.drafts[draft] = value;
      else delete w.drafts[draft];
    });
    input.on(InputRenderableEvents.ENTER, () => {
      const text = input.value;
      if (!text.trim()) return;
      input.value = "";
      delete w.drafts[draft];
      this.replyFocus = undefined;
      this.composer.focus();
      w.reply(id, text).catch((error) => {
        w.drafts[draft] = text;
        this.report(error);
      });
    });
    input.on("focused", () => {
      this.replyFocus = id;
    });
    input.on("blurred", () =>
      queueMicrotask(() => {
        if (!input.isDestroyed && this.replyFocus === id) this.replyFocus = undefined;
      }),
    );
    // The input starts where the text of the box starts, with the line of the thread at its left.
    input.onKeyDown = (key) => {
      if (key.name !== "up" || input.value) return;
      key.preventDefault();
      void w
        .takeBackMessage(id)
        .then((text) => {
          if (text === undefined || input.isDestroyed) return;
          input.value = text;
          input.cursorOffset = text.length;
          w.drafts[draft] = text;
        })
        .catch(this.report);
    };
    const row = this.row({ backgroundColor: c.panel, onMouseDown: () => input.focus() });
    row.add(input);
    box.add(row);
    if (this.replyFocus === id) queueMicrotask(() => !input.isDestroyed && input.focus());
  },
});

export type ThreadPart = typeof thread;

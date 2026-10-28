import { type OptimizedBuffer, type RGBA, TextareaRenderable } from "@opentui/core";
import { clip } from "../format.ts";
import { wrapped } from "./shared.ts";

const HINT_ROWS = 3;

/** A text area that draws the grey rest of a line after its cursor, as fish does. The rest is no text of the area: it
 * is not in its buffer, its cursor never enters it, and it is cut at the end of the row. */
export class GhostTextarea extends TextareaRenderable {
  /** The rest of the line. Only its first line shows, with an ellipsis where more lines follow. */
  ghost = "";
  /** The color of the rest, which a theme can change. */
  tint?: () => RGBA;
  /** What the empty area asks for, which the area draws in the rows of its width. The placeholder of the area wraps at
   * the width it had when it was set, before the layout gave it its own, so it is not used. */
  hint = "";
  /** Runs when the area takes a width, since the rows of its hint depend on it. */
  onHintRows?: () => void;

  /** The rows of the hint at a width: none once the area holds text, at most three, the last cut where the hint goes on. */
  hintRows(width: number): string[] {
    return !this.plainText && this.hint && width > 0 ? wrapped(this.hint, width, HINT_ROWS) : [];
  }

  override onResize(width: number, height: number): void {
    super.onResize(width, height);
    this.onHintRows?.();
  }

  protected override renderSelf(buffer: OptimizedBuffer): void {
    super.renderSelf(buffer);
    this.hintRows(this.width).forEach((row, at) => {
      buffer.drawText(row, this._screenX, this._screenY + at, this.placeholderColor);
    });
    if (!this.ghost || !this.tint) return;
    const cursor = this.visualCursor;
    const x = this._screenX + cursor.visualCol;
    const y = this._screenY + cursor.visualRow - this.scrollY;
    const room = this._screenX + this.width - x;
    if (room < 1 || y < this._screenY || y >= this._screenY + this.height) return;
    const [first = "", ...more] = this.ghost.split("\n");
    buffer.drawText(clip(first.replaceAll("\t", "  ") + (more.length ? "…" : ""), room), x, y, this.tint());
  }
}

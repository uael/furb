import { type OptimizedBuffer, type RGBA, TextareaRenderable } from "@opentui/core";
import { clip } from "../format.ts";

/** A text area that draws the grey rest of a line after its cursor, as fish does. The rest is no text of the area: it
 * is not in its buffer, its cursor never enters it, and it is cut at the end of the row. */
export class GhostTextarea extends TextareaRenderable {
  /** The rest of the line. Only its first line shows, with an ellipsis where more lines follow. */
  ghost = "";
  /** The color of the rest, which a theme can change. */
  tint?: () => RGBA;

  protected override renderSelf(buffer: OptimizedBuffer): void {
    super.renderSelf(buffer);
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

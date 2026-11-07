import {
  type LineInfoProvider,
  type OptimizedBuffer,
  Renderable,
  type RenderableOptions,
  type RenderContext,
  type RGBA,
} from "@opentui/core";
import { theme as c, pulse, spin } from "../theme.ts";
import { mix, type Part } from "../ui.ts";
import { gutterWidth, STATE } from "./shared.ts";

/** The code that a gutter numbers: a node that says which line of its content each row shows. */
export type Numbering = Renderable & LineInfoProvider;

export interface GutterOptions extends RenderableOptions<Gutter> {
  /** The code, which stands at the right of the gutter. */
  target: Numbering;
  /** The least count of digits of the column of the numbers, so that the blocks of one thread share one column. */
  digits?: number;
  /** The number of each line that shows one. With no map, each line shows its place plus the offset. */
  numbers?: ReadonlyMap<number, number>;
  /** What the first line counts from when no map gives the numbers. */
  offset?: number;
  /** The color of the numbers. It is read at each frame, so a new palette takes it at once. */
  fg?: () => RGBA;
  /** The light of each line, over its gutter and its code. */
  colors?: ReadonlyMap<number, RGBA>;
  /** The mark of each line in the column of the states, as the line that failed. */
  states?: ReadonlyMap<number, Part>;
  /** The lines whose cell of the state turns the loader, as the line that a model writes. */
  loaders?: ReadonlySet<number>;
  /** The lines that run, whose number fades out into the loader and back. */
  running?: ReadonlySet<number>;
}

/** The gutter of a block of code, which furb draws in each cell that it owns: at the left edge the number of each line
 * in a column of its own, then one blank cell, then the column of the states, then one blank cell before the code. The number of a line that
 * runs fades out into the loader and back, and a line that a model writes turns the loader in its state. The gutter
 * reads the time at each frame, so a tick only asks for a frame. The code stands inside the box, past the cells of the
 * gutter, so the pointer over the gutter reaches the box. */
export class Gutter extends Renderable {
  readonly target: Numbering;
  private readonly digits: number;
  private _numbers?: ReadonlyMap<number, number>;
  private readonly offset: number;
  private readonly fg: () => RGBA;
  private readonly colors: ReadonlyMap<number, RGBA>;
  private readonly states: ReadonlyMap<number, Part>;
  private readonly loaders: ReadonlySet<number>;
  private readonly running: ReadonlySet<number>;
  private readonly moved = () => {
    this.fit();
    this.requestRender();
  };

  constructor(ctx: RenderContext, options: GutterOptions) {
    const { target, digits, numbers, offset, fg, colors, states, loaders, running, ...rest } = options;
    super(ctx, { ...rest, flexDirection: "row", height: "auto" });
    this.target = target;
    this.digits = digits ?? 1;
    this._numbers = numbers;
    this.offset = offset ?? 0;
    this.fg = fg ?? (() => c.faint);
    this.colors = colors ?? new Map();
    this.states = states ?? new Map();
    this.loaders = loaders ?? new Set();
    this.running = running ?? new Set();
    target.flexGrow = 1;
    target.flexShrink = 1;
    this.add(target);
    target.on("line-info-change", this.moved);
    this.fit();
  }

  /** The number of each line that shows one. */
  set numbers(numbers: ReadonlyMap<number, number>) {
    this._numbers = numbers;
    this.moved();
  }

  /** The cells of the column of the numbers: two at least, so the code of each thread of under a hundred lines starts at
   * one column. */
  get column(): number {
    const most = this._numbers ? Math.max(0, ...this._numbers.values()) : this.target.lineCount + this.offset;
    return Math.max(2, this.digits, String(most).length);
  }

  /** The cells of the whole gutter, which the box keeps at its left for it. */
  get cells(): number {
    return gutterWidth(this.column) + STATE;
  }

  private fit(): void {
    if (this.paddingLeft !== this.cells) this.paddingLeft = this.cells;
  }

  private numberOf(line: number): number | undefined {
    return this._numbers ? this._numbers.get(line) : line + 1 + this.offset;
  }

  override destroyRecursively(): void {
    this.target.off("line-info-change", this.moved);
    super.destroyRecursively();
  }

  protected override renderSelf(buffer: OptimizedBuffer): void {
    const y = Math.trunc(this.y);
    const start = Math.max(0, -y);
    const end = Math.min(this.height, buffer.height - y);
    if (end <= start || this.x >= buffer.width || this.x + this.width <= 0) return;
    const top = Math.trunc(this.target.scrollY) + start;
    // The row above the first one tells whether that one continues a line that wraps, which shows no number.
    const from = Math.max(0, top - 1);
    const sources = this.target.getLineSources
      ? this.target.getLineSources(from, end - start + top - from)
      : this.target.lineInfo.lineSources.slice(from, end + top - from);
    const fg = this.fg();
    const column = this.column;
    let last = top > from ? sources[0] : -1;
    for (let row = 0; row + top - from < sources.length && row < end - start; row++) {
      const line = sources[row + top - from] ?? -1;
      const at = y + start + row;
      const light = this.colors.get(line);
      if (light) buffer.fillRect(this.x, at, this.width, 1, light);
      if (line !== last) {
        const number = this.numberOf(line);
        if (number !== undefined) {
          const [loader, share] = this.running.has(line) ? pulse() : [false, 1];
          const text = loader ? spin() : String(number);
          const color =
            share < 1 ? mix(light ?? c.panel, loader ? c.accent : fg, share) : loader ? c.accent : fg;
          buffer.drawText(text, this.x + column - Bun.stringWidth(text), at, color, light);
        }
        const [mark, tone] = this.states.get(line) ?? (this.loaders.has(line) ? [spin(), c.accent] : [""]);
        if (mark) buffer.drawText(mark, this.x + gutterWidth(column), at, tone ?? fg, light);
      }
      last = line;
    }
  }
}

import {
  type LineNumberOptions,
  LineNumberRenderable,
  type OptimizedBuffer,
  type Renderable,
} from "@opentui/core";

/** The numbers of the lines of a block of code at its right. OpenTUI puts the numbers at the left of the code and
 * lights a line of the code from the end of that gutter, so a gutter at the right lights its line from the left edge. */
export class RightNumbered extends LineNumberRenderable {
  constructor(renderer: ConstructorParameters<typeof LineNumberRenderable>[0], options: LineNumberOptions) {
    super(renderer, options);
    this.flexDirection = "row-reverse";
  }

  protected override renderSelf(buffer: OptimizedBuffer): void {
    const inner = this as unknown as {
      target:
        | (Renderable & {
            scrollY: number;
            getLineSources?: (from: number, count: number) => number[];
            lineInfo: { lineSources: number[] };
          })
        | null;
      gutter: Renderable | null;
    };
    const target = inner.target;
    const lit = this.getLineColors().content;
    if (!target || !inner.gutter || lit.size === 0) return;
    const y = Math.trunc(this.y);
    const start = Math.max(0, -y);
    const end = Math.min(this.height, buffer.height - y);
    if (end <= start || this.x >= buffer.width || this.x + this.width <= 0) return;
    const from = Math.trunc(target.scrollY) + start;
    const sources = target.getLineSources
      ? target.getLineSources(from, end - start)
      : target.lineInfo.lineSources.slice(from, from + end - start);
    const width = this.width - (inner.gutter.visible ? inner.gutter.width : 0);
    for (const [at, line] of sources.entries()) {
      const color = lit.get(line);
      if (color) buffer.fillRect(this.x, y + start + at, width, 1, color);
    }
  }
}

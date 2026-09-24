import { join } from "node:path";
import type { CapturedFrame, RGBA } from "@opentui/core";
import { Resvg } from "@resvg/resvg-js";
import UPNG from "upng-js";

/** The pixels of a cell, the size of the text, and the gap from the top of a row to its baseline, at one scale. */
const cell = 9,
  row = 20,
  size = 15,
  baseline = 15.2;
/** The inset of the terminal in its window, the height of the title bar, and the room around the window for its
 * shadow. */
const inset = 14,
  bar = 34,
  margin = 28;
/** The scale of the image, which a screen of high density shows sharp. */
const scale = 2;
const fonts = ["Regular", "Bold", "Italic"].map((weight) =>
  join(import.meta.dir, "fonts", `GeistMono-${weight}.ttf`),
);

const hex = (value: RGBA) =>
  `#${value
    .toInts()
    .slice(0, 3)
    .map((part) => part.toString(16).padStart(2, "0"))
    .join("")}`;
const xml = (value: string) =>
  value.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;");

/** A block or a line of a box, which a terminal draws to the edges of its cell and a font leaves short of them: the
 * blocks it fills, as parts of the cell, and the path of the line through the center of the cell, with the weight of
 * the line. A corner is round. */
const drawn: Record<string, { blocks?: [number, number, number, number][]; line?: string; heavy?: boolean }> =
  {
    "█": { blocks: [[0, 0, 1, 1]] },
    "▀": { blocks: [[0, 0, 1, 0.5]] },
    "▄": { blocks: [[0, 0.5, 1, 0.5]] },
    "▌": { blocks: [[0, 0, 0.5, 1]] },
    "▐": { blocks: [[0.5, 0, 0.5, 1]] },
    "▎": { blocks: [[0, 0, 0.25, 1]] },
    "│": { line: "M c T V h" },
    "┃": { line: "M c T V h", heavy: true },
    "╻": { line: "M c m V h", heavy: true },
    "╹": { line: "M c T V m", heavy: true },
    "─": { line: "M L m H w" },
    "━": { line: "M L m H w", heavy: true },
    "╭": { line: "M c h V r Q c m s m H w" },
    "╮": { line: "M c h V r Q c m t m H L" },
    "╰": { line: "M c T V u Q c m s m H w" },
    "╯": { line: "M c T V u Q c m t m H L" },
    "└": { line: "M c T V m H w" },
    "├": { line: "M c T V h M c m H w" },
    "╋": { line: "M c T V h M L m H w", heavy: true },
  };

/** A mark of the TUI as a shape around the center of its cell, so that each mark has one weight and one size
 * whatever font the machine has: its SVG, from the center and the color. */
function mark(glyph: string, x: number, y: number, color: string): string | undefined {
  const stroke = (width: number, path: string) =>
    `<path d="${path}" stroke="${color}" stroke-width="${width}" stroke-linecap="round" stroke-linejoin="round" fill="none"/>`;
  const fill = (path: string) => `<path d="${path}" fill="${color}"/>`;
  switch (glyph) {
    case "●":
      return `<circle cx="${x}" cy="${y}" r="3.1" fill="${color}"/>`;
    case "○":
      return `<circle cx="${x}" cy="${y}" r="2.9" stroke="${color}" stroke-width="1.1" fill="none"/>`;
    case "◌":
      return Array.from({ length: 8 }, (_, index) => {
        const angle = (index / 8) * Math.PI * 2;
        return `<circle cx="${x + Math.cos(angle) * 3}" cy="${y + Math.sin(angle) * 3}" r="0.62" fill="${color}"/>`;
      }).join("");
    case "◆":
      return fill(`M ${x} ${y - 3.7} L ${x + 3.3} ${y} L ${x} ${y + 3.7} L ${x - 3.3} ${y} Z`);
    case "✓":
      return stroke(1.5, `M ${x - 3.1} ${y + 0.2} L ${x - 0.9} ${y + 2.6} L ${x + 3.2} ${y - 2.8}`);
    case "✗":
      return stroke(
        1.5,
        `M ${x - 2.7} ${y - 2.7} L ${x + 2.7} ${y + 2.7} M ${x + 2.7} ${y - 2.7} L ${x - 2.7} ${y + 2.7}`,
      );
    case "⊘":
      return `<circle cx="${x}" cy="${y}" r="3" stroke="${color}" stroke-width="1.1" fill="none"/>${stroke(1.1, `M ${x - 2.1} ${y + 2.1} L ${x + 2.1} ${y - 2.1}`)}`;
    case "❯":
      return stroke(1.7, `M ${x - 1.9} ${y - 3.5} L ${x + 2} ${y} L ${x - 1.9} ${y + 3.5}`);
    case "›":
      return stroke(1.3, `M ${x - 1.3} ${y - 2.6} L ${x + 1.5} ${y} L ${x - 1.3} ${y + 2.6}`);
    case "■":
      return `<rect x="${x - 3.4}" y="${y - 3.4}" width="6.8" height="6.8" rx="1.3" fill="${color}"/>`;
    case "▸":
      return fill(`M ${x - 1.8} ${y - 3} L ${x + 2.4} ${y} L ${x - 1.8} ${y + 3} Z`);
    case "▾":
      return fill(`M ${x - 3} ${y - 1.7} L ${x + 3} ${y - 1.7} L ${x} ${y + 2.5} Z`);
  }
  const code = glyph.codePointAt(0) ?? 0;
  // A braille cell holds eight dots in two columns, and each bit of the code point above U+2800 sets one of them.
  if (glyph.length === 1 && code > 0x2800 && code <= 0x28ff) {
    const dots = [
      [0, 0],
      [0, 1],
      [0, 2],
      [1, 0],
      [1, 1],
      [1, 2],
      [0, 3],
      [1, 3],
    ];
    return dots
      .filter((_, bit) => (code - 0x2800) & (1 << bit))
      .map(
        ([column = 0, line = 0]) =>
          `<circle cx="${x - 1.7 + column * 3.4}" cy="${y - 4.6 + line * 3.1}" r="0.95" fill="${color}"/>`,
      )
      .join("");
  }
  return undefined;
}

/** The cells of a frame as the SVG of a terminal. The inset around the cells takes the color of the cell beside it,
 * as a terminal that extends its padding shows it, so a sidebar reaches the edge of the window. */
function terminal(frame: CapturedFrame): string {
  const parts: string[] = [];
  const width = frame.cols * cell,
    last = frame.lines.length - 1;
  for (const [line, content] of frame.lines.entries()) {
    const first = content.spans[0],
      end = content.spans.at(-1);
    const bottom = line === last ? inset : 0;
    if (first)
      parts.push(
        `<rect x="${-inset}" y="${line * row}" width="${inset}" height="${row + bottom}" fill="${hex(first.bg)}"/>`,
      );
    if (end)
      parts.push(
        `<rect x="${width}" y="${line * row}" width="${inset}" height="${row + bottom}" fill="${hex(end.bg)}"/>`,
      );
    if (line === last) {
      let column = 0;
      for (const span of content.spans) {
        parts.push(
          `<rect x="${column * cell}" y="${(line + 1) * row}" width="${span.width * cell}" height="${inset}" fill="${hex(span.bg)}"/>`,
        );
        column += span.width;
      }
    }
  }
  for (const [line, content] of frame.lines.entries()) {
    let column = 0;
    const y = line * row;
    for (const span of content.spans) {
      const left = column * cell;
      parts.push(
        `<rect x="${left}" y="${y}" width="${span.width * cell}" height="${row}" fill="${hex(span.bg)}"/>`,
      );
      const color = hex(span.fg);
      // A run of text is set as one text, and a block, a line of a box, or a mark is drawn in its cell.
      let run = "",
        start = column,
        at = column;
      const flush = () => {
        if (run.trim()) {
          const width = Bun.stringWidth(run) * cell;
          parts.push(
            `<text x="${start * cell}" y="${y + baseline}" font-family="Geist Mono" font-size="${size}" font-weight="${span.attributes & 1 ? 700 : 400}" font-style="${span.attributes & 4 ? "italic" : "normal"}" fill="${color}" textLength="${width}" lengthAdjust="spacing" xml:space="preserve">${xml(run)}</text>`,
          );
          if (span.attributes & 8)
            parts.push(
              `<rect x="${start * cell}" y="${y + baseline + 2.4}" width="${width}" height="1" fill="${color}"/>`,
            );
        }
        run = "";
      };
      for (const { segment } of new Intl.Segmenter().segment(span.text)) {
        const shape = drawn[segment];
        const x = at * cell;
        const figure = shape ? undefined : mark(segment, x + cell / 2, y + row / 2 + 0.5, color);
        if (shape || figure) {
          flush();
          if (figure) parts.push(figure);
          for (const [bx, by, bw, bh] of shape?.blocks ?? [])
            parts.push(
              `<rect x="${x + bx * cell}" y="${y + by * row}" width="${bw * cell}" height="${bh * row}" fill="${color}"/>`,
            );
          if (shape?.line) {
            const radius = cell / 2;
            // The center of the cell, its edges, and the ends of the curve of a corner.
            const place: Record<string, number> = {
              c: x + cell / 2,
              m: y + row / 2,
              L: x,
              T: y,
              w: x + cell,
              h: y + row,
              r: y + row / 2 + radius,
              u: y + row / 2 - radius,
              s: x + cell / 2 + radius,
              t: x + cell / 2 - radius,
            };
            const path = shape.line
              .split(" ")
              .map((token) => (/^[MVHQ]$/.test(token) ? token : String(place[token])))
              .join(" ");
            parts.push(
              `<path d="${path}" stroke="${color}" stroke-width="${shape.heavy ? 2.4 : 1.2}" fill="none"/>`,
            );
          }
        } else {
          if (!run) start = at;
          run += segment;
        }
        at += Bun.stringWidth(segment);
      }
      flush();
      column += span.width;
    }
  }
  return parts.join("");
}

/** The colors of a window that the palette of a theme gives. */
type Window = { background: string; border: string; muted: string };

/** A frame as a PNG of a terminal window: the cells, a title bar with the title, and a soft shadow on a clear
 * ground. The palette gives the colors of the window. */
export function rasterize(frame: CapturedFrame, palette: Window, title: string): Buffer {
  const image = pixels(frame, palette, title, scale);
  // A terminal has few colors, and 256 of them keep every edge of its text while they make the file a third as large.
  return Buffer.from(UPNG.encode([image.pixels.buffer as ArrayBuffer], image.width, image.height, 256));
}

/** A frame as the pixels of a terminal window at a scale: its width, its height, and four bytes for each pixel. */
export function pixels(
  frame: CapturedFrame,
  palette: Window,
  title: string,
  zoom: number,
): { width: number; height: number; pixels: Uint8Array } {
  const inner = { width: frame.cols * cell, height: frame.rows * row };
  const window = { width: inner.width + inset * 2, height: inner.height + bar + inset };
  const width = window.width + margin * 2,
    height = window.height + margin * 2;
  const lights = ["#ff5f57", "#febc2e", "#28c840"]
    .map(
      (fill, index) =>
        `<circle cx="${margin + 20 + index * 20}" cy="${margin + bar / 2}" r="6" fill="${fill}"/>`,
    )
    .join("");
  const svg = [
    `<svg xmlns="http://www.w3.org/2000/svg" width="${width * zoom}" height="${height * zoom}" viewBox="0 0 ${width} ${height}">`,
    `<defs><filter id="shadow" x="-10%" y="-10%" width="120%" height="125%"><feDropShadow dx="0" dy="10" stdDeviation="12" flood-color="#000" flood-opacity="0.35"/></filter>`,
    `<clipPath id="window"><rect x="${margin}" y="${margin}" width="${window.width}" height="${window.height}" rx="12"/></clipPath></defs>`,
    `<rect x="${margin}" y="${margin}" width="${window.width}" height="${window.height}" rx="12" fill="${palette.background}" filter="url(#shadow)"/>`,
    `<g clip-path="url(#window)">`,
    lights,
    `<text x="${margin + window.width / 2}" y="${margin + bar / 2 + 4.5}" text-anchor="middle" font-family="Geist Mono" font-size="13" fill="${palette.muted}">${xml(title)}</text>`,
    `<g transform="translate(${margin + inset} ${margin + bar})">${terminal(frame)}</g>`,
    `</g>`,
    `<rect x="${margin + 0.5}" y="${margin + 0.5}" width="${window.width - 1}" height="${window.height - 1}" rx="11.5" fill="none" stroke="${palette.border}" stroke-opacity="0.9"/>`,
    `</svg>`,
  ].join("");
  const image = new Resvg(svg, {
    font: { fontFiles: fonts, loadSystemFonts: true, defaultFontFamily: "Geist Mono" },
  }).render();
  return { width: image.width, height: image.height, pixels: new Uint8Array(image.pixels) };
}

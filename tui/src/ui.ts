import { safeText } from "@furb/engine";
import { RGBA, StyledText, type TextChunk } from "@opentui/core";

/** One run of text in a line, with its color, its attributes, and the color behind it. */
export type Part = readonly [text: string, fg?: RGBA, attributes?: number, bg?: RGBA];
export const bold = 1,
  italic = 4,
  underline = 8;

/** An address of the web in a text, which ends before a space, a quote, a bracket, or a mark that ends a sentence. */
const address = /https?:\/\/[^\s"'`<>()[\]{}]+[^\s"'`<>()[\]{}.,;:!?]/g;

/** The parts as one styled line, each part made safe for the terminal. Each address of the web in a part is a link,
 * which a terminal that knows OSC 8 opens at a click. */
export function styled(parts: readonly Part[]): StyledText {
  const chunks: TextChunk[] = [];
  for (const [whole, fg, attributes, bg] of parts) {
    const text = safeText(whole);
    let at = 0;
    for (const found of text.matchAll(address)) {
      if (found.index > at)
        chunks.push({ __isChunk: true, text: text.slice(at, found.index), fg, bg, attributes });
      chunks.push({ __isChunk: true, text: found[0], fg, bg, attributes, link: { url: found[0] } });
      at = found.index + found[0].length;
    }
    if (at < text.length) chunks.push({ __isChunk: true, text: text.slice(at), fg, bg, attributes });
  }
  return new StyledText(chunks);
}

/** The text of the parts, with no style. */
export function plain(parts: readonly Part[]): string {
  return parts.map(([text]) => text).join("");
}

/** A color between two colors, at a share of the way from the first to the second. */
export function mix(from: RGBA, to: RGBA, share: number): RGBA {
  // A default color of the terminal has no value to mix, so the nearer of the two stands.
  if (from.intent !== "rgb" || to.intent !== "rgb") return share < 0.5 ? from : to;
  const at = (one: number, other: number) => one + (other - one) * share;
  return RGBA.fromValues(at(from.r, to.r), at(from.g, to.g), at(from.b, to.b), 1);
}

/** The name of furb drawn in pixels, two to a row of the terminal: a stem is two columns wide and a bar is half a
 * row high, so that each letter keeps one weight. */
const letters = [
  "...###                  ##    ",
  "..##                    ##    ",
  "######  ##  ##  ## ###  ##### ",
  "..##    ##  ##  ####    ##  ##",
  "..##    ##  ##  ##      ##  ##",
  "..##    ##  ##  ##      ##  ##",
  "..##    ##  ##  ##      ##  ##",
  "..##     #####  ##      ##### ",
];
/** The logo of furb: its pixels as rows of half blocks. */
export const logo = Array.from({ length: letters.length / 2 }, (_, row) => {
  const top = letters[row * 2] ?? "",
    bottom = letters[row * 2 + 1] ?? "";
  return [...top]
    .map((pixel, column) => {
      const upper = pixel === "#",
        lower = bottom[column] === "#";
      return upper && lower ? "█" : upper ? "▀" : lower ? "▄" : " ";
    })
    .join("")
    .trimEnd();
});

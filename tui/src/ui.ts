import { safeText } from "@furb/engine";
import { RGBA, StyledText } from "@opentui/core";

/** One run of text in a line, with its color, its attributes, and the color behind it. */
export type Part = readonly [text: string, fg?: RGBA, attributes?: number, bg?: RGBA];
export const bold = 1,
  italic = 4,
  underline = 8;

/** The parts as one styled line, each part made safe for the terminal. */
export function styled(parts: readonly Part[]): StyledText {
  return new StyledText(
    parts
      .filter(([text]) => text)
      .map(([text, fg, attributes, bg]) => ({ __isChunk: true, text: safeText(text), fg, bg, attributes })),
  );
}

/** A line of markdown as parts in a tone: code in the color of code, strong text in the color of what stands out,
 * emphasis in italic, and a link as its label, underlined. No part is bold. */
export function inline(line: string, tone: RGBA, colors: { strong: RGBA; code: RGBA; link: RGBA }): Part[] {
  const parts: Part[] = [];
  let at = 0;
  for (const match of line.matchAll(
    /(`+)(.+?)\1|\*\*(\S(?:.*?\S)?)\*\*|__(\S(?:.*?\S)?)__|\*(\S(?:.*?\S)?)\*|\b_(\S(?:.*?\S)?)_\b|\[([^\]]+)\]\([^)]*\)/g,
  )) {
    if (match.index > at) parts.push([line.slice(at, match.index), tone]);
    const [, , code, strong, underscores, emphasis, underscore, link] = match;
    if (code !== undefined) parts.push([code, colors.code]);
    else if (strong !== undefined || underscores !== undefined)
      parts.push([strong ?? underscores ?? "", colors.strong]);
    else if (emphasis !== undefined || underscore !== undefined)
      parts.push([emphasis ?? underscore ?? "", tone, italic]);
    else parts.push([link ?? "", colors.link, underline]);
    at = match.index + match[0].length;
  }
  if (at < line.length) parts.push([line.slice(at), tone]);
  return parts;
}

/** The text of the parts, with no style. */
export function plain(parts: readonly Part[]): string {
  return parts.map(([text]) => text).join("");
}

/** The lines that a patch adds and removes, with no header of a file or a hunk. */
export function lineCounts(patch: string): { added: number; removed: number } {
  let added = 0,
    removed = 0;
  for (const line of patch.split("\n"))
    if (line.startsWith("+") && !line.startsWith("+++")) added++;
    else if (line.startsWith("-") && !line.startsWith("---")) removed++;
  return { added, removed };
}

/** A color between two colors, at a share of the way from the first to the second. */
export function mix(from: RGBA, to: RGBA, share: number): RGBA {
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

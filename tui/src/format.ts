/** A count of tokens, short: 1234 is 1.2k. */
export function count(value: number): string {
  return value >= 1_000_000
    ? `${Number((value / 1_000_000).toFixed(1))}M`
    : value >= 1000
      ? `${Number((value / 1000).toFixed(1))}k`
      : String(value);
}

const money = new Intl.NumberFormat("en-US", {
  style: "currency",
  currency: "USD",
  minimumFractionDigits: 2,
  maximumFractionDigits: 4,
});
/** An amount of dollars. */
export function dollars(value: number): string {
  return money.format(value);
}

/** A size in bytes, in KiB. */
export function kibibytes(bytes: number): string {
  return `${(bytes / 1024).toFixed(1)} KiB`;
}

/** A share of the window of a model. */
export function share(value: number): string {
  return `${Number((value * 100).toFixed(1))}% context`;
}

/** The graphemes of a text, which the terminal draws and the composer deletes one at a time. */
export const graphemes = new Intl.Segmenter();
/** A line cut to a width of terminal cells, with an ellipsis where it was cut: at its end, or at its start to keep
 * the end of a path. */
export function clip(line: string, width: number, keep: "start" | "end" = "start"): string {
  if (Bun.stringWidth(line) <= width) return line;
  const parts = [...graphemes.segment(line)].map(({ segment }) => segment);
  if (keep === "end") parts.reverse();
  let result = "";
  for (const part of parts) {
    if (Bun.stringWidth(result + part) >= width - 1) break;
    result = keep === "end" ? part + result : result + part;
  }
  return keep === "end" ? `…${result}` : `${result}…`;
}

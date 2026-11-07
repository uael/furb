import { expect, test } from "bun:test";
import { composing } from "./composing.ts";
import { idle } from "./idle.ts";

/** The runs of one character that a row holds, as [start, end) columns. */
function runs(row: string, mark: string): [number, number][] {
  const found: [number, number][] = [];
  for (let at = 0; at < row.length; at++) {
    if (row[at] !== mark) continue;
    let end = at;
    while (row[end] === mark) end++;
    found.push([at, end]);
    at = end;
  }
  return found;
}

for (const [width, height] of [
  [152, 46],
  [120, 40],
] as const)
  test(`the input and the box Context stand level with one gap between them at ${width}x${height}, and the commands stand below Context`, () =>
    composing(
      async ({ session, frame }) => {
        await idle(session);
        const rows = (await frame()).split("\n");
        const top = rows.findIndex((row) => runs(row, "▄").length >= 2 && row.includes("╻"));
        expect(top).toBeGreaterThan(0);
        const [input, context] = runs(rows[top] ?? "", "▄");
        if (!input || !context) throw new Error("No input and no Context box on the row of their top.");
        // The gap between the input and the box Context is as wide as the left margin of the input.
        const gap = context[0] - input[1];
        expect(gap).toBeGreaterThan(0);
        // The bar of the input stands one column before its first half block.
        expect(input[0] - 1).toBe(gap);
        // Both boxes end on the same row, and the Context box reads Context where its first row is.
        const bottom = rows.findIndex((row, y) => y > top && runs(row, "▀").length >= 2);
        expect(bottom).toBeGreaterThan(top);
        const [lower, lowerContext] = runs(rows[bottom] ?? "", "▀");
        expect(lower?.[1]).toBe(input[1]);
        expect(lowerContext?.[0]).toBe(context[0]);
        expect(rows.slice(top, bottom).some((row) => row.slice(context[0]).includes("Context"))).toBe(true);
        // The line of the commands and of the help stands below Context, on the row under both boxes.
        const line = rows[bottom + 1] ?? "";
        expect(line.indexOf("⌃P commands")).toBeGreaterThanOrEqual(context[0]);
        expect(line.indexOf("F1 help")).toBeGreaterThan(line.indexOf("⌃P commands"));
      },
      { width, height },
      true,
    ));

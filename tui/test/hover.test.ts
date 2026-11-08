// Every element that takes a click lights while the pointer is on it, as one state of the layer that draws: the test moves
// the pointer over each kind of element of the window and reads the cell under it, which differs from what it was while
// the pointer is over it and is the same again once it leaves.
import { expect, test } from "bun:test";
import type { RGBA } from "@opentui/core";
import { cellAt, composing } from "./composing.ts";

/** The colors of a cell: its text and its ground. */
const look = (cell: { fg?: RGBA; bg?: RGBA }) => [cell.fg?.toString(), cell.bg?.toString()].join(" on ");

// The kinds of element that take a click in a window of the demo, each with a text that stands on it and the row of
// the frame that holds it when more than one does.
const kinds: [kind: string, text: string, last?: boolean][] = [
  ["a row of the sidebar", "+ New session"],
  ["a row of the workspaces", "+ Add a workspace"],
  ["a fold", "Collapse"],
  ["a link of a thread", "Reply"],
  ["the model and the effort", "sonnet/high", true],
  ["a button of the composer", "+ New thread"],
  ["the mode of the composer", "Python"],
];

test("each element that takes a click lights while the pointer is on it and is as it was once the pointer leaves", async () => {
  await composing(
    async ({ frame, screen }) => {
      for (const [kind, text, last] of kinds) {
        const rows = (await frame()).split("\n");
        const y = last
          ? rows.findLastIndex((row) => row.includes(text))
          : rows.findIndex((row) => row.includes(text));
        expect(y, `${kind} stands in the window`).toBeGreaterThanOrEqual(0);
        const x = (rows[y] ?? "").indexOf(text) + 1;
        await screen.mockMouse.moveTo(100, 22);
        await frame();
        const rest = look(cellAt(screen.captureSpans(), x, y));
        await screen.mockMouse.moveTo(x, y);
        await frame();
        expect(look(cellAt(screen.captureSpans(), x, y)), `${kind} lights under the pointer`).not.toBe(rest);
        await screen.mockMouse.moveTo(100, 22);
        await frame();
        expect(look(cellAt(screen.captureSpans(), x, y)), `${kind} is as it was once the pointer left`).toBe(
          rest,
        );
      }
    },
    { width: 140, height: 46 },
    true,
  );
});

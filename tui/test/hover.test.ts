// Every element that takes a click lights while the pointer is on it, as one state of the layer that draws: the test moves
// the pointer over each kind of element of the window and reads the cell under it, which differs from what it was while
// the pointer is over it and is the same again once it leaves.
import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import type { RGBA } from "@opentui/core";
import { Glob } from "bun";
import { type Composing, cellAt, composing } from "./composing.ts";

/** The colors of a cell: its text and its ground. */
const look = (cell: { fg?: RGBA; bg?: RGBA }) => [cell.fg?.toString(), cell.bg?.toString()].join(" on ");

// The kinds of element that take a click in a window of the demo, each with a text that stands on it and the row of
// the frame that holds it when more than one does.
const kinds: [kind: string, text: string, last?: boolean][] = [
  ["a row of the sidebar", "+ New session"],
  ["a row of the workspaces", "+ Add a workspace"],
  ["a session of the sidebar", "✓ demo"],
  ["a fold", "Collapse"],
  ["a link of a thread", "Reply"],
  ["the model and the effort", "sonnet/high", true],
  ["a button of the composer", "+ New thread"],
  ["the mode of the composer", "Python"],
];

/** Move the pointer over the first cell of a text and read it before, over and after: it differs while the pointer
 * is over it and is the same again once it leaves. */
async function lights(
  { frame, screen }: Pick<Composing, "frame" | "screen">,
  kind: string,
  text: string,
  last = false,
  selects = false,
): Promise<void> {
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
  // A button of a dialog lights as the choice that Enter takes, which stays once the pointer leaves.
  if (!selects)
    expect(look(cellAt(screen.captureSpans(), x, y)), `${kind} is as it was once the pointer left`).toBe(
      rest,
    );
}

test("each element that takes a click lights while the pointer is on it and is as it was once the pointer leaves", async () => {
  await composing(
    async (context) => {
      for (const [kind, text, last] of kinds) await lights(context, kind, text, last);
    },
    { width: 140, height: 46 },
    true,
  );
});

test("each button of a dialog lights while the pointer is on it and is as it was once the pointer leaves", async () => {
  await composing(
    async (context) => {
      void context.app.partConfirm("Sure?", "Do it now?", { yes: "Do it", no: "Leave it" });
      // The first button is the one that Enter takes, so the pointer lights the others.
      await lights(context, "the second button of a dialog", "Leave it", false, true);
    },
    { width: 140, height: 46 },
    true,
  );
});

test("no part of the window lights a node by a call of its own: box and text give the one hover state", () => {
  const callers = [...new Glob("src/**/*.ts").scanSync(join(import.meta.dir, ".."))].filter((file) =>
    readFileSync(join(import.meta.dir, "..", "src", file.replace(/^src\//, "")), "utf8").includes(
      "this.hoverable(",
    ),
  );
  expect(callers.map((file) => file.replace(/^src\//, ""))).toEqual(["app.ts"]);
});

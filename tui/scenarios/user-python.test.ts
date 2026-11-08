// A word that the operator sends as Python draws as a word of a model draws: the same layout and the same color of each
// cell of its code, live and after a window opens the life again.
import { expect } from "bun:test";
import { scenario, test, Window } from "./harness.ts";

const LINES = ['total = len("abc") + 1', "double = total * 2"];
const word = (last: string) => `${LINES.join("\n")}\nclose("${last}")`;

/** Where each line of the code stands in a frame, with the colors of its cells. */
function shapes(window: Window, frame: string): { column: number; cells: string[] }[][] {
  const rows = frame.split("\n");
  const spans = window.screen.captureSpans();
  return LINES.map((line) =>
    rows.flatMap((row, y) => {
      const column = row.indexOf(line);
      if (column < 0) return [];
      const cells: string[] = [];
      let x = 0;
      for (const span of spans.lines[y]?.spans ?? []) {
        for (let k = 0; k < span.width; k++, x++)
          if (x >= column && x < column + line.length)
            cells.push(`${span.fg.toString()}/${span.bg.toString()}`);
      }
      return [{ column, cells }];
    }),
  );
}

/** The two words draw alike: each line of the code stands twice, the model's first, in one column and in one color. */
async function alike(window: Window, what: string): Promise<void> {
  const frame = await window.shows(
    (shown) => LINES.every((line) => shown.split("\n").filter((row) => row.includes(line)).length >= 2),
    `the two words ${what}`,
  );
  for (const [model, operator] of shapes(window, frame)) {
    expect(operator, `the word of the operator ${what}`).toBeDefined();
    expect(operator?.column).toBe(model?.column);
    expect(operator?.cells).toEqual(model?.cells);
  }
}

test("a word of the operator in Python draws as a word of a model, live and after a reopen", () =>
  scenario([{ cue: "Shape: show.", word: word("model") }], async (window, yard) => {
    await window.type("Shape: show.");
    await window.shows((frame) => frame.includes(LINES[0] as string), "the word of the model");
    // The operator sends the same code as a word: Python input, a program of two lines and a close.
    window.screen.mockInput.pressKey("r", { ctrl: true });
    await window.frame();
    window.app.composer.setText(word("operator"));
    window.screen.mockInput.pressEnter();
    await alike(window, "live");
    await window.close(false);
    const again = await Window.open(yard);
    try {
      await alike(again, "after a reopen");
    } finally {
      await again.close(true);
    }
  }));

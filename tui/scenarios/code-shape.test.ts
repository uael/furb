// A word and a Markdown code block of the same code draw alike: the same background, the same colors of each cell of the
// code, and their numbers in the same column.
import { expect } from "bun:test";
import { palettes } from "../src/theme.ts";
import { scenario, test } from "./harness.ts";

const CODE = 'total = len("abc") + 1';
const WORD = `${CODE}\nclose("Here:\\n\\n\`\`\`python\\n${CODE.replaceAll('"', '\\"')}\\n\`\`\`")`;

test("a word and a fence of the same code draw each cell of the code alike", () =>
  scenario([{ cue: "Shape: show.", word: WORD }], async (window) => {
    await window.type("Shape: show.");
    await window.shows(
      (shown) => shown.split("\n").filter((row) => row.includes(CODE)).length >= 2,
      "the word and the fence",
    );
    // Each palette of the TUI draws the two alike.
    for (const theme of Object.keys(palettes)) {
      await window.type(`/theme ${theme}`);
      const frame = await window.frame();
      const rows = frame.split("\n");
      const at = rows.flatMap((row, y) => (row.includes(CODE) ? [y] : []));
      const spans = window.screen.captureSpans();
      const cells = (y: number) => {
        const x0 = (rows[y] ?? "").indexOf(CODE);
        const out: string[] = [];
        let x = 0;
        for (const span of spans.lines[y]?.spans ?? []) {
          for (let k = 0; k < span.width; k++, x++)
            if (x >= x0 && x < x0 + CODE.length) out.push(`${span.fg.toString()}/${span.bg.toString()}`);
        }
        return { column: x0, out };
      };
      const [word, fence] = [cells(at[0] ?? -1), cells(at.at(-1) ?? -1)];
      expect(fence.column).toBe(word.column);
      expect([theme, ...fence.out]).toEqual([theme, ...word.out]);
    }
  }));

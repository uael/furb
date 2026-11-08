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

const LONG = Array.from({ length: 14 }, (_, i) => `value_${i} = len("abc") + ${i}`);
const LONG_WORD = `${LONG.join("\n")}\nclose("Here:\\n\\n\`\`\`python\\n${LONG.join("\\n").replaceAll('"', '\\"')}\\n\`\`\`")`;

test("a long word and a long fence of the same code fold under one row at the column of the code, and open to the same cells", () =>
  scenario([{ cue: "Shape: long.", word: LONG_WORD }], async (window) => {
    await window.type("Shape: long.");
    await window.shows((shown) => /more lines/.test(shown) && /earlier lines/.test(shown), "the two folds");
    const code = (row: string) => row.search(/value_\d+ = /);
    const rowsOf = async () => (await window.frame()).split("\n");
    const rows = await rowsOf();
    const fold = (word: RegExp) => (rows.find((one) => word.test(one)) ?? "").search(/▸/);
    // The row of each fold stands at the column of the code that it folds, in the word as in the fence.
    const column = code(rows.find((row) => /value_13 = /.test(row)) ?? "");
    expect([fold(/earlier lines/), fold(/more lines/)]).toEqual([column, column]);
    // Each line of the code draws alike in the word and in the fence, with its number: the first lines once the word
    // opens, beside the first lines that the folded fence shows, and the last lines once the fence opens.
    const same = async (one: number) => {
      const both = (await rowsOf()).filter((row) => row.includes(`value_${one} = `));
      expect(both.length).toBe(2);
      const [word, fence] = both.map((row) => row.slice(0, code(row) + 12));
      expect(fence).toBe(word);
    };
    await window.click("earlier lines");
    await window.shows(
      (shown) => shown.split("\n").filter((row) => /value_0 = /.test(row)).length >= 2,
      "the word open",
    );
    for (const one of [0, 4, 8]) await same(one);
    await window.click("more lines");
    await window.shows(
      (shown) => shown.split("\n").filter((row) => /value_13 = /.test(row)).length >= 2,
      "the fence open",
    );
    for (const one of [9, 13]) await same(one);
  }));

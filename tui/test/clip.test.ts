// clip fills its width to the last cell, wide and joined graphemes included, and shows an ellipsis only where one fits.
import { expect, test } from "bun:test";
import { clip } from "../src/format.ts";

const width = (text: string) => Bun.stringWidth(text);

test("a grapheme that fills the cells that are left stays", () => {
  expect(clip("abcdefgh", 5)).toBe("abcd…");
  expect(clip("abcdefgh", 5, "end")).toBe("…efgh");
  expect(width(clip("abcdefgh", 5))).toBe(5);
});

test("a wide character and a joined emoji never cross the bound", () => {
  for (const text of ["日本語のテキストです", "a👍🏽b👨‍👩‍👧c日本語", "😀😀😀😀😀😀"])
    for (const size of [2, 3, 4, 5, 6, 7])
      for (const keep of ["start", "end"] as const) {
        const cut = clip(text, size, keep);
        expect(width(cut)).toBeLessThanOrEqual(size);
        expect(cut.split("…").length - 1).toBeLessThanOrEqual(1);
      }
  expect(clip("日本語のテキスト", 5)).toBe("日本…");
});

test("a width of 0 or 1 shows no ellipsis, and a width of 1 keeps the first grapheme of one cell", () => {
  expect(clip("abcdef", 0)).toBe("");
  expect(clip("abcdef", 1)).toBe("a");
  expect(clip("abcdef", 1, "end")).toBe("f");
  expect(clip("日本語", 1)).toBe("");
  expect(clip("日a本", 1)).toBe("");
  expect(clip("a日本語", 1, "end")).toBe("");
  expect(clip("abcdef", 2)).toBe("a…");
});

test("the renderer draws a clipped row in the cells of its width, none past them and the last one filled", async () => {
  const { createTestRenderer } = await import("@opentui/core/testing");
  const { TextRenderable } = await import("@opentui/core");
  // A mark stands on the first cell past the bound. A cell that the clip let past the bound lies under it, and the row
  // that the renderer draws then lacks the end of the line that was clipped.
  const lines = [
    "abcdefghijklmnop",
    "日本語のテキストです",
    "a👍🏽b👨‍👩‍👧c日本語😀😀",
    "e\u0301e\u0301e\u0301e\u0301e\u0301e\u0301e\u0301e\u0301",
  ];
  for (const text of lines)
    for (const size of [2, 3, 4, 5, 6, 7, 8])
      for (const keep of ["start", "end"] as const) {
        const cut = clip(text, size, keep);
        const screen = await createTestRenderer({ width: size + 6, height: 1, exitSignals: [] });
        try {
          screen.renderer.root.add(new TextRenderable(screen.renderer, { content: cut, left: 0, top: 0 }));
          screen.renderer.root.add(new TextRenderable(screen.renderer, { content: "|", left: size, top: 0 }));
          await screen.renderOnce();
          const row = (screen.captureCharFrame().split("\n")[0] ?? "").trimEnd();
          expect(row.endsWith("|")).toBe(true);
          expect(row.slice(0, -1).trimEnd()).toBe(cut.trimEnd());
          // A line of one-cell graphemes fills its last cell, with the ellipsis where it was cut.
          if (/^[a-p]+$/.test(text)) expect(cut.length).toBe(size);
        } finally {
          screen.renderer.destroy();
        }
      }
});

test("a line that Bun measures wider than another table does is cut by Bun's measure, and a long line costs little", () => {
  expect(width(clip("ab가cd", 6))).toBeLessThanOrEqual(6);
  expect(width(clip("abกำcd", 5))).toBeLessThanOrEqual(5);
  const long = "ab日本😀".repeat(2000);
  const began = performance.now();
  for (let n = 0; n < 20; n++) clip(long, 80, n % 2 ? "end" : "start");
  expect(performance.now() - began).toBeLessThan(500);
});

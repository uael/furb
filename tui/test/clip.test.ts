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
  expect(clip("日a本", 1)).toBe("a");
  expect(clip("abcdef", 2)).toBe("a…");
});

test("the renderer draws no cell past the bound of a clipped row", async () => {
  const { createTestRenderer } = await import("@opentui/core/testing");
  const { TextRenderable } = await import("@opentui/core");
  for (const text of ["日本語のテキストです", "a👍🏽b👨‍👩‍👧c日本語", "abcdefghijklmnop"])
    for (const size of [1, 2, 3, 4, 5, 6])
      for (const keep of ["start", "end"] as const) {
        const screen = await createTestRenderer({ width: 20, height: 2, exitSignals: [] });
        try {
          screen.renderer.root.add(
            new TextRenderable(screen.renderer, { content: clip(text, size, keep), left: 0, top: 0 }),
          );
          await screen.renderOnce();
          const row = screen.captureCharFrame().split("\n")[0] ?? "";
          // The row reads in cells: the text that the renderer drew ends within the bound.
          expect(Bun.stringWidth(row.trimEnd())).toBeLessThanOrEqual(size);
        } finally {
          screen.renderer.destroy();
        }
      }
});

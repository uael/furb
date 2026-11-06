import { expect, test } from "bun:test";
import { TextRenderable } from "@opentui/core";
import { createTestRenderer } from "@opentui/core/testing";
import { Gutter } from "../src/app/gutter.ts";
import { markOf, motion } from "../src/theme.ts";

test("the mark of a line stands one blank cell from its number, for each state and for numbers of one and two digits", async () => {
  motion.still = true;
  const screen = await createTestRenderer({ width: 40, height: 14, exitSignals: [] });
  try {
    const lines = Array.from({ length: 12 }, (_, n) => `code${n + 1}`);
    const code = new TextRenderable(screen.renderer, { content: lines.join("\n"), wrapMode: "none" });
    const states = new Map([
      [0, markOf("failed")],
      [1, markOf("done")],
      [9, markOf("failed")],
      [10, markOf("runs")],
      [11, markOf("thinks")],
    ]);
    const gutter = new Gutter(screen.renderer, {
      target: code,
      states,
      running: new Set([10]),
      width: "100%",
    });
    screen.renderer.root.add(gutter);
    await screen.renderOnce();
    const rows = screen.captureCharFrame().split("\n");
    const at = (n: number) => rows[n] ?? "";
    // The number, one blank cell, the mark, one blank cell, then the code.
    expect(at(0).startsWith(" 1 ✗ code1")).toBe(true);
    expect(at(1).startsWith(" 2 ✓ code2")).toBe(true);
    expect(at(9).startsWith("10 ✗ code10")).toBe(true);
    // The loader of a line that runs takes the place of its number, and the mark keeps its cell after the blank one.
    expect(at(10).startsWith(" ✻ ◉ code11")).toBe(true);
    expect(at(11).startsWith("12 ✻ code12")).toBe(true);
  } finally {
    motion.still = false;
    screen.renderer.destroy();
  }
});

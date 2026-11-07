import { expect, setSystemTime, test } from "bun:test";
import { TextRenderable } from "@opentui/core";
import { createTestRenderer } from "@opentui/core/testing";
import { Gutter, type GutterOptions } from "../src/app/gutter.ts";
import { frame, markOf, motion } from "../src/theme.ts";

/** A frame of the loader of a thought, alone in its cell. */
const thinking = /^[⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏]$/;

/** The rows that a gutter draws over twelve lines of code, with the options given. */
async function drawn(options: Omit<GutterOptions, "target">): Promise<string[]> {
  const screen = await createTestRenderer({ width: 40, height: 14, exitSignals: [] });
  try {
    const lines = Array.from({ length: 12 }, (_, n) => `code${n + 1}`);
    const code = new TextRenderable(screen.renderer, { content: lines.join("\n"), wrapMode: "none" });
    screen.renderer.root.add(new Gutter(screen.renderer, { target: code, width: "100%", ...options }));
    await screen.renderOnce();
    return screen.captureCharFrame().split("\n");
  } finally {
    screen.renderer.destroy();
  }
}

test("the mark of a line stands two blank cells from its number, for numbers of one and two digits", async () => {
  motion.still = true;
  try {
    const rows = await drawn({
      states: new Map([
        [0, markOf("failed")],
        [1, markOf("done")],
        [9, markOf("failed")],
      ]),
    });
    // The number, two blank cells, the mark, one blank cell, then the code.
    expect(rows[0]?.startsWith(" 1  ✗ code1")).toBe(true);
    expect(rows[1]?.startsWith(" 2  ✓ code2")).toBe(true);
    expect(rows[9]?.startsWith("10  ✗ code10")).toBe(true);
    expect(rows[10]?.startsWith("11    code11")).toBe(true);
  } finally {
    motion.still = false;
  }
});

test("the number of a line that runs fades into the loader of work that runs and back, and a line that a model writes shows the loader of a thought in its place", async () => {
  // A line of each state with its number, and a line that a model writes with no number.
  const working = new Map([
    [9, "runs" as const],
    [10, "thinks" as const],
    [11, "thinks" as const],
  ]);
  const numbers = new Map(Array.from({ length: 11 }, (_, n) => [n, n + 1]));
  const cells: string[][] = [];
  // A time at the start of a whole fade, and of the frames of each loader.
  const start = 26 * 40 * frame;
  try {
    // The frames of one whole fade, a tick apart.
    for (let tick = 0; tick < 26; tick++) {
      setSystemTime(new Date(start + tick * frame));
      const rows = await drawn({ working, numbers });
      cells.push([9, 10, 11].map((n) => (rows[n] ?? "").slice(0, 2)));
    }
  } finally {
    setSystemTime();
  }
  const runs = cells.map((row) => row[0] ?? "");
  const thinks = cells.map((row) => row[1] ?? "");
  // The number shows, fades into the loader of its state, and comes back.
  expect(runs[0]).toBe("10");
  expect(runs.at(-1)).toBe("10");
  expect(runs.some((cell) => ["◐", "◓", "◑", "◒"].includes(cell.trim()))).toBe(true);
  // At each tick the two loaders differ, and no cell shows the still mark of a thought.
  for (const [at, cell] of thinks.entries()) expect(cell).not.toBe(runs[at]);
  expect(cells.flat().join("")).not.toContain("✻");
  // A line that a model writes turns the loader of a thought in place of its number at each tick, one frame after
  // another, with a number or without one.
  expect(thinks.every((cell) => thinking.test(cell.trim()))).toBe(true);
  const alone = cells.map((row) => (row[2] ?? "").trim());
  expect(alone.every((cell) => thinking.test(cell))).toBe(true);
  expect(new Set(alone).size).toBe(10);
});

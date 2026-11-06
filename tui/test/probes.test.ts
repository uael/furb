import { expect, test } from "bun:test";
import { changed, chord, leaks, shut } from "../scenarios/probes.ts";

test("a value of the code that reaches the screen is a leak, and words of the life are not", () => {
  const frame = [
    "│ cost undefined",
    "│ tokens NaN of 200000",
    "│ [object Object]",
    "│ undefinedness is a word",
    "│ None",
  ].join("\n");
  expect(leaks(frame)).toEqual(["│ cost undefined", "│ tokens NaN of 200000", "│ [object Object]"]);
});

test("a frame that says its life is not open is shut", () => {
  expect(shut("The life is not open.")).toBe(true);
  expect(shut("The life opens.")).toBe(false);
});

test("the rows that changed name each row, before and after, and count the rest past the bound", () => {
  expect(changed("a\nb\nc", "a\nB\nc")).toEqual(['row 2: "b" -> "B"']);
  expect(changed("a", "a\nnew")).toEqual(['row 2: "" -> "new"']);
  expect(changed("1\n2\n3", "x\ny\nz", 2)).toEqual([
    'row 1: "1" -> "x"',
    'row 2: "2" -> "y"',
    "and 1 more rows",
  ]);
});

test("a chord of the table of keys is the key and the modifiers that the mock input presses", () => {
  expect(chord("ctrl+p")).toEqual({ key: "p", modifiers: { ctrl: true, meta: false, shift: false } });
  expect(chord("ctrl+meta+left")).toEqual({
    key: "ARROW_LEFT",
    modifiers: { ctrl: true, meta: true, shift: false },
  });
  expect(chord("shift+tab")).toEqual({ key: "TAB", modifiers: { ctrl: false, meta: false, shift: true } });
  expect(chord("ctrl+\\")).toEqual({ key: "\\", modifiers: { ctrl: true, meta: false, shift: false } });
  expect(chord("pageup").key).toBe("\u001b[5~");
});

// A cut line of a word ends with the count of what it hides, in its trailer, and no row of its own stands under it.
import { expect } from "bun:test";
import { scenario, test } from "./harness.ts";

test("a word with a string longer than its line shows the count that it hides at the end of the cut line", () =>
  scenario(
    [
      {
        cue: "Cut: show a long string.",
        word: `note = "${"word ".repeat(48)}ENDMARK"\nclose("Shown.")`,
      },
    ],
    async (window) => {
      await window.type("Cut: show a long string.");
      const frame = await window.shows((shown) => shown.includes("Shown."), "the answer");
      const rows = frame.split("\n").map((row) => row.trimEnd());
      const cut = rows.find((row) => row.includes('note = "'));
      expect(cut).toMatch(/…"\s+…\+\d+\s*(\S.*)?$/);
      expect(rows.some((row) => /more characters/.test(row))).toBe(false);
    },
  ));

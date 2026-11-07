// Every kind of Markdown of a thread takes the one softer prose role of the theme.
import { expect } from "bun:test";
import { RGBA } from "@opentui/core";
import { palettes } from "../src/theme.ts";
import { scenario, test } from "./harness.ts";

const MARKDOWN =
  "# A heading\\n\\nA plain paragraph of prose.\\n\\n- a list item of prose\\n\\n> a quote of prose";

test("a heading, a paragraph, a list and a quote of Markdown are drawn in the prose role", () =>
  scenario([{ cue: "Prose: show.", word: `close("${MARKDOWN}")` }], async (window) => {
    await window.type("Prose: show.");
    const frame = await window.shows((shown) => shown.includes("a quote of prose"), "the answer");
    const rows = frame.split("\n");
    const spans = window.screen.captureSpans();
    const found: string[] = [];
    for (const text of ["A heading", "A plain paragraph", "a list item", "a quote of prose"]) {
      const y = rows.findLastIndex((row) => row.includes(text));
      const x = (rows[y] ?? "").indexOf(text);
      const line = spans.lines[y];
      let at = 0;
      let fg = "none";
      for (const span of line?.spans ?? []) {
        if (x >= at && x < at + span.width) fg = span.fg.toString();
        at += span.width;
      }
      found.push(`${text}=${fg}`);
    }
    // The word of the model stands above its answer, so the last row of each text is the Markdown itself.
    const prose = RGBA.fromHex(palettes.ink.prose).toString();
    expect(found.map((one) => one.split("=")[1])).toEqual([prose, prose, prose, prose]);
  }));

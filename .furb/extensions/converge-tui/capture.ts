/**
 * The captures of real lives for the extension converge. Each record that FURB_CONVERGE_RECORDS names, one path on
 * each line, is a copy that opens read-only, so it asks no model and runs nothing. The real renderer draws its root
 * and its two newest other chains with no window, at each size, as PNG files in the folder FURB_CONVERGE_OUT, each with
 * the cells that the renderer drew, as JSON beside it.
 * converge copies this file into tui/script of the tree that it captures, and runs it there with bun.
 */
import { mkdir, writeFile } from "node:fs/promises";
import { basename, dirname, join } from "node:path";
import type { CapturedFrame, RGBA } from "@opentui/core";
import { createTestRenderer } from "@opentui/core/testing";
import { openEngine } from "../src/bridge.ts";
import { loadParsers } from "../src/parsers.ts";
import { Session } from "../src/session.ts";
import { paletteOf } from "../src/theme.ts";
import { rasterize } from "./raster.ts";
import { highlighting, home, mount } from "./stage.ts";

// The home of the user is a folder of its own, so that the preferences of the captures never touch those of the owner.
await home("furb-converge");
const folder = process.env.FURB_CONVERGE_OUT ?? "";
const records = (process.env.FURB_CONVERGE_RECORDS ?? "").split("\n").filter(Boolean);
const sizes = [
  [200, 56],
  [120, 40],
  [80, 30],
] as const;
await mkdir(folder, { recursive: true });
const test = await createTestRenderer({ width: 200, height: 56 });
await loadParsers();
let failed = 0;
/** The cells of a frame, as cells in raster.ts of the tree gives them. This file runs on trees that are older than
 * that function, so it holds its own. */
function cells(frame: CapturedFrame): string {
  const hex = (value: RGBA) =>
    `#${value
      .toInts()
      .slice(0, 3)
      .map((part) => part.toString(16).padStart(2, "0"))
      .join("")}`;
  const lines = frame.lines.map((line) =>
    line.spans.map((span) => [span.text, hex(span.fg), hex(span.bg), span.attributes]),
  );
  return JSON.stringify({ cols: frame.cols, rows: frame.rows, lines });
}
for (const record of records) {
  const name = basename(record, ".jsonl");
  try {
    const life = await openEngine({ record, cwd: dirname(record), readOnly: true });
    const session = new Session(life.engine, life.host);
    await session.refresh();
    const { library, app } = await mount(session, test.renderer);
    const root = session.engine.root;
    const others = session.chains.map((chain) => chain.id).filter((id) => id !== root);
    for (const chain of [root, ...others.slice(-2)]) {
      await session.select(chain);
      for (const [width, height] of sizes) {
        test.resize(width, height);
        do await session.refresh();
        while (session.loading);
        app().render();
        await test.flush();
        await Promise.all(highlighting(test.renderer.root));
        await app().colorsDone();
        app().render();
        await test.flush();
        const frame = test.captureSpans();
        const shot = join(folder, `${name}-${chain}-${width}x${height}`);
        await writeFile(`${shot}.png`, rasterize(frame, paletteOf(session.theme), "furb"));
        await writeFile(`${shot}.json`, cells(frame));
      }
    }
    app().dispose();
    await library.dispose();
  } catch (error) {
    failed++;
    console.error(`${record}: ${error instanceof Error ? error.message : String(error)}`);
  }
}
process.exit(records.length && failed === records.length ? 1 : 0);

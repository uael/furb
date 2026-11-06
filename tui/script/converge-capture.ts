/**
 * The captures of real lives for the audit of the TUI, which the skill converge-tui runs. Each record that
 * FURB_CONVERGE_RECORDS names, one path on each line, is a copy that opens read-only, so it asks no model and runs
 * nothing. The real renderer draws its root and its two newest other chains with no window, at each size, as PNG files
 * in the folder FURB_CONVERGE_OUT, each with the cells that the renderer drew, as JSON beside it.
 *
 * FURB_CONVERGE_SLICE names a recorded life cut from the log of a long real life, as tui/script/fixture.ts makes it,
 * on which the renderer plays the actions of an operator at 222x58 and captures each: the end of the feed, its top with
 * each fold of earlier steps open, the sidebar on a worker, the agents of each thread open, a fold of the feed opened,
 * the palette, the refusal of a message that a life open to read gives, and the feed at 82x32 and 60x24. An action
 * that fails is named on stderr, the others still run, and the script exits 1.
 */
import { mkdir, writeFile } from "node:fs/promises";
import { basename, dirname, join } from "node:path";
import { createTestRenderer } from "@opentui/core/testing";
import { openEngine } from "../src/bridge.ts";
import { loadParsers } from "../src/parsers.ts";
import { Session } from "../src/session.ts";
import { paletteOf } from "../src/theme.ts";
import { cells, rasterize } from "./raster.ts";
import { refusal } from "./refusal.ts";
import { highlighting, home, mount } from "./stage.ts";

// The home of the user is a folder of its own, so that the preferences of the captures never touch those of the owner.
await home("furb-converge");
const folder = process.env.FURB_CONVERGE_OUT ?? "";
const records = (process.env.FURB_CONVERGE_RECORDS ?? "").split("\n").filter(Boolean);
const slice = process.env.FURB_CONVERGE_SLICE ?? "";
const sizes = [
  [200, 56],
  [120, 40],
  [80, 30],
] as const;
await mkdir(folder, { recursive: true });
const test = await createTestRenderer({ width: 200, height: 56 });
await loadParsers();
let failed = 0;
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
let missed = 0;
if (slice) {
  const name = `slice-${basename(slice, ".jsonl")}`;
  try {
    const life = await openEngine({ record: slice, cwd: dirname(slice), readOnly: true });
    const session = new Session(life.engine, life.host);
    await session.refresh();
    const { library, app } = await mount(session, test.renderer);
    test.resize(222, 58);
    /** The frame as the renderer draws it once each read and each highlight is done, written under the name of a step. */
    const capture = async (step: string, end = false) => {
      do await session.refresh();
      while (session.loading);
      app().render();
      await test.flush();
      if (end) {
        app().scroll.stickyScroll = true;
        app().scrollNow("end");
        await test.flush();
      }
      await Promise.all(highlighting(test.renderer.root));
      await app().colorsDone();
      app().render();
      await test.flush();
      const frame = test.captureSpans();
      const shot = join(folder, `${name}-${step}-${test.renderer.width}x${test.renderer.height}`);
      await writeFile(`${shot}.png`, rasterize(frame, paletteOf(session.theme), "furb"));
      await writeFile(`${shot}.json`, cells(frame));
    };
    /** An action of the operator and its capture: an action that fails is named, and the next one runs. */
    const step = async (named: string, act: () => Promise<unknown> | unknown, end = false) => {
      try {
        await act();
        await capture(named, end);
      } catch (error) {
        missed++;
        console.error(`${name} ${named}: ${error instanceof Error ? error.message : String(error)}`);
      }
    };
    // The dialog that says why a record opens to read closes, as the operator closes it at once.
    app().closeOverlay();
    const root = session.engine.root;
    await session.select(root);
    await step("01-end", () => {}, true);
    await step("02-top-expanded", async () => {
      // Each fold of earlier steps opens, as a click on each opens it, until the feed holds none.
      for (let pass = 0; pass < 8; pass++) {
        const folds = app()
          .scroll.getChildren()
          .filter((node) => node.id.startsWith("more-"));
        if (!folds.length) break;
        for (const node of folds) session.folds[node.id.slice("more-".length)] = false;
        app().render();
        await test.flush();
      }
      app().scroll.stickyScroll = false;
      app().scroll.scrollTo(0);
    });
    session.folds = Object.fromEntries(Object.entries(session.folds).filter(([, open]) => open !== false));
    const workers = session.chains.map((chain) => chain.id).filter((id) => id !== root);
    await step(
      "03-sidebar-workers",
      async () => {
        const worker = workers.at(-1);
        if (!worker) throw new Error("The life holds no worker.");
        await session.select(worker);
      },
      true,
    );
    await step(
      "04-agents-open",
      async () => {
        await session.select(root);
        for (const act of session.activity) if (act.kind === "prompt") app().unfolded.add(`agents:${act.id}`);
      },
      true,
    );
    await step("05-fold-opened", async () => {
      app().render();
      await test.flush();
      const rows = test.captureCharFrame().split("\n");
      const row = rows.findIndex((line) => / earlier (steps?|rungs?|messages?)\b|…["']/.test(line));
      if (row < 0) throw new Error("The feed shows no fold.");
      const column = (rows[row] ?? "").search(/\d+ earlier|…["']/);
      await test.mockMouse.click(column + 1, row);
      await test.mockMouse.moveTo(0, test.renderer.height - 1);
    });
    await step("06-palette", () => app().palette(), true);
    app().closeOverlay();
    await step("07-read-only-refusal", () => refusal(test, app()), true);
    for (const [width, height] of [
      [82, 32],
      [60, 24],
    ] as const)
      await step("08-resized", () => test.resize(width, height), true);
    app().dispose();
    await library.dispose();
  } catch (error) {
    missed++;
    console.error(`${slice}: ${error instanceof Error ? error.message : String(error)}`);
  }
}
process.exit((records.length && failed === records.length) || missed ? 1 : 0);

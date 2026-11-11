/** What a window holds of a life, measured as a window of the operator opens it: the record in a daemon, drawn in the
 * App on a test terminal. The feed scrolls from its end to its start a page each frame, then back to its end, and each
 * tenth of the way the window collects its garbage and measures its heap, which holds what the window keeps. It is the
 * one measure of `bun run memory` and of tui/test/window-heap.test.ts, which the gates run. On macOS the resident size
 * counts pages that the process freed and the system did not take back yet, so the heap is the measure of what the
 * window keeps. The heap before the open is the baseline: it holds the code of the window and what the process held
 * already. The caller gives the daemon its cache, which an environment variable names before the life opens. */

import { heapStats } from "bun:jsc";
import { join } from "node:path";
import { createTestRenderer } from "@opentui/core/testing";
import { App } from "../src/app.ts";
import { demoLibrary, demoSession } from "../src/demo.ts";
import { Preferences } from "../src/preferences.ts";

/** What a window held of a life, in bytes of heap, and what it drew of it. */
export interface Held {
  facts: number;
  /** The rows that the feed holds once it laid out its cards, and the acts that they show. */
  rows: number;
  acts: number;
  /** The seconds from the open of the life to the end of its layout. */
  seconds: number;
  /** The heap before the open of the life, once it opened, and the highest of the scroll. */
  before: number;
  opened: number;
  most: number;
  /** How much the heap grew over the baseline at its highest. */
  grew: number;
  /** The highest resident size of the process. */
  resident: number;
  /** The pages that the feed scrolled to its start. */
  pages: number;
  /** Whether the scroll back lost the first lines that the feed showed at its start. */
  lost: boolean;
}

/** The heap after a collection of the garbage. */
function heapNow(): number {
  Bun.gc(true);
  return heapStats().heapSize + heapStats().extraMemorySize;
}

/** A window that opens the life of a record and scrolls the whole of its feed, and what it held. The folder takes the
 * preferences of the window. */
export async function hold(record: string, folder: string): Promise<Held> {
  const before = heapNow();
  let most = before;
  let resident = 0;
  const measure = () => {
    const now = heapNow();
    most = Math.max(most, now);
    resident = Math.max(resident, process.memoryUsage().rss);
    return now;
  };
  const began = performance.now();
  const session = await demoSession({
    record,
    cwd: folder,
    preferences: new Preferences(join(folder, "ui.json")),
  });
  const library = await demoLibrary(session);
  const screen = await createTestRenderer({ width: 152, height: 46, exitSignals: [] });
  const app = new App(screen.renderer, session, { quit() {}, workspaces: library });
  app.render();
  // The feed lays out its cards over a few frames, after which it holds the whole height of the life.
  for (let k = 0, height = -1, still = 0; k < 600 && still < 20; k++) {
    still = app.scroll.scrollHeight === height ? still + 1 : 0;
    height = app.scroll.scrollHeight;
    await screen.renderOnce();
    await Bun.sleep(50);
  }
  const seconds = (performance.now() - began) / 1000;
  const opened = measure();
  const [facts, rows, acts] = [session.host.seq, app.scroll.scrollHeight, session.activity.length];

  /** The feed scrolls a page each frame until it stands still at its end, and the frames at a tenth of the way. */
  async function scroll(down: boolean): Promise<{ steps: number; first: string }> {
    let steps = 0;
    let still = 0;
    let first = "";
    while (still < 20) {
      const at = app.scroll.scrollTop;
      app.scroll.scrollBy(down ? 40 : -40);
      await screen.renderOnce();
      const frame = screen.captureCharFrame();
      still = app.scroll.scrollTop === at ? still + 1 : 0;
      if (!down && still) first = frame;
      if (++steps % 200 === 0) measure();
    }
    measure();
    return { steps, first };
  }
  const up = await scroll(false);
  await scroll(true);
  const back = await scroll(false);
  app.dispose();
  screen.renderer.destroy();
  await library.dispose();
  return {
    facts,
    rows,
    acts,
    seconds,
    before,
    opened,
    most,
    grew: most - before,
    resident,
    pages: up.steps,
    lost: up.first !== back.first,
  };
}

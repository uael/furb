/** bun run memory <record> [bound in MB]: how much memory a window holds on a long life, while it scrolls the whole
 * life. It copies the record into a folder of its own, so the life that it opens writes the copy alone, opens it in a
 * daemon of its own, as a window of the operator does, and draws it in the App on a test terminal. The feed scrolls
 * from its end to its start a page each frame, then back to its end, and each tenth of the way the window collects its
 * garbage and measures its heap, which holds what the window keeps. It prints the facts of the life, the heap after the
 * open, the highest heap of the scroll, and the highest resident size of the process, and fails when the heap ever
 * passes the bound, 512 MB by default. On macOS the resident size counts pages that the process freed and the system
 * did not take back yet, so the heap is the measure of what the window keeps. It fails too when the scroll back loses
 * the first lines that the feed showed at its start. */

import { heapStats } from "bun:jsc";
import { copyFileSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";
import { createTestRenderer } from "@opentui/core/testing";

const [given, bound = "512"] = process.argv.slice(2);
if (!given) {
  console.error("Name a record: bun run memory <record> [bound in MB].");
  process.exit(2);
}
const root = mkdtempSync(join(tmpdir(), "furb-memory-"));
const record = join(root, basename(given));
copyFileSync(resolve(given), record);
// The daemon of the window is one of this build, in a cache of its own, which ends once the window lets the life go.
process.env.XDG_CACHE_HOME = join(root, "cache");
const { App } = await import("../src/app.ts");
const { demoLibrary, demoSession } = await import("../src/demo.ts");
const { Preferences } = await import("../src/preferences.ts");

const mb = (bytes: number) => Math.round(bytes / 2 ** 20);
let heap = 0;
let resident = 0;
/** The heap after a collection, and the highest resident size so far. */
const measure = () => {
  Bun.gc(true);
  const now = heapStats().heapSize + heapStats().extraMemorySize;
  heap = Math.max(heap, now);
  resident = Math.max(resident, process.memoryUsage().rss);
  return now;
};

const began = performance.now();
const session = await demoSession({ record, cwd: root, preferences: new Preferences(join(root, "ui.json")) });
const library = await demoLibrary(session);
const screen = await createTestRenderer({ width: 152, height: 46, exitSignals: [] });
const app = new App(screen.renderer, session, { quit() {}, workspaces: library });
app.render();
await screen.flush();
const opened = measure();
const facts = session.host.seq;
console.log(`A life of ${facts} facts opened in ${((performance.now() - began) / 1000).toFixed(1)} s.`);

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
const top = up.first;
await scroll(true);
const back = await scroll(false);
console.log(
  `The feed scrolled ${up.steps} pages to its start and back. The heap held ${mb(opened)} MB once open, and ${mb(heap)} MB at most; the resident size was ${mb(resident)} MB at most.`,
);
const lost = top !== back.first;
if (lost) console.log("The scroll back did not show the start of the feed as the first scroll did.");
app.dispose();
screen.renderer.destroy();
await library.dispose();
process.exit(heap > Number(bound) * 2 ** 20 || lost ? 1 : 0);

/** bun run memory <record> [bound in MB]: how much memory a window holds on a long life, while it scrolls the whole
 * life. It copies the record into a folder of its own, so the life that it opens writes the copy alone, opens it in a
 * daemon of its own, as a window of the operator does, and measures it as tui/script/heap.ts says, the measure that the
 * test of the gates, tui/test/window-heap.test.ts, takes too. It prints the facts of the life, the heap before the
 * open, the heap after it, the highest heap of the scroll, and the highest resident size of the process, and fails when
 * the heap ever passes the bound, 512 MB by default. It fails too when the scroll back loses the first lines that the
 * feed showed at its start. */

import { copyFileSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";

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
const { hold } = await import("./heap.ts");

const mb = (bytes: number) => Math.round(bytes / 2 ** 20);
const held = await hold(record, root);
console.log(`The feed holds ${held.rows} rows of ${held.acts} acts.`);
console.log(`A life of ${held.facts} facts opened in ${held.seconds.toFixed(1)} s.`);
console.log(
  `The feed scrolled ${held.pages} pages to its start and back. The heap held ${mb(held.before)} MB before the open, ${mb(held.opened)} MB once open, and ${mb(held.most)} MB at most; the resident size was ${mb(held.resident)} MB at most.`,
);
if (held.lost) console.log("The scroll back did not show the start of the feed as the first scroll did.");
process.exit(held.most > Number(bound) * 2 ** 20 || held.lost ? 1 : 0);

// A window holds what it shows of a life, not the length of the life: on a real life whose commands printed megabytes,
// which a daemon replays, its heap grows by less than a bound once it opened the life and scrolled the whole feed. The
// measure is tui/script/heap.ts, which `bun run memory` takes too.
import { expect, test } from "bun:test";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { hold } from "../script/heap.ts";
import { openEngine } from "../src/bridge.ts";
import { ended } from "./ended.ts";

/** How many commands the life holds, and how many numbers each prints on one line: about a megabyte. */
const COMMANDS = 12;
const PRINTED = 150000;
/** The most that the heap of the window may grow over what the process held before it opened the life, in bytes. On
 * this life the window grows by about 15 MB, by 84 MB when it keeps the whole texts of each act, and by 99 MB when it
 * keeps each fact. */
const BOUND = 40 * 2 ** 20;
/** The least that the life weighs, in bytes of its record: a command that printed less would make a proof of nothing. */
const WEIGHT = 40 * 2 ** 20;

test("a window on a life that printed megabytes keeps a heap that no text of the life grows", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-window-heap-"));
  const record = join(directory, "life.jsonl");
  try {
    const first = await openEngine({ cwd: directory, record, demo: true });
    let last = "";
    for (let k = 0; k < COMMANDS; k++)
      last = await first.engine.rung({
        word: `await bash("seq -s ' ' 1 ${PRINTED}")`,
        on: first.engine.root,
      });
    await first.engine.result(last);
    await first.host.dispose();
    await ended(record);
    // The life opens again from its record whole, with no checkpoint, so that its window hears each fact of it.
    const kept = await readFile(record, "utf8");
    await writeFile(
      record,
      kept
        .split("\n")
        .filter((line) => !line.startsWith('[["checkpoint",'))
        .join("\n"),
    );
    expect(Buffer.byteLength(kept)).toBeGreaterThan(WEIGHT);
    const held = await hold(record, directory);
    expect(held.lost).toBe(false);
    expect(held.grew).toBeLessThan(BOUND);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}, 600000);

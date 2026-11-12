// A window holds what it shows of a life, not the length of the life: on a real life whose commands printed megabytes,
// which a daemon replays, its heap grows by less than a bound once it opened the life and scrolled the whole feed. The
// measure is tui/script/heap.ts, which `bun run memory` takes too.
import { expect, test } from "bun:test";
import { mkdtemp, rm, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { heapNow, hold } from "../script/heap.ts";
import { realLife } from "./real-life.ts";

/** How many commands the life holds, and how many numbers each prints on one line: about a megabyte. */
const COMMANDS = 12;
const PRINTED = 150000;
/** The most that the heap of the window may grow over what the process held before it made the life, in bytes. It
 * stands between the growth of the slim window and that of a window that keeps the whole texts of each act or each
 * fact, which grow it several times more. Only a Mac measured those two, so the bound holds on a runner of CI by that
 * margin alone: the runners ran it with the slim window in, and never out. */
const BOUND = 40 * 2 ** 20;
/** The least that the record of the life weighs once its checkpoint is gone, which is what its window hears: the bytes
 * that the commands print, which the record holds at least once. A command that printed less would make a proof of
 * nothing. The size of the record is no such count: the same life weighed 22.7 MB on a Mac and 15.9 MB on a runner of
 * Linux, so the weight counts the prints. */
const WEIGHT = COMMANDS * Array.from({ length: PRINTED }, (_, at) => at + 1).join(" ").length;

test("a window on a life that printed megabytes keeps a heap that no text of the life grows", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-window-heap-"));
  const record = join(directory, "life.jsonl");
  const before = heapNow();
  try {
    await realLife(
      record,
      Array.from({ length: COMMANDS }, () => `await bash("seq -s ' ' 1 ${PRINTED}")`),
    );
    expect((await stat(record)).size).toBeGreaterThan(WEIGHT);
    const held = await hold(record, directory, before);
    expect(held.lost).toBe(false);
    expect(held.grew).toBeLessThan(BOUND);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}, 600000);

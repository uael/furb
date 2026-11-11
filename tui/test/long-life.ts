import { readFile, writeFile } from "node:fs/promises";
import { dirname } from "node:path";
import { openEngine } from "../src/bridge.ts";
import { ended } from "./ended.ts";

/** A real life, made through the daemon: each word runs as a rung of the root, queued one after the other with no
 * wait, as a person sends them while the life works. The record ends with no checkpoint, so a later window hears each
 * fact of the life and its feed holds a card for each act. */
export async function longLife(record: string, words: string[]): Promise<void> {
  const first = await openEngine({ cwd: dirname(record), record, demo: true });
  let last = "";
  for (const word of words) last = await first.engine.rung({ word, on: first.engine.root });
  await first.engine.result(last);
  await first.host.dispose();
  await ended(record);
  const kept = await readFile(record, "utf8");
  await writeFile(
    record,
    kept
      .split("\n")
      .filter((line) => !line.startsWith('[["checkpoint",'))
      .join("\n"),
  );
}

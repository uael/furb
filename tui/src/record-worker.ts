import { inspectRecord } from "furb";
import { recordName } from "./names.ts";

declare const self: Worker & { close(): void };
self.onmessage = async ({ data }: MessageEvent<{ paths: string[]; replay: string[] }>) => {
  const states: Record<
    string,
    { pending: number; error?: string; reading?: boolean; name?: string; said?: string; replayed: boolean }
  > = {};
  for (const path of data.paths) {
    // The name that the operator gave comes from a plain scan of the record, which no drift and no other build stops.
    const said = recordName(path);
    const named = said === undefined ? {} : { said };
    if (!data.replay.includes(path)) {
      states[path] = { pending: 0, ...named, replayed: false };
      continue;
    }
    try {
      const { pending, name } = await inspectRecord(path);
      states[path] = { pending: pending.length, ...(name ? { name } : {}), ...named, replayed: true };
    } catch (error) {
      const why = error instanceof Error ? error.message : String(error);
      // A drift is a record that another build made, which opens to read.
      states[path] = {
        pending: 0,
        error: why,
        ...(why.startsWith("Drift") ? { reading: true } : {}),
        ...named,
        replayed: true,
      };
    }
  }
  setTimeout(() => {
    self.postMessage(states);
    self.close();
  }, 0);
};

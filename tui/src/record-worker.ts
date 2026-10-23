import { inspectRecord } from "furb";

declare const self: Worker & { close(): void };
self.onmessage = async ({ data }: MessageEvent<string[]>) => {
  const states: Record<string, { pending: number; error?: string; reading?: boolean }> = {};
  for (const path of data) {
    try {
      states[path] = { pending: (await inspectRecord(path)).pending.length };
    } catch (error) {
      const why = error instanceof Error ? error.message : String(error);
      // A drift is a record that another build made, which opens to read.
      states[path] = { pending: 0, error: why, ...(why.startsWith("Drift") ? { reading: true } : {}) };
    }
  }
  setTimeout(() => {
    self.postMessage(states);
    self.close();
  }, 0);
};

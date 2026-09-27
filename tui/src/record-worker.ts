import { inspectRecord } from "@furb/engine";

declare const self: Worker & { close(): void };
self.onmessage = async ({ data }: MessageEvent<string[]>) => {
  const states: Record<string, { pending: number; error?: string }> = {};
  for (const path of data) {
    try {
      states[path] = { pending: (await inspectRecord(path)).pending.length };
    } catch (error) {
      states[path] = { pending: 0, error: error instanceof Error ? error.message : String(error) };
    }
  }
  setTimeout(() => {
    self.postMessage(states);
    self.close();
  }, 0);
};

import { inspectRecord } from "@furb/engine";

declare const self: Worker & { close(): void };
self.onmessage = async ({ data }: MessageEvent<string[]>) => {
  const states: Record<string, { held: number; error?: string }> = {};
  for (const path of data) {
    try {
      states[path] = { held: (await inspectRecord(path)).held.length };
    } catch (error) {
      states[path] = { held: 0, error: error instanceof Error ? error.message : String(error) };
    }
  }
  setTimeout(() => {
    self.postMessage(states);
    self.close();
  }, 0);
};

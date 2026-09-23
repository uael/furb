import { inspectRecord } from "@furb/engine";
import { hostModels } from "./models.ts";

declare const self: Worker & { close(): void };
self.onmessage = async ({ data }: MessageEvent<string[]>) => {
  const states: Record<string, { pending: number; error?: string }> = {};
  const host = hostModels();
  for (const path of data) {
    try {
      states[path] = { pending: (await inspectRecord(path, host.models)).pending.length };
    } catch (error) {
      states[path] = { pending: 0, error: error instanceof Error ? error.message : String(error) };
    }
  }
  host.dispose();
  setTimeout(() => {
    self.postMessage(states);
    self.close();
  }, 0);
};

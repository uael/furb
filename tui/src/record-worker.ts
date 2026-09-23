import { inspectRecord } from "@furb/engine";
import { hostModels } from "./models.ts";

declare const self: Worker & { close(): void };
self.onmessage = async ({ data }: MessageEvent<string[]>) => {
  const states: Record<string, { held: number; error?: string }> = {};
  const host = hostModels();
  for (const path of data) {
    try {
      states[path] = { held: (await inspectRecord(path, host.models)).held.length };
    } catch (error) {
      states[path] = { held: 0, error: error instanceof Error ? error.message : String(error) };
    }
  }
  host.dispose();
  setTimeout(() => {
    self.postMessage(states);
    self.close();
  }, 0);
};

import { inspectRecord, retold } from "furb";

declare const self: Worker & { close(): void };

/** The name that the operator said last for the session of a record, as the crate reads it with no life, and nothing
 * when the record names none or cannot be read. */
function toldSaid(path: string): string | undefined {
  try {
    return retold(path).said ?? undefined;
  } catch {
    return undefined;
  }
}
self.onmessage = async ({ data }: MessageEvent<{ paths: string[]; replay: string[] }>) => {
  const states: Record<
    string,
    {
      state: "runs" | "saved";
      error?: string;
      reading?: boolean;
      name?: string;
      said?: string;
      replayed: boolean;
    }
  > = {};
  for (const path of data.paths) {
    // The name that the operator gave comes from the reader of the crate, which runs no word, so no drift and no other
    // build stops it.
    const said = toldSaid(path);
    const named = said === undefined ? {} : { said };
    if (!data.replay.includes(path)) {
      states[path] = { state: "saved", ...named, replayed: false };
      continue;
    }
    try {
      const { state, name } = await inspectRecord(path);
      states[path] = { state, ...(name ? { name } : {}), ...named, replayed: true };
    } catch (error) {
      const why = error instanceof Error ? error.message : String(error);
      // A drift is a record that another build made, which opens to read.
      states[path] = {
        state: "saved",
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

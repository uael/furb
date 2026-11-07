import { inspectRecord, retold } from "furb";

declare const self: Worker & { close(): void };

/** The names of the session of a record, as the fold of the crate reads them with no life: the one that the operator
 * said last, and the one that wins, which is that one else the one that the extension naming gave. Nothing when the
 * record names none or cannot be read. */
function toldNames(path: string): { said?: string; name?: string } {
  try {
    const told = retold(path);
    return {
      ...(told.said == null ? {} : { said: told.said }),
      ...(told.name == null ? {} : { name: told.name }),
    };
  } catch {
    return {};
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
    const named = toldNames(path);
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

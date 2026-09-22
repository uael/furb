import { readdir, readFile, stat } from "node:fs/promises";
import { basename, resolve } from "node:path";

/** The CLI and screenshots use this same picker. */
export async function sessionChoices(
  directory: string,
  open: (path: string) => Promise<void>,
  create: () => Promise<void>,
) {
  const files = (await readdir(directory))
    .filter((path) => path.endsWith(".jsonl") && !path.endsWith(".changes.jsonl"))
    .sort()
    .reverse();
  const saved = await Promise.all(
    files.map(async (file) => {
      const path = resolve(directory, file);
      const [info, metadata, world] = await Promise.all([
        stat(path),
        readFile(`${path}.ui.json`, "utf8")
          .then((text) => JSON.parse(text) as { sessionName?: string; cost?: number })
          .catch(() => ({}) as { sessionName?: string; cost?: number }),
        readFile(`${path}.world.json`, "utf8")
          .then((text) => JSON.parse(text) as { held?: unknown[] })
          .catch(() => ({}) as { held?: unknown[] }),
      ]);
      const unfinished = world.held?.length ?? 0;
      return {
        label: metadata.sessionName ?? basename(file, ".jsonl"),
        detail: `${unfinished ? `Paused · ${unfinished} unfinished acts · ` : ""}${info.mtime.toLocaleString()} · $${(metadata.cost ?? 0).toFixed(4)} · ${(info.size / 1024).toFixed(1)} KiB`,
        run: () => open(path),
      };
    }),
  );
  return [{ label: "+ New session", detail: "Start a fresh life in this project", run: create }, ...saved];
}

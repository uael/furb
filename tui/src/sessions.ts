import { readdir, readFile, stat } from "node:fs/promises";
import { basename, resolve } from "node:path";
import { inspectRecords } from "./records.ts";
import { type SessionEntry, statusLabels } from "./workspaces.ts";

/** The CLI and screenshots use this same picker. */
export async function sessionChoices(
  directory: string,
  open: (path: string) => Promise<void>,
  create: () => Promise<void>,
  live?: ReadonlyMap<string, SessionEntry>,
) {
  const files = (await readdir(directory))
    .filter((path) => path.endsWith(".jsonl") && !path.endsWith(".changes.jsonl"))
    .sort()
    .reverse();
  const paths = [...new Set([...files.map((file) => resolve(directory, file)), ...(live?.keys() ?? [])])];
  const states = await inspectRecords(paths.filter((path) => !live?.has(path)));
  const saved = await Promise.all(
    paths.map(async (path) => {
      const [info, metadata] = await Promise.all([
        stat(path),
        readFile(`${path}.ui.json`, "utf8")
          .then((text) => JSON.parse(text) as { sessionName?: string; cost?: number })
          .catch(() => ({}) as { sessionName?: string; cost?: number }),
      ]);
      const savedAt = `${info.mtime.toLocaleDateString()} ${info.mtime.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", hour12: false })}`;
      return {
        label: live?.get(path)?.name ?? metadata.sessionName ?? basename(path, ".jsonl"),
        detail: `${live?.has(path) ? `${statusLabels[live.get(path)?.status ?? "saved"]} · ` : states[path]?.error ? "Error · " : states[path]?.held ? `Paused · ${states[path]?.held} acts · ` : ""}${savedAt} · $${(metadata.cost ?? 0).toFixed(4)} · ${(info.size / 1024).toFixed(1)} KiB`,
        run: () => open(path),
      };
    }),
  );
  return [{ label: "+ New session", detail: "Start a fresh life in this project", run: create }, ...saved];
}

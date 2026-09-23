import { type SessionEntry, statusLabels, type Workspace } from "./workspaces.ts";

/** All pickers read the same inventory as the workspace tree. */
export function sessionChoices(
  group: Workspace | undefined,
  open: (entry: SessionEntry) => Promise<void>,
  create: () => Promise<void>,
) {
  const saved = (group?.sessions ?? []).map((entry) => {
    const date = entry.modified ? new Date(entry.modified) : undefined;
    const at = date
      ? `${date.toLocaleDateString()} ${date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", hour12: false })}`
      : "";
    return {
      label: entry.name,
      detail: [
        statusLabels[entry.status],
        at,
        `$${(entry.cost ?? 0).toFixed(4)}`,
        `${((entry.size ?? 0) / 1024).toFixed(1)} KiB`,
      ]
        .filter(Boolean)
        .join(" · "),
      run: () => open(entry),
    };
  });
  return [{ label: "+ New session", detail: "Start a fresh life in this project", run: create }, ...saved];
}

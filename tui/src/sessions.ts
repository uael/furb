import { ago, dollars, kibibytes } from "./format.ts";
import { statusLabels } from "./session.ts";
import { theme } from "./theme.ts";
import type { Part } from "./ui.ts";
import type { SessionEntry, Workspace } from "./workspaces.ts";

/** All pickers read the same inventory as the workspace tree. */
export function sessionChoices(
  group: Workspace | undefined,
  open: (entry: SessionEntry) => Promise<void>,
  create: () => Promise<void>,
) {
  const saved = (group?.sessions ?? []).map((entry) => {
    const at = entry.modified ? ago(entry.modified) : "";
    return {
      label: entry.name,
      status: entry.status,
      detail: [statusLabels[entry.status], at, dollars(entry.cost ?? 0), kibibytes(entry.size ?? 0)]
        .filter(Boolean)
        .join("   "),
      run: () => open(entry),
    };
  });
  return [
    {
      label: "New session",
      detail: "Start a fresh life in this project",
      mark: ["+ ", theme.faint] as Part,
      run: create,
    },
    ...saved,
  ];
}

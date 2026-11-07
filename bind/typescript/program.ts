import { fileURLToPath } from "node:url";

/** The program furb of this tree, which `bun run build` copies here from the target of cargo, as napi copies the
 * engine beside index.cjs. The worktrees of a clone share one target, where the program of the last build of any tree
 * stands, so the TUI and its tests serve the daemon from this copy and never from the target. */
export const program = fileURLToPath(
  new URL(process.platform === "win32" ? "furb.exe" : "furb", import.meta.url),
);

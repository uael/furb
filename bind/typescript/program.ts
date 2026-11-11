import { fileURLToPath } from "node:url";

/** The program furb of this tree, which `bun run build` copies here from the target of cargo, as napi copies the
 * engine beside index.cjs, so the TUI and its tests serve the daemon from the package and never from the target. */
export const program = fileURLToPath(
  new URL(process.platform === "win32" ? "furb.exe" : "furb", import.meta.url),
);

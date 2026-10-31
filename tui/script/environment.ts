import { existsSync } from "node:fs";
import { delimiter, join } from "node:path";

/**
 * The environment of the tests, which the runner and the preload of `bun test` both give. The tests find no claude
 * command line, as on a machine that has none: a roster of the demo names a model that the aliases of a real program
 * may lack. A FURB_CLAUDE_BIN of the caller goes too. The paths are absolute, so that a run from any folder writes the
 * configs and the cache under the .furb of the root, which git ignores.
 */
export function environment(
  base: Record<string, string | undefined> = process.env,
): Record<string, string | undefined> {
  const program = process.platform === "win32" ? "claude.exe" : "claude";
  const tests = join(import.meta.dir, "../../.furb/tests");
  return {
    ...base,
    PATH: (base.PATH ?? "")
      .split(delimiter)
      .filter((folder) => folder && !existsSync(join(folder, program)))
      .join(delimiter),
    FURB_CONFIG_DIR: tests,
    XDG_CACHE_HOME: join(tests, "cache"),
    FURB_CLAUDE_BIN: "",
    FURB_LOCAL_URL: "",
    MAX_THINKING_TOKENS: "",
    FURB_TESTS: "1",
  };
}

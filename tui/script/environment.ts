import { join } from "node:path";

/**
 * The environment of the tests, which the runner and the preload of `bun test` both give. The tests find no claude
 * command line, as on a machine that has none: a roster of the demo names a model that the aliases of a real program
 * may lack. A FURB_CLAUDE_BIN that is set and empty hides it, and the PATH stays whole, since a folder with claude may
 * hold node too. The paths are absolute, so that a run from any folder writes the configs and the cache under the
 * .furb of the root, which git ignores. It holds no variable of git: a commit through the hook of a worktree hands
 * GIT_DIR and GIT_INDEX_FILE to each process that it starts, and a git init of a test under them set core.bare = true
 * in the config of the clone, which each worktree shares.
 */
export function environment(
  base: Record<string, string | undefined> = process.env,
): Record<string, string | undefined> {
  const tests = join(import.meta.dir, "../../.furb/tests");
  const kept = Object.fromEntries(
    Object.entries(base).filter(([name]) => !name.toUpperCase().startsWith("GIT_")),
  );
  return {
    ...kept,
    FURB_CONFIG_DIR: tests,
    XDG_CACHE_HOME: join(tests, "cache"),
    FURB_CLAUDE_BIN: "",
    FURB_LOCAL_URL: "",
    MAX_THINKING_TOKENS: "",
    FURB_TESTS: "1",
  };
}

import { createHash } from "node:crypto";
import { tmpdir } from "node:os";
import { join } from "node:path";

/**
 * The environment of the tests, which the runner and the preload of `bun test` both give. The tests find no claude
 * command line, as on a machine that has none: a roster of the demo names a model that the aliases of a real program
 * may lack. A FURB_CLAUDE_BIN that is set and empty hides it, and the PATH stays whole, since a folder with claude may
 * hold node too. The paths are absolute, so that a run from any folder writes the configs under the .furb of the root,
 * which git ignores, and the cache in /tmp, as the last lines say. It holds no variable of git: a commit through the
 * hook of a worktree hands GIT_DIR and GIT_INDEX_FILE to each process that it starts, and a git init of a test under
 * them set core.bare = true in the config of the clone, which each worktree shares. The cache holds the socket of the
 * daemon of the tests, which a Unix socket limits to 103 bytes on macOS, and a worktree, or the temporary directory of
 * a life, can stand at a long path. So the cache stands in /tmp, under a short name that holds a hash of the root,
 * which keeps two worktrees apart.
 */
export function environment(
  base: Record<string, string | undefined> = process.env,
  root = join(import.meta.dir, "../.."),
): Record<string, string | undefined> {
  const tests = join(root, ".furb/tests");
  const cache = join(
    process.platform === "win32" ? base.FURB_MACHINE_TMPDIR || tmpdir() : "/tmp",
    `furb-cache-${createHash("sha256").update(root).digest("hex").slice(0, 8)}`,
  );
  const kept = Object.fromEntries(
    Object.entries(base).filter(([name]) => !name.toUpperCase().startsWith("GIT_")),
  );
  return {
    ...kept,
    FURB_CONFIG_DIR: tests,
    XDG_CACHE_HOME: cache,
    FURB_CLAUDE_BIN: "",
    FURB_LOCAL_URL: "",
    MAX_THINKING_TOKENS: "",
    FURB_TESTS: "1",
  };
}

/** The words that run a heavy step, as the tests or a run of the gallery, through script/heavy.sh, which starts it only
 * while the machine has the memory for it. Windows has no sh, so a step there runs as it is. */
export const heavy =
  process.platform === "win32" ? [] : ["sh", join(import.meta.dir, "../../script/heavy.sh")];

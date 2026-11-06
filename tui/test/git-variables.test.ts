import { expect, test } from "bun:test";
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { environment } from "../script/environment.ts";

// A run of the runner under the hook of a worktree starts git with the environment of the tests. The test gives git
// that environment itself and runs no second runner: a second runner shares the daemon of the tests with the run that
// holds it, and the tests that it stalls time out.
test("git that a test starts under the variables of the hook of a worktree leaves the config of its clone as it was", () => {
  const yard = mkdtempSync(join(tmpdir(), "furb-git-variables-"));
  const clean = Object.fromEntries(
    Object.entries(process.env).filter(([name]) => !name.toUpperCase().startsWith("GIT_")),
  );
  const git = (env: Record<string, string | undefined>, ...args: string[]) =>
    execFileSync("git", ["-c", "user.name=t", "-c", "user.email=t@t", ...args], {
      cwd: yard,
      env,
      stdio: "pipe",
    });
  git(clean, "init", "-q", "-b", "main", "main");
  git(clean, "-C", "main", "commit", "-q", "--allow-empty", "-m", "base");
  git(clean, "-C", "main", "worktree", "add", "-q", "-b", "side", join(yard, "side"));
  const config = join(yard, "main", ".git", "config");
  const before = readFileSync(config, "utf8");
  const side = join(yard, "main", ".git", "worktrees", "side");
  const tests = environment({ ...clean, GIT_DIR: side, GIT_INDEX_FILE: join(side, "index") });
  expect(Object.keys(tests).filter((name) => name.toUpperCase().startsWith("GIT_"))).toEqual([]);
  // The git init of a test, as tui/test/history.test.ts makes one.
  git(tests, "init", "-q", join(yard, "made"));
  expect(readFileSync(config, "utf8")).toBe(before);
});

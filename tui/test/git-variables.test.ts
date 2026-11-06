import { expect, test } from "bun:test";
import { execFileSync, spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dir, "../..");
// Each test file that starts git. Bun gives a child the environment that its process started with, whatever a
// preload deletes, so the runner, which gives each run the environment of the tests, is the one that clears it.
const starts = [
  "tui/test/history.test.ts",
  "tui/test/review.test.ts",
  "tui/test/workspaces.test.ts",
  "bind/typescript/test/images.test.ts",
];

test("the tests that start git under the variables of the hook of a worktree leave the config of its clone as it was", () => {
  const yard = mkdtempSync(join(tmpdir(), "furb-git-variables-"));
  const clean = Object.fromEntries(Object.entries(process.env).filter(([name]) => !name.startsWith("GIT_")));
  const git = (...args: string[]) =>
    execFileSync("git", ["-c", "user.name=t", "-c", "user.email=t@t", ...args], {
      cwd: yard,
      env: clean,
      stdio: "pipe",
    });
  git("init", "-q", "-b", "main", "main");
  git("-C", "main", "commit", "-q", "--allow-empty", "-m", "base");
  git("-C", "main", "worktree", "add", "-q", "-b", "side", join(yard, "side"));
  const config = join(yard, "main", ".git", "config");
  const before = readFileSync(config, "utf8");
  const side = join(yard, "main", ".git", "worktrees", "side");
  const hook = { ...clean, GIT_DIR: side, GIT_INDEX_FILE: join(side, "index") };
  const ran = spawnSync(process.execPath, ["run", join(root, "tui/script/test.ts"), ...starts], {
    cwd: root,
    env: hook,
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  });
  // A runner that did not start or did not pass ran no test, so an unchanged config would prove nothing.
  if (ran.error || ran.status !== 0)
    throw new Error(
      `the runner failed (${ran.error ?? `exit ${ran.status}`}):\n${`${ran.stdout}${ran.stderr}`.slice(-4000)}`,
    );
  expect(readFileSync(config, "utf8")).toBe(before);
}, 600000);

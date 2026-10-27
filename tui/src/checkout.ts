import { execFileSync } from "node:child_process";
import { existsSync, realpathSync } from "node:fs";
import { basename, dirname, resolve } from "node:path";

function git(directory: string, ...args: string[]): string | undefined {
  try {
    return execFileSync("git", ["-C", directory, ...args], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
      timeout: 10_000,
    }).trim();
  } catch {
    return undefined;
  }
}

function real(path: string): string {
  try {
    return realpathSync.native(path);
  } catch {
    return path;
  }
}

/** The main checkout of the repository whose worktree a directory is, which `git rev-parse --git-common-dir` names:
 * a worktree belongs to the workspace of its main checkout, and is no workspace of its own. A directory that is no
 * root of a worktree, as a folder inside a repository, a bare repository, or a folder that git does not hold, stands
 * for itself. */
export function mainCheckout(directory: string): string {
  const top = git(directory, "rev-parse", "--show-toplevel");
  if (!top || real(top) !== real(directory)) return directory;
  const common = git(directory, "rev-parse", "--path-format=absolute", "--git-common-dir");
  return common && basename(common) === ".git" ? real(dirname(common)) : directory;
}

/** The main checkout, then each worktree that git lists for it and that stands on the disk. */
export function checkouts(main: string): string[] {
  const listed = (git(main, "worktree", "list", "--porcelain") ?? "")
    .split("\n\n")
    .filter((block) => !block.split("\n").includes("bare"))
    .map((block) => /^worktree (.+)$/m.exec(block)?.[1])
    .filter((path): path is string => path !== undefined && existsSync(path))
    .map(real);
  return [main, ...listed.filter((path) => path !== main)];
}

/** The checkout whose .furb/sessions holds a record, or the fallback when the record stands elsewhere. */
export function checkoutOf(record: string, fallback: string): string {
  const folder = dirname(resolve(record));
  return basename(folder) === "sessions" && basename(dirname(folder)) === ".furb"
    ? dirname(dirname(folder))
    : fallback;
}

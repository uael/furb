import { readFileSync, realpathSync, statSync } from "node:fs";
import { basename, dirname, join, resolve } from "node:path";

function real(path: string): string {
  try {
    return realpathSync.native(path);
  } catch {
    return path;
  }
}

/** The main checkout of the repository whose worktree a directory is: a worktree belongs to the workspace of its main
 * checkout, and is no workspace of its own. It reads the `.git` of the directory and the `commondir` that git keeps
 * beside it, and starts no process, since a process starts slowly on Windows and a window asks for it at every
 * workspace that it adds. A directory that is no root of a worktree, as a folder inside a repository, a bare
 * repository, a submodule, or a folder that git does not hold, stands for itself. */
export function mainCheckout(directory: string): string {
  const mark = join(directory, ".git");
  try {
    if (statSync(mark).isDirectory()) return real(directory);
    const line = /^gitdir:\s*(.+)$/m.exec(readFileSync(mark, "utf8"))?.[1];
    if (!line) return directory;
    const admin = resolve(directory, line.trim());
    const common = resolve(admin, readFileSync(join(admin, "commondir"), "utf8").trim());
    return basename(common) === ".git" ? real(dirname(common)) : directory;
  } catch {
    return directory;
  }
}

/** The checkout whose .furb/sessions holds a record, or the fallback when the record stands elsewhere. */
export function checkoutOf(record: string, fallback: string): string {
  const folder = dirname(resolve(record));
  return basename(folder) === "sessions" && basename(dirname(folder)) === ".furb"
    ? dirname(dirname(folder))
    : fallback;
}

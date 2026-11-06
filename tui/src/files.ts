import { realpathSync } from "node:fs";
import { stat } from "node:fs/promises";
import { homedir } from "node:os";
import { basename, dirname, isAbsolute, join, relative, resolve, sep } from "node:path";
import { walkedFiles } from "furb";

/** A path as the user typed it, with a leading `~` read as the home directory. */
export function expandHome(path: string): string {
  return path === "~" || path.startsWith("~/") || path.startsWith(`~${sep}`)
    ? join(homedir(), path.slice(1))
    : path;
}

/** A path as the review shows it: from the directory, with `/` between its parts, when the path is under it, and else
 * as it stands. A system can name one folder in two spellings, as Windows does with a short name such as `RUNNER~1`,
 * so the path and the directory are each compared in the spelling of the file system too. */
export function fromDirectory(directory: string, path: string): string {
  const real = (one: string) => {
    try {
      return realpathSync.native(one);
    } catch {
      return one;
    }
  };
  const places = new Set([directory, real(directory)]);
  for (const one of new Set([path, join(real(dirname(path)), basename(path))]))
    for (const place of places) {
      const inside = relative(place, one);
      if (inside && !inside.startsWith("..") && !isAbsolute(inside)) return inside.split(sep).join("/");
    }
  return path;
}

/** A path as the view shows it: under the home directory, with `~` in place of the home directory. */
export function shortenHome(path: string): string {
  const home = homedir();
  return path === home ? "~" : path.startsWith(`${home}${sep}`) ? `~${path.slice(home.length)}` : path;
}

/** A text as the view shows it, with `~` in place of the home directory in each path under it. */
export function shortenHomes(text: string): string {
  const home = homedir();
  return home.length > 1 ? text.replaceAll(`${home}${sep}`, `~${sep}`) : text;
}

/** The files of the project, as the crate walks them with the walker of ripgrep: each file that a .gitignore hides left
 * out, each dot file listed, and .git itself left out. A file is named by its path from the directory with `/` between
 * its parts, which every system reads, so a reference to it is the same text on every system. */
export async function projectFiles(directory: string): Promise<string[]> {
  return walkedFiles(directory);
}

/** The files that a text references from a directory: each @word, or @"words" and @'words' for a path with
 * spaces, that names a file there. Any other @word, such as a decorator, a package scope or a handle, is text. */
export async function fileReferences(text: string, directory: string): Promise<string[]> {
  const words = new Set(
    [...text.matchAll(/(?:^|\s)@(?:"([^"\n]+)"|'([^'\n]+)'|([^\s]+))/g)].map(
      (match) => match[1] ?? match[2] ?? match[3] ?? "",
    ),
  );
  const named = await Promise.all(
    [...words].map(async (word) => {
      try {
        return (await stat(resolve(directory, expandHome(word)))).isFile() ? word : undefined;
      } catch (error) {
        if (["ENOENT", "ENOTDIR", "ENAMETOOLONG"].includes((error as NodeJS.ErrnoException).code ?? ""))
          return undefined;
        throw error;
      }
    }),
  );
  return named.filter((word) => word !== undefined);
}

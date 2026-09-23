import { mkdir, readdir, stat, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import { join, resolve } from "node:path";

/** A path as the user typed it, with a leading `~` read as the home directory. */
export function expandHome(path: string): string {
  return path === "~" ? homedir() : path.startsWith("~/") ? join(homedir(), path.slice(2)) : path;
}

/** The directory `.furb/<parts>` of a project. The TUI keeps transcripts, file contents and shares there, so the
 * `.furb` it makes gets an ignore rule that keeps all of it out of version control. */
export async function furbDirectory(project: string, ...parts: string[]): Promise<string> {
  const furb = join(project, ".furb");
  try {
    await mkdir(furb);
    await writeFile(join(furb, ".gitignore"), "*\n");
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "EEXIST") throw error;
  }
  const directory = join(furb, ...parts);
  await mkdir(directory, { recursive: true });
  return directory;
}

/** Match project files through rg, with a directory walk when it is not installed. A folder that cannot be read
 * holds no file to offer. */
export async function projectFiles(directory: string): Promise<string[]> {
  try {
    const child = Bun.spawn(
      ["rg", "--files", "--hidden", "-g", "!.git", "-g", "!.furb", "-g", "!node_modules"],
      { cwd: directory, stdout: "pipe", stderr: "pipe" },
    );
    const text = await new Response(child.stdout).text();
    const code = await child.exited;
    const files = text.split("\n").filter(Boolean).sort();
    // rg exits with 2 when it could not read a part of the project, after it listed every file it could read.
    if (code === 0 || code === 1 || (code === 2 && files.length)) return files;
    throw new Error(await new Response(child.stderr).text());
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
    const result: string[] = [];
    const walk = async (relative: string) => {
      const entries = await readdir(join(directory, relative), { withFileTypes: true }).catch(
        (error: NodeJS.ErrnoException) => {
          if (relative && (error.code === "EACCES" || error.code === "EPERM")) return [];
          throw error;
        },
      );
      for (const entry of entries) {
        if ([".git", ".furb", "node_modules"].includes(entry.name)) continue;
        const path = join(relative, entry.name);
        if (entry.isDirectory()) await walk(path);
        else if (entry.isFile()) result.push(path);
      }
    };
    await walk("");
    return result.sort();
  }
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
        return (await stat(resolve(directory, word))).isFile() ? word : undefined;
      } catch (error) {
        if (["ENOENT", "ENOTDIR", "ENAMETOOLONG"].includes((error as NodeJS.ErrnoException).code ?? ""))
          return undefined;
        throw error;
      }
    }),
  );
  return named.filter((word) => word !== undefined);
}

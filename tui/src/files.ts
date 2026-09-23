import { readdir } from "node:fs/promises";
import { join } from "node:path";

/** Match project files through rg, with a directory walk when it is not installed. */
export async function projectFiles(directory: string): Promise<string[]> {
  try {
    const child = Bun.spawn(
      ["rg", "--files", "--hidden", "-g", "!.git", "-g", "!.furb", "-g", "!node_modules"],
      { cwd: directory, stdout: "pipe", stderr: "pipe" },
    );
    const text = await new Response(child.stdout).text();
    const code = await child.exited;
    if (code === 0 || code === 1) return text.split("\n").filter(Boolean).sort();
    throw new Error(await new Response(child.stderr).text());
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
    const result: string[] = [];
    const walk = async (relative: string) => {
      for (const entry of await readdir(join(directory, relative), { withFileTypes: true })) {
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

export function fileReferences(text: string): string[] {
  return [
    ...new Set(
      [...text.matchAll(/(?:^|\s)@(?:"([^"\n]+)"|'([^'\n]+)'|([^\s]+))/g)].map(
        (match) => match[1] ?? match[2] ?? match[3] ?? "",
      ),
    ),
  ];
}

import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

/** The directory `.furb/<parts>` of a project, made when it is absent. Records, file contents, shares and images
 * stay there, so the `.furb` it makes holds an ignore rule that keeps all of it out of version control. */
export function furbDirectory(project: string, ...parts: string[]): string {
  const furb = join(project, ".furb");
  try {
    mkdirSync(furb);
    writeFileSync(join(furb, ".gitignore"), "*\n");
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "EEXIST") throw error;
  }
  const directory = join(furb, ...parts);
  mkdirSync(directory, { recursive: true });
  return directory;
}

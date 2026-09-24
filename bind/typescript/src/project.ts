import { mkdirSync, renameSync, writeFileSync } from "node:fs";
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

/** Save a file whole: the text goes to `<path>.tmp`, which then takes the name of the file, so no reader sees a part
 * of it. A save that fails throws an error that names its file, with the error of the system as its cause, since
 * the error of the runtime does not always name the file: bun 1.4 on Windows says only "write". */
export function saveFile(path: string, text: string): void {
  const draft = `${path}.tmp`;
  try {
    writeFileSync(draft, text, { mode: 0o600 });
    renameSync(draft, path);
  } catch (error) {
    throw new Error(`Could not save ${path} through ${draft}: ${(error as Error).message}`, { cause: error });
  }
}

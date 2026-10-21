import { mkdirSync, renameSync, writeFileSync } from "node:fs";
import { join } from "node:path";

/** The directory `.furb/<parts>` of a project, made when it is absent. Records, file contents, shares and images
 * stay there, so the `.furb` it makes holds an ignore rule that keeps all of it out of version control, but the
 * config of the project, which a project shares. */
export function furbDirectory(project: string, ...parts: string[]): string {
  const furb = join(project, ".furb");
  try {
    mkdirSync(furb);
    writeFileSync(join(furb, ".gitignore"), "*\n!config.json\n");
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "EEXIST") throw error;
  }
  const directory = join(furb, ...parts);
  mkdirSync(directory, { recursive: true });
  return directory;
}

/** The codes by which Windows refuses to replace a file that another handle holds open, as a reader of it does. */
const HELD = new Set(["EPERM", "EACCES", "EBUSY"]);
/** How long a save waits for another handle to let its file go, in milliseconds. */
const RELEASE = 5000;
/** The cell that a save waits on between two tries, which nothing wakes. */
const pause = new Int32Array(new SharedArrayBuffer(4));

/** Save a file whole: the text goes to `<path>.tmp`, which then takes the name of the file, so no reader sees a part
 * of it. On Windows a file that another handle holds open cannot take a new file in its place, so the save tries
 * again until that handle lets the file go, for RELEASE milliseconds at most: a session that reads the state of a
 * record as it lists it, in this process or in another, holds that file for a moment. A save that fails throws an
 * error that names its file, with the error of the system as its cause, since the error of the runtime does not
 * always name the file: bun 1.4 on Windows says only "write". */
export function saveFile(path: string, text: string): void {
  const draft = `${path}.tmp`;
  try {
    writeFileSync(draft, text, { mode: 0o600 });
    for (const deadline = Date.now() + RELEASE; ; Atomics.wait(pause, 0, 0, 20))
      try {
        renameSync(draft, path);
        break;
      } catch (error) {
        const code = (error as NodeJS.ErrnoException).code ?? "";
        if (process.platform !== "win32" || !HELD.has(code) || Date.now() >= deadline) throw error;
      }
  } catch (error) {
    throw new Error(`Could not save ${path} through ${draft}: ${(error as Error).message}`, { cause: error });
  }
}

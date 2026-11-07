import { lstatSync, readdirSync, realpathSync } from "node:fs";
import { homedir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";

/** The folder of the config directory where the engine installs the words that it ships, which no test chooses. */
const installed = "words";

/** Each file under a folder, by its path, with its size and the time of its change, but for the words that the engine
 * installs. A folder that is not there holds none. */
export function snapshot(folder: string): Map<string, string> {
  const files = new Map<string, string>();
  const walk = (at: string): void => {
    let names: string[];
    try {
      names = readdirSync(at);
    } catch {
      return;
    }
    for (const name of names) {
      if (at === folder && name === installed) continue;
      const path = join(at, name);
      try {
        const stat = lstatSync(path);
        if (stat.isDirectory()) walk(path);
        else files.set(path, `${stat.size}:${stat.mtimeMs}`);
      } catch {
        // A file that went while the folder was read is a change that the next snapshot shows.
      }
    }
  };
  walk(folder);
  return files;
}

/** The paths that one snapshot holds and the other does not, or holds with another size or time. */
export function changed(before: Map<string, string>, after: Map<string, string>): string[] {
  return [...new Set([...before.keys(), ...after.keys()])]
    .filter((path) => before.get(path) !== after.get(path))
    .sort();
}

/** The config directories that no test may write: the one that the native part of this process reads, and the one that
 * the user has on this machine, each but the directory of the tests. The two differ when a plain `bun test` gives the
 * tests their folder through process.env, which the native part, reading the environment that the process started
 * with, never sees. */
export function realConfigs(
  tests: string,
  native: string,
  env: Record<string, string | undefined> = process.env,
): string[] {
  const base =
    env.XDG_CONFIG_HOME || (process.platform === "win32" && env.APPDATA) || join(homedir(), ".config");
  return [...new Set([native, join(base, "furb")].map((one) => resolve(one)))].filter(
    (one) => one !== resolve(tests),
  );
}

/** A path as the files of the machine name it: its folder with the links followed, since a config directory is often
 * reached through a link, as /var is on macOS. A path whose folder is not there stays as it is. */
export function real(path: string): string {
  const full = resolve(path);
  try {
    return join(realpathSync(dirname(full)), basename(full));
  } catch {
    return full;
  }
}

/** Whether a path is the draft that a save writes before it renames it over the file. */
const draft = (path: string): boolean => path.endsWith(".tmp");

/** The paths that this process saved through saveFile since the guard last looked. */
export const saved = new Set<string>();

/** The saveFile that notes each path that it saves in `saved`, and saves it as `save` does. */
export function recording(save: (path: string, text: string) => void): (path: string, text: string) => void {
  return (path, text) => {
    saved.add(real(path));
    save(path, text);
  };
}

/** A function that gives the files of the folders that the run wrote since the last call, or since the start. A file
 * that appeared or went is a write of the run, but for the draft of a save, which another process of the machine
 * makes and renames in the time between two looks. A file that changed is a write of the run only if the run saved it
 * through saveFile. Each watcher owns the saves it reads when it is given its own set. */
export function watch(folders: string[], saves: Set<string> = saved): () => string[] {
  let before = folders.map(snapshot);
  return () => {
    const after = folders.map(snapshot);
    const found = after.flatMap((now, at) => {
      const was = before[at] ?? new Map();
      return changed(was, now).filter(
        (path) => !draft(path) && (!(was.has(path) && now.has(path)) || saves.has(real(path))),
      );
    });
    saves.clear();
    before = after;
    return found;
  };
}

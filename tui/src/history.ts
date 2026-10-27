import { existsSync, readFileSync, realpathSync, statSync } from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { furbDirectory, saveFile } from "furb";

/** A line that the input sent: its text, its mode, and the record of the session that sent it. */
export interface Sent {
  text: string;
  mode: string;
  session: string;
}

/** How many lines the history of a workspace keeps. The oldest go first. */
const LIMIT = 1000;

/** The mode of a draft key, `<chain>:<mode>`, with its colon. */
export function modeOf(key: string): string {
  return key.slice(key.indexOf(":"));
}

function real(path: string): string {
  try {
    return realpathSync(path);
  } catch {
    return resolve(path);
  }
}

/** The main checkout of the worktree whose `.git` file is at a marker, or the folder itself when the file names no
 * worktree, as the file of a submodule does. */
function linked(directory: string, marker: string): string {
  try {
    const gitdir = resolve(
      directory,
      /^gitdir:\s*(.+)$/m.exec(readFileSync(marker, "utf8"))?.[1]?.trim() ?? "",
    );
    const common = resolve(gitdir, readFileSync(join(gitdir, "commondir"), "utf8").trim());
    return real(basename(common) === ".git" ? dirname(common) : common);
  } catch {
    return directory;
  }
}

/** The folder that owns the history of a directory: the nearest folder above it that holds a `.git`, and for a
 * worktree the main checkout that its `.git` file leads to, since a worktree is part of the workspace of its
 * repository. A directory in no repository owns its own history. */
export function workspaceRoot(directory: string): string {
  const start = real(directory);
  for (let folder = start; ; folder = dirname(folder)) {
    const marker = join(folder, ".git");
    if (existsSync(marker)) return statSync(marker).isDirectory() ? folder : linked(folder, marker);
    if (dirname(folder) === folder) return start;
  }
}

const path = (root: string) => join(root, ".furb", "history.json");
const read = new Map<string, { stamp: string; sent: Sent[] }>();

/** What the sessions of a workspace sent, the oldest first, as `<root>/.furb/history.json` holds it. A read costs a
 * look at the time and the size of the file, and parses it again only when they changed. */
export function sentIn(root: string): Sent[] {
  const file = path(root);
  let stamp: string;
  try {
    const info = statSync(file);
    stamp = `${info.mtimeMs}:${info.size}`;
  } catch {
    return [];
  }
  const held = read.get(file);
  if (held?.stamp === stamp) return held.sent;
  let sent: Sent[] = [];
  try {
    const data: unknown = JSON.parse(readFileSync(file, "utf8"));
    if (Array.isArray(data))
      sent = data.filter(
        (one): one is Sent =>
          Boolean(one) &&
          typeof one.text === "string" &&
          typeof one.mode === "string" &&
          typeof one.session === "string",
      );
  } catch {
    // A file that holds no list of lines is the same as no history. The next line saved replaces it.
  }
  read.set(file, { stamp, sent });
  return sent;
}

function store(root: string, sent: Sent[]): void {
  furbDirectory(root);
  saveFile(path(root), JSON.stringify(sent.slice(-LIMIT)));
}

/** Add a line to the history of a workspace, unless the last line is the same. */
export function append(root: string, one: Sent): void {
  const all = sentIn(root);
  const last = all.at(-1);
  if (last && last.text === one.text && last.mode === one.mode && last.session === one.session) return;
  store(root, [...all, one]);
}

/** Give the history of a workspace the lines that a session kept in its own view before the workspace kept any, once:
 * nothing happens when the history holds a line of that session already. They count as older than the lines it holds. */
export function seed(root: string, session: string, histories: Record<string, string[]>): void {
  const all = sentIn(root);
  if (all.some((one) => one.session === session)) return;
  const old = Object.entries(histories).flatMap(([key, texts]) =>
    texts.map((text) => ({ text, mode: modeOf(key), session })),
  );
  if (old.length) store(root, [...old, ...all]);
}

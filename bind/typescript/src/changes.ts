import {
  closeSync,
  existsSync,
  fsyncSync,
  mkdirSync,
  mkdtempSync,
  openSync,
  readFileSync,
  readSync,
  renameSync,
  rmSync,
  truncateSync,
  writeFileSync,
  writeSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { call, type Ear } from "./ears.js";
import { type Fact, isQuestion } from "./types.js";

export interface FileChange {
  path: string;
  before: string;
  after: string;
  /** The write that made the change. A change that an older session kept names none. */
  act?: string;
}

/** A journal of file changes that holds a line that is no change, as a crash or another program leaves it. Its message
 * says what broke in plain words, and the journal can be repaired, or started anew. */
export class DamagedJournal extends Error {
  override name = "DamagedJournal";
  constructor(
    readonly path: string,
    readonly change: number,
  ) {
    super(`The journal of file changes at ${path} is damaged: change ${change + 1} cannot be read.`);
  }
}

/** Whether a line of a journal is a change. */
function isChange(line: string): boolean {
  try {
    const change: unknown = JSON.parse(line);
    return typeof change === "object" && change !== null && typeof (change as FileChange).path === "string";
  } catch {
    return false;
  }
}

/** Append file snapshots once. Keep only their positions in memory and read a page on demand. As an ear that comes
 * before the files, it reads the text a write replaces before the files take the write, and keeps the change once the
 * write is done. */
export class FileChanges {
  private readonly positions: { start: number; size: number }[] = [];
  private readonly temporary?: string;
  /** The path of the journal, beside the record or in a temporary directory. */
  readonly path?: string;
  private fd?: number;
  private end = 0;
  constructor(record?: string, readOnly = false) {
    if (readOnly) return;
    if (!record) this.temporary = mkdtempSync(join(tmpdir(), "furb-changes-"));
    const path = record ? `${record}.changes.jsonl` : join(this.temporary ?? "", "changes.jsonl");
    this.path = path;
    // The changes stand beside the record, whose directory may not stand yet when the session is made.
    mkdirSync(dirname(path), { recursive: true });
    this.index(path);
    this.fd = openSync(path, "a+", 0o600);
  }
  /** The position of each complete line of the journal, which a page reads; a line that a crash cut goes. A line that
   * is no change opens with the session, and the page that holds it says that the journal is damaged. */
  private index(path: string): void {
    this.positions.length = 0;
    this.end = 0;
    if (!existsSync(path)) return;
    const data = readFileSync(path);
    while (this.end < data.length) {
      const newline = data.indexOf(10, this.end);
      if (newline < 0) {
        truncateSync(path, this.end);
        break;
      }
      this.positions.push({ start: this.end, size: newline - this.end });
      this.end = newline + 1;
    }
  }
  get length(): number {
    return this.positions.length;
  }
  /** The ear of the changes, for a life that stands on this directory: it says nothing, so each write goes on to the
   * ear that takes it. */
  *ear(directory: string, changed?: () => void): Ear {
    const before = new Map<string, { path: string; before: string }>();
    for (;;) {
      const fact = (yield null) as Fact | undefined;
      if (!fact) continue;
      const [kind, id, , ...words] = fact;
      if (kind === "write" && isQuestion(kind, id)) {
        const text = words[1] as { path?: unknown } | undefined;
        if (typeof text?.path !== "string" || text.path.includes("://")) continue;
        const here = String(yield call("cwd", [], { on: words[0] }));
        const path = resolve(directory, here, text.path);
        let old = "";
        try {
          old = readFileSync(path, "utf8");
        } catch {}
        before.set(id, { path, before: old });
      } else if (kind === "done" && before.has(id)) {
        const held = before.get(id);
        before.delete(id);
        const after = words[0] as { is?: unknown; content?: unknown } | undefined;
        if (!held || after?.is !== "Text" || typeof after.content !== "string") continue;
        this.append({ ...held, after: after.content, act: id });
        // A listener may ask the life, which no ear may do while it hears, so the host hears of it after the ear.
        if (changed) queueMicrotask(changed);
      }
    }
  }
  append(change: FileChange): void {
    if (this.fd === undefined) return;
    const data = Buffer.from(`${JSON.stringify(change)}\n`);
    writeSync(this.fd, data);
    fsyncSync(this.fd);
    this.positions.push({ start: this.end, size: data.length - 1 });
    this.end += data.length;
  }
  /** A page of changes; a line of it that is no change throws a `DamagedJournal`. */
  read(start = 0, count = 20): FileChange[] {
    const fd = this.fd;
    if (fd === undefined) return [];
    return this.positions.slice(start, start + count).map((position, at) => {
      const data = Buffer.alloc(position.size);
      readSync(fd, data, 0, position.size, position.start);
      const line = data.toString("utf8");
      if (!isChange(line)) throw new DamagedJournal(this.path ?? "", start + at);
      return JSON.parse(line) as FileChange;
    });
  }
  /** The journal written again with each line that is a change, in its order, and the count of lines that it lost. */
  repair(): number {
    const path = this.path;
    if (this.fd === undefined || !path) return 0;
    const lines = readFileSync(path, "utf8").split("\n").slice(0, this.positions.length);
    const kept = lines.filter(isChange);
    this.reopen(path, () => {
      writeFileSync(`${path}.repair`, kept.map((line) => `${line}\n`).join(""), { mode: 0o600 });
      renameSync(`${path}.repair`, path);
    });
    return lines.length - kept.length;
  }
  /** A new journal that holds no change, and the path that the damaged journal is kept at beside it. */
  renew(): string {
    const path = this.path;
    if (this.fd === undefined || !path) return "";
    const aside = `${path}.damaged-${Date.now()}`;
    this.reopen(path, () => renameSync(path, aside));
    return aside;
  }
  /** The journal closed, changed on disk, and opened again on its lines. */
  private reopen(path: string, change: () => void): void {
    if (this.fd !== undefined) closeSync(this.fd);
    change();
    this.index(path);
    this.fd = openSync(path, "a+", 0o600);
  }
  dispose(): void {
    if (this.fd !== undefined) closeSync(this.fd);
    if (this.temporary) rmSync(this.temporary, { recursive: true });
  }
}

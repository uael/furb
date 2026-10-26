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
  /** Whether the write made the file, which did not stand before it. A change that an older session kept says
   * nothing of it. */
  made?: boolean;
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

/** The change that a line of a journal holds, or nothing when the line is no change. */
function parse(line: string): FileChange | undefined {
  try {
    const change: unknown = JSON.parse(line);
    return typeof change === "object" &&
      change !== null &&
      typeof (change as FileChange).path === "string" &&
      typeof (change as FileChange).before === "string" &&
      typeof (change as FileChange).after === "string"
      ? (change as FileChange)
      : undefined;
  } catch {
    return undefined;
  }
}

/** What a repair of a journal kept and removed: the count of changes that it kept, and of lines that it removed. */
export interface Repaired {
  kept: number;
  removed: number;
}

/** Append file snapshots once. Keep only their positions in memory and read a page on demand. As an ear that comes
 * before the files, it reads the text a write replaces before the files take the write, and keeps the change once the
 * write is done. Open to read, it reads the existing journal without creating or changing a file, and refuses writes.
 * A failed open is reported by a read, so the other views of a record can still open. */
export class FileChanges {
  private readonly positions: { start: number; size: number }[] = [];
  private readonly temporary?: string;
  /** The path of the journal, beside the record or in a temporary directory. */
  readonly path?: string;
  private fd?: number;
  private end = 0;
  private tail = -1;
  private cut = false;
  private failure: unknown;
  constructor(
    record?: string,
    private readonly readOnly = false,
  ) {
    if (readOnly && !record) return;
    if (!record) this.temporary = mkdtempSync(join(tmpdir(), "furb-changes-"));
    const path = record ? `${record}.changes.jsonl` : join(this.temporary ?? "", "changes.jsonl");
    this.path = path;
    if (readOnly) {
      try {
        this.index(path);
      } catch (error) {
        // A record can still be read when its journal cannot: its changes view reports this failed read.
        this.failure = error;
      }
      return;
    }
    // The changes stand beside the record, whose directory may not stand yet when the session is made.
    mkdirSync(dirname(path), { recursive: true });
    this.index(path);
    this.fd = openSync(path, "a+", 0o600);
  }
  /** The position of each complete line of the journal. A writable journal replaces a cut tail only on a write,
   * since opening a life can still fail. Open to read, the tail stays and its page reports it as damaged. */
  private index(path: string): void {
    this.positions.length = 0;
    this.end = 0;
    this.tail = -1;
    this.cut = false;
    if (!existsSync(path)) return;
    const data = readFileSync(path);
    while (this.end < data.length) {
      const newline = data.indexOf(10, this.end);
      if (newline < 0) {
        if (this.readOnly) {
          this.tail = this.positions.length;
          this.positions.push({ start: this.end, size: data.length - this.end });
        } else this.cut = true;
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
    const before = new Map<string, { path: string; before: string; made?: boolean }>();
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
        let made = false;
        try {
          old = readFileSync(path, "utf8");
        } catch {
          made = !existsSync(path);
        }
        before.set(id, { path, before: old, ...(made ? { made } : {}) });
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
  private writable(): void {
    if (this.readOnly) throw new Error("The journal of file changes is open to read.");
  }
  append(change: FileChange): void {
    this.writable();
    const path = this.path;
    if (this.fd === undefined || !path) return;
    if (this.cut) this.reopen(path, () => truncateSync(path, this.end));
    const data = Buffer.from(`${JSON.stringify(change)}\n`);
    writeSync(this.fd, data);
    fsyncSync(this.fd);
    this.positions.push({ start: this.end, size: data.length - 1 });
    this.end += data.length;
  }
  /** Read a journal page without holding its file open between calls. */
  private withReader<T>(read: (fd: number) => T): T | undefined {
    if (!this.readOnly) return this.fd === undefined ? undefined : read(this.fd);
    const path = this.path;
    if (!path || !existsSync(path)) return undefined;
    const fd = openSync(path, "r");
    try {
      return read(fd);
    } finally {
      closeSync(fd);
    }
  }
  /** The line of the journal at a position. */
  private line(fd: number, position: { start: number; size: number }): string {
    const data = Buffer.alloc(position.size);
    readSync(fd, data, 0, position.size, position.start);
    return data.toString("utf8");
  }
  /** A page of changes; a line of it that is no change throws a `DamagedJournal`. */
  read(start = 0, count = 20): FileChange[] {
    if (this.failure !== undefined) throw this.failure;
    if (this.readOnly) this.index(this.path ?? "");
    return (
      this.withReader((fd) =>
        this.positions.slice(start, start + count).map((position, at) => {
          const change = start + at === this.tail ? undefined : parse(this.line(fd, position));
          if (!change) throw new DamagedJournal(this.path ?? "", start + at);
          return change;
        }),
      ) ?? []
    );
  }
  /** A page of lines of the journal as changes, with each line that is no change left out. A reader that must not
   * stop for a damaged journal, as the rewind, reads so. */
  readable(start = 0, count = 20): FileChange[] {
    if (this.failure !== undefined) throw this.failure;
    if (this.readOnly) this.index(this.path ?? "");
    return (
      this.withReader((fd) =>
        this.positions
          .slice(start, start + count)
          .flatMap((position, at) =>
            start + at === this.tail ? [] : (parse(this.line(fd, position)) ?? []),
          ),
      ) ?? []
    );
  }
  /** The journal read again whole from its file, which holds each change that was appended since the session opened,
   * and written again with each line that is a change, in its order, then indexed anew. It says the count of changes
   * that it kept and of lines that it removed. */
  repair(): Repaired {
    this.writable();
    const path = this.path;
    if (this.fd === undefined || !path) return { kept: 0, removed: 0 };
    const lines = readFileSync(path, "utf8").split("\n");
    // What follows the last newline is empty, or a line that a crash cut.
    const cut = lines.pop() ?? "";
    const kept = lines.filter((line) => parse(line));
    this.reopen(path, () => {
      writeFileSync(`${path}.repair`, kept.map((line) => `${line}\n`).join(""), { mode: 0o600 });
      renameSync(`${path}.repair`, path);
    });
    return { kept: kept.length, removed: lines.length - kept.length + (cut ? 1 : 0) };
  }
  /** A new journal that holds no change, and the path that the damaged journal is kept at beside it. */
  renew(): string {
    this.writable();
    const path = this.path;
    if (this.fd === undefined || !path) return "";
    const aside = `${path}.damaged-${Date.now()}`;
    this.reopen(path, () => renameSync(path, aside));
    return aside;
  }
  /** The journal closed, changed on disk, and opened again on its lines. */
  private reopen(path: string, change: () => void): void {
    this.close();
    try {
      change();
    } finally {
      this.index(path);
      this.fd = openSync(path, "a+", 0o600);
    }
  }
  private close(): void {
    const fd = this.fd;
    this.fd = undefined;
    if (fd !== undefined) closeSync(fd);
  }
  dispose(): void {
    this.close();
    if (this.temporary) rmSync(this.temporary, { recursive: true });
  }
}

import {
  closeSync,
  existsSync,
  fsyncSync,
  mkdtempSync,
  openSync,
  readFileSync,
  readSync,
  rmSync,
  truncateSync,
  writeSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import type { Ear } from "./ears.js";
import { type Fact, isQuestion } from "./types.js";

export interface FileChange {
  path: string;
  before: string;
  after: string;
}

/** Append file snapshots once. Keep only their positions in memory and read a page on demand. As an ear that comes
 * before the files, it reads the text a write replaces before the files take the write, and keeps the change once the
 * write is done. */
export class FileChanges {
  private readonly positions: { start: number; size: number }[] = [];
  private readonly temporary?: string;
  private readonly fd?: number;
  private end = 0;
  constructor(record?: string, readOnly = false) {
    if (readOnly) return;
    if (!record) this.temporary = mkdtempSync(join(tmpdir(), "furb-changes-"));
    const path = record ? `${record}.changes.jsonl` : join(this.temporary ?? "", "changes.jsonl");
    if (existsSync(path)) {
      const data = readFileSync(path);
      while (this.end < data.length) {
        const newline = data.indexOf(10, this.end);
        if (newline < 0) {
          truncateSync(path, this.end);
          break;
        }
        // A complete line that is no change fails here, before the session opens on it.
        JSON.parse(data.subarray(this.end, newline).toString("utf8"));
        this.positions.push({ start: this.end, size: newline - this.end });
        this.end = newline + 1;
      }
    }
    this.fd = openSync(path, "a+", 0o600);
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
        const here = String(yield { verb: "cwd", kwargs: { on: words[0] } });
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
        this.append({ ...held, after: after.content });
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
  read(start = 0, count = 20): FileChange[] {
    const fd = this.fd;
    if (fd === undefined) return [];
    return this.positions.slice(start, start + count).map(({ start, size }) => {
      const data = Buffer.alloc(size);
      readSync(fd, data, 0, size, start);
      return JSON.parse(data.toString("utf8")) as FileChange;
    });
  }
  dispose(): void {
    if (this.fd !== undefined) closeSync(this.fd);
    if (this.temporary) rmSync(this.temporary, { recursive: true });
  }
}

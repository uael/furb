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
import { join } from "node:path";

export interface FileChange {
  path: string;
  before: string;
  after: string;
}

/** Append file snapshots once. Keep only their positions in memory and read a page on demand. */
export class FileChanges {
  readonly paths: string[] = [];
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
        const value = JSON.parse(data.subarray(this.end, newline).toString("utf8")) as FileChange;
        this.paths.push(value.path);
        this.positions.push({ start: this.end, size: newline - this.end });
        this.end = newline + 1;
      }
    }
    this.fd = openSync(path, "a+", 0o600);
  }
  get length(): number {
    return this.paths.length;
  }
  append(change: FileChange): void {
    if (this.fd === undefined) return;
    const data = Buffer.from(`${JSON.stringify(change)}\n`);
    writeSync(this.fd, data);
    fsyncSync(this.fd);
    this.paths.push(change.path);
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

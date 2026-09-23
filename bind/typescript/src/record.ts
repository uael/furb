import {
  closeSync,
  existsSync,
  fsyncSync,
  mkdirSync,
  openSync,
  readFileSync,
  truncateSync,
  unlinkSync,
  writeSync,
} from "node:fs";
import { dirname } from "node:path";
import { decodeRecord } from "../index.cjs";
import type { Entry } from "./types.js";

export function readRecord(path: string, repair = false): Entry[] {
  const entries: Entry[] = [];
  if (existsSync(path)) {
    const data = readFileSync(path);
    let start = 0;
    while (start < data.length) {
      const end = data.indexOf(10, start);
      const line = data.subarray(start, end < 0 ? data.length : end).toString("utf8");
      if (!line.trim()) {
        start = end < 0 ? data.length : end + 1;
        continue;
      }
      let value: unknown;
      try {
        value = decodeRecord(line);
      } catch (error) {
        if (end < 0) {
          try {
            JSON.parse(line);
          } catch {
            if (repair) truncateSync(path, start);
            break;
          }
        }
        throw new Error(`Invalid record entry at byte ${start} in ${path}.`, { cause: error });
      }
      if (
        !Array.isArray(value) ||
        ![2, 3].includes(value.length) ||
        typeof value[0] !== "string" ||
        !Array.isArray(value[1]) ||
        value[1].length < 3
      ) {
        throw new Error(`Invalid record entry at byte ${start} in ${path}.`);
      }
      entries.push(value as Entry);
      if (end < 0) {
        if (repair) {
          const append = openSync(path, "a");
          writeSync(append, "\n");
          closeSync(append);
        }
        break;
      }
      start = end + 1;
    }
  }
  return entries;
}

/** An exclusive record lease shared by a running World and session file operations. */
export class RecordLock {
  private fd?: number;
  constructor(readonly path: string) {
    mkdirSync(dirname(path), { recursive: true });
    try {
      this.fd = openSync(`${path}.lock`, "wx", 0o600);
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "EEXIST") throw error;
      const pid = Number(readFileSync(`${path}.lock`, "utf8"));
      if (!Number.isInteger(pid) || pid <= 0) throw new Error(`Invalid record lock: ${path}.lock`);
      try {
        process.kill(pid, 0);
      } catch (dead) {
        if ((dead as NodeJS.ErrnoException).code !== "ESRCH") throw error;
        unlinkSync(`${path}.lock`);
        this.fd = openSync(`${path}.lock`, "wx", 0o600);
      }
      if (this.fd === undefined) throw new Error(`Another process owns ${path}.`);
    }
    writeSync(this.fd, String(process.pid));
  }
  dispose(): void {
    if (this.fd === undefined) return;
    closeSync(this.fd);
    this.fd = undefined;
    unlinkSync(`${this.path}.lock`);
  }
}

/** One owner and one complete JSON value per line. A torn final line is removed before appending. */
export class RecordFile {
  readonly entries: Entry[] = [];
  private fd?: number;
  private lock?: RecordLock;
  constructor(
    readonly path?: string,
    readOnly = false,
  ) {
    if (!path) return;
    if (readOnly) {
      this.entries.push(...readRecord(path));
      return;
    }
    this.lock = new RecordLock(path);
    try {
      this.entries.push(...readRecord(path, true));
      this.fd = openSync(path, "a", 0o600);
    } catch (error) {
      this.dispose();
      throw error;
    }
  }
  keep(entry: Entry): void {
    if (this.fd !== undefined) {
      writeSync(this.fd, `${JSON.stringify(entry)}\n`);
      fsyncSync(this.fd);
    }
    this.entries.push(entry);
  }
  dispose(): void {
    if (this.fd !== undefined) {
      closeSync(this.fd);
      this.fd = undefined;
    }
    this.lock?.dispose();
    this.lock = undefined;
  }
}

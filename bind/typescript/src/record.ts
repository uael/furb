import { closeSync, existsSync, fsyncSync, openSync, readFileSync, truncateSync, writeSync } from "node:fs";
import { decodeRecord, RecordLock } from "../index.cjs";
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
        value.length !== 1 ||
        !Array.isArray(value[0]) ||
        value[0].length < 3 ||
        typeof value[0][0] !== "string"
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

/** One owner and one complete JSON value per line. A torn final line is removed before appending. */
export class RecordFile {
  readonly entries: Entry[];
  private fd?: number;
  private lock?: RecordLock;
  constructor(
    readonly path?: string,
    readOnly = false,
  ) {
    if (!path || readOnly) {
      this.entries = path ? readRecord(path) : [];
      return;
    }
    this.lock = new RecordLock(path);
    try {
      this.entries = readRecord(path, true);
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

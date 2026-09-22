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

/** One owner and one complete JSON value per line. A torn final line is removed before appending. */
export class RecordFile {
  readonly entries: Entry[] = [];
  private fd?: number;
  private lock?: number;
  constructor(readonly path?: string) {
    if (!path) return;
    mkdirSync(dirname(path), { recursive: true });
    try {
      this.lock = openSync(`${path}.lock`, "wx", 0o600);
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "EEXIST") throw error;
      const pid = Number(readFileSync(`${path}.lock`, "utf8"));
      if (!Number.isInteger(pid) || pid <= 0) throw new Error(`Invalid record lock: ${path}.lock`);
      try {
        process.kill(pid, 0);
      } catch (dead) {
        if ((dead as NodeJS.ErrnoException).code !== "ESRCH") throw error;
        unlinkSync(`${path}.lock`);
        this.lock = openSync(`${path}.lock`, "wx", 0o600);
      }
      if (this.lock === undefined) throw new Error(`Another process owns ${path}.`);
    }
    writeSync(this.lock, String(process.pid));
    try {
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
                truncateSync(path, start);
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
          this.entries.push(value as Entry);
          if (end < 0) {
            const append = openSync(path, "a");
            writeSync(append, "\n");
            closeSync(append);
            break;
          }
          start = end + 1;
        }
      }
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
    if (this.lock !== undefined) {
      closeSync(this.lock);
      this.lock = undefined;
      if (this.path) unlinkSync(`${this.path}.lock`);
    }
  }
}

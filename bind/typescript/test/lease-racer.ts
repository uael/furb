#!/usr/bin/env bun
// One contender for the lease of a record, which takes it again and again until a time. While it holds the lease, it
// makes a file that only one process at a time can make, so a second holder fails with EEXIST. A contender that
// moves also moves the lock file away while it holds the lease, as the delete of a session does.
import { renameSync, unlinkSync, writeFileSync } from "node:fs";
import { RecordLock } from "../src/index.ts";

const [path = "", until = "0", role = ""] = process.argv.slice(2);
let taken = 0;
while (Date.now() < Number(until)) {
  let lease: RecordLock;
  try {
    lease = new RecordLock(path);
  } catch (error) {
    if (String(error).includes("Another process owns")) continue;
    throw error;
  }
  try {
    writeFileSync(`${path}.owner`, String(process.pid), { flag: "wx" });
    taken++;
    // The record is no longer this holder's once its lock file moves, since the record moves with it.
    unlinkSync(`${path}.owner`);
    if (role === "move") renameSync(`${path}.lock`, `${path}.moved`);
  } finally {
    lease.dispose();
  }
}
console.log(taken);

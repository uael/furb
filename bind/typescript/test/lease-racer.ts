#!/usr/bin/env bun
// One contender for the lease of a record, which takes it a number of times. While it holds the lease, it makes a
// file that only one process at a time can make, so a second holder fails with EEXIST. A contender that moves also
// moves the lock file away while it holds the lease, as the delete of a session does, each time to a name of its
// own, so that no move replaces a file that another contender holds open, which Windows can refuse.
// The contenders start together: each says it is ready with a file, and waits for the file that says go.
import { existsSync, renameSync, unlinkSync, writeFileSync } from "node:fs";
import { type NativeEar, store } from "../src/index.ts";

const [path = "", count = "0", role = ""] = process.argv.slice(2);
writeFileSync(`${path}.ready.${process.pid}`, "");
while (!existsSync(`${path}.go`)) await Bun.sleep(5);
// A contender that never gets the lease fails by this deadline and says how far it came.
const deadline = Date.now() + 20000;
let taken = 0;
while (taken < Number(count)) {
  if (Date.now() > deadline) throw new Error(`Took the lease ${taken} of ${count} times.`);
  let lease: NativeEar;
  try {
    lease = store(path).ear;
  } catch (error) {
    if (String(error).includes("Another process owns")) continue;
    throw error;
  }
  try {
    writeFileSync(`${path}.owner`, String(process.pid), { flag: "wx" });
    taken++;
    // The record is no longer this holder's once its lock file moves, since the record moves with it.
    unlinkSync(`${path}.owner`);
    if (role === "move") renameSync(`${path}.lock`, `${path}.moved.${process.pid}.${taken}`);
  } finally {
    lease.dispose();
  }
}
console.log(taken);

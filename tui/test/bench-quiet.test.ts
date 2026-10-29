import { expect, test } from "bun:test";
import { existsSync, mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { refusal, take } from "../script/quiet.ts";

const lock = () => join(mkdtempSync(join(tmpdir(), "furb-quiet-")), "bench.lock");

test("a bench takes the lock with its pid and frees it", async () => {
  const path = lock();
  const free = await take(path, 1000, 10);
  expect(readFileSync(join(path, "pid"), "utf8")).toBe(String(process.pid));
  free();
  expect(existsSync(path)).toBe(false);
});

test("a second bench waits until the first frees the lock", async () => {
  const path = lock();
  const first = await take(path, 1000, 10);
  const told: string[] = [];
  let took = false;
  const second = take(path, 5000, 10, (line) => told.push(line)).then((free) => {
    took = true;
    return free;
  });
  await Bun.sleep(100);
  expect(took).toBe(false);
  expect(told[0]).toContain(`process ${process.pid}`);
  first();
  const free = await second;
  expect(took).toBe(true);
  free();
});

test("a bench takes over the lock of a bench that died", async () => {
  const path = lock();
  const dead = Bun.spawn(["true"]);
  await dead.exited;
  await Bun.write(join(path, "pid"), String(dead.pid));
  const free = await take(path, 1000, 10);
  expect(readFileSync(join(path, "pid"), "utf8")).toBe(String(process.pid));
  free();
});

test("a bench that waits too long fails and names the holder", async () => {
  const path = lock();
  const free = await take(path, 1000, 10);
  await expect(take(path, 100, 10)).rejects.toThrow(`held by the process ${process.pid}`);
  free();
});

test("the bench refuses under a load over the cores, and names CI", () => {
  const said = refusal(30, 8, false) ?? "";
  expect(said).toContain("30.0");
  expect(said).toContain("8 cores");
  expect(said).toContain("CI");
});

test("the bench runs under a quiet machine, on CI, and when forced", () => {
  expect(refusal(8, 8, false)).toBeUndefined();
  expect(refusal(230, 8, true)).toBeUndefined();
  expect(refusal(230, 8, false, true)).toBeUndefined();
});

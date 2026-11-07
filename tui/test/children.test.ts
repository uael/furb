// A child of the test script that the watcher of memory ends with SIGKILL is reported as ended by memory, and a child
// that fails or passes is not.
import { expect, test } from "bun:test";
import { endedByMemory, endOf } from "../script/children.ts";

const child = (script: string) =>
  Bun.spawn([process.execPath, "-e", script], { stdout: "ignore", stderr: "ignore" });

test("a child that the watcher kills is ended by memory, and a child that fails or passes is not", async () => {
  const big = child("setInterval(() => {}, 1000)");
  const small = child("setInterval(() => {}, 1000)");
  const failing = child("process.exit(3)");
  const passing = child("process.exit(0)");
  // The watcher ends the larger of the two heavy children while the memory is tight.
  big.kill("SIGKILL");
  small.kill("SIGTERM");
  expect(await endOf(big)).toEqual({ code: null, memory: true });
  expect((await endOf(small)).memory).toBe(false);
  expect(await endOf(failing)).toEqual({ code: 3, memory: false });
  expect(await endOf(passing)).toEqual({ code: 0, memory: false });
  expect(endedByMemory("a.test.ts")).toContain("ended by the watcher of memory");
});

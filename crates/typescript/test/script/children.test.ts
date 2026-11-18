// A child of the test script that SIGKILL ends is reported as ended by memory, and a child that fails or passes is not;
// and a bun test child that the runner of the tests runs, which SIGKILL ends as the watcher of memory ends it, is named
// so by the runner. Which child the watcher ends, the tests of `cargo xtask memwatch` say, beside the watcher.
import { expect, test } from "bun:test";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { endedByMemory, endOf } from "../../script/children.ts";
import { runTest } from "../../script/run.ts";

const child = (script: string) =>
  Bun.spawn([process.execPath, "-e", script], { stdout: "ignore", stderr: "ignore" });

test("a child that SIGKILL ends is ended by memory, and a child that fails or passes is not", async () => {
  const big = child("setInterval(() => {}, 1000)");
  const small = child("setInterval(() => {}, 1000)");
  const failing = child("process.exit(3)");
  const passing = child("process.exit(0)");
  big.kill("SIGKILL");
  small.kill("SIGTERM");
  expect(await endOf(big)).toEqual({ code: null, memory: true });
  expect((await endOf(small)).memory).toBe(false);
  expect(await endOf(failing)).toEqual({ code: 3, memory: false });
  expect(await endOf(passing)).toEqual({ code: 0, memory: false });
  expect(endedByMemory("a.test.ts")).toContain("was ended by SIGKILL");
});

test.skipIf(process.platform === "win32")(
  "a bun test child that SIGKILL ends as the watcher of memory does is named by the runner as ended by memory",
  async () => {
    const at = mkdtempSync(join(tmpdir(), "furb-watch-"));
    const pid = (name: string) => join(at, `${name}.pid`);
    // Each child writes its pid and waits: the larger long, the smaller only a moment.
    for (const [name, ms] of [
      ["big", 60000],
      ["small", 1000],
    ] as const)
      writeFileSync(
        join(at, `${name}.test.ts`),
        `import { test } from "bun:test";\ntest("holds", async () => {\n  await Bun.write(${JSON.stringify(pid(name))}, String(process.pid));\n  await Bun.sleep(${ms});\n});\n`,
      );
    const runs = ["big", "small"].map((name) =>
      runTest(process.env, ["--timeout", "120000", join(at, `${name}.test.ts`)]),
    );
    try {
      const deadline = Date.now() + 20000;
      while (!existsSync(pid("big")) || readFileSync(pid("big"), "utf8") === "") {
        if (Date.now() > deadline) throw new Error("the bun test child big never started");
        await Bun.sleep(50);
      }
      process.kill(Number(readFileSync(pid("big"), "utf8")), "SIGKILL");
      const [big, small] = await Promise.all(runs);
      expect([big?.passed, big?.memory]).toEqual([false, true]);
      expect([small?.passed, small?.memory]).toEqual([true, false]);
    } finally {
      // A child that wrote its pid after the deadline is ended too.
      for (const name of ["big", "small"]) {
        try {
          process.kill(Number(readFileSync(pid(name), "utf8")), "SIGKILL");
        } catch {}
      }
      await Promise.allSettled(runs);
      rmSync(at, { recursive: true, force: true });
    }
  },
  60000,
);

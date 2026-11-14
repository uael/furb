// A child of the test script that SIGKILL ends is reported as ended by memory, and a child that fails or passes is not;
// and the watcher of memory, on a tight reading, ends the larger of two bun test children that the runner of the tests
// runs, which names it so.
import { expect, test } from "bun:test";
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
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

const watcher = join(import.meta.dir, "../../../../script/memwatch.sh");
const MB = 1024;

test.skipIf(process.platform === "win32")(
  "on a tight memory the watcher ends the larger bun test child, and the runner names it as ended by memory",
  async () => {
    const at = mkdtempSync(join(tmpdir(), "furb-watch-"));
    const pid = (name: string) => join(at, `${name}.pid`);
    // Each child writes its pid and waits: the larger long, the smaller only past the look of the watcher.
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
    const pids: number[] = [];
    try {
      for (const name of ["big", "small"]) {
        const deadline = Date.now() + 20000;
        while (!existsSync(pid(name)) || readFileSync(pid(name), "utf8") === "") {
          if (Date.now() > deadline) throw new Error(`the bun test child ${name} never started`);
          await Bun.sleep(50);
        }
        pids.push(Number(readFileSync(pid(name), "utf8")));
      }
      // The real ps lists each test process of the machine, so a fake one lists these two children alone, and sysctl
      // gives a tight level of the kernel.
      const shim = join(at, "shim");
      mkdirSync(shim);
      const uid = process.getuid?.() ?? 0;
      writeFileSync(
        join(at, "table"),
        `${uid} ${pids[0]} ${3000 * MB} bun test ./big.test.ts\n${uid} ${pids[1]} ${1000 * MB} bun test ./small.test.ts\n`,
      );
      writeFileSync(
        join(shim, "ps"),
        `#!/bin/sh\nwhile read -r uid pid rest; do kill -0 "$pid" 2>/dev/null && echo "$uid $pid $rest"; done < ${join(at, "table")}\n`,
      );
      writeFileSync(join(shim, "sysctl"), "#!/bin/sh\necho 4\n");
      chmodSync(join(shim, "ps"), 0o755);
      chmodSync(join(shim, "sysctl"), 0o755);
      const look = Bun.spawnSync(["sh", watcher, "--once"], {
        // The fake children hold 4000 MB in all, under the sizes that make the work the cause by default.
        env: { ...process.env, PATH: `${shim}:${process.env.PATH}`, FURB_WATCH_MIN: "1" },
      });
      expect(look.stdout.toString()).toBe(
        `memwatch: pressure level 4, ended pid ${pids[0]} (3000 MB): bun test ./big.test.ts\n`,
      );
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

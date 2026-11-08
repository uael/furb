// The loop that runs one test again and again stops at the run that fails and keeps its output and its yard, and a
// test that never fails passes every run.
import { expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

const root = join(import.meta.dir, "../..");

async function repeat(runs: number, fail: number) {
  const place = await mkdtemp(join(tmpdir(), "furb-repeat-"));
  try {
    const child = Bun.spawn(
      [
        process.execPath,
        "tui/script/repeat.ts",
        "tui/test/repeat-fixture.ts",
        "fails on a chosen run",
        "--runs",
        String(runs),
        "--keep",
        join(place, "kept"),
        "--no-daemon",
      ],
      {
        cwd: root,
        env: { ...process.env, FURB_REPEAT_COUNTER: join(place, "count"), FURB_REPEAT_FAIL: String(fail) },
        stdout: "pipe",
        stderr: "pipe",
      },
    );
    const [out, code] = await Promise.all([new Response(child.stdout).text(), child.exited]);
    return {
      out,
      code,
      kept: join(place, "kept"),
      cleanup: () => rm(place, { recursive: true, force: true }),
    };
  } catch (error) {
    await rm(place, { recursive: true, force: true });
    throw error;
  }
}

test("the loop stops at the run that fails and keeps its output and its yard", async () => {
  const { out, code, kept, cleanup } = await repeat(6, 3);
  try {
    expect(code).toBe(1);
    expect(out).toContain("run 2 of 6: pass");
    expect(out).toContain("run 3 of 6: fail");
    expect(out).not.toContain("run 4 of 6");
    expect(readFileSync(join(kept, "run-3/output.txt"), "utf8")).toContain("(fail) fails on a chosen run");
    expect(readFileSync(join(kept, "run-3/yard/trace.txt"), "utf8")).toBe("run 3");
    expect(existsSync(join(kept, "run-2"))).toBe(false);
  } finally {
    await cleanup();
  }
}, 60000);

test("a test that never fails passes each run of the loop and keeps nothing", async () => {
  const { out, code, kept, cleanup } = await repeat(3, 99);
  try {
    expect(code).toBe(0);
    expect(out).toContain("3 runs of fails on a chosen run passed");
    expect(existsSync(kept)).toBe(false);
  } finally {
    await cleanup();
  }
}, 60000);

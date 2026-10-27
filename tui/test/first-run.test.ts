import { expect, test } from "bun:test";
import { chmod, mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { delimiter, join } from "node:path";
import { remove } from "../../bind/typescript/test/processes.ts";

/** The frame of the first run of a real life, not a demo, in a new home whose environment holds no key, and on a
 * PATH that holds only the programs of the system and the claude command line that a test gives, when it gives one;
 * then the acts of its chain, one a line. The folder of bun stays off the PATH, since it can hold a claude. */
async function firstRun(claude?: string): Promise<{ frame: string; acts: string[] }> {
  const home = await mkdtemp(join(tmpdir(), "furb-first-run-"));
  try {
    const project = join(home, "project");
    const programs = join(home, "programs");
    await mkdir(project, { recursive: true });
    await mkdir(programs, { recursive: true });
    if (claude) {
      await writeFile(join(programs, "claude"), `#!/bin/sh\n${claude}\n`);
      await chmod(join(programs, "claude"), 0o755);
    }
    const run = Bun.spawn([process.execPath, join(import.meta.dir, "first-run.ts"), project], {
      env: {
        HOME: home,
        TMPDIR: join(home, "tmp"),
        PATH: [programs, "/usr/bin", "/bin"].join(delimiter),
      },
      stdout: "pipe",
      stderr: "pipe",
    });
    await mkdir(join(home, "tmp"), { recursive: true });
    const [out, err, code] = await Promise.all([
      new Response(run.stdout).text(),
      new Response(run.stderr).text(),
      run.exited,
    ]);
    if (code !== 0) throw new Error(`The first run ended with ${code}: ${err}`);
    const [frame = "", acts = ""] = out.split("\n---\n");
    return { frame, acts: acts.trim().split("\n") };
  } finally {
    await remove(home);
  }
}

test.skipIf(process.platform === "win32")(
  "a real life on a home with no claude and no key shows the check of the setup over the steps of its extensions",
  async () => {
    const { frame, acts } = await firstRun();
    // The extensions played their words on the chain, and the welcome shows all the same.
    expect(acts.some((line) => line.endsWith(" extensions"))).toBe(true);
    expect(frame).toContain("No model answers yet");
    expect(frame).toContain("Claude command line  Not found.");
    expect(frame).toContain("Other providers  No model available.");
    expect(frame).toContain("Watch a life");
    // The check of the setup comes first, then the life to watch, and no starter offers a prompt that nothing answers.
    expect(frame.indexOf("No model answers yet")).toBeLessThan(frame.indexOf("Watch a life"));
    expect(frame).not.toContain("Explore a codebase");
    expect(frame).toContain("No model answers yet. Fix the");
    expect(frame).toContain("setup above, or watch a life, which needs none.");
    // The line under the input names no model, and not the operator that the actor falls to.
    expect(frame).toContain("on Main   no model   returns str");
    expect(frame).not.toContain("operator");
  },
);

test.skipIf(process.platform === "win32")(
  "a claude command line that says that it is not logged in shows how to sign in, and that no model answers",
  async () => {
    const { frame } = await firstRun("echo 'Not logged in'; exit 1");
    expect(frame).toContain("Claude command line  Found, but not signed in.");
    expect(frame).toContain("Run claude in a terminal, sign in, then press F5 here to check");
    expect(frame).toContain("No model answers yet");
  },
);

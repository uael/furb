import { expect, test } from "bun:test";
import { chmod, mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { delimiter, join } from "node:path";
import { remove } from "../../bind/typescript/test/processes.ts";
import { ownDaemon } from "../script/daemon.ts";

/** The frame of the first run of a real life, not a demo, in a new home whose environment holds no key, and on a
 * PATH that holds only the programs of the system and the claude command line that a test gives, when it gives one;
 * then the acts of its chain, one a line. The folder of bun stays off the PATH, since it can hold a claude. */
async function firstRun(claude?: string): Promise<{ frame: string; acts: string[] }> {
  const home = await mkdtemp(join(tmpdir(), "furb-first-run-"));
  // The cache of the home holds the socket of its daemon, which needs a short path, since a socket has a bound on it.
  const cache = await mkdtemp(join("/tmp", "furb-cache-"));
  let ended = async () => {};
  try {
    const project = join(home, "project");
    const programs = join(home, "programs");
    await mkdir(project, { recursive: true });
    await mkdir(programs, { recursive: true });
    if (claude) {
      await writeFile(join(programs, "claude"), `#!/bin/sh\n${claude}\n`);
      await chmod(join(programs, "claude"), 0o755);
    }
    // The program furb is the one that the build left in the tree, as a PATH of the system alone holds none.
    const env = {
      ...(process.env.FURB_BIN === undefined ? {} : { FURB_BIN: process.env.FURB_BIN }),
      HOME: home,
      XDG_CACHE_HOME: cache,
      TMPDIR: join(home, "tmp"),
      PATH: [programs, "/usr/bin", "/bin"].join(delimiter),
    };
    await mkdir(env.TMPDIR, { recursive: true });
    // The daemon of the home answers the window, as it would answer a first run of the user, and ends with the test.
    ended = await ownDaemon(env);
    const run = Bun.spawn([process.execPath, join(import.meta.dir, "first-run.ts"), project], {
      env,
      stdout: "pipe",
      stderr: "pipe",
    });
    const [out, err, code] = await Promise.all([
      new Response(run.stdout).text(),
      new Response(run.stderr).text(),
      run.exited,
    ]);
    if (code !== 0) throw new Error(`The first run ended with ${code}: ${err}`);
    const [frame = "", acts = ""] = out.split("\n---\n");
    return { frame, acts: acts.trim().split("\n") };
  } finally {
    await ended();
    await remove(home);
    await remove(cache);
  }
}

test.skipIf(process.platform === "win32")(
  "a real life on a home with no claude and no key shows the check of the setup over the steps of its extensions",
  async () => {
    const { frame, acts } = await firstRun();
    // The root wrote its setup on itself, the rungs of the skills with code wrote the code of each, and the life
    // words of the default skills started the watchers, and the welcome shows all the same. The extension naming plays
    // on the root alone, so its life word comes after the others.
    const own = acts.filter((line) => line.endsWith(" chain1")).map((line) => line.split(" ")[0]);
    // The debug, the read of the doctrine and the read of the word of each default skill come first, each on the root.
    expect(own.slice(0, 6)).toEqual([1, 2, 3, 4, 5, 6].map((n) => `rung${n}`));
    // Each read of a word of a skill made the rung that plays the word, the four of the default skills first and the
    // extension naming after them, and each life word is a rung of the root that the watchers it started name.
    const nested = acts
      .filter((line) => /^rung\d+ rung\d+$/.test(line))
      .map((line) => line.split(" ")[1] ?? "");
    expect(nested.slice(0, 4)).toEqual(["rung3", "rung4", "rung5", "rung6"]);
    expect(nested.every((maker) => own.includes(maker))).toBe(true);
    const maker = (name: string) => acts.find((line) => line.startsWith(`${name} `))?.split(" ")[1] ?? "";
    for (const name of ["catchup1", "remember1", "compacted1", "fitted1", "name_session1"])
      expect(own).toContain(maker(name));
    expect(frame).toContain("No model answers yet");
    expect(frame).toContain("Claude command line  Not found.");
    expect(frame).toContain("Other providers  No model available.");
    expect(frame).toContain("Watch a life");
    // The check of the setup comes first, then the life to watch, and no starter offers a prompt that nothing answers.
    expect(frame.indexOf("No model answers yet")).toBeLessThan(frame.indexOf("Watch a life"));
    expect(frame).not.toContain("Explore a codebase");
    // The input wraps the hint at its own width, at a word.
    expect(frame).toContain("No model answers yet. Fix the setup above, or watch a life, which");
    expect(frame).toContain("needs none.");
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

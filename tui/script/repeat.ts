// Runs one test of one file again and again, each run in a yard of its own, until a run fails or the count is reached.
// A run that fails keeps its output, and its yard (the captures and the record that it wrote), in a folder that the
// tool names, so that a flake leaves its evidence and is fixed at its cause. The tool diagnoses: it never makes a test
// pass by running it again, and a pass of every run says only that this load did not fail it.
//   bun tui/script/repeat.ts <file> <name of the test> [--runs 100] [--keep folder] [--load 0] [--no-daemon]
// The file is a path, or a part of the path of a test file. The name is the whole name of one test. --load starts that
// many busy processes (yes), to run the test on a loaded machine. --no-daemon leaves the daemon of the tests alone, for
// a test that needs none.
import { existsSync, rmSync } from "node:fs";
import { cp, mkdir, rm, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { ownDaemon } from "./daemon.ts";
import { environment } from "./environment.ts";
import { LIMIT } from "./order.ts";
import { exactly, runTest, testFiles } from "./run.ts";

const root = join(import.meta.dir, "../..");
process.chdir(root);
const args = process.argv.slice(2);
const usage = () => {
  console.log(
    "usage: bun tui/script/repeat.ts <file> <name of the test> [--runs 100] [--keep folder] [--load 0] [--no-daemon]",
  );
  process.exit(2);
};
const flag = (name: string, fallback: string): string => {
  const at = args.indexOf(`--${name}`);
  if (at < 0) return fallback;
  const value = args[at + 1];
  if (value === undefined || value.startsWith("--")) return usage();
  return args.splice(at, 2)[1] as string;
};
const runs = Number(flag("runs", "100"));
const keep = resolve(flag("keep", join(root, ".furb/repeat")));
const load = Number(flag("load", "0"));
const daemon = !args.includes("--no-daemon");
const [wanted, name] = args.filter((one) => !one.startsWith("--"));
if (!wanted || !name || !(runs > 0) || !(load >= 0)) usage();
const found = existsSync(resolve(wanted as string))
  ? [resolve(wanted as string)]
  : testFiles([wanted as string]);
if (found.length !== 1) {
  console.log(`${found.length} files match ${wanted}${found.length ? `: ${found.join(", ")}` : ""}`);
  process.exit(2);
}
const file = found[0] as string;
const env = environment();
// A signal ends the processes of the load first, as the daemon of the tests ends with it: the handler of the daemon
// exits at once, so this one stands before it and runs first.
const spinning = Array.from({ length: load }, () => Bun.spawn(["yes"], { stdout: "ignore" }));
for (const signal of ["SIGINT", "SIGTERM", "SIGHUP"] as const)
  process.once(signal, () => {
    for (const one of spinning) one.kill();
    if (!daemon) process.exit(128);
  });
const ended = daemon ? await ownDaemon(env) : async () => {};
let failed = 0;
try {
  for (let run = 1; run <= runs; run++) {
    const began = Date.now();
    const { text, passed, yard } = await runTest(
      env,
      ["--timeout", String(LIMIT), "--test-name-pattern", exactly([name as string]), file],
      true,
    );
    const seconds = ((Date.now() - began) / 1000).toFixed(1);
    const tests = text.match(/^\((pass|fail)\)/gm)?.length ?? 0;
    // The name names one test: another count says that it matched none, or more than one.
    if (tests !== 1) {
      console.log(`run ${run} of ${runs}: ${tests} tests ran for the name ${name}, and one test should`);
      failed = 2;
    } else if (passed) {
      console.log(`run ${run} of ${runs}: pass (${seconds}s)`);
    } else {
      const folder = join(keep, `run-${run}`);
      await rm(folder, { recursive: true, force: true });
      await mkdir(folder, { recursive: true });
      await writeFile(join(folder, "output.txt"), text);
      if (yard) await cp(yard, join(folder, "yard"), { recursive: true });
      console.log(`run ${run} of ${runs}: fail (${seconds}s), its output and its yard are in ${folder}`);
      failed = 1;
    }
    if (yard) rmSync(yard, { recursive: true, force: true });
    if (failed) break;
  }
  if (!failed) console.log(`${runs} runs of ${name} passed`);
} finally {
  for (const one of spinning) one.kill();
  await ended();
}
process.exit(failed);

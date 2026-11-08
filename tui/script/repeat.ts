// Runs one test of one file again and again, each run in a yard of its own, until a run fails or the count is reached.
// A run that fails keeps its output, and its yard (the captures and the record that it wrote), in a folder that the
// tool names, so that a flake leaves its evidence and is fixed at its cause. The tool diagnoses: it never makes a test
// pass by running it again, and a pass of every run says only that this load did not fail it.
//   bun tui/script/repeat.ts <file> <name of the test> [--runs 100] [--keep folder] [--load 0] [--no-daemon]
// The file is a path, or a part of the path of a test file. --load starts that many processes that spin, to run the
// test under a loaded machine. --no-daemon leaves the daemon of the tests alone, for a test that needs none.
import { existsSync } from "node:fs";
import { cp, mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { Glob } from "bun";
import { ownDaemon } from "./daemon.ts";
import { environment } from "./environment.ts";
import { LIMIT } from "./order.ts";

const root = join(import.meta.dir, "../..");
const args = process.argv.slice(2);
const flag = (name: string, fallback: string): string => {
  const at = args.indexOf(`--${name}`);
  return at < 0 ? fallback : (args.splice(at, 2)[1] ?? fallback);
};
const runs = Number(flag("runs", "100"));
const keep = resolve(flag("keep", join(root, ".furb/repeat")));
const load = Number(flag("load", "0"));
const daemon = !args.includes("--no-daemon");
const [wanted, name] = args.filter((one) => !one.startsWith("--"));
if (!wanted || !name || !(runs > 0)) {
  console.log(
    "usage: bun tui/script/repeat.ts <file> <name of the test> [--runs 100] [--keep folder] [--load 0]",
  );
  process.exit(2);
}
const found = existsSync(resolve(wanted))
  ? [resolve(wanted)]
  : ["bind/typescript/test", "tui/test", "tui/scenarios"].flatMap((dir) =>
      [...new Glob("**/*.test.ts").scanSync(join(root, dir))]
        .map((file) => join(root, dir, file))
        .filter((path) => path.includes(wanted)),
    );
if (found.length !== 1) {
  console.log(`${found.length} files match ${wanted}${found.length ? `: ${found.join(", ")}` : ""}`);
  process.exit(2);
}
const file = found[0] as string;
const env = environment();
const ended = daemon ? await ownDaemon(env) : async () => {};
const spinning = Array.from({ length: load }, () => Bun.spawn(["yes"], { stdout: "ignore" }));
let failed = 0;
try {
  for (let run = 1; run <= runs; run++) {
    const machine = tmpdir();
    const yard = await mkdtemp(join(machine, "furb-yard-"));
    const own = {
      ...env,
      TMPDIR: yard,
      TMP: yard,
      TEMP: yard,
      FURB_MACHINE_TMPDIR: machine,
      FURB_YARD: yard,
      // The run keeps what its processes wrote in the yard, which the preload clears otherwise, and removes it itself.
      FURB_KEEP_YARDS: "1",
    };
    const began = Date.now();
    const child = Bun.spawn(
      [process.execPath, "test", "--timeout", String(LIMIT), "--test-name-pattern", name, file],
      { cwd: root, env: own, stdout: "pipe", stderr: "pipe" },
    );
    const [out, err, code] = await Promise.all([
      new Response(child.stdout).text(),
      new Response(child.stderr).text(),
      child.exited,
    ]);
    const text = out + err;
    // A run that ran no test passed nothing: the name matched none.
    const passed = code === 0 && /^\(pass\)/m.test(text);
    const seconds = ((Date.now() - began) / 1000).toFixed(1);
    if (passed) {
      console.log(`run ${run} of ${runs}: pass (${seconds}s)`);
      await rm(yard, { recursive: true, force: true });
      continue;
    }
    const folder = join(keep, `run-${run}`);
    await rm(folder, { recursive: true, force: true });
    await mkdir(folder, { recursive: true });
    await writeFile(join(folder, "output.txt"), text);
    await cp(yard, join(folder, "yard"), { recursive: true });
    await rm(yard, { recursive: true, force: true });
    console.log(`run ${run} of ${runs}: fail (${seconds}s), its output and its yard are in ${folder}`);
    failed = 1;
    break;
  }
  if (!failed) console.log(`${runs} runs of ${name} passed`);
} finally {
  for (const one of spinning) one.kill();
  await ended();
}
process.exit(failed);

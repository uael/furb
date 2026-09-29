// The tests of the workspace: each file in a process of its own, every process at once, and the tests of a file at
// once but those marked serial, so the slowest file alone sets how long they take. A word given filters the files
// by their path, and a flag goes to each run. What a run wrote shows when it ends, and one failed run fails them all.
import { Glob } from "bun";

const given = process.argv.slice(2);
const flags = given.filter((one) => one.startsWith("-"));
const filters = given.filter((one) => !one.startsWith("-"));
const files = ["bind/typescript/test", "tui/test"]
  .flatMap((root) => [...new Glob("**/*.test.ts").scanSync(root)].map((file) => `./${root}/${file}`))
  .filter((file) => filters.length === 0 || filters.some((filter) => file.includes(filter)));
const env = { ...process.env, FURB_CONFIG_DIR: ".furb/tests", XDG_CACHE_HOME: ".furb/tests/cache" };
const codes = await Promise.all(
  files.map(async (file) => {
    const run = Bun.spawn([process.execPath, "test", "--concurrent", "--timeout", "30000", ...flags, file], {
      env,
      stdout: "pipe",
      stderr: "pipe",
    });
    const [out, err, code] = await Promise.all([
      new Response(run.stdout).text(),
      new Response(run.stderr).text(),
      run.exited,
    ]);
    process.stdout.write(out + err);
    return code;
  }),
);
const failed = files.filter((_, at) => codes[at] !== 0);
if (failed.length > 0)
  console.log(`\n${failed.length} of ${files.length} files failed: ${failed.join(", ")}`);
process.exit(failed.length > 0 ? 1 : 0);

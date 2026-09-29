// The tests of the workspace: each file in a process of its own, as many at once as the machine has cores, the
// largest first, and in each file as many tests at once, but those marked serial. A machine of few cores that ran
// more at once would run each test past its time. A word given filters the files by their path, and a flag goes to
// each run. What a run wrote shows when it ends, and one failed run fails them all.
import { statSync } from "node:fs";
import { availableParallelism } from "node:os";
import { Glob } from "bun";

const given = process.argv.slice(2);
const flags = given.filter((one) => one.startsWith("-"));
const filters = given.filter((one) => !one.startsWith("-"));
const files = ["bind/typescript/test", "tui/test"]
  .flatMap((root) => [...new Glob("**/*.test.ts").scanSync(root)].map((file) => `./${root}/${file}`))
  .filter((file) => filters.length === 0 || filters.some((filter) => file.includes(filter)))
  .sort((one, other) => statSync(other).size - statSync(one).size);
const cores = availableParallelism();
const env = { ...process.env, FURB_CONFIG_DIR: ".furb/tests", XDG_CACHE_HOME: ".furb/tests/cache" };
const waiting = [...files];
const failed: string[] = [];
await Promise.all(
  Array.from({ length: cores }, async () => {
    for (let file = waiting.shift(); file; file = waiting.shift()) {
      const run = Bun.spawn(
        [
          process.execPath,
          "test",
          "--concurrent",
          `--max-concurrency=${cores}`,
          "--timeout",
          "30000",
          ...flags,
          file,
        ],
        { env, stdout: "pipe", stderr: "pipe" },
      );
      const [out, err, code] = await Promise.all([
        new Response(run.stdout).text(),
        new Response(run.stderr).text(),
        run.exited,
      ]);
      process.stdout.write(out + err);
      if (code !== 0) failed.push(file);
    }
  }),
);
if (failed.length > 0)
  console.log(`\n${failed.length} of ${files.length} files failed: ${failed.join(", ")}`);
process.exit(failed.length > 0 ? 1 : 0);

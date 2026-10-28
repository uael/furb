// The tests of the workspace. A run is a process of its own that runs some tests of one file, one test at a time.
// So a test counts its own time alone against its limit of thirty seconds, and no test runs beside a test that
// changes what they share, such as the environment or the clock. A first run of each file, which matches no name,
// lists its tests. Then the tests of all the files go into runs of at most one size, about four runs for each core,
// the runs of the largest files first. Twice as many runs go at once as the machine has cores, since a test waits for
// its session as much as it works. A word given filters the files by their path, and a flag goes to each run. A test
// that passes shows its line, a run that fails shows all that it wrote, and one failed run fails them all. The
// last line counts the tests that passed and failed, and a filter that matches no file fails. FURB_TEST_OUTPUT=1
// shows all that each run wrote, so a console.log in a test shows too. Each
// program that a test runs compiles first, all at once, so that no compile takes the machine from a test.
import { existsSync } from "node:fs";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { availableParallelism, tmpdir } from "node:os";
import { delimiter, join } from "node:path";
import { Glob } from "bun";

const given = process.argv.slice(2);
const flags = given.filter((one) => one.startsWith("-"));
const filters = given.filter((one) => !one.startsWith("-"));
const files = ["bind/typescript/test", "tui/test", "gui/test"]
  .flatMap((root) => [...new Glob("**/*.test.ts").scanSync(root)].map((file) => `./${root}/${file}`))
  .filter((file) => filters.length === 0 || filters.some((filter) => file.includes(filter)))
  .sort((one, other) => Bun.file(other).size - Bun.file(one).size);
if (files.length === 0) {
  console.log(`0 pass, 0 fail: no test file matches ${filters.join(" ")}`);
  process.exit(1);
}
const cores = availableParallelism();
// The tests find no claude command line, as on a machine that has none: a roster of the demo names a model that the
// aliases of a real program may lack. A FURB_CLAUDE_BIN of the caller goes too.
const program = process.platform === "win32" ? "claude.exe" : "claude";
const path = (process.env.PATH ?? "")
  .split(delimiter)
  .filter((folder) => folder && !existsSync(join(folder, program)))
  .join(delimiter);
const everything = process.env.FURB_TEST_OUTPUT === "1";
let passes = 0;
let fails = 0;
const env = {
  ...process.env,
  PATH: path,
  FURB_CONFIG_DIR: ".furb/tests",
  XDG_CACHE_HOME: ".furb/tests/cache",
  FURB_CLAUDE_BIN: "",
  FURB_LOCAL_URL: "",
  MAX_THINKING_TOKENS: "",
};
const entities: Record<string, string> = { quot: '"', amp: "&", lt: "<", gt: ">", apos: "'" };

/** What a run of the tests wrote, and whether it passed. */
async function test(args: string[]): Promise<{ text: string; passed: boolean }> {
  const run = Bun.spawn([process.execPath, "test", ...args], { env, stdout: "pipe", stderr: "pipe" });
  const [out, err, code] = await Promise.all([
    new Response(run.stdout).text(),
    new Response(run.stderr).text(),
    run.exited,
  ]);
  return { text: out + err, passed: code === 0 };
}

/** Each work, as many at once as `slots`, in the order given, and what each came to. */
async function deal<T, R>(works: T[], slots: number, work: (one: T) => Promise<R>): Promise<R[]> {
  const results: R[] = [];
  let next = 0;
  await Promise.all(
    Array.from({ length: slots }, async () => {
      for (let at = next++; at < works.length; at = next++) results[at] = await work(works[at] as T);
    }),
  );
  return results;
}

const programs = join(import.meta.dir, "../../bind/typescript/test/programs");
const compiling = Promise.all(
  [...new Glob("*.ts").scanSync(programs)].map(async (file) => {
    const run = Bun.spawn([process.execPath, join(programs, "../executable.ts"), join(programs, file)], {
      env,
      stdout: "pipe",
      stderr: "pipe",
    });
    const [out, err, code] = await Promise.all([
      new Response(run.stdout).text(),
      new Response(run.stderr).text(),
      run.exited,
    ]);
    return { file: join(programs, file), text: out + err, passed: code === 0 };
  }),
);
const listing = await mkdtemp(join(tmpdir(), "furb-tests-"));
const listed = await deal(files, cores, async (file) => {
  const report = join(listing, `${files.indexOf(file)}.xml`);
  const { text } = await test([
    "--test-name-pattern",
    "^$",
    "--reporter=junit",
    `--reporter-outfile=${report}`,
    file,
  ]);
  const xml = await readFile(report, "utf8").catch(() => "");
  const names = [...xml.matchAll(/<testcase name="([^"]*)"/g)].map(([, name = ""]) =>
    name.replace(/&(#x[\da-f]+|#\d+|\w+);/gi, (entity, code: string) =>
      code.startsWith("#")
        ? String.fromCodePoint(Number(code.startsWith("#x") ? `0x${code.slice(2)}` : code.slice(1)))
        : (entities[code] ?? entity),
    ),
  );
  return { file, names, text };
});
await rm(listing, { recursive: true, force: true });
const size = Math.ceil(listed.reduce((sum, { names }) => sum + names.length, 0) / (4 * cores));
const runs = listed.flatMap(({ file, names }) =>
  Array.from({ length: Math.ceil(names.length / size) }, (_, at) => ({
    file,
    names: names.slice(at * size, (at + 1) * size),
  })),
);
// A file that lists no test failed to load, and its listing says why, as a program that does not compile does.
const failed: string[] = [];
for (const { file, text, passed } of await compiling)
  if (!passed) {
    process.stdout.write(text);
    failed.push(file);
  }
for (const { file, names, text } of listed)
  if (names.length === 0) {
    process.stdout.write(text);
    failed.push(file);
  }
await deal(runs, 2 * cores, async ({ file, names }) => {
  const pattern = `^(${names.map((name) => name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")).join("|")})$`;
  const { text, passed } = await test(["--timeout", "30000", "--test-name-pattern", pattern, ...flags, file]);
  passes += text.match(/^\(pass\)/gm)?.length ?? 0;
  fails += text.match(/^\(fail\)/gm)?.length ?? 0;
  if (passed) process.stdout.write(everything ? text : text.replace(/^(?!\(pass\)).*\n/gm, ""));
  else {
    process.stdout.write(`\n${text}\n`);
    failed.push(file);
  }
});
if (failed.length > 0) console.log(`\n${failed.length} of ${runs.length} runs failed: ${failed.join(", ")}`);
console.log(`${passes} pass, ${fails} fail`);
process.exit(failed.length > 0 ? 1 : 0);

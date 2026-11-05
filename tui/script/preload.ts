// A plain `bun test` loads this file through bunfig.toml. It gives the tests the folders of the runner, but it cannot
// hide the claude command line, since the native part reads the PATH that the process started with. So it says so.
import { afterAll, afterEach, beforeEach, setDefaultTimeout } from "bun:test";
import { existsSync, mkdtempSync, readdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { delimiter, join, resolve } from "node:path";
import { configDirectory } from "furb";
import { environment } from "./environment.ts";
import { realConfigs, watch } from "./guard.ts";

const program = process.platform === "win32" ? "claude.exe" : "claude";
const found = (process.env.PATH ?? "")
  .split(delimiter)
  .some((folder) => folder && existsSync(join(folder, program)));
if (found && process.env.FURB_TESTS !== "1") {
  console.error(
    "A claude command line stands on the PATH, so tests that name a model can fail: run `bun run test <filter>`.",
  );
}
Object.assign(process.env, environment());
setDefaultTimeout(30000);

// The temporary directory of the tests of this process is a yard of its own, which each process that a test starts
// takes too: the one that the runner made for the run, or else a new one that goes after the last test of the
// process. Each folder that a test makes there goes when the test ends, on a pass and on a failure, unless
// FURB_KEEP_YARDS is set. The programs that the tests compile stay in the temporary directory of the machine, which
// every run shares, as FURB_MACHINE_TMPDIR names it.
const kept = process.env.FURB_KEEP_YARDS !== undefined;
const given = process.env.FURB_YARD;
const machine = process.env.FURB_MACHINE_TMPDIR ?? tmpdir();
const yard = given ?? mkdtempSync(join(machine, "furb-yard-"));
Object.assign(process.env, {
  FURB_MACHINE_TMPDIR: machine,
  FURB_YARD: yard,
  TMPDIR: yard,
  TMP: yard,
  TEMP: yard,
});
// No test writes the config of the machine, whatever the way that it runs. The native part reads the environment that
// the process started with, so a plain `bun test` gives it the folder of the tests nowhere, and a test that builds
// Preferences with no path then writes the file of the user. A test that changes a file there fails with its name.
const tests = resolve(environment().FURB_CONFIG_DIR ?? "");
const native = configDirectory();
if (resolve(native) !== tests)
  console.error(
    `The native part reads ${native}, not ${tests}: a plain bun test cannot give it that folder, so give each test that builds Preferences or Workspaces a config directory of its own, or run \`bun run test <filter>\`.`,
  );
const written = watch(realConfigs(tests, native));
let before = new Set<string>();
beforeEach(() => {
  before = new Set(readdirSync(yard));
  written();
});
// On Windows a folder that a demo or a child process still holds raises EBUSY. The cleanup retries, and a folder that
// stays busy is left to the end of the run, where the runner removes the yard, with one line on stderr: it never fails
// a test, while a failure of the test body still does.
function clear(path: string, what: string): void {
  try {
    rmSync(path, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 });
  } catch (error) {
    const code = (error as { code?: string }).code ?? "error";
    console.error(`FURB yard: left ${path} (${code}) for the end of the run; ${what}`);
  }
}
afterEach(() => {
  if (!kept)
    for (const name of readdirSync(yard)) {
      if (!before.has(name)) clear(join(yard, name), "a process still holds it");
    }
  const wrote = written();
  if (wrote.length > 0)
    throw new Error(
      `This test wrote the config of the machine, which no test may write: ${wrote.join(", ")}`,
    );
});
afterAll(() => {
  if (kept) console.error(`FURB_KEEP_YARDS keeps ${yard}`);
  else if (given === undefined) clear(yard, "the temporary directory of the machine keeps it");
});

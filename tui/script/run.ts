// A run of the tests, which the runner of the suite and the loop that repeats one test share: a process of `bun test`
// in a yard of its own, and the test files that a filter names.
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Glob } from "bun";
import { endOf } from "./children.ts";

/** The test files, from the root of the clone, whose path or whose package and file (as tui/host) hold one of the words
 * given, or all of them when no word is given. */
export function testFiles(filters: string[]): string[] {
  return ["bind/typescript/test", "tui/test", "tui/scenarios"]
    .flatMap((root) =>
      [...new Glob("**/*.test.ts").scanSync(root)].map((file) => ({
        path: `./${root}/${file}`,
        named: `${root.replace(/\/test$/, "")}/${file}`,
      })),
    )
    .filter(
      ({ path, named }) =>
        filters.length === 0 || filters.some((one) => path.includes(one) || named.includes(one)),
    )
    .map(({ path }) => path);
}

/** The pattern of the names of tests, each of them whole. */
export const exactly = (names: string[]): string =>
  `^(${names.map((name) => name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")).join("|")})$`;

/** What a run of the tests wrote, whether it passed, whether the watcher of memory ended it, and its yard when it is
 * kept. The run, and each process that it starts, takes a temporary directory of its own, a yard that goes when the run
 * ends, on a pass and on a failure, unless FURB_KEEP_YARDS is set or `keep` is true: a library of the run reads the
 * temporary directory that its process started with. A kept yard keeps what the processes of the run wrote in it. */
export async function runTest(
  env: Record<string, string | undefined>,
  args: string[],
  keep = false,
): Promise<{ text: string; passed: boolean; memory: boolean; yard?: string }> {
  const machine = process.env.FURB_MACHINE_TMPDIR ?? tmpdir();
  const yard = await mkdtemp(join(machine, "furb-yard-"));
  const own = {
    ...env,
    TMPDIR: yard,
    TMP: yard,
    TEMP: yard,
    FURB_MACHINE_TMPDIR: machine,
    FURB_YARD: yard,
    ...(keep ? { FURB_KEEP_YARDS: "1" } : {}),
  };
  const run = Bun.spawn([process.execPath, "test", ...args], { env: own, stdout: "pipe", stderr: "pipe" });
  const [out, err, end] = await Promise.all([
    new Response(run.stdout).text(),
    new Response(run.stderr).text(),
    endOf(run),
  ]);
  const kept = keep || process.env.FURB_KEEP_YARDS !== undefined;
  if (!kept) await rm(yard, { recursive: true, force: true });
  return { text: out + err, passed: end.code === 0, memory: end.memory, yard: kept ? yard : undefined };
}

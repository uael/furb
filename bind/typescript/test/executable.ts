import { createHash } from "node:crypto";
import {
  copyFileSync,
  existsSync,
  linkSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  renameSync,
  rmSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

/** The folder that holds each program that the tests compiled, for all the runs of the tests on this machine. */
const built = join(tmpdir(), "furb-executables");

/** The folder of the scripts that the tests run as programs, which the runner of the tests compiles before any test. */
export const programs = join(import.meta.dir, "programs");

/** A program named `name` in a directory, compiled from a script by the bun that runs the test, and its path, which
 * ends in .exe on Windows. A test runs a fake program in this way on every system, since Windows runs no script by
 * its shebang. The script imports only the modules of the runtime, and it finds its files from the path of the program.
 *
 * A compiled program holds the whole runtime, about 100 MB, and its first start is slow, as the system checks it:
 * on Windows, a compile and a first start took more than 30 s while the other runs of the tests worked. So each text of
 * a script compiles once for each bun into the folder `built`, which every run on the machine shares, and starts there
 * once. The program in the directory is a hard link to that file, which the system checked already, and a copy where
 * no link can be made. A compile that runs beside the tests takes the disk and the processors from them all: on
 * Windows every test timed out for a minute while runs of the tests compiled. So the runner of the tests runs this module first,
 * which compiles each script of `programs`, and a test compiles only when it runs with no runner. */
export function executable(script: string, directory: string, name: string): string {
  const file = process.platform === "win32" ? `${name}.exe` : name;
  const home = compiled(script);
  const path = join(directory, file);
  const program = join(home, "program", process.platform === "win32" ? "program.exe" : "program");
  try {
    linkSync(program, path);
  } catch {
    copyFileSync(program, path);
  }
  return path;
}

/** The folder of the program of a script, which compiles it when the folder holds none. */
function compiled(script: string): string {
  const key = createHash("sha256")
    .update(`${Bun.version} ${Bun.revision}\n`)
    .update(readFileSync(script))
    .digest("hex");
  const home = join(built, key.slice(0, 16));
  if (!existsSync(join(home, "program"))) compile(script, home);
  return home;
}

/** Compile the script into `home`/program, in a folder of its own that takes its name at once when it is done, so a
 * run that compiles the same script at the same time finds either nothing or the whole program. */
function compile(script: string, home: string): void {
  mkdirSync(home, { recursive: true });
  const work = mkdtempSync(join(home, "work-"));
  const path = join(work, process.platform === "win32" ? "program.exe" : "program");
  // The compiler leaves a file of its work in the directory it runs in, so it runs in the directory of the program.
  const made = Bun.spawnSync([process.execPath, "build", "--compile", script, "--outfile", path], {
    cwd: work,
    stdout: "pipe",
    stderr: "pipe",
  });
  if (made.exitCode) throw new Error(`${script} does not compile: ${made.stderr.toString()}`);
  const program = join(home, "program");
  try {
    renameSync(work, program);
  } catch (error) {
    // Another run compiled the same script first, and its program stands.
    if (!existsSync(program)) throw error;
    rmSync(work, { recursive: true, force: true });
    return;
  }
  // The program starts here once, with no input and no log, so no test bounds the time of the first start. It starts
  // after the move: on Windows the system holds a program that just started, and a folder that holds it cannot move.
  const { FURB_FAKE_LOG: _, ...env } = process.env;
  const file = process.platform === "win32" ? "program.exe" : "program";
  Bun.spawnSync([join(program, file)], {
    cwd: program,
    env,
    stdin: "ignore",
    stdout: "ignore",
    stderr: "ignore",
  });
}

if (import.meta.main) compiled(process.argv[2] ?? "");

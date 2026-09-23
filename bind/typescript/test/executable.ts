import { join } from "node:path";

/** A program named `name` in a directory, compiled from a script by the bun that runs the test, and its path, which
 * ends in .exe on Windows. A test runs a fake program in this way on every system, since Windows runs no script by
 * its shebang. */
export function executable(script: string, directory: string, name: string): string {
  const path = join(directory, process.platform === "win32" ? `${name}.exe` : name);
  // The compiler leaves a file of its work in the directory it runs in, so it runs in the directory of the program.
  const built = Bun.spawnSync([process.execPath, "build", "--compile", script, "--outfile", path], {
    cwd: directory,
    stdout: "pipe",
    stderr: "pipe",
  });
  if (built.exitCode) throw new Error(`${script} does not compile: ${built.stderr.toString()}`);
  return path;
}

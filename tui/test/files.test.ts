import { expect, test } from "bun:test";
import { chmod, mkdir, mkdtemp, readdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { projectFiles } from "../src/files.ts";

/** Take from every user the right to list a folder, or give it back: by its mode on Unix, and on Windows, where the
 * mode holds no such right, by an entry that denies it in the access list of the folder. */
async function readable(folder: string, allowed: boolean): Promise<void> {
  if (process.platform !== "win32") return chmod(folder, allowed ? 0o700 : 0o000);
  // S-1-1-0 is the group of every user, and RD the right to list a folder.
  const done = Bun.spawnSync([
    "icacls",
    folder,
    ...(allowed ? ["/remove:d", "*S-1-1-0"] : ["/deny", "*S-1-1-0:(RD)"]),
  ]);
  if (done.exitCode) throw new Error(`icacls failed: ${done.stdout.toString()}${done.stderr.toString()}`);
}

test("the project files leave out a folder that cannot be read, through rg and through the walk without rg", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-files-"));
  const project = join(directory, "project");
  try {
    await mkdir(join(project, "src"), { recursive: true });
    await mkdir(join(project, "locked"));
    await writeFile(join(project, "src/a.py"), "a = 1\n");
    await writeFile(join(project, "README.md"), "# Read me\n");
    await readable(join(project, "locked"), false);
    await expect(readdir(join(project, "locked"))).rejects.toThrow();
    expect(await projectFiles(project)).toEqual(["README.md", "src/a.py"]);
    // A process with no PATH finds no rg, so it walks the folders itself.
    const walk = Bun.spawn(
      [
        process.execPath,
        "-e",
        `import { projectFiles } from ${JSON.stringify(join(import.meta.dir, "../src/files.ts"))};
console.log(JSON.stringify(await projectFiles(${JSON.stringify(project)})));`,
      ],
      { env: { ...process.env, PATH: "" }, stdout: "pipe", stderr: "pipe" },
    );
    const [code, output, errors] = await Promise.all([
      walk.exited,
      new Response(walk.stdout).text(),
      new Response(walk.stderr).text(),
    ]);
    expect(errors).toBe("");
    expect(code).toBe(0);
    expect(JSON.parse(output)).toEqual(["README.md", "src/a.py"]);
  } finally {
    await readable(join(project, "locked"), true);
    await rm(directory, { recursive: true, force: true });
  }
}, 30000);

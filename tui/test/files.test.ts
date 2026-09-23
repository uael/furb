import { expect, test } from "bun:test";
import { chmod, mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { projectFiles } from "../src/files.ts";

test("the project files leave out a folder that cannot be read, through rg and through the walk without rg", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-files-"));
  const project = join(directory, "project");
  try {
    await mkdir(join(project, "src"), { recursive: true });
    await mkdir(join(project, "locked"));
    await writeFile(join(project, "src/a.py"), "a = 1\n");
    await writeFile(join(project, "README.md"), "# Read me\n");
    await chmod(join(project, "locked"), 0o000);
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
    await chmod(join(project, "locked"), 0o700);
    await rm(directory, { recursive: true, force: true });
  }
}, 30000);

import { expect, test } from "bun:test";
import { mkdtemp, readdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

test.skipIf(process.platform === "win32")(
  "two runs of the gallery with two roots hold their homes at once",
  async () => {
    const directory = await mkdtemp(join(tmpdir(), "furb-stage-"));
    try {
      const marks = join(directory, "marks");
      const script = join(directory, "run.ts");
      // Each run says that it holds its home, then ends only once the other run holds its own home too.
      await writeFile(
        script,
        `import { mkdirSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { home } from ${JSON.stringify(join(import.meta.dir, "../script/stage.ts"))};
const place = await home("furb-stage-test");
const marks = ${JSON.stringify(marks)};
mkdirSync(marks, { recursive: true });
writeFileSync(join(marks, String(process.pid)), place);
for (const deadline = Date.now() + 20000; readdirSync(marks).length < 2; await Bun.sleep(50))
  if (Date.now() > deadline) process.exit(3);
process.exit(0);
`,
      );
      const roots = [join(directory, "one"), join(directory, "two")];
      const env = { ...process.env };
      delete env.FURB_GALLERY_HOME;
      delete env.FURB_GALLERY_PARENT;
      const runs = roots.map((root) =>
        Bun.spawn([process.execPath, script], {
          env: { ...env, FURB_GALLERY_ROOT: root },
          stderr: "inherit",
        }),
      );
      expect(await Promise.all(runs.map((run) => run.exited))).toEqual([0, 0]);
      const homes = await Promise.all(
        (await readdir(marks)).map((one) => readFile(join(marks, one), "utf8")),
      );
      expect(homes.map((one) => one.split("/").slice(-2).join("/")).sort()).toEqual([
        "one/furb-stage-test",
        "two/furb-stage-test",
      ]);
      expect((await readdir(roots[0] ?? "")).length + (await readdir(roots[1] ?? "")).length).toBe(0);
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  },
);

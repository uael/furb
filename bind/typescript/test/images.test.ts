import { expect, test } from "bun:test";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { imageContent, Session } from "../src/index.ts";

/** A PNG of one pixel, in base64. */
const pixel = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/l9sAAAAASUVORK5CYII=";

test("a session with no record keeps its images in a .furb that keeps itself out of version control", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-images-"));
  const session = new Session({ cwd: directory });
  try {
    Bun.spawnSync(["git", "init", "-q"], { cwd: directory });
    const path = join(directory, "pixel.png");
    await writeFile(path, Buffer.from(pixel, "base64"));
    const image = session.attachImage(path);
    expect(imageContent(session.imageDirectory, image.uri).data).toBe(pixel);
    expect(session.imageDirectory).toBe(join(directory, ".furb/images"));
    expect(await readFile(join(directory, ".furb/.gitignore"), "utf8")).toBe("*\n!config.json\n");
    const status = Bun.spawnSync(["git", "status", "--porcelain", "--untracked-files=all"], {
      cwd: directory,
    });
    expect(status.stdout.toString()).toBe("?? pixel.png\n");
  } finally {
    await session.dispose();
    await rm(directory, { recursive: true, force: true });
  }
});

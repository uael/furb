import { expect, test } from "bun:test";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { builtinModels } from "@earendil-works/pi-ai/providers/all";
import { ImageCache } from "../src/images.ts";
import { imageContent, imagePath, imageReference, imageReferences, Session } from "../src/index.ts";
import { claudeProvider } from "../src/providers/claude.ts";
import { executable } from "./executable.ts";
import { remove } from "./processes.ts";

/** A PNG of one pixel, in base64. */
const pixel = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/l9sAAAAASUVORK5CYII=";

test("image attachments reach pi-ai and the Claude CLI as image blocks and remain available after replay", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-images-"));
  const record = join(directory, "session.jsonl");
  const path = join(directory, "pixel.png");
  const data = pixel;
  await writeFile(path, Buffer.from(data, "base64"));
  const bin = executable(join(import.meta.dir, "fake-claude.ts"), directory, "claude");
  const log = join(directory, "cli.jsonl");
  const previous = process.env.FURB_FAKE_LOG;
  process.env.FURB_FAKE_LOG = log;
  // The Claude CLI is a provider the host adds to the models of pi-ai, as a host of a session does.
  const cli = claudeProvider({ bin });
  const models = builtinModels();
  models.setProvider(cli.provider);
  let session = new Session({ cwd: directory, record, models, roster: ["claude-cli:sonnet"] });
  try {
    const image = session.attachImage(path);
    const engine = session.open();
    expect(
      await engine.prompt("str", { message: `Describe this pixel. ![pixel](${image.uri})`, on: engine.root }),
    ).toBe("reply 1");
    const requests = (await readFile(log, "utf8"))
      .trim()
      .split("\n")
      .map((line) => JSON.parse(line));
    const blocks = requests.find((request) => request.message)?.message.content;
    expect(blocks).toContainEqual({
      type: "image",
      source: { type: "base64", media_type: "image/png", data },
    });
    expect(blocks.find((block: { type: string }) => block.type === "text").text).toContain(image.uri);
    expect(imageContent(session.imageDirectory, image.uri).data).toBe(data);
    const cache = new ImageCache();
    const first = cache.get(session.imageDirectory, image.uri);
    first.data = "a caller cannot change the cached bytes";
    expect(cache.get(session.imageDirectory, image.uri).data).toBe(data);
    const asset = join(session.imageDirectory, image.uri.slice("furb-image://".length));
    const original = await readFile(asset);
    await writeFile(asset, Buffer.concat([original, Buffer.from("changed")]));
    expect(() => cache.get(session.imageDirectory, image.uri)).toThrow("changed");
    await writeFile(asset, original);
    await session.dispose();
    const before = await readFile(log, "utf8");
    session = new Session({ record, models });
    session.open();
    expect(imageContent(session.imageDirectory, image.uri).data).toBe(data);
    expect(await readFile(log, "utf8")).toBe(before);
  } finally {
    await session.dispose();
    cli.dispose();
    if (previous === undefined) delete process.env.FURB_FAKE_LOG;
    else process.env.FURB_FAKE_LOG = previous;
    await remove(directory);
  }
});

test("an image attachment is written and read back by one grammar, and a bare uri is no attachment", () => {
  const digest = "a".repeat(64);
  const image = { name: "plan [v2]\nfinal", uri: `furb-image://${digest}.png` };
  const reference = imageReference(image);
  expect(reference).toBe(`![plan _v2__final](furb-image://${digest}.png)`);
  expect(imageReferences(`See ${reference} beside furb-image://${"b".repeat(64)}.png.`)).toEqual([
    { text: reference, name: "plan _v2__final", uri: image.uri },
  ]);
  expect(imagePath("/images", image.uri)).toEqual({ path: join("/images", `${digest}.png`), digest });
  expect(() => imagePath("/images", "furb-image://short.png")).toThrow("Invalid image attachment.");
});

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
    expect(await readFile(join(directory, ".furb/.gitignore"), "utf8")).toBe("*\n");
    const status = Bun.spawnSync(["git", "status", "--porcelain", "--untracked-files=all"], {
      cwd: directory,
    });
    expect(status.stdout.toString()).toBe("?? pixel.png\n");
  } finally {
    await session.dispose();
    await rm(directory, { recursive: true, force: true });
  }
});

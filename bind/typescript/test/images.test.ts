import { expect, test } from "bun:test";
import { chmod, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { imageContent, World } from "../src/index.ts";

test("image attachments reach pi-ai and the Claude CLI as image blocks and remain available after replay", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-images-"));
  const record = join(directory, "session.jsonl");
  const path = join(directory, "pixel.png");
  const data = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/l9sAAAAASUVORK5CYII=";
  await writeFile(path, Buffer.from(data, "base64"));
  const bin = new URL("fake-claude.ts", import.meta.url).pathname;
  await chmod(bin, 0o755);
  const log = join(directory, "cli.jsonl");
  const previous = process.env.FURB_FAKE_LOG;
  process.env.FURB_FAKE_LOG = log;
  let world = new World({ cwd: directory, record, claude: { bin } });
  try {
    const image = world.attachImage(path);
    const life = world.open();
    expect(await life.prompt<string>("str", `Describe this pixel. ![pixel](${image.uri})`)).toBe("reply 1");
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
    expect(imageContent(world.imageDirectory, image.uri).data).toBe(data);
    await world.dispose();
    const before = await readFile(log, "utf8");
    world = new World({ record, claude: { bin } });
    world.open();
    expect(imageContent(world.imageDirectory, image.uri).data).toBe(data);
    expect(await readFile(log, "utf8")).toBe(before);
  } finally {
    await world.dispose();
    if (previous === undefined) delete process.env.FURB_FAKE_LOG;
    else process.env.FURB_FAKE_LOG = previous;
    await rm(directory, { recursive: true, force: true });
  }
}, 30000);

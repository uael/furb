import { expect, test } from "bun:test";
import { mkdtemp, readdir, readFile } from "node:fs/promises";
import { join } from "node:path";
import { remove } from "../../bind/typescript/test/processes.ts";

/** The text of a stream, which a log takes as it comes. */
async function tee(stream: ReadableStream<Uint8Array>, log: NodeJS.WriteStream): Promise<string> {
  const decoder = new TextDecoder();
  let text = "";
  for await (const chunk of stream) {
    log.write(chunk);
    text += decoder.decode(chunk, { stream: true });
  }
  return text;
}

/** The whole gallery runs on CI, to its last capture, so that a scene that hangs or fails stops a pull request and not
 * the audit that draws the gallery later. It draws in a home of its own, with a daemon of its own, and writes to a
 * folder of the test the text and the SVG of each capture, which fails on a character that the fonts lack, and the
 * picture of the last one. The pictures of the others are drawn by the workflow of the gallery and by `bun run
 * screenshots`, outside the path of the gates, where 125 renders would take most of the time of the run. The render does
 * not depend on the system, so it runs on Linux alone, and spares the runners of macOS. */
test.skipIf(process.platform !== "linux")(
  "the gallery runs through to its last capture",
  async () => {
    // The home holds the socket of its daemon, which needs a short path, since a socket has a bound on it.
    const root = await mkdtemp("/tmp/furb-gallery-test-");
    try {
      const script = join(import.meta.dir, "../script/screenshots.ts");
      // The last capture of the script is the one that its last scene draws.
      const source = await readFile(script, "utf8");
      const last = /capture\("([^"]+)"\)(?![\s\S]*capture\(")/.exec(source)?.[1];
      if (!last) throw new Error("The script names no capture.");
      const env = { ...process.env };
      delete env.FURB_GALLERY_HOME;
      delete env.FURB_GALLERY_PARENT;
      delete env.FURB_GALLERY_ONLY;
      env.FURB_GALLERY_PROOF = last;
      env.FURB_GALLERY_ROOT = root;
      env.FURB_GALLERY_OUT = join(root, "out");
      const run = Bun.spawn([process.execPath, script], { env, stdout: "pipe", stderr: "pipe" });
      // The script writes a line for each capture, and the log takes it as it comes: a limit that ends the test keeps
      // the step that the script stood at.
      const [out, , code] = await Promise.all([
        tee(run.stdout, process.stdout),
        tee(run.stderr, process.stderr),
        run.exited,
      ]);
      if (code !== 0) throw new Error(`The gallery ended with ${code}`);
      expect(out).toMatch(/Wrote \d+ captures/);
      // Each capture that the script named has its file: the SVG, and for the last one its picture.
      const names = [...new Set([...out.matchAll(/^capture (\S+)$/gm)].map((one) => one[1] ?? ""))];
      expect(names).toContain(last);
      const files = new Set(await readdir(join(root, "out")));
      expect(names.filter((name) => !files.has(name === last ? `${name}.png` : `${name}.svg`))).toEqual([]);
    } finally {
      await remove(root);
    }
  },
  420000,
);

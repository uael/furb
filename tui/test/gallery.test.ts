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
 * the audit that draws the gallery later. It draws in a home of its own, with a daemon of its own, and writes its
 * pictures to a folder of the test. The gallery draws about 125 shots in about 3 minutes, and a loaded runner takes
 * about twice that. The render does not depend on the system, so it runs on Linux alone, and spares the runners of
 * macOS. */
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
      expect(last).toBeDefined();
      const env = { ...process.env };
      delete env.FURB_GALLERY_HOME;
      delete env.FURB_GALLERY_PARENT;
      delete env.FURB_GALLERY_ONLY;
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
      expect(await readdir(join(root, "out"))).toContain(`${last}.png`);
    } finally {
      await remove(root);
    }
  },
  420000,
);

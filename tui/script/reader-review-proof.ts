import { appendFile, mkdir, readdir, readFile, rm, stat, writeFile } from "node:fs/promises";
import { basename, join, resolve } from "node:path";
import { createTestRenderer } from "@opentui/core/testing";
import { App } from "../src/app.ts";
import { openEngine } from "../src/bridge.ts";
import { loadParsers } from "../src/parsers.ts";
import { Preferences } from "../src/preferences.ts";
import { paletteOf } from "../src/theme.ts";
import { Workspaces } from "../src/workspaces.ts";
import { gif, pack, type Still } from "./gif.ts";
import { pixels, rasterize } from "./raster.ts";
import { end, home } from "./stage.ts";

const projects = await home("furb-reader-review-proof");
const output = resolve(process.env.FURB_READER_REVIEW_OUT ?? "docs/reader-review");
await mkdir(output, { recursive: true });
await loadParsers();
let failed = false;

/** The side files of a fixture record, except its derived told cache, with the bytes and times that a read keeps. */
async function held(record: string) {
  const directory = join(record, "..");
  const name = basename(record);
  return Promise.all(
    (await readdir(directory))
      .sort()
      .filter((one) => (one === name || one.startsWith(`${name}.`)) && one !== `${name}.told`)
      .map(async (one) => ({
        name: one,
        bytes: (await readFile(join(directory, one))).toString("base64"),
        time: (await stat(join(directory, one))).mtimeMs,
      })),
  );
}

try {
  for (const [width, height] of [
    [80, 24],
    [160, 48],
  ] as const) {
    const stills: Still[] = [];
    let dimensions = { width: 0, height: 0 };
    for (const kind of ["whole", "damaged", "missing"] as const) {
      const directory = join(projects, `${kind}-${width}`);
      await mkdir(directory);
      const record = join(directory, "earlier.jsonl");
      const first = await openEngine({ cwd: directory, record, demo: true });
      try {
        await first.engine.result(
          await first.engine.rung({
            word: 'write(Text("notes.txt", "A note.\\nAnother line.\\n"))',
            on: first.engine.root,
          }),
        );
      } finally {
        await first.host.dispose();
      }
      await appendFile(
        `${record}.changes.jsonl`,
        JSON.stringify({ path: join(directory, "legacy.txt"), before: "Before.\n", after: "After.\n" }) +
          "\n",
      );
      const text = await readFile(record, "utf8");
      const made = /\["made","chain1","store","[0-9a-f]{64}",\d+\]/.exec(text)?.[0];
      if (!made) throw new Error("The fixture names no engine.");
      await writeFile(
        record,
        text.replace(made, JSON.stringify(["made", "chain1", "store", "0".repeat(64)])),
      );
      if (kind === "damaged") await appendFile(`${record}.changes.jsonl`, '{"path":"cut');
      if (kind === "missing") await rm(`${record}.changes.jsonl`);
      const before = await held(record);
      const library = new Workspaces(new Preferences(join(directory, "config/ui.json")), { demo: true });
      const screen = await createTestRenderer({ width, height, exitSignals: [] });
      let app: App | undefined;
      try {
        await library.import(record, await library.add(directory));
        const session = library.current?.session;
        if (!session?.host.reading) throw new Error("The fixture did not open to read.");
        app = new App(screen.renderer, session, { quit() {}, workspaces: library });
        app.render();
        app.closeOverlay();
        session.review("life");
        await session.refresh();
        app.render();
        await screen.flush();
        await app.colorsDone();
        app.render();
        await screen.flush();
        const frame = screen.captureCharFrame();
        if (!frame.includes("The life") || !frame.includes("Read only"))
          throw new Error(`${kind}: the Life review or read-only status is hidden.`);
        if (/Revert|Comment a line|revert hunk|Repair the journal|Start a new journal/.test(frame))
          throw new Error(`${kind}: a read offers a write.`);
        if (
          kind === "whole" &&
          (!frame.includes("2 files") || !frame.includes("notes.txt") || !frame.includes("legacy.txt"))
        )
          throw new Error("The journal's current and older changes are not visible.");
        if (kind === "damaged" && !frame.includes("The journal of file changes is damaged"))
          throw new Error("The cut journal is not reported.");
        if (
          kind === "missing" &&
          (session.changes.length !== 0 ||
            !frame.includes("No file-change snapshots are available") ||
            frame.includes("No net change"))
        )
          throw new Error("A missing journal is shown as a known empty net change.");
        const spans = screen.captureSpans();
        const prefix = `${output}/${kind}-${width}x${height}`;
        await writeFile(`${prefix}.txt`, frame);
        await writeFile(`${prefix}.png`, rasterize(spans, paletteOf(session.theme), "furb"));
        const image = pixels(spans, paletteOf(session.theme), "furb", 1);
        dimensions = image;
        stills.push(pack(image.pixels, 90));
      } finally {
        app?.dispose();
        screen.renderer.destroy();
        await library.dispose();
      }
      const after = await held(record);
      if (
        before.length !== after.length ||
        before.some(
          (one, at) =>
            one.name !== after[at]?.name ||
            one.bytes !== after[at]?.bytes ||
            Math.abs(one.time - (after[at]?.time ?? NaN)) >= 1,
        )
      )
        throw new Error(`${kind}: the read changed the record or a side file.`);
      console.log(`${width}x${height} ${kind}: actual Life review, no writes, record bytes and times kept`);
    }
    await writeFile(
      `${output}/reader-review-${width}x${height}.gif`,
      gif(stills, dimensions.width, dimensions.height),
    );
  }
} catch (error) {
  failed = true;
  console.error(error);
} finally {
  await end(failed);
}

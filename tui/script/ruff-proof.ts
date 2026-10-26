import { createHash } from "node:crypto";
import { appendFile, mkdir, readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { until } from "../../bind/typescript/test/until.ts";
import { loadParsers } from "../src/parsers.ts";
import { paletteOf } from "../src/theme.ts";
import { composing } from "../test/composing.ts";
import { original, ruffFixture } from "../test/ruff-fixture.ts";
import { gif, pack, type Still } from "./gif.ts";
import { pixels, rasterize } from "./raster.ts";
import * as stage from "./stage.ts";

const out = resolve(process.env.FURB_RUFF_OUT ?? "docs/ruff");
await mkdir(out, { recursive: true });
await loadParsers();
// The raster gallery font has no CJK/emoji glyphs. Renderer tests cover those exact values; these pictures use
// printable ASCII so rasterization remains honest instead of silently substituting missing glyphs.
const capturedOriginal = original.replaceAll("雪😀", "Unicode value");
const fixture = await ruffFixture(capturedOriginal);
const sources = [
  "bind/typescript/src/life/program.ts",
  "tui/src/app.ts",
  "tui/src/ruff.ts",
  "tui/src/session.ts",
  "tui/src/preferences.ts",
  "tui/src/commands.ts",
  "tui/src/keys.ts",
  "tui/test/ruff-fixture.ts",
  "tui/script/ruff-proof.ts",
];
const manifest: Record<string, string> = {};
const sha = (bytes: Uint8Array | string) => createHash("sha256").update(bytes).digest("hex");
const head = Bun.spawn(["git", "rev-parse", "HEAD"], { stdout: "pipe" });
const sourceHead = (await new Response(head.stdout).text()).trim();
if (await head.exited) throw new Error("Could not read source HEAD");
for (const path of sources) manifest[path] = sha(await readFile(path));
try {
  await composing(
    async ({ session, app, screen, frame }) => {
      const stills: Still[] = [];
      let size = { width: 0, height: 0 };
      const capture = async (name: string, expected: string, animate = false) => {
        await frame();
        await app.colorsDone();
        const text = await frame();
        if (!text.includes(expected)) throw new Error(`${name} is missing ${expected}:\n${text}`);
        const spans = screen.captureSpans();
        const png = rasterize(spans, paletteOf(session.theme), "furb");
        await writeFile(`${out}/${name}.png`, png);
        // Text proof omits terminal padding; the PNG retains the exact terminal cells.
        await writeFile(`${out}/${name}.txt`, text.replace(/[ \t]+$/gm, ""));
        manifest[`${name}.png`] = sha(png);
        if (animate) {
          const image = pixels(spans, paletteOf(session.theme), "furb", 1);
          size = image;
          // GIF delay is in centiseconds: a readable 1.4-second hold per walkthrough state.
          stills.push(pack(image.pixels, 140));
        }
      };
      for (let n = 0; n < 300; n++) {
        const text = await frame();
        if (/\b7 next_value/.test(text)) break;
        if (n === 299) throw new Error("Ruff did not settle");
        await Bun.sleep(10);
      }
      const focused = app.composer.focused;
      await capture("160-running", "# Choosing the next value", true);
      screen.resize(80, 24);
      await capture("80-running", "streamed = [1,");
      app.scroll.scrollTo(0);
      let rows = (await frame()).split("\n");
      let y = rows.findIndex((row) => row.includes("brief = "));
      let x = (rows[y] ?? "").indexOf("...");
      if (x < 0 || y < 0) throw new Error("No visible quote fold");
      await screen.mockMouse.click(x + 1, y);
      await capture("80-expanded", "Keep every quoted line");
      app.scroll.scrollTo(99999);
      await capture("80-expanded-tail", "Last instruction");
      let copied = "";
      screen.renderer.copyToClipboardOSC52 = (text) => {
        copied = text;
        return true;
      };
      app.scroll.scrollTo(0);
      rows = (await frame()).split("\n");
      y = rows.findIndex((row) => row.includes("brief = "));
      x = (rows[y] ?? "").indexOf("brief");
      await screen.mockMouse.click(x + 2, y, 2);
      await frame();
      await stage.click(screen, "Copy its source");
      if (copied !== capturedOriginal) throw new Error("Copy changed original source");
      await writeFile(`${out}/copied.json`, `${JSON.stringify(copied)}\n`);
      session.folds = { "thread:prompt1": false };
      screen.resize(160, 48);
      await session.command("/python raw");
      await capture("160-raw", "streamed = [1,", true);
      screen.mockInput.pressKey("r", { ctrl: true, meta: true });
      await screen.waitFor(() => !session.preferences.rawPython);
      await capture("160-formatted", "result = await bash(", true);
      if (app.composer.focused !== focused) throw new Error("A passive display update changed focus");
      // Selecting text must not open a line or change its original-source copy action.
      rows = (await frame()).split("\n");
      y = rows.findIndex((row) => row.includes("result = await bash("));
      x = (rows[y] ?? "").indexOf("result");
      await screen.mockMouse.drag(x, y, x + 10, y);
      await capture("160-selection", "result = await bash(");
      const completed = [
        ["run", "run3", "chain1", "chain1", "rung3", "streamed = [1, 2]", ""],
        ["done", "run3", "", null],
        ["done", "rung3", "", null],
        ["rung", "rung4", "prompt1", "chain1", "appended = 4", "", ""],
        ["run", "run4", "chain1", "chain1", "rung4", "appended = 4", ""],
        ["done", "run4", "", null],
        ["done", "rung4", "", null],
        ["done", "prompt1", "", null],
        ["prompt", "prompt2", "operator", "chain1", "str", "Independent thread", "codex:gpt-6.1-sol/high"],
        ["rung", "rung5", "prompt2", "chain1", "independent = 1", "", ""],
        ["run", "run5", "chain1", "chain1", "rung5", "independent = 1", ""],
        ["done", "run5", "", null],
        ["done", "rung5", "", null],
        ["done", "prompt2", "", null],
      ];
      await appendFile(fixture.record, `${completed.map((fact) => JSON.stringify(fact)).join("\n")}\n`);
      await until(session, () => Boolean(session.actOf("rung5")));
      session.host.streams.delete("rung3");
      session.folds = { "thread:prompt1": false, "thread:prompt2": false };
      await capture("160-finished", "independent = 1", true);
      screen.resize(80, 24);
      await frame();
      app.scroll.scrollTo(99999);
      await capture("80-finished", "independent = 1");
      session.folds["thread:prompt1"] = true;
      await capture("80-folded-thread", "Independent thread");
      const animation = gif(stills, size.width, size.height);
      await writeFile(`${out}/ruff.gif`, animation);
      manifest["ruff.gif"] = sha(animation);
    },
    { width: 160, height: 48, useMouse: true, kittyKeyboard: true },
    fixture.session,
  );
  await writeFile(
    `${out}/manifest.json`,
    JSON.stringify({ sourceHead, ruff: Bun.which("ruff"), files: manifest }, null, 2) + "\n",
  );
} finally {
  await fixture.remove();
}

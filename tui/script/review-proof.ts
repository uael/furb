import { spawnSync } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { until } from "../../bind/typescript/test/until.ts";
import { removeDemoDirectories } from "../src/demo.ts";
import { loadParsers } from "../src/parsers.ts";
import { paletteOf } from "../src/theme.ts";
import { composing } from "../test/composing.ts";
import { gif, pack, type Still } from "./gif.ts";
import { pixels, rasterize } from "./raster.ts";

const out = resolve(process.env.FURB_REVIEW_OUT ?? "docs/review");
await mkdir(out, { recursive: true });
await loadParsers();
try {
  for (const [width, height] of [
    [80, 24],
    [160, 48],
  ] as const) {
    await composing(
      async ({ session, app, screen, frame }) => {
        const root = session.engine.root;
        const directory = session.host.directory;
        const git = (...args: string[]) => {
          const result = spawnSync(
            "git",
            [
              "-C",
              directory,
              "-c",
              "user.name=furb",
              "-c",
              "user.email=furb@example.com",
              "-c",
              "commit.gpgsign=false",
              ...args,
            ],
            { encoding: "utf8" },
          );
          if (result.status !== 0) throw new Error(result.stderr || `git ${args[0]} failed`);
        };
        await writeFile(join(directory, ".gitignore"), "demo.jsonl*\n.furb/\n");
        await writeFile(join(directory, "notes.txt"), "before\n");
        git("init", "-q", "-b", "main");
        git("add", ".");
        git("commit", "-q", "-m", "Review fixture");
        git("checkout", "-q", "-b", "review");
        const written = async (path: string, content: string, on = root) => {
          const count = session.host.changes;
          await session.engine.result(
            await session.engine.rung({
              word: `write(Text(${JSON.stringify(path)}, ${JSON.stringify(content)}))`,
              on,
            }),
          );
          await until(session.host, () => session.host.changes > count);
        };
        await written("notes.txt", "after\n");
        await written("fresh.txt", "created\n");
        await session.engine.result(
          await session.engine.rung({ word: 'other = chain("Other writes")', on: root }),
        );
        const other = await session.engine.inspect("other", root);
        if (typeof other.value !== "string") throw new Error("No other fixture chain.");
        await written("elsewhere.txt", "another chain\n", other.value);
        const stills: Still[] = [];
        let dimensions = { width: 0, height: 0 };
        const capture = async (name: string, expected: string) => {
          await frame();
          await app.colorsDone();
          const text = await frame();
          if (!text.includes(expected)) throw new Error(`${name} is not in its expected state: ${expected}`);
          if (text.match(/Changes\s+(\d+)/)?.[1] !== String(session.changes.length))
            throw new Error(`${name} shows journal writes instead of the selected scope's file count.`);
          const spans = screen.captureSpans();
          const prefix = `${out}/${name}-${width}x${height}`;
          await writeFile(`${prefix}.txt`, text);
          await writeFile(`${prefix}.png`, rasterize(spans, paletteOf(session.theme), "furb"));
          const image = pixels(spans, paletteOf(session.theme), "furb", 1);
          dimensions = image;
          stills.push(pack(image.pixels, 80));
        };
        for (const scope of ["turn", "chain", "life", "branch"] as const) {
          session.review(scope);
          await session.refresh();
          const count = scope === "life" || scope === "branch" ? 3 : 2;
          if (session.changes.length !== count)
            throw new Error(`${scope} has ${session.changes.length} files, expected ${count}`);
          app.scroll.scrollTo(0);
          await capture(`scope-${scope}`, "notes.txt");
        }
        session.review("chain");
        await session.refresh();
        await frame();
        await session.command("/changes comment");
        await capture("comment-select", "Comment a line");
        screen.mockInput.pressEnter();
        await screen.flush();
        if (!app.composer.plainText.includes("On notes.txt,"))
          throw new Error("The selected line did not enter the composer.");
        if (!app.composer.focused) throw new Error("The comment did not return focus to the composer.");
        await capture("comment-draft", "On notes.txt,");
        app.composer.setText("");
        await session.command("/changes revert");
        if ((await readFile(join(directory, "notes.txt"), "utf8")) !== "before\n")
          throw new Error("The selected hunk was not reverted.");
        if (!app.composer.focused) throw new Error("Revert did not return focus to the composer.");
        await capture("reverted", "fresh.txt");
        await writeFile(
          `${out}/review-${width}x${height}.gif`,
          gif(stills, dimensions.width, dimensions.height),
        );
        console.log(
          `${width}x${height}: four scopes, keyboard line comment, focused draft, selected revert; ${stills.length} frames`,
        );
      },
      { width, height, useMouse: true, kittyKeyboard: true },
    );
  }
} finally {
  await removeDemoDirectories();
}

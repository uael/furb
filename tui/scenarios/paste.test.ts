// A pasted image takes the name image-N and its mark goes into the text at the caret: the owner presses Ctrl+V twice in
// the middle of a line, and the message that goes out keeps the order of its text and its images.
import { expect } from "bun:test";
import { chmod, mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { imageReferences } from "furb";
import { scenario, test, until } from "./harness.ts";
import { chord } from "./probes.ts";

const PIXEL = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/l9sAAAAASUVORK5CYII=";

test("two pastes of an image with Ctrl+V in the middle of a line name image-1 and image-2, and the message keeps their order", () =>
  scenario([{ cue: "Paste: see", word: 'close("Seen.")' }], async (window) => {
    // The clipboard of the machine holds a pixel: a reader of the platform that prints it stands first on the PATH.
    const bin = join(window.yard.root, "bin");
    await mkdir(bin, { recursive: true });
    const png = join(bin, "pixel.png");
    await writeFile(png, Buffer.from(PIXEL, "base64"));
    for (const [name, body] of [
      ["osascript", `base64 < '${png}' | tr -d '\\n'`],
      ["wl-paste", `cat '${png}'`],
      ["xclip", `cat '${png}'`],
    ] as const) {
      await writeFile(join(bin, name), `#!/bin/sh\n${body}\n`);
      await chmod(join(bin, name), 0o755);
    }
    const path = process.env.PATH;
    process.env.PATH = `${bin}:${path}`;
    try {
      const { app, screen } = window;
      const { key, modifiers } = chord("ctrl+v");
      const pasted = async (count: number) => {
        screen.mockInput.pressKey(key, modifiers);
        await until(() => imageReferences(app.composer.plainText).length === count, `${count} mark`, 30);
      };
      await screen.mockInput.typeText("Paste: see  and  now");
      app.composer.cursorOffset = "Paste: see ".length;
      await pasted(1);
      app.composer.cursorOffset = app.composer.plainText.indexOf(" now");
      await pasted(2);
      const text = app.composer.plainText;
      const named = imageReferences(text);
      expect(named.map((one) => one.name)).toEqual(["image-1.png", "image-2.png"]);
      expect(text).toBe(`Paste: see ${named[0]?.text} and ${named[1]?.text} now`);
      expect(await window.frame()).not.toMatch(/\d+ images? attached/);
      screen.mockInput.pressEnter();
      await window.shows((frame) => frame.includes("Seen."), "the answer");
      const prompts = await window.engine.transcript({ on: window.root, of: "prompt" });
      const sent = String(prompts.at(-1)?.[5]);
      expect(sent).toBe(text);
      expect(imageReferences(sent).map((one) => one.name)).toEqual(["image-1.png", "image-2.png"]);
    } finally {
      process.env.PATH = path;
    }
  }));

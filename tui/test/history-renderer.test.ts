import { afterAll, expect, test } from "bun:test";
import { pixels } from "../script/raster.ts";
import { removeDemoDirectories } from "../src/demo.ts";
import { paletteOf } from "../src/theme.ts";
import { composing } from "./composing.ts";

afterAll(removeDemoDirectories);

test("prompt-history suggestions render and accept by keyboard at 80 by 24", () =>
  composing(
    async ({ session, app, screen, frame }) => {
      session.histories[session.draftKey] = ["summarize the notes", "summarize the issue"];
      await screen.mockInput.typeText("summarize the");
      const shown = await frame();
      expect(shown).toContain("prompt history · Tab to use");
      expect(shown).toContain("summarize the issue");
      const captured = screen.captureSpans();
      const image = pixels(captured, paletteOf(session.theme), "Prompt history", 2);
      expect(image.width).toBeGreaterThan(0);
      expect(image.pixels.some((value) => value !== 0)).toBe(true);
      screen.mockInput.pressTab();
      expect(app.composer.plainText).toBe("summarize the issue");
      expect(await frame()).not.toContain("prompt history · Tab to use");
      await frame();
      screen.mockInput.pressEscape();
      await frame();
      expect(app.composer.plainText).toBe("summarize the issue");
      expect(session.activity.filter((act) => session.isUserPrompt(act))).toHaveLength(0);
    },
    { width: 80, height: 24 },
  ));

test("prompt-history navigation renders at 160 by 48 and restores its draft", () =>
  composing(
    async ({ session, app, screen, frame }) => {
      session.histories[session.draftKey] = ["deploy the service"];
      const draft = "deploy";
      await screen.mockInput.typeText(draft);
      const shown = await frame();
      expect(shown).toContain("prompt history · Tab to use");
      const captured = screen.captureSpans();
      const image = pixels(captured, paletteOf(session.theme), "Prompt history", 2);
      expect(image.width).toBeGreaterThan(0);
      expect(image.pixels.some((value) => value !== 0)).toBe(true);
      screen.mockInput.pressArrow("up");
      expect(app.composer.plainText).toBe("deploy the service");
      expect(await frame()).not.toContain("prompt history · Tab to use");
      await frame();
      screen.mockInput.pressTab();
      expect(app.composer.plainText).toBe("deploy the service");
      expect(app.composer.cursorOffset).toBe(app.composer.plainText.length);
      await frame();
      screen.mockInput.pressEscape();
      await frame();
      expect(app.composer.plainText).toBe("deploy the service");
      app.composer.setText(draft);
      screen.mockInput.pressArrow("up");
      expect(app.composer.plainText).toBe("deploy the service");
      screen.mockInput.pressEscape();
      for (let attempt = 0; attempt < 100 && app.composer.plainText !== draft; attempt++) {
        await frame();
        await Bun.sleep(10);
      }
      expect(app.composer.plainText).toBe(draft);
      await frame();
      app.composer.setText("first line\nsecond line");
      const multiline = app.composer.plainText;
      app.composer.cursorOffset = multiline.length;
      screen.mockInput.pressArrow("up");
      expect(app.composer.plainText).toBe(multiline);
    },
    { width: 160, height: 48 },
  ));

test("a multiline history hint is one line and accepts the exact saved text", () =>
  composing(async ({ session, app, screen, frame }) => {
    const exact = `one\n  ${"detail ".repeat(12)}end  `;
    session.histories[session.draftKey] = [exact];
    await screen.mockInput.typeText("one");
    const shown = await frame();
    const preview = shown.split("\n").find((line) => line.includes("detail"));
    expect(preview).toContain("\\n");
    expect(preview).toContain("…");
    expect(shown.split("\n")).not.toContain(`  ${"detail ".repeat(12)}end  `);
    screen.mockInput.pressTab();
    expect(app.composer.plainText).toBe(exact);
  }));

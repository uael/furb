// ⌃G and /inspect with no name never leave the window silent: each opens the list of the names that it can inspect.
import { expect, test } from "bun:test";
import { composing } from "./composing.ts";

test("⌃G and /inspect with no name open the list of the names to inspect", () =>
  composing(
    async ({ app, screen, frame }) => {
      const before = await frame();
      screen.mockInput.pressKey("g", { ctrl: true });
      for (let k = 0; k < 20 && !app.overlay; k++) await frame();
      const keyed = await frame();
      expect(keyed).not.toBe(before);
      expect(keyed).toContain("Named in this chain");
      app.closeOverlay();
      await frame();
      app.composer.setText("/inspect");
      await app.submit();
      for (let k = 0; k < 20 && !app.overlay; k++) await frame();
      expect(await frame()).toContain("Named in this chain");
    },
    { width: 140, height: 44 },
    true,
  ));

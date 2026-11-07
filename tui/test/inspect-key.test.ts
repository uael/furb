// ⌃G and /inspect with no name never leave the window silent: each opens the list of the names that it can inspect.
import { test } from "bun:test";
import { composing, framed } from "./composing.ts";

test("⌃G and /inspect with no name open the list of the names to inspect", () =>
  composing(
    async ({ app, screen, frame }) => {
      const listed = (shown: string) => shown.includes("Named in this chain");
      screen.mockInput.pressKey("g", { ctrl: true });
      await framed(frame, listed, "the list of the names after ⌃G");
      app.closeOverlay();
      app.composer.setText("/inspect");
      await app.submit();
      await framed(frame, listed, "the list of the names after /inspect");
    },
    { width: 140, height: 44 },
    true,
  ));

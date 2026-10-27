import { afterAll, expect, test } from "bun:test";
import { removeDemoDirectories } from "../src/demo.ts";
import { composing } from "./composing.ts";

afterAll(removeDemoDirectories);

const letters = "abcdefghijklmnopqrstuvwxyz[]".split("");

test("typed text wins over every key with no modifier while the hint of the setup shows", () =>
  composing(async ({ session, app, screen, frame }) => {
    session.setup = { claude: false, signed: false, providers: [] };
    session.catalog = [];
    session.catalogRead = true;
    session.emit("change");
    const shown = await frame();
    expect(shown).toContain("F5");
    expect(shown).toContain("check again");
    for (const letter of letters) {
      app.composer.setText("");
      app.composer.focus();
      await screen.mockInput.typeText(`${letter} then more words`);
      expect(app.composer.plainText).toBe(`${letter} then more words`);
      expect(session.notice).not.toBe("Checking the setup again.");
    }
  }));

test("typed text wins over every key with no modifier while the watch shows its keys", () =>
  composing(async ({ session, app, screen, frame }) => {
    session.watching = true;
    session.emit("change");
    await frame();
    for (const letter of letters) {
      app.composer.setText("");
      app.composer.focus();
      await screen.mockInput.typeText(`${letter} then more words`);
      expect(app.composer.plainText).toBe(`${letter} then more words`);
    }
  }));

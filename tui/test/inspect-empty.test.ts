// A picker with no row says why in plain words, so that the operator tells an empty chain from a broken picker: Ctrl-G on
// a chain with no name yet opens "Inspect a name" and reads its reason.
import { expect, test } from "bun:test";
import { composing } from "./composing.ts";

test("Ctrl-G on a chain with no name says that there is no name to inspect yet", async () => {
  await composing(async ({ session, screen, frame }) => {
    Object.assign(session.engine, { names: async () => [] });
    screen.mockInput.pressKey("g", { ctrl: true });
    await Bun.sleep(50);
    const shown = await frame();
    expect(shown).toContain("Inspect a name");
    expect(shown).toContain("No name to inspect in this chain yet.");
  });
});

test("a picker that has rows, and a filter that matches none of them, keeps its own words", async () => {
  await composing(async ({ session, screen, frame }) => {
    Object.assign(session.engine, { names: async () => ["alpha", "beta"] });
    screen.mockInput.pressKey("g", { ctrl: true });
    await Bun.sleep(50);
    await screen.mockInput.typeText("zzz");
    const shown = await frame();
    expect(shown).toContain("Nothing matches this filter.");
    expect(shown).not.toContain("No name to inspect in this chain yet.");
  });
});

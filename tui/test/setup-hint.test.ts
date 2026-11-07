// A chain whose feed holds work shows no welcome, so the hint of the input that says that no model answers names the
// command that sets a provider up, and no setup above it that is not there.
import { expect, test } from "bun:test";
import { composing } from "./composing.ts";
import { idle } from "./idle.ts";

test("with no model and no welcome in view, the input names /providers and not a setup above", () =>
  composing(
    async ({ session, app, frame }) => {
      await idle(session);
      app.setupState = () => ({ found: [], none: true, shown: false });
      expect(app.welcoming).toBe(false);
      const shown = await frame();
      expect(shown).toContain("No model answers yet. Add a provider with /providers");
      expect(shown).not.toContain("setup above");
    },
    { width: 152, height: 46 },
    true,
  ));

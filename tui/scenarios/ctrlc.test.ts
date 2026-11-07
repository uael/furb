// Ctrl+C on a question of a model: the first press empties the answer being typed, the second refuses the question.
import { expect } from "bun:test";
import { scenario, test } from "./harness.ts";
import { chord } from "./probes.ts";

test("Ctrl+C on a question empties the typed answer, then a second Ctrl+C refuses the question", () =>
  scenario(
    [
      {
        cue: "Ctrlc: ask me.",
        word: 'color = await prompt(str, "Ctrlc: which color?", to="operator")\nclose(f"You said {color}.")',
      },
    ],
    async (window) => {
      await window.type("Ctrlc: ask me.");
      await window.shows((frame) => frame.includes("Ctrlc: which color?"), "the question");
      await window.screen.mockInput.typeText("blu");
      const { key, modifiers } = chord("ctrl+c");
      window.screen.mockInput.pressKey(key, modifiers);
      await window.frame();
      expect(window.app.composer.plainText).toBe("");
      const first = await window.frame();
      expect(first).toContain("Ctrlc: which color?");
      // The footer says what the next press does.
      expect(first).toContain("⌃C refuse");
      window.screen.mockInput.pressKey(key, modifiers);
      const second = await window.shows((frame) => !frame.includes("Input needed"), "the question refused");
      expect(second).not.toContain("Take back");
      const prompts = await window.engine.transcript({ on: window.root, of: "prompt" });
      const asked = prompts.find((fact) => fact[5] === "Ctrlc: which color?");
      expect(JSON.stringify(await window.engine.peek(String(asked?.[1])))).toContain("refused the question");
    },
  ));

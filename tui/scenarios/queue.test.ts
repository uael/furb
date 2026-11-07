// A message that the operator queues while the model works gets its answer once the work ends: the queue of the life
// sends it, and the window keeps no second copy of it.
import { expect } from "bun:test";
import { chord } from "./probes.ts";
import { scenario, test } from "./harness.ts";

test("a message queued with the shortcut while the model works gets its answer once the work ends", () =>
  scenario(
    [
      { cue: "Queue: hold.", word: 'close("Held, then done.")', hold: "go" },
      { cue: "Queue: later.", word: 'close("Queued, then answered.")' },
    ],
    async (window, yard) => {
      await window.type("Queue: hold.");
      await Bun.sleep(1500);
      await window.screen.mockInput.typeText("Queue: later.");
      const { key, modifiers } = chord("meta+return");
      window.screen.mockInput.pressKey(key, modifiers);
      await window.shows((frame) => frame.includes("Queued 1"), "the follow-up in the queue");
      expect(window.app.composer.plainText).toBe("");
      await yard.release("go");
      await window.shows((frame) => frame.includes("Queued, then answered."), "the answer of the queued message");
      const prompts = await window.engine.transcript({ on: window.root, of: "prompt" });
      const answers = await Promise.all(prompts.map((fact) => window.engine.peek(String(fact[1]))));
      expect(answers).toEqual(["Held, then done.", "Queued, then answered."]);
    },
  ));

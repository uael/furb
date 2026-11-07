// The keys of the operator reach the input after a click on a list: a click on the sidebar or on the feed must not
// leave the keys with the list, where a typed text goes nowhere.
import { expect } from "bun:test";
import { scenario, test } from "./harness.ts";

test("the input takes the keys after a click on the sidebar and after a click on the feed", () =>
  scenario([{ cue: "Focus: hi.", word: 'close("Hi back.")' }], async (window) => {
    await window.type("Focus: hi.");
    await window.shows((frame) => frame.includes("Hi back."), "the answer");
    for (const text of ["Focus: hi.", "Hi back."]) {
      await window.click(text);
      await window.screen.mockInput.typeText("zzqq");
      await window.frame();
      expect(window.app.composer.plainText).toBe("zzqq");
      window.app.composer.setText("");
    }
  }));

import { scenario, test } from "./harness.ts";

test("a message on a thread whose word closes it before the model read it still gets its answer", () =>
  scenario(
    [
      { cue: "Scenario: first.", word: "close('First answered.')", chunk: 2, pause: 300 },
      { cue: "Scenario: second.", word: "close('Second answered.')" },
    ],
    async (window) => {
      await window.type("Scenario: first.");
      await new Promise((r) => setTimeout(r, 500));
      await window.type("Scenario: second.");
      await window.shows((frame) => frame.includes("First answered."), "the answer of the first message");
      await window.shows((frame) => frame.includes("Second answered."), "the answer of the second message", 30);
    },
  ));

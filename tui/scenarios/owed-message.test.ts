import { scenario, test } from "./harness.ts";

test("a message on a thread whose word closes it before the model read it still gets its answer", () =>
  scenario(
    [
      { cue: "Scenario: first.", word: "close('First answered.')", chunk: 2, pause: 300 },
      { cue: "Scenario: second.", word: "close('Second answered.')" },
    ],
    async (window) => {
      await window.type("Scenario: first.");
      // The word of the first message streams for about 4 s, and the second goes once its first piece shows, while the
      // first is unanswered.
      await window.shows(
        (frame) => frame.includes("close('Fi"),
        "the word of the first message, as it streams",
      );
      await window.type("Scenario: second.");
      await window.shows((frame) => frame.includes("First answered."), "the answer of the first message");
      await window.shows(
        (frame) => frame.includes("Second answered."),
        "the answer of the second message",
        30,
      );
    },
  ));

// What streams between the model, the life and the window: a quote that the model writes in small chunks, and a
// command that tells more than a model reads.
import { expect } from "bun:test";
import { scenario, test, until, type Window } from "./harness.ts";

/** The value of the first prompt of the operator on the root. */
async function answered(window: Window): Promise<unknown> {
  const facts = await window.engine.transcript({ on: window.root, of: "prompt" });
  const first = facts.find((fact) => fact[2] === "operator");
  return first && window.engine.peek(first[1]);
}

// #1255: a quote whose close mark came in a later chunk than its open mark was cut at its open mark, and the model was
// told that its quote had no close mark.
test("a quote that the model streams in small chunks binds whole, and a quote that the gate refuses is told its finding", () =>
  scenario(
    [
      {
        cue: "Scenario: quote.",
        word: "<s:note>\nHello\nworld\n</s:note>\nclose(str(len(note)))",
        chunk: 3,
        pause: 5,
      },
      {
        cue: "Scenario: quote a name.",
        word: "<s:prompt1>\nHello\n</s:prompt1>\nclose(str(len(prompt1)))",
        chunk: 3,
        pause: 5,
      },
      {
        cue: "binds `prompt1`",
        word: "<s:text>\nHello\n</s:text>\nclose(str(len(text)))",
        chunk: 3,
        pause: 5,
      },
    ],
    async (window, yard) => {
      await window.type("Scenario: quote.");
      await until(async () => (await answered(window)) === "12", "the quote to bind whole");
      await window.type("Scenario: quote a name.");
      await until(() => yard.turns().length === 3, "the turn told the finding");
      expect(yard.turns().map((one) => one.cue)).toEqual([
        "Scenario: quote.",
        "Scenario: quote a name.",
        "binds `prompt1`",
      ]);
      expect(yard.turns().some((one) => one.text.includes("has no close mark"))).toBe(false);
    },
  ));

// #1189: the tell of a command gave the model each byte that its show picked, so a stream of megabytes flooded the
// turn of the model.
test("a command that writes megabytes tells the model its two ends, and its Exit holds it whole", () =>
  scenario(
    [
      {
        cue: "Scenario: flood.",
        word: 'out = await bash("yes zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz | head -n 30000", show=span(1, -1))',
      },
      { cue: "chars)", word: "close(str(len(out.stdout.content)))" },
    ],
    async (window, yard) => {
      await window.type("Scenario: flood.");
      await until(
        async () => (await answered(window)) === String(30000 * 93),
        "the length of the whole stream",
      );
      const [, told] = yard.turns();
      expect(told?.cue).toBe("chars)");
      expect(told?.text.length ?? 0).toBeLessThan(524288);
      await window.shows((frame) => frame.includes("2790000"), "the answer in the feed");
    },
  ));

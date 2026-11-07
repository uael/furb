// The messages of the operator: each gets the answer of the model, while a word of its thread awaits a command too,
// and a step that leaves one unanswered is asked again.

import { expect } from "bun:test";
import { operated } from "furb/life";
import { scenario, test, until, type Window } from "./harness.ts";

/** The prompts of the operator on the root, in order: each id, shape, message and value. */
async function asked(window: Window) {
  const facts = await window.engine.transcript({ on: window.root, of: "prompt" });
  const mine = facts.filter((fact) => operated(String(fact[2])));
  return Promise.all(
    mine.map(async (fact) => ({
      id: fact[1],
      shape: fact[4],
      message: fact[5] as string,
      value: await window.engine.peek(fact[1]),
    })),
  );
}

/** The acts of a kind on the root, each id and whether it is over. */
async function made(window: Window, kind: string) {
  const facts = await window.engine.transcript({ on: window.root });
  return Promise.all(
    facts
      .filter((fact) => fact[0] === kind)
      .map(async (fact) => ({ id: fact[1], over: (await window.engine.peek(fact[1])) !== null })),
  );
}

test("a message of the operator gets the answer of the model", () =>
  scenario([{ cue: "Scenario: say hi.", word: "close('Hi, owner.')" }], async (window, yard) => {
    await window.type("Scenario: say hi.");
    await window.shows((frame) => frame.includes("Hi, owner."), "the answer in the feed");
    expect(await asked(window)).toMatchObject([{ message: "Scenario: say hi.", value: "Hi, owner." }]);
    expect(yard.turns().map((one) => one.cue)).toEqual(["Scenario: say hi."]);
  }));

test("a message goes to the thread that the view shows by default, also once the model closed it, and a new thread only on its own key", () =>
  scenario(
    [
      { cue: "Scenario: first.", word: "close('First answer.')" },
      { cue: "Scenario: second.", word: "close('Second answer.')" },
      { cue: "Scenario: third.", word: "close('Third answer.')" },
    ],
    async (window) => {
      await window.type("Scenario: first.");
      await window.shows((frame) => frame.includes("First answer."), "the first answer");
      const [first] = await asked(window);
      // The thread is over, and the view still shows it: the next message is a message on it, shown by the line of the input.
      await window.shows((frame) => frame.includes("to ↳"), "the thread as the target of the input");
      await window.type("Scenario: second.");
      await window.shows((frame) => frame.includes("Second answer."), "the second answer");
      const prompts = await window.engine.transcript({ on: window.root, of: "prompt" });
      const second = prompts.find((fact) => fact[5] === "Scenario: second.");
      expect(second?.[3]).toBe(first?.id);
      // The key of a new thread starts a thread of its own on the chain.
      window.session.aim();
      await window.type("Scenario: third.");
      await window.shows((frame) => frame.includes("Third answer."), "the third answer");
      const third = (await window.engine.transcript({ on: window.root, of: "prompt" })).find(
        (fact) => fact[5] === "Scenario: third.",
      );
      expect(third?.[3]).toBe(window.root);
    },
  ));

// #1118: a message waited until the command that the word of its thread awaited ended. #1256: a word that awaited a
// command and closed nothing was asked again only when every act of the chain ended, so a message waited behind a
// command of ten minutes.
test("a message on a thread gets its answer while a word of the thread awaits a command, and again once a later word awaited a short one", () =>
  scenario(
    [
      { cue: "Scenario: work.", word: 'slow = await bash("sleep 600", timeout=700)\nclose("Slept.")' },
      { cue: "Scenario: and now?", word: 'close("Still at work.")' },
      { cue: "Scenario: and the benchmarks?", word: 'bench = await bash("sleep 0.3")' },
      { cue: "exited 0", word: 'close("Benchmarks: 5.")' },
    ],
    async (window, yard) => {
      await window.type("Scenario: work.");
      await until(async () => (await made(window, "bash")).length === 1, "the long command to start");
      await window.type("Scenario: and now?");
      await window.shows((frame) => frame.includes("Still at work."), "the answer while the command runs");
      const [work] = await asked(window);
      expect(work).toMatchObject({ message: "Scenario: work.", value: "Still at work." });
      await window.type("Scenario: and the benchmarks?");
      await window.shows((frame) => frame.includes("Benchmarks: 5."), "the answer after the short command");
      const all = await asked(window);
      expect(all.at(-1)).toMatchObject({ message: "Scenario: and the benchmarks?", value: "Benchmarks: 5." });
      expect(yard.turns().map((one) => one.cue)).toEqual([
        "Scenario: work.",
        "Scenario: and now?",
        "Scenario: and the benchmarks?",
        "exited 0",
      ]);
    },
  ));

// #1188: a step that read a message and answered nothing left the operator with no answer, and his thread waited.
test("a step that leaves a message unanswered is asked again, and the message gets its answer", () =>
  scenario(
    [
      { cue: "Scenario: work.", word: "await wait(600, __name__)" },
      { cue: "Scenario: use this.", word: "x = 1" },
      { cue: "unanswered", word: 'close(f"Used it, x = {x}.")' },
    ],
    async (window, yard) => {
      await window.type("Scenario: work.");
      await until(async () => (await made(window, "wait")).length > 0, "the wait");
      await window.type("Scenario: use this.");
      await window.shows((frame) => frame.includes("Used it, x = 1."), "the answer of the step asked again");
      expect((await asked(window))[0]).toMatchObject({
        message: "Scenario: work.",
        value: "Used it, x = 1.",
      });
      expect(yard.turns().map((one) => one.cue)).toEqual([
        "Scenario: work.",
        "Scenario: use this.",
        "unanswered",
      ]);
    },
  ));

test("a word that ends with no close and tells something is asked again, and its next word answers", () =>
  scenario(
    [
      { cue: "Scenario: count.", word: 'n = 41\ndebug(t"{n}")' },
      // The first turn holds "advance on" too, so the step that is asked again is cued on what only it reads.
      { cue: "debugged", word: 'close(f"Counted {n + 1}.")' },
    ],
    async (window, yard) => {
      await window.type("Scenario: count.");
      await window.shows((frame) => frame.includes("Counted 42."), "the answer of the word asked again");
      expect((await asked(window))[0]).toMatchObject({ message: "Scenario: count.", value: "Counted 42." });
      expect(yard.turns().map((one) => one.cue)).toEqual(["Scenario: count.", "debugged"]);
    },
  ));

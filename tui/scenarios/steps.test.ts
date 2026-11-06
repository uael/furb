// The steps of a chain: the order in which the chain asks its rungs, and a fault of the Kernel while a word awaits.

import { expect } from "bun:test";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { operated } from "furb/life";
import { scenario, test, until, type Window } from "./harness.ts";

/** The value of each prompt of the operator on the root, by its message. */
async function answers(window: Window): Promise<Record<string, unknown>> {
  const facts = await window.engine.transcript({ on: window.root, of: "prompt" });
  const mine = facts.filter((fact) => operated(String(fact[2])));
  return Object.fromEntries(
    await Promise.all(mine.map(async (fact) => [fact[5] as string, await window.engine.peek(fact[1])])),
  );
}

// #1182: the chain advanced first the rung whose maker said the fewest facts, so a message of the operator that had
// taken a step already waited behind a prompt that a word made after it.
test("a message of the operator never waits behind a prompt that came after it", () =>
  scenario(
    [
      // Each step reads every message that came since the step before, so a step is told apart by the prompt that it
      // advances: prompt1 is the first message, prompt2 the second, and prompt3 the prompt that the word of prompt2
      // makes. The first message takes two steps.
      {
        cue: "advance on prompt1",
        word: 'y = globals().get("y", 0) + 1\nif y == 2:\n  close("First done.")',
        hold: "go",
      },
      { cue: "advance on prompt2", word: 'inner = prompt(str, "Scenario: inner.")\nclose("Second done.")' },
      { cue: "advance on prompt3", word: 'close("Inner done.")' },
    ],
    async (window, yard) => {
      await window.type("Scenario: first.");
      await until(() => existsSync(join(yard.model, "turns")), "the first turn");
      await window.type("/newthread");
      await window.type("Scenario: second.");
      await until(
        async () => (await answers(window))["Scenario: second."] !== undefined,
        "the second message",
      );
      await yard.release("go");
      await until(() => yard.turns().length === 4, "four turns");
      expect(yard.turns().map((one) => one.cue)).toEqual([
        "advance on prompt1",
        "advance on prompt2",
        "advance on prompt1",
        "advance on prompt3",
      ]);
      await until(
        async () => (await answers(window))["Scenario: first."] === "First done.",
        "the first answer",
      );
    },
  ));

// #1221: an ok of a command that raised in the Kernel, ok=lambda e: True on 11-04, lost the word of each run that the
// Kernel held, and no step came after, so the chain stood still for ever.
test("a fault of a command while another word awaits ends that word alone, and the chain goes on", () =>
  scenario(
    [
      { cue: "Scenario: run long.", word: 'slow = await bash("sleep 600", timeout=700)\nclose("Slept.")' },
      { cue: "Scenario: check.", word: 'checked = await bash("true", ok=lambda e: True)\nclose("Checked.")' },
      { cue: "raised", word: 'close("The check raised.")' },
      { cue: "Scenario: and after?", word: 'close("Still here.")' },
    ],
    async (window, yard) => {
      await window.type("Scenario: run long.");
      await until(
        async () => (await window.engine.transcript({ on: window.root, of: "bash" })).length > 0,
        "the long command",
      );
      const [long] = (await window.engine.transcript({ on: window.root, of: "run" }))
        .filter((one) => /^run\d+$/.test(one[1]))
        .slice(-1);
      await window.type("/newthread");
      await window.type("Scenario: check.");
      await until(
        async () => (await answers(window))["Scenario: check."] === "The check raised.",
        "the answer to the check",
      );
      await window.type("/newthread");
      await window.type("Scenario: and after?");
      await until(
        async () => (await answers(window))["Scenario: and after?"] === "Still here.",
        "a later answer",
      );
      // The word that awaits the long command did not end with the fault.
      expect(long && (await window.engine.peek(long[1]))).toBeNull();
      expect(yard.turns().map((one) => one.cue)).toEqual([
        "Scenario: run long.",
        "Scenario: check.",
        "raised",
        "Scenario: and after?",
      ]);
    },
  ));

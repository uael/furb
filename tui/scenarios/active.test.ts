// A click on the count of active prompts lists each one with its chain, its model and its state, and a paused one with
// the cause of its pause.
import { expect } from "bun:test";
import { scenario, test, until } from "./harness.ts";

test("a click on the count lists each active prompt with its model, and a paused one says why", () =>
  scenario(
    [{ cue: "Active: hold.", word: 'close("Held, then done.")', hold: "go" }],
    async (window, yard) => {
      const { engine, session } = window;
      const root = window.root;
      await window.type("Active: hold.");
      await until(
        () => session.threadsOf(root).list.some((one) => one.model === "claude-cli:opus/high"),
        "the thread with its model",
      );
      await engine.pause(root);
      await until(
        () => session.threadsOf(root).list.some((one) => one.state === "paused"),
        "the paused thread",
      );
      const frame = await window.shows((shown) => /\d+ active/.test(shown), "the count");
      expect(frame).toMatch(/1 active · 1 paused/);
      await window.click("1 active");
      const list = await window.shows((shown) => shown.includes("Active prompts"), "the list");
      expect(list).toContain("Active: hold.");
      expect(list).toContain("Main · claude-cli:opus/high · paused · the operator paused it");
      await yard.release("go");
    },
  ));

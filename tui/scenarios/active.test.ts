// A click on the count of active prompts lists each one with its chain, its model and its state, and a paused one with
// the cause of its pause.
import { expect } from "bun:test";
import { scenario, test, until } from "./harness.ts";

test("a click on the count lists each active prompt, and a paused one says why", () =>
  scenario([], async (window) => {
    const { engine, session } = window;
    const root = window.root;
    await engine.pause(root);
    await engine.prompt("str", { message: "A held thread", on: root });
    await until(
      () => session.threadsOf(root).list.some((one) => one.state === "paused"),
      "the paused thread",
    );
    const frame = await window.shows((shown) => /\d+ active/.test(shown), "the count");
    expect(frame).toMatch(/1 active · 1 paused/);
    await window.click("1 active");
    const list = await window.shows((shown) => shown.includes("Active prompts"), "the list");
    expect(list).toContain("A held thread");
    expect(list).toContain("paused");
    expect(list).toContain("Main · no model · paused · the operator paused it");
  }));

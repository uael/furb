// /<skill> <text> loads the skill on the chain and sends the text as the message that uses it.
import { expect } from "bun:test";
import { scenario, test, until } from "./harness.ts";

test("/ouroboros with a text loads the skill and sends the text as a message", () =>
  scenario([{ cue: "build a todo list", word: 'close("Built.")' }], async (window) => {
    await until(() => window.session.skills.some((one) => one.name === "ouroboros"), "the skill in the list");
    await window.type("/ouroboros build a todo list with a panel");
    await window.shows((frame) => frame.includes("Built."), "the answer to the message");
    const { engine, root } = window;
    await until(async () => {
      const words = (await engine.transcript({ on: root })).map((fact) => JSON.stringify(fact));
      return words.some((one) => one.includes('skill(\\"ouroboros\\")'));
    }, "the skill loaded on the chain");
    expect(window.session.acts.some((act) => String(act.words[1]).includes("build a todo list"))).toBe(true);
  }));

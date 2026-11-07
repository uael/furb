// /<skill> <text> loads the skill on the chain and sends the text as the message that uses it.
import { expect } from "bun:test";
import { scenario, test, until } from "./harness.ts";

test("/ouroboros with a text loads the skill and sends the text as a message", () =>
  scenario([{ cue: "build a todo list", word: 'close("Built.")' }], async (window) => {
    await window.type("/ouroboros build a todo list with a panel");
    await window.shows((frame) => frame.includes("Built."), "the answer to the message");
    const { engine, root } = window;
    await until(async () => {
      const words = (await engine.transcript({ on: root })).map((fact) => JSON.stringify(fact));
      return words.some((one) => one.includes('skill(\\"ouroboros\\")'));
    }, "the skill loaded on the chain");
    expect(window.session.acts.some((act) => String(act.words[1]).includes("build a todo list"))).toBe(true);
  }));

test("the text of a skill command is a message and nothing else, and it goes after the load of the skill", () =>
  scenario([{ cue: "!echo hi", word: 'close("Plain.")' }], async (window) => {
    await window.type("/ouroboros !echo hi");
    await window.shows((frame) => frame.includes("Plain."), "the answer to the message");
    const acts = window.session.acts;
    const load = acts.find(
      (act) => act.kind === "rung" && String(act.words[0]).includes('skill("ouroboros")'),
    );
    const message = acts.find((act) => act.kind === "prompt" && String(act.words[1]) === "!echo hi");
    expect(load).toBeDefined();
    expect(message).toBeDefined();
    expect(acts.some((act) => act.kind === "bash")).toBe(false);
    expect((message?.began ?? 0) > (load?.began ?? Number.POSITIVE_INFINITY)).toBe(true);
  }));

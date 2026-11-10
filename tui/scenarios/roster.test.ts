// A model of the catalog that the roster of a life lacks joins the life of the session that the window shows: the
// window opens that life again in place, so no send goes through a life that the window let go, and the open of the
// model asks for no second open.
import { expect } from "bun:test";
import { scenario, test, until } from "./harness.ts";

test("a model of the catalog that the roster lacks joins the life of the shown session, which takes the next message", () =>
  scenario([{ cue: "Scenario: after the model.", word: 'close("taken with sonnet")' }], async (window) => {
    const { session } = window;
    await window.type("/model claude-cli:sonnet");
    const held = () => session.roster.some(([name]) => name === "claude-cli:sonnet");
    await until(() => held() || Boolean(session.error), "the roster of the life to hold the model", 30).catch(
      () => {},
    );
    await window.type("Scenario: after the model.");
    const frame = await window.shows(
      (one) => one.includes("taken with sonnet"),
      "the answer after the model",
      30,
    );
    expect(frame).not.toContain("disposed");
    expect(held()).toBe(true);
    expect(session.actor.startsWith("claude-cli:sonnet")).toBe(true);
  }));

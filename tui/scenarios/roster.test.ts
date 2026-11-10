// A model of the catalog that the roster of a life lacks joins the life of the session that the window shows at a
// stand of that same life: the life goes on with no end and no replay of its record, and the model takes the next
// message.

import { expect } from "bun:test";
import { readFileSync } from "node:fs";
import { scenario, test, until } from "./harness.ts";

/** How many checkpoints the record holds, one for each end of a life on it. */
const checkpoints = (record: string) =>
  readFileSync(record, "utf8")
    .split("\n")
    .filter(Boolean)
    .filter((line) => (JSON.parse(line) as unknown[][])[0]?.[0] === "checkpoint").length;

test("a model of the catalog that the roster lacks joins the same life at a stand, and takes the next message", () =>
  scenario([{ cue: "Scenario: after the model.", word: 'close("taken with sonnet")' }], async (window) => {
    const { session, record } = window;
    const ended = checkpoints(record);
    await window.type("/model claude-cli:sonnet");
    const held = () => session.roster.some(([name]) => name === "claude-cli:sonnet");
    await until(() => held() || Boolean(session.error), "the roster of the life to hold the model", 30);
    expect(session.error).toBeFalsy();
    expect(checkpoints(record)).toBe(ended);
    await window.type("Scenario: after the model.");
    const frame = await window.shows(
      (one) => one.includes("taken with sonnet"),
      "the answer after the model",
      30,
    );
    expect(frame).not.toContain("disposed");
    expect(session.actor.startsWith("claude-cli:sonnet")).toBe(true);
    expect(checkpoints(record)).toBe(ended);
  }));

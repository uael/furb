// The settings of a life as the operator edits them: /config lists each key by the part that gives it, the filter finds
// a key, and an edit in place writes it to the config that the operator chooses.
import { expect } from "bun:test";
import { scenario, test, until } from "./harness.ts";

test("/config finds a key, edits it in place and writes it to the config chosen", () =>
  scenario([], async (window) => {
    const { screen, session } = window;
    // A key of a schema that takes a plain value, whichever the schemas give.
    const listed = await session.host.settings();
    const first = listed.find(
      (one) =>
        one.property && !one.property.enum && ["integer", "number", "string"].includes(one.property.type),
    );
    if (!first) throw new Error("No key of a schema takes a plain value.");
    const name = first.key.join(".");
    const given = first.property?.type === "string" ? "a value of the scenario" : 123;
    const key = async () => (await session.host.settings()).find((one) => one.key.join(".") === name);
    await window.type("/config");
    await window.shows(
      (frame) => frame.includes("Settings") && frame.includes("furb"),
      "the settings by part",
    );
    await screen.mockInput.typeText(name);
    // The key under the pointer shows its description, its type, its default and the layer that gives its value.
    await window.shows(
      (frame) =>
        frame.includes("default") && frame.includes("layer") && frame.includes(String(first.property?.type)),
      `the key ${name} with its type, its default and where it comes from`,
    );
    screen.mockInput.pressEnter();
    await window.shows((frame) => frame.includes(`${name},`), `the field of the value of ${name}`);
    for (const _ of JSON.stringify(first.value) ?? "") screen.mockInput.pressBackspace();
    await screen.mockInput.typeText(String(given));
    screen.mockInput.pressEnter();
    await window.shows(
      (frame) => frame.includes(`Write ${name} = ${JSON.stringify(given)} to`),
      "the choice of the config to write",
    );
    screen.mockInput.pressEnter();
    await until(async () => (await key())?.value === given, `${name} to be ${given}`);
    expect((await key())?.layer).toBe(first.layer ?? "project");
  }));

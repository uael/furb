import { expect, test } from "bun:test";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { until } from "../../bind/typescript/test/until.ts";
import { openEngine } from "../src/bridge.ts";
import { rowsOf, type Setting, shown, typed } from "../src/configs.ts";
import { Preferences } from "../src/preferences.ts";
import { Session } from "../src/session.ts";
import { composing } from "./composing.ts";

test("the form of a key gives back the value that it showed, for a string of the schema and for any other", () => {
  const text = { type: "string" as const, default: "", description: "" };
  for (const value of ["12", "true", "null", '"quoted"', "plain", ""])
    expect(typed(shown(value, text), text)).toBe(value);
  for (const value of ["12", "null", 12, true, null, ["a"]]) expect(typed(shown(value))).toEqual(value);
});

test("a value that is no JSON is the text that the operator typed", () => {
  expect([typed("false"), typed("12"), typed('"a b"'), typed("a b")]).toEqual([false, 12, "a b", "a b"]);
});

test("/config shows each key with its schema, flips a flag, takes a key out and refuses a value of another kind", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-config-menu-"));
  const path = join(directory, ".furb", "config.json");
  // A skill of the project gives a flag of its own, since furb names none.
  const skill = join(directory, ".furb", "note");
  await mkdir(skill, { recursive: true });
  const manifest =
    "name: note\ndescription: A test.\nmetadata:\n  furb-word: note.py\n  furb-config: keys.json";
  await writeFile(join(skill, "SKILL.md"), `---\n${manifest}\n---\n`);
  await writeFile(join(skill, "note.py"), "");
  const loud = { type: "boolean", default: true, description: "Say each note aloud." };
  await writeFile(join(skill, "keys.json"), JSON.stringify({ properties: { loud } }));
  const held = { other: 1, skills: { note: "note" } };
  await writeFile(path, JSON.stringify(held));
  const opened = await openEngine({
    cwd: directory,
    record: join(directory, "life.jsonl"),
    config: join(directory, "user"),
    roster: [],
    skills: false,
  });
  const session = new Session(opened.engine, opened.host, false, new Preferences(join(directory, "ui.json")));
  try {
    await session.refresh();
    const flag = (await session.host.settings()).find((one) => one.key.join(".") === "note.loud");
    expect([flag?.value, flag?.layer, flag?.property?.type]).toEqual([true, undefined, "boolean"]);
    await composing(
      async ({ app, frame, screen }) => {
        app.configs();
        await session.host.settings();
        expect(await frame()).toContain("Not in a schema");
        await screen.mockInput.typeText("loud");
        expect(await frame()).toContain("Say each note aloud.");
        // A flag offers its two values, with the one that holds first, then the config to write.
        screen.mockInput.pressEnter();
        expect(await frame()).toContain("the default");
        screen.mockInput.pressArrow("down");
        screen.mockInput.pressEnter();
        expect(await frame()).toContain("Write note.loud = false to");
        screen.mockInput.pressEnter();
        await until(session, () => session.notice.startsWith("note.loud is false"));
        expect(JSON.parse(await readFile(path, "utf8"))).toEqual({ ...held, note: { loud: false } });
        const refusal = await session.command("/config note.loud maybe").catch(String);
        expect(refusal).toContain("takes a boolean");
        await session.command("/config note.loud");
        expect(JSON.parse(await readFile(path, "utf8"))).toEqual({ ...held, note: {} });
      },
      { width: 120, height: 44, useMouse: true },
      session,
    );
  } catch (error) {
    await session.dispose().catch(() => {});
    throw error;
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("the rows of a key with an enum give its values on a row of their own, and each key its layer", () => {
  const key: Setting = {
    key: ["mode"],
    value: "a",
    layer: "project",
    property: { type: "string", default: "a", description: "How it runs.", enum: ["a", "b"] },
  };
  expect(rowsOf(key, "project config")).toEqual([
    ["type", "string"],
    ["values", '"a"  "b"'],
    ["default", '"a"'],
    ["layer", "project config"],
  ]);
  const flag: Setting = {
    key: ["loud"],
    value: true,
    property: { type: "boolean", default: true, description: "" },
  };
  expect(rowsOf(flag, "default").map(([name]) => name)).toEqual(["type", "default", "layer"]);
  expect(rowsOf({ key: ["other"], value: 1, layer: "user" }, "user config")).toEqual([
    ["layer", "user config"],
  ]);
});

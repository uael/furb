import { afterAll, expect, test } from "bun:test";
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { demoSession } from "../src/demo.ts";
import { loadPart, Parts, plainError } from "../src/parts.ts";
import { composing } from "./composing.ts";

const made: string[] = [];
afterAll(() => {
  for (const one of made) rmSync(one, { recursive: true, force: true });
});

/** The extension todo of the fixtures, whose manifest names its part for the TUI. */
const fixture = resolve(import.meta.dir, "..", "fixtures", "todo");
/** The part of the extension todo, and its word, less its import of furb, as a life plays it. */
const PART = readFileSync(join(fixture, "tui.ts"), "utf8");
const TODO = readFileSync(join(fixture, "todo.py"), "utf8").replace(/^from furb\.engine import .*\n+/m, "");

/** A project whose config turns on the extension todo of the fixtures, with the part that it is given. */
function project(part = PART): { directory: string; file: string } {
  const directory = mkdtempSync(join(tmpdir(), "furb-parts-"));
  made.push(directory);
  const folder = join(directory, ".furb", "extensions", "todo");
  mkdirSync(folder, { recursive: true });
  cpSync(fixture, folder, { recursive: true });
  writeFileSync(join(folder, "tui.ts"), part);
  writeFileSync(join(directory, ".furb", "config.json"), '{"extensions": {"todo": "extensions/todo"}}\n');
  return { directory, file: join(folder, "tui.ts") };
}

/** The frame of the App once it shows a text, drawn again until it does. */
async function shows(frame: () => Promise<string>, text: string): Promise<string> {
  let shown = await frame();
  for (let tries = 0; tries < 400 && !shown.includes(text); tries++) {
    await Bun.sleep(25);
    shown = await frame();
  }
  expect(shown).toContain(text);
  return shown;
}

test("a life gains the panel of a part of a project extension, which draws again when its file changes or a write lands, and shows the error of a broken module", async () => {
  const { directory, file } = project();
  const session = await demoSession({ cwd: directory });
  await composing(
    async ({ app, frame, click, screen }) => {
      await session.command(`/run ${TODO}\ntodo("write the part")\ntodo("read the capture", done=True)`);
      let shown = await shows(frame, "○ write the part");
      expect(shown).toContain("✓ read the capture");
      expect(shown).toContain("1 open of 2");
      expect(app.parts.loaded.map((one) => one.name)).toEqual(["todo"]);
      // The rung opens, and the row of each act todo says what the part draws for it.
      await click("rung1");
      await shows(frame, "to do: write the part");
      // A change of the file of the part draws again, with no restart.
      writeFileSync(file, PART.replace('"Todo"', '"Todo list"'));
      await shows(frame, "Todo list");
      // A write that the life makes in the folder of the extension lands, and the part loads again.
      const word = `write(Text(".furb/extensions/todo/tui.ts", ${JSON.stringify(PART.replace('"Todo"', '"Plan"'))}))`;
      await session.command(`/run ${word}`);
      await shows(frame, "Plan");
      // A broken module shows its error in its panel, in plain words, and the TUI goes on.
      writeFileSync(file, "export default (furb) => { furb.panel(");
      shown = await shows(frame, "tui.ts of todo");
      expect(shown).not.toContain("○ write the part");
      writeFileSync(file, PART);
      await shows(frame, "○ write the part");
      // The command of the part runs from the input, and its key from an empty input.
      app.composer.setText("/todos");
      await app.submit();
      await shows(frame, "1 of 2 items are open");
      app.composer.setText("");
      await screen.mockInput.typeText("+");
      await shows(frame, "○ one more");
    },
    { width: 120, height: 44 },
    session,
  );
});

test("a part that exports no function, that throws, or that takes a name of the TUI loads with its error and no component", async () => {
  const { file } = project("export const panels = []\n");
  expect((await loadPart("todo", file)).error).toBe(
    "tui.ts of todo: Its module exports no function as its default.",
  );
  writeFileSync(file, 'export default () => { throw new Error("No list.") }\n');
  expect((await loadPart("todo", file)).error).toBe("tui.ts of todo: No list.");
  writeFileSync(file, 'export default (furb) => furb.command("model", "", () => {})\n');
  expect((await loadPart("todo", file)).error).toBe("tui.ts of todo: The command /model is the TUI's own.");
  writeFileSync(file, 'export default (furb) => furb.row("rung", () => ({}))\n');
  const rung = await loadPart("todo", file);
  expect(rung.error).toBe("tui.ts of todo: The row of a rung is the TUI's own.");
  expect(rung.panels).toEqual([]);
  expect(plainError({ message: "Unexpected }", position: { line: 3, column: 7 } })).toBe(
    "Unexpected } at line 3, column 7",
  );
});

test("the parts load the part of each extension that the configs give, and a config that a life cannot read shows its error", async () => {
  const { directory, file } = project();
  let changes = 0;
  const parts = new Parts(directory, () => changes++);
  await parts.load();
  expect(parts.loaded.map((one) => [one.name, one.file, one.error])).toEqual([["todo", file, undefined]]);
  expect(parts.row("todo")).toBeDefined();
  expect(parts.command("todos")?.detail).toBe("Say how many items are open");
  writeFileSync(join(directory, ".furb", "config.json"), '{"extensions": []}\n');
  await parts.load();
  expect(parts.loaded.map((one) => one.name)).toEqual(["extensions"]);
  expect(parts.loaded[0]?.error).toContain("A config is a map, whose extensions is a map");
  expect(changes).toBe(2);
  parts.dispose();
});

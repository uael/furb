import { afterAll, expect, test } from "bun:test";
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { demoSession } from "../src/demo.ts";
import { loadPart, Parts, plainError } from "../src/parts.ts";
import { composing } from "./composing.ts";
import { idle } from "./idle.ts";

const made: string[] = [];
afterAll(() => {
  for (const one of made) rmSync(one, { recursive: true, force: true });
});

/** The extension todo of the fixtures, whose manifest names its part for the TUI. */
const fixture = resolve(import.meta.dir, "..", "fixtures", "todo");
/** The part of the extension todo, and its word, less its import of furb, as a life plays it. */
const PART = readFileSync(join(fixture, "tui.ts"), "utf8");
const TODO = readFileSync(join(fixture, "todo.py"), "utf8").replace(/^from furb\.engine import .*\n+/m, "");

/** The word of the extension compaction, less its import of furb, as a life plays it. */
const COMPACTION = readFileSync(
  resolve(import.meta.dir, "..", "..", "extensions", "compaction", "compaction.py"),
  "utf8",
).replace(/^from furb\.engine import \([^)]*\)\n+/m, "");

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

/** The frame of the App once it shows a text, and once `ready` holds of it when it is given, drawn again until it
 * does: a machine under load draws the feed in more than one frame. */
async function shows(
  frame: () => Promise<string>,
  text: string,
  ready: (shown: string) => boolean = () => true,
): Promise<string> {
  let shown = await frame();
  for (let tries = 0; tries < 400 && !(shown.includes(text) && ready(shown)); tries++) {
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
    async ({ app, frame, screen }) => {
      await session.command(`/run ${TODO}\ntodo("write the part")\ntodo("read the capture", done=True)`);
      let shown = await shows(frame, "○ write the part");
      expect(shown).toContain("✓ read the capture");
      expect(shown).toContain("1 open of 2");
      expect(app.parts.loaded.map((one) => one.name)).toEqual(["compaction", "todo"]);
      // The hint of the line of each act todo says what the part draws as its row.
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
  expect(parts.loaded.map((one) => [one.name, one.file, one.error])).toEqual([
    ["compaction", "extensions/compaction/tui.ts", undefined],
    ["todo", file, undefined],
  ]);
  expect(parts.row("todo")).toBeDefined();
  expect(parts.command("todos")?.detail).toBe("Say how many items are open");
  writeFileSync(join(directory, ".furb", "config.json"), '{"extensions": []}\n');
  await parts.load();
  expect(parts.loaded.map((one) => one.name)).toEqual(["extensions"]);
  expect(parts.loaded[0]?.error).toContain("A config is a map, whose extensions is a map");
  expect(changes).toBe(2);
  parts.dispose();
});

test("the part of compaction, which the crate carries, marks each compaction in the feed of its chain with its summary, and the board counts them", async () => {
  const { directory } = project();
  const session = await demoSession({ cwd: directory });
  await composing(
    async ({ app, frame }) => {
      await session.command(`/run ${COMPACTION}`);
      await session.command(`/run chain("aside")\nbefore = "the first step"`);
      // The word shows in the feed too, so its summary is split in pieces: the whole text stands only in the mark.
      await session.command('/run compact(summary="It read " + "the plan.\\nThe tests " + "wait.")');
      let shown = await shows(frame, "The tests wait.");
      expect(shown).toContain("Compaction: the model reads on from here");
      expect(app.parts.loaded.find((one) => one.name === "compaction")?.error).toBeUndefined();
      expect(shown).toContain("It read the plan.");
      expect(shown).toContain("The tests wait.");
      expect(shown.indexOf("the first step")).toBeLessThan(shown.indexOf("Compaction:"));
      await session.command('/run after = "the next step"');
      await session.command('/run compact(summary="It ran " + "the tests.")');
      shown = await shows(
        frame,
        "It ran the tests.",
        (now) =>
          now.includes("compacted 2×") &&
          now.indexOf("Compaction 1 of 2") < now.indexOf("the next step") &&
          now.indexOf("the next step") < now.indexOf("Compaction 2 of 2"),
      );
      expect(shown).toContain("Compaction 2 of 2");
      expect(shown).toContain("Compaction 1 of 2");
      expect(shown).toMatch(/Compaction 1 of 2[\s\S]*the next step[\s\S]*Compaction 2 of 2/);
      expect(shown.indexOf("Compaction 1 of 2")).toBeLessThan(shown.indexOf("the next step"));
      expect(shown.indexOf("the next step")).toBeLessThan(shown.indexOf("Compaction 2 of 2"));
      expect(shown).toContain("compacted 2×");
    },
    { width: 120, height: 60 },
    session,
  );
});

test("the command /compact of the part of compaction asks a model what the chain on screen keeps, compacts it with the summary, and says so", async () => {
  const { directory } = project();
  const session = await demoSession({ cwd: directory });
  await composing(
    async ({ app, frame }) => {
      for (let tries = 0; tries < 400 && !app.parts.command("compact"); tries++) await Bun.sleep(25);
      expect(app.parts.command("compact")?.detail).toBe(
        "Compact this chain now: a model keeps what the work still needs and sums up the rest",
      );
      app.composer.setText("/compact");
      await app.submit();
      await shows(frame, "This chain has no compaction");
      // The operator plays the model that chooses, so the test asks no model.
      await session.command(`/run ${COMPACTION}\nCHOOSER = OPERATOR\nbefore = "the first step"`);
      // The acts of the life reach the view a moment after the word ran.
      for (let tries = 0; tries < 400 && !app.seen().acts.some((act) => act.kind === "rung"); tries++)
        await Bun.sleep(25);
      app.composer.setText("/compact");
      await app.submit();
      await shows(frame, "A model compacts this chain");
      let asked = "";
      for (let tries = 0; tries < 400 && !asked; tries++) {
        await session.refresh();
        asked =
          session.acts.find(
            (act) => act.kind === "prompt" && String(act.words[1] ?? "").startsWith("The chain below"),
          )?.id ?? "";
        if (!asked) await Bun.sleep(25);
      }
      expect(asked).toMatch(/^prompt\d+$/);
      await session.command(`/run close(([], "It read " + "the plan."), id="${asked}")`);
      const shown = await shows(frame, "It read the plan.", (seen) =>
        seen.includes("Compaction: the model reads on from here"),
      );
      expect(shown).toContain("Compaction: the model reads on from here");
      expect(shown.indexOf("the first step")).toBeLessThan(shown.indexOf("Compaction:"));
    },
    { width: 132, height: 60 },
    session,
  );
});

test("a mark of a part stands at its place in the feed of its chain, and a mark that throws shows its error", async () => {
  const { file } = project(
    'export default (furb) => furb.mark((seen) => { if (seen.chain === "none") throw new Error("No marks."); return [{ chain: seen.chain, at: 2, title: "Here" }, { chain: "other", at: 1, title: "Not here" }] })\n',
  );
  const part = await loadPart("todo", file);
  const parts = new Parts(
    "",
    () => {},
    () => [],
  );
  parts.loaded = [part];
  const seen = {
    chain: "chain1",
    acts: [],
    compactions: [],
    of: () => [],
    run: async () => {},
    say: () => {},
  };
  expect(parts.marks(seen)).toEqual([{ chain: "chain1", at: 2, title: "Here" }]);
  expect(parts.marks({ ...seen, chain: "none" })).toEqual([
    { chain: "none", at: 0, title: "A mark of todo failed: No marks." },
  ]);
});

test("a run of compactions into new chains, as an older compaction made them, shows as one chain: one row named first -> last, and one feed with the rule of each compaction", async () => {
  const { directory } = project();
  const session = await demoSession({ cwd: directory });
  const root = session.engine.root;
  await composing(
    async ({ frame }) => {
      await session.command(`/run ${COMPACTION}\ndef restore(source, on=""):\n  return None\nchain("aside")`);
      await session.command('/run before = "the first step"');
      const summary = "<s:summary>\\nIt read the plan.</s:summary>\\nreground()";
      await session.command(
        `/run older = chain("compacted", "${root}")\nrung("restore('${root}')\\n${summary}", on=older)\nrung('after = "the step after"', on=older)`,
      );
      const shown = await shows(
        frame,
        "the step after",
        (now) => session.runOf(root).length > 1 && now.includes("goes on in a new chain from here"),
      );
      const newer = session.chains.find((chain) => chain.words[0] === "compacted")?.id ?? "";
      expect(session.runOf(root)).toEqual([root, newer]);
      expect(session.labelOf(newer)).toBe("Main -> compacted");
      expect(session.activeChains().active.map(([chain]) => chain.id)).not.toContain(root);
      // A link or the graph that names the first chain of the run puts the operator on its last chain, which the headline
      // names and which a message, a command and a grant go on.
      await session.select(root);
      expect(session.selected).toBe(newer);
      expect(session.label).toBe("Main -> compacted");
      expect(shown).toContain("Main -> compacted");
      expect(shown).toContain("It read the plan.");
      expect(shown).toContain("compacted  ");
      expect(shown.indexOf("the first step")).toBeLessThan(shown.indexOf("goes on in a new chain from here"));
      // The word of the operator that made the step stands in the feed too, so the step is its last line.
      expect(shown.indexOf("goes on in a new chain from here")).toBeLessThan(
        shown.lastIndexOf("the step after"),
      );
    },
    { width: 120, height: 60 },
    session,
  );
});

/** Whether the frame stops showing a text, drawn again until it does. */
async function hides(frame: () => Promise<string>, text: string): Promise<string> {
  let shown = await frame();
  for (let tries = 0; tries < 400 && shown.includes(text); tries++) {
    await Bun.sleep(25);
    shown = await frame();
  }
  expect(shown).not.toContain(text);
  return shown;
}

test("a word that writes an extension with extend() gives the TUI its part in a life that runs, with no restart, and a config that turns the extension off takes the part away", async () => {
  const directory = mkdtempSync(join(tmpdir(), "furb-parts-"));
  made.push(directory);
  const session = await demoSession({ cwd: directory, extensions: true });
  await composing(
    async ({ app, frame, screen }) => {
      expect(app.parts.loaded.map((one) => one.name)).toEqual(["compaction"]);
      await session.command(`/run ${TODO}`);
      const word = 'next(k for k, w in program().items() if "def todo" in w)';
      await session.command(`/run <s:part>\n${PART}</s:part>\nextend("todo", ${word}, tui=part)`);
      await session.command('/run todo("write the part")\ntodo("read the capture", done=True)');
      const shown = await shows(frame, "○ write the part");
      expect(shown).toContain("✓ read the capture");
      expect(app.parts.loaded.map((one) => [one.name, one.error])).toEqual([
        ["compaction", undefined],
        ["todo", undefined],
      ]);
      app.composer.setText("/todos");
      await app.submit();
      await shows(frame, "1 of 2 items are open");
      // The extension goes off in the config: the panel, the row, the command and the key go with it.
      await session.command(
        `/run write(Text(".furb/config.json", ${JSON.stringify('{"extensions": {"todo": false}}\n')}))`,
      );
      await hides(frame, "○ write the part");
      expect(app.parts.loaded.map((one) => one.name)).toEqual(["compaction"]);
      expect(app.parts.command("todos")).toBeUndefined();
      expect(app.parts.row("todo")).toBeUndefined();
      await screen.mockInput.typeText("+");
      expect(app.composer.plainText).toBe("+");
      app.composer.setText("");
      // The config turns it on again, and the part comes back.
      await session.command(
        `/run write(Text(".furb/config.json", ${JSON.stringify('{"extensions": {"todo": "extensions/todo"}}\n')}))`,
      );
      await shows(frame, "○ write the part");
    },
    { width: 120, height: 44 },
    session,
  );
});

test("/extensions opens a dialog of the extensions that the life runs, each with what its part gives, or the error that stopped its load", async () => {
  const { directory, file } = project();
  const session = await demoSession({ cwd: directory });
  await composing(
    async ({ app, frame, screen }) => {
      await shows(frame, "Ask");
      app.composer.setText("/extensions");
      await app.submit();
      let shown = await shows(frame, "Its part gives panel Todo");
      expect(shown).toContain("Extensions");
      expect(shown).toContain("rows of todo, key +, /todos");
      expect(shown).toContain("Its part gives 1 mark in the feed");
      screen.mockInput.pressEscape();
      await hides(frame, "Its part gives panel Todo");
      // A part that fails to load shows its error in the dialog, and the extension still stands in it.
      writeFileSync(file, "export default (furb) => { furb.panel(");
      while (!app.parts.loaded.find((one) => one.name === "todo")?.error) await Bun.sleep(10);
      app.composer.setText("/extensions");
      await app.submit();
      shown = await shows(frame, "tui.ts of todo");
      expect(shown).not.toContain("Its part gives panel Todo");
    },
    { width: 150, height: 44 },
    session,
  );
});

test("the model of the demo adds a component to a life that runs: it reads the teaching, writes the extension, and its panel stands, fails and mends with no restart", async () => {
  const directory = mkdtempSync(join(tmpdir(), "furb-parts-"));
  made.push(directory);
  const session = await demoSession({ cwd: directory, extensions: true });
  await composing(
    async ({ app, frame }) => {
      await session.submit(
        "Add a component for this todo list: a panel with the items, and a line in the feed for each.",
      );
      await idle(session);
      let shown = await shows(frame, "○ Draw the todo list in a panel", (now) =>
        now.includes("✓ Read the capture"),
      );
      expect(shown).toContain("2 open of 3");
      expect(app.parts.loaded.map((one) => [one.name, one.error])).toEqual([
        ["compaction", undefined],
        ["todo", undefined],
      ]);
      // A change that the model gets wrong shows its error in the panel, and the rest of the TUI goes on.
      await session.submit("Sort the Todo panel: the open items first.");
      await idle(session);
      shown = await shows(frame, "evaluating 'seen.todos");
      expect(shown).not.toContain("2 open of 3");
      // The operator tells the model, and the panel draws again from the file that it mends.
      await session.submit("The Todo panel shows an error. Fix it.");
      await idle(session);
      shown = await shows(frame, "2 open of 3", (now) => !now.includes("evaluating 'seen.todos"));
      expect(shown.indexOf("○ Draw the todo list in a panel")).toBeLessThan(
        shown.indexOf("✓ Read the capture"),
      );
    },
    { width: 140, height: 50 },
    session,
  );
});

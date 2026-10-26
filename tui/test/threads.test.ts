import { expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { LineNumberRenderable, RGBA, TextRenderable } from "@opentui/core";
import { chainThreads } from "furb/life";
import { until } from "../../bind/typescript/test/until.ts";
import { demoSession } from "../src/demo.ts";
import { palettes } from "../src/theme.ts";
import { type Composing, cellAt, composing } from "./composing.ts";
import { idle } from "./idle.ts";

const size = { width: 152, height: 46 };

/** A frame that the QA of a change reads, when FURB_QA names a folder. */
const keep = (name: string, frame: string) => {
  if (process.env.FURB_QA) writeFileSync(`${process.env.FURB_QA}/${name}.txt`, frame);
};

/** The part of each row of a frame that the sidebar shows. */
const rail = (session: Composing["session"], frame: string) =>
  frame.split("\n").map((line) => line.slice(size.width - session.preferences.sidebarWidth));

/** A click on the first row of the sidebar that shows a text. */
async function railClick({ session, screen, frame }: Composing, text: string): Promise<void> {
  const left = size.width - session.preferences.sidebarWidth;
  const rows = rail(session, await frame());
  expect(rows.join("\n")).toContain(text);
  const y = rows.findIndex((line) => line.includes(text));
  await screen.mockMouse.click(left + (rows[y] ?? "").indexOf(text) + 1, y);
  await frame();
}

/** The threads of the chain shown, as the sidebar lists them. */
const listed = (session: Composing["session"]) =>
  chainThreads(session.ladderIndex.on, session.ladderIndex.ids, new Set(session.host.prompts.keys()));

test("the chain switcher stays visible while the selected chain thread list stays in the rail", () =>
  composing(
    async (use) => {
      const { frame } = use;
      await idle(use.session);
      const shown = await frame();
      keep("navigator", shown);
      expect(shown).toContain("⌃B switch");
      expect(shown).toContain("Threads");
      expect(shown).toContain("Find threads");
      await railClick(use, "⌃B switch");
      const picker = await frame();
      expect(picker).toContain("Chains");
      expect(picker).toContain("Main");
      use.screen.mockInput.pressKey("escape");
      const back = await frame();
      expect(back).toContain("⌃B switch");
      expect(back).toContain("Threads");
    },
    size,
    true,
  ));

test("inspecting a thread does not retarget the composer; explicit Reply here does", () =>
  composing(
    async (use) => {
      const { session, frame } = use;
      await idle(session);
      const before = session.thread;
      await railClick(use, "Explore this project");
      const inspectedThread = listed(session).find((one) => one.title.includes("Explore this project"));
      expect(session.thread).toBe(before);
      const inspected = await frame();
      expect(inspected).toContain("Reopen and reply");
      await railClick(use, "Reopen and reply");
      expect(session.thread).toBe(inspectedThread?.act.id);
      expect(await frame()).toContain("reply");
    },
    size,
    true,
  ));

test("a thread can be explicitly targeted, safely resolved, and reopened", () =>
  composing(
    async (use) => {
      const { session, frame } = use;
      await idle(session);
      const root = session.engine.root;
      await session.engine.pause(root);
      const id = await session.engine.prompt("str", { message: "Hold this thread open", on: root });
      await session.refresh();
      await until(session, () => listed(session).some((one) => one.act.id === id));
      await railClick(use, "Hold this thread open");
      expect(session.thread).not.toBe(id);
      await railClick(use, "Reply here");
      expect(session.thread).toBe(id);
      await railClick(use, "New thread");
      expect(session.thread).toBeUndefined();
      await railClick(use, "Resolve");
      use.screen.mockInput.pressArrow("down");
      use.screen.mockInput.pressEnter();
      await until(session, () => listed(session).find((one) => one.act.id === id)?.resolved === true);
      expect(await session.engine.result(id)).toBe("resolved");
      expect(await frame()).toContain("Reopen and reply");
      await railClick(use, "Reopen and reply");
      expect(session.thread).toBe(id);
    },
    size,
    true,
  ));

test("Needs-you is global, one row per question, without duplicate choice chips", () =>
  composing(
    async ({ session, frame }) => {
      await idle(session);
      const question = await session.engine.prompt("bool", {
        message: "Apply the search shortcut?",
        on: session.engine.root,
        to: "operator",
      });
      await until(session.host, () => session.host.prompts.has(question));
      await session.refresh();
      const shown = await frame();
      keep("needs-you", shown);
      const rows = rail(session, shown);
      const needs = rows.findIndex((line) => line.includes("Needs you"));
      expect(needs).toBeGreaterThan(-1);
      expect(rows[needs + 1]).toContain("Apply the search shortcut?");
      expect(rows[needs + 1]).toContain("Main");
      expect(rows.filter((line) => line.includes("Apply the search shortcut?")).length).toBe(1);
      expect(rows.join(" ")).not.toContain("1 yes");
      expect(rows.join(" ")).toContain("Threads");
      expect(rows.join(" ")).toContain("Find threads");
    },
    size,
    true,
  ));

test("a string too long for its line folds to its start and an underlined ..., which a hover shows whole and a click opens in place", () =>
  composing(
    async ({ session, screen, frame }) => {
      const long = `${"word ".repeat(48)}ENDMARK`;
      const rung = await session.engine.rung({ word: `brief = "${long}"`, on: session.engine.root });
      await session.engine.result(rung);
      await session.refresh();
      const folded = await frame();
      keep("fold", folded);
      const rows = folded.split("\n");
      const y = rows.findIndex((row) => row.includes('brief = "word'));
      expect(y).toBeGreaterThanOrEqual(0);
      expect(rows[y]).toContain('..."');
      expect(folded).not.toContain("ENDMARK");
      const x = (rows[y] ?? "").indexOf('..."') + 1;
      await screen.mockMouse.moveTo(x, y);
      await until(screen.renderer, () => screen.captureCharFrame().includes("ENDMARK"), "frame");
      await screen.mockMouse.click(x, y);
      await until(session, () => session.folds[`string:${rung}:1:0`] === false);
      const opened = await frame();
      keep("fold-open", opened);
      expect(opened.split("\n")[y]).toContain('brief = "word');
      expect(opened.split("\n").filter((row) => row.includes("word word")).length).toBeGreaterThan(1);
      expect(
        opened
          .split("\n")
          .filter((row) => row.includes("word word"))
          .some((row) => row.includes('..."')),
      ).toBe(false);
    },
    { ...size, useMouse: true },
    true,
  ));

test("a line cut to its room and a folded string keep the colors of their whole statement", () =>
  composing(
    async ({ session, screen, frame }) => {
      const names = Array.from({ length: 16 }, (_, k) => `"n${k}xyz"`).join(", ");
      const word = [
        `names = [${names}]`,
        `brief = "${"word ".repeat(40)}end"`,
        "def shout():",
        "  return names",
      ].join("\n");
      const rung = await session.engine.rung({ word, on: session.engine.root });
      await session.engine.result(rung);
      await session.refresh();
      const rows = (await frame()).split("\n");
      const cut = rows.findIndex((row) => row.includes("names = ["));
      const fold = rows.findIndex((row) => row.includes('brief = "word'));
      const def = rows.findIndex((row) => row.includes("def shout"));
      expect(rows[cut]).toContain("…");
      expect(rows[cut]).not.toContain("]");
      expect(rows[fold]).toContain('..."');
      const palette = palettes.ink;
      const fg = (x: number, y: number) => cellAt(screen.captureSpans(), x, y).fg;
      const keyword = RGBA.fromHex(palette.syntaxKeyword);
      const x = (rows[def] ?? "").indexOf("def");
      await until(screen.renderer, () => Boolean(fg(x, def)?.equals(keyword)), "frame");
      expect(fg((rows[cut] ?? "").indexOf("n0xyz"), cut)?.equals(RGBA.fromHex(palette.syntaxString))).toBe(
        true,
      );
      // The closing mark of the folded string, after its underlined ..., is a string.
      expect(
        fg((rows[fold] ?? "").indexOf('..."') + 3, fold)?.equals(RGBA.fromHex(palette.syntaxString)),
      ).toBe(true);
      expect(fg((rows[def + 1] ?? "").indexOf("return"), def + 1)?.equals(keyword)).toBe(true);
    },
    size,
    true,
  ));

for (const width of [80, 160]) {
  test(`all repeated task subjects remain reachable by id at ${width} columns`, () =>
    composing(
      async ({ session, app, screen, frame }) => {
        const root = session.engine.root;
        await session.engine.pause(root);
        const ids: string[] = [];
        for (let index = 0; index < 25; index++)
          ids.push(
            await session.engine.prompt("str", {
              message: "Review the repeated task subject with all of its details intact",
              on: root,
            }),
          );
        await session.refresh();
        app.composer.setText("unsent draft");
        const target = session.thread;
        app.details();
        await frame();
        await screen.mockInput.typeText("Find threads");
        screen.mockInput.pressEnter();
        await frame();
        expect(await frame()).toContain("Threads");
        await screen.mockInput.typeText(ids.at(-1) ?? "");
        expect(await frame()).toContain(ids.at(-1) ?? "");
        screen.mockInput.pressEnter();
        await frame();
        expect(session.thread).toBe(target);
        expect(app.composer.plainText).toBe("unsent draft");
        const last = ids.at(-1);
        const node = app.scroll.getChildren().find((one) => one.id === last);
        expect(node).toBeDefined();
        if (!node) throw new Error("No selected thread in the feed.");
        expect(node.y).toBeGreaterThanOrEqual(app.scroll.viewport.y);
        expect(node.y).toBeLessThan(app.scroll.viewport.y + app.scroll.viewport.height);
        if (width >= 100) {
          screen.mockInput.pressKey("t", { meta: true });
          await frame();
          screen.mockInput.pressArrow("up");
          const moved = await frame();
          expect(moved).toContain("Reply here");
          screen.mockInput.pressKey("t");
          await frame();
          expect(session.thread).toBe(ids.at(-2));
        } else {
          screen.mockInput.pressKey("t", { meta: true });
          expect(await frame()).toContain("Threads");
          await screen.mockInput.typeText(ids.at(-2) ?? "");
          screen.mockInput.pressArrow("right");
          expect(await frame()).toContain("Reply here");
          await screen.mockInput.typeText("Reply here");
          screen.mockInput.pressEnter();
          await frame();
          expect(session.thread).toBe(ids.at(-2));
          expect(app.composer.plainText).toBe("unsent draft");
          screen.mockInput.pressKey("t", { meta: true });
          await frame();
          await screen.mockInput.typeText(ids.at(-2) ?? "");
          screen.mockInput.pressArrow("right");
          await frame();
          await screen.mockInput.typeText("New thread");
          screen.mockInput.pressEnter();
          await frame();
          expect(session.thread).toBeUndefined();
          expect(app.composer.plainText).toBe("unsent draft");
        }
      },
      { width, height: 36, kittyKeyboard: true },
    ));
}

test("a live source selection and its syntax stay in place as elapsed time changes", () =>
  composing(
    async ({ session, app, screen, frame }) => {
      session.preferences.reducedMotion = true;
      const rung = await session.engine.rung({
        word: 'value = "steady source"\nawait wait(60)',
        on: session.engine.root,
      });
      await session.refresh();
      await frame();
      await app.colorsDone();
      const initial = await frame();
      const rows = initial.split("\n");
      const y = rows.findIndex((row) => row.includes('value = "steady source"'));
      const x = (rows[y] ?? "").indexOf("steady source");
      expect(y).toBeGreaterThanOrEqual(0);
      await until(
        screen.renderer,
        () =>
          Boolean(cellAt(screen.captureSpans(), x, y).fg?.equals(RGBA.fromHex(palettes.ink.syntaxString))),
        "frame",
      );
      await screen.mockMouse.drag(x, y, x + 5, y);
      const selected = screen.renderer.getSelection()?.getSelectedText();
      expect(selected).toBe("steady");
      const source = app.scroll.getChildren().find((node) => node.id === `program-${rung}`);
      const wait = session.acts.find((act) => act.kind === "wait" && act.by === rung);
      if (!wait) throw new Error("No pending wait.");
      const top = app.scroll.scrollTop;
      const focused = app.composer.focused;
      session.started[wait.id] = Date.now() - 4000;
      await frame();
      expect(app.scroll.getChildren().find((node) => node.id === `program-${rung}`) === source).toBe(true);
      expect(screen.renderer.getSelection()?.getSelectedText()).toBe(selected);
      expect(app.scroll.scrollTop).toBe(top);
      expect(app.composer.focused).toBe(focused);
      expect(
        cellAt(screen.captureSpans(), x + 8, y).fg?.equals(RGBA.fromHex(palettes.ink.syntaxString)),
      ).toBe(true);
      await session.engine.close(null, { id: wait.id });
      await session.engine.result(rung);
    },
    { width: 140, height: 42, useMouse: true },
  ));

test("source selection survives command output while the pointer, focus and scroll stay still", () =>
  composing(
    async ({ session, app, screen, frame }) => {
      session.preferences.reducedMotion = true;
      const rung = await session.engine.rung({
        word: 'value = "steady output"\nawait bash("printf first; sleep 0.5; printf second; sleep 0.5; printf third")',
        on: session.engine.root,
      });
      await session.refresh();
      await frame();
      await app.colorsDone();
      const rows = (await frame()).split("\n");
      const y = rows.findIndex((row) => row.includes('value = "steady output"'));
      const x = (rows[y] ?? "").indexOf("steady output");
      await screen.mockMouse.drag(x, y, x + 5, y);
      const source = app.scroll.getChildren().find((node) => node.id === `program-${rung}`);
      const top = app.scroll.scrollTop;
      const focused = app.composer.focused;
      await session.engine.result(rung);
      await session.refresh();
      await frame();
      expect(app.scroll.getChildren().find((node) => node.id === `program-${rung}`) === source).toBe(true);
      expect(screen.renderer.getSelection()?.getSelectedText()).toBe("steady");
      expect(app.scroll.scrollTop).toBe(top);
      expect(app.composer.focused).toBe(focused);
    },
    { width: 140, height: 42, useMouse: true },
  ));

/** A file followed by the real reader and renderer. Its words are inert facts, never a model or Python execution. */
async function sourceRecord(
  initial: unknown[][],
  use: (context: Composing, facts: unknown[][], save: () => Promise<void>) => Promise<void>,
): Promise<void> {
  const directory = await mkdtemp(join(tmpdir(), "furb-source-"));
  const record = join(directory, "source.jsonl");
  const facts: unknown[][] = [
    ["chain", "chain1", "operator", "", "Live source", ""],
    ["stand", "stand1", "operator", "chain1"],
    [
      "done",
      "stand1",
      "",
      [
        [
          ["codex:gpt-6.1-sol", ["high"], 475000],
          ["operator", [], 200000],
        ],
        directory,
        "codex:gpt-6.1-sol/high",
      ],
    ],
    ["prompt", "prompt1", "operator", "chain1", "str", "Keep the source alive", "codex:gpt-6.1-sol/high"],
    ...initial,
  ];
  const save = () =>
    writeFile(
      record,
      `${JSON.stringify({ recorded: true })}\n${facts.map((fact) => JSON.stringify(fact)).join("\n")}\n`,
    );
  try {
    await save();
    await composing(
      (context) => use(context, facts, save),
      { width: 140, height: 42, useMouse: true },
      await demoSession({ record, cwd: directory }),
    );
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

test("appending display line 100 widens the same source gutter without moving selection or losing original coordinates", async () => {
  const word = Array.from({ length: 99 }, (_, index) => `line_${index + 1} = ${index + 1}`).join("\n");
  await sourceRecord(
    [
      ["rung", "rung1", "prompt1", "chain1", word, "", ""],
      ["run", "run1", "chain1", "chain1", "rung1", word, ""],
      ["done", "run1", "", null],
      ["done", "rung1", "", null],
    ],
    async ({ session, app, screen, frame }, facts, save) => {
      session.preferences.reducedMotion = true;
      await frame();
      await app.colorsDone();
      app.scroll.scrollTo(60);
      const rows = (await frame()).split("\n");
      const y = rows.findIndex((row) => row.includes("87 line_87"));
      const x = (rows[y] ?? "").indexOf("line_87");
      expect(y).toBeGreaterThan(0);
      await screen.mockMouse.drag(x, y, x + 6, y);
      expect(screen.renderer.getSelection()?.getSelectedText()).toBe("line_87");
      const card = app.scroll.getChildren().find((node) => node.id === "program-prompt1");
      const source = card?.getChildren().find((node) => node instanceof LineNumberRenderable);
      const target = source?.getChildren().find((node) => node instanceof TextRenderable);
      const top = app.scroll.scrollTop;
      const focused = app.composer.focused;
      facts.push(
        ["rung", "rung2", "prompt1", "chain1", "line_100 = 100", "", ""],
        ["run", "run2", "chain1", "chain1", "rung2", "line_100 = 100", ""],
        ["done", "run2", "", null],
        ["done", "rung2", "", null],
      );
      await save();
      await until(session, () => session.acts.some((act) => act.id === "rung2"));
      await frame();
      await app.colorsDone();
      expect(app.scroll.getChildren().find((node) => node.id === "program-prompt1")).toBe(card);
      expect(card?.getChildren().find((node) => node instanceof LineNumberRenderable)).toBe(source);
      expect(source?.getChildren().find((node) => node instanceof TextRenderable)).toBe(target);
      expect(screen.renderer.getSelection()?.getSelectedText()).toBe("line_87");
      expect(app.composer.focused).toBe(focused);
      expect(app.scroll.scrollTop).toBe(top);
      expect(source instanceof LineNumberRenderable && source.getLineNumbers().get(99)).toBe(100);
      app.scroll.scrollTo(99999);
      const after = (await frame()).split("\n");
      const last = after.find((row) => row.includes("100 line_100"));
      const previous = after.find((row) => row.includes("99 line_99"));
      expect(last).toBeDefined();
      expect(previous?.indexOf("line_99")).toBe(last?.indexOf("line_100"));
      expect(last?.indexOf("line_100")).toBe(x + 1);
    },
  );
});

test("a completed statement is colored while tokens stream and the full word keeps the same source, colors, selection and focus", async () => {
  await sourceRecord(
    [["rung", "rung1", "prompt1", "chain1", "", "", ""]],
    async ({ session, app, screen, frame }, facts, save) => {
      session.preferences.reducedMotion = true;
      const actual = session.host.streams;
      const held = new Map([["rung1", { text: 'value = "steady source"\nif Tr', thinking: "" }]]);
      Object.defineProperty(session.host, "streams", { configurable: true, get: () => held, set: () => {} });
      try {
        await frame();
        await app.colorsDone();
        const rows = (await frame()).split("\n");
        const y = rows.findIndex((row) => row.includes('value = "steady source"'));
        const x = (rows[y] ?? "").indexOf("steady source");
        const sourceColumn = (rows[y] ?? "").indexOf("value");
        expect(y).toBeGreaterThan(0);
        expect(rows[y + 1]).toContain("2 if Tr");
        expect(rows[y + 2]).toContain("3 # thinking");
        expect(cellAt(screen.captureSpans(), x, y).fg?.equals(RGBA.fromHex(palettes.ink.syntaxString))).toBe(
          true,
        );
        await screen.mockMouse.drag(x, y, x + 5, y);
        expect(screen.renderer.getSelection()?.getSelectedText()).toBe("steady");
        const card = app.scroll.getChildren().find((node) => node.id === "program-prompt1");
        const source = card?.getChildren().find((node) => node instanceof LineNumberRenderable);
        const target = source?.getChildren().find((node) => node instanceof TextRenderable);
        const top = app.scroll.scrollTop;
        const focused = app.composer.focused;
        const word = 'value = "steady source"\nif True:\n    answer = value';
        held.set("rung1", { text: word, thinking: "Keep the current statement" });
        await frame();
        await app.colorsDone();
        expect(screen.renderer.getSelection()?.getSelectedText()).toBe("steady");
        expect(
          cellAt(screen.captureSpans(), sourceColumn, y + 1).fg?.equals(
            RGBA.fromHex(palettes.ink.syntaxKeyword),
          ),
        ).toBe(true);
        // Stream removal precedes the full-word fact. No empty frame may erase the source or its selection.
        held.clear();
        await frame();
        await app.colorsDone();
        expect(screen.captureCharFrame()).toContain("answer = value");
        expect(screen.renderer.getSelection()?.getSelectedText()).toBe("steady");
        facts.push(
          ["run", "run1", "chain1", "chain1", "rung1", word, ""],
          ["done", "run1", "", null],
          ["done", "rung1", "", null],
        );
        await save();
        await until(session, () => session.acts.some((act) => act.id === "rung1" && act.done));
        await frame();
        await app.colorsDone();
        expect(app.scroll.getChildren().find((node) => node.id === "program-prompt1")).toBe(card);
        expect(card?.getChildren().find((node) => node instanceof LineNumberRenderable)).toBe(source);
        expect(source?.getChildren().find((node) => node instanceof TextRenderable)).toBe(target);
        expect(screen.renderer.getSelection()?.getSelectedText()).toBe("steady");
        expect(app.composer.focused).toBe(focused);
        expect(app.scroll.scrollTop).toBe(top);
        expect(
          cellAt(screen.captureSpans(), x + 7, y).fg?.equals(RGBA.fromHex(palettes.ink.syntaxString)),
        ).toBe(true);
        expect(screen.captureCharFrame()).not.toContain("# thinking");
        expect(screen.captureCharFrame()).toContain("answer = value");
      } finally {
        Object.defineProperty(session.host, "streams", { configurable: true, writable: true, value: actual });
      }
    },
  );
});

test("CJK, emoji and combining characters before an expanded literal refold on its second click at terminal-cell coordinates", () =>
  composing(
    async ({ session, screen, frame }) => {
      // This checks the literal indices and terminal-cell coordinates of the ORIGINAL semicolon line.
      // Ruff may split that statement into two lines; its projected fold behavior has its own renderer fixture.
      session.preferences.rawPython = true;
      const value = "漢字 👩‍💻 e\u0301 ".repeat(26);
      const rung = await session.engine.rung({
        word: `prefix = "漢字👩‍💻e\u0301"; brief = ${JSON.stringify(value)}`,
        on: session.engine.root,
      });
      await session.engine.result(rung);
      await session.refresh();
      const rows = (await frame()).split("\n");
      const y = rows.findIndex((row) => row.includes("prefix = "));
      const before = (rows[y] ?? "").split("...")[0] ?? "";
      expect(rows[y]).toContain("...");
      await screen.mockMouse.click(Bun.stringWidth(before) + 1, y);
      expect(session.folds[`string:${rung}:1:1`]).toBe(false);
      const opened = (await frame()).split("\n");
      const row = opened[y] ?? "";
      const at = row.indexOf("brief = ") + "brief = ".length + 2;
      await screen.mockMouse.click(Bun.stringWidth(row.slice(0, at)), y);
      await frame();
      expect(session.folds[`string:${rung}:1:1`]).not.toBe(false);
      expect(screen.captureCharFrame().split("\n")[y]).toContain("...");
    },
    { width: 140, height: 42, useMouse: true },
  ));

import { expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { RGBA } from "@opentui/core";
import { chainThreads } from "furb/life";
import { until } from "../../bind/typescript/test/until.ts";
import { glyph, palettes } from "../src/theme.ts";
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

/** Whether the sidebar shows the tree of the chains, whose heading is Chains, and not the list of the threads of one. */
const tree = (session: Composing["session"], frame: string) =>
  rail(session, frame).some((line) => line.trim().startsWith("Chains "));

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

test("a click on a chain lists the threads of that chain in place of the tree, and ← Chains gives the tree back", () =>
  composing(
    async (use) => {
      const { session, frame } = use;
      await idle(session);
      const shown = await frame();
      keep("tree", shown);
      expect(tree(session, shown)).toBe(true);
      await railClick(use, "Main");
      await until(session, () => session.selected === session.engine.root);
      const list = await frame();
      keep("list", list);
      expect(list).toContain("← Chains");
      expect(tree(session, list)).toBe(false);
      const first = listed(session)[0];
      expect(first).toBeDefined();
      expect(rail(session, list).join("\n")).toContain((first?.title ?? "").slice(0, 12));
      await railClick(use, "← Chains");
      expect(tree(session, await frame())).toBe(true);
    },
    size,
    true,
  ));

test("after ⌥T the keys open the threads of a chain, Space collapses a thread in the feed to its header line, and ← goes back", () =>
  composing(
    async (use) => {
      const { session, screen, frame } = use;
      await idle(session);
      await frame();
      screen.mockInput.pressKey("t", { meta: true });
      await frame();
      screen.mockInput.pressEnter();
      await until(session, () => session.selected === session.engine.root);
      expect(await frame()).toContain("← Chains");
      const last = listed(session).at(-1);
      expect(last).toBeDefined();
      const id = last?.act.id ?? "";
      screen.mockInput.pressKey(" ");
      await frame();
      expect(session.folds[`thread:${id}`]).toBe(true);
      const folded = await frame();
      keep("collapsed", folded);
      expect(folded).toContain(`${glyph.closed} Explore this project`);
      screen.mockInput.pressKey(" ");
      await frame();
      expect(session.folds[`thread:${id}`]).toBe(false);
      // The feed lays the thread out again over a few frames, and its folded line goes.
      let shown = await frame();
      for (let step = 0; step < 40 && shown.includes(`${glyph.closed} Explore this project`); step++) {
        await Bun.sleep(25);
        shown = await frame();
      }
      expect(shown).not.toContain(`${glyph.closed} Explore this project`);
      expect(shown).toContain("Collapse");
      screen.mockInput.pressArrow("left");
      expect(tree(session, await frame())).toBe(true);
    },
    size,
    true,
  ));

test("a thread resolves from its row, and a message to it opens it again", () =>
  composing(
    async (use) => {
      const { session, frame } = use;
      await idle(session);
      const root = session.engine.root;
      await session.engine.pause(root);
      const id = await session.engine.prompt("str", { message: "Hold this thread open", on: root });
      await session.refresh();
      await railClick(use, "Main");
      await until(session, () => listed(session).some((one) => one.act.id === id));
      await railClick(use, "Hold this thread open");
      expect(session.thread).toBe(id);
      const open = await frame();
      keep("chosen", open);
      await railClick(use, " Resolve ");
      await until(session, () => listed(session).find((one) => one.act.id === id)?.resolved === true);
      expect(await session.engine.result(id)).toBe("resolved");
      expect(await frame()).toContain(" Open again ");
      await session.submit("Look at it once more");
      await until(session, () => listed(session).find((one) => one.act.id === id)?.resolved === false);
      keep("reopened", await frame());
    },
    size,
    true,
  ));

test("a question that waits stands in Needs you over the tree, and stays over the list of the threads of its chain", () =>
  composing(
    async (use) => {
      const { session, frame } = use;
      await idle(session);
      const question = await session.engine.prompt("bool", {
        message: "Apply the search shortcut?",
        on: session.engine.root,
        to: "operator",
      });
      await until(session.host, () => session.host.prompts.has(question));
      await session.refresh();
      const shown = await frame();
      keep("question", shown);
      const rows = rail(session, shown);
      const needs = rows.findIndex((line) => line.includes("Needs you"));
      expect(needs).toBeGreaterThan(-1);
      expect(rows.findIndex((line) => line.trim().startsWith("Chains "))).toBeGreaterThan(needs);
      await railClick(use, "Main     asks");
      const list = await frame();
      keep("question-list", list);
      expect(rail(session, list).join("\n")).toContain("Needs you");
      expect(list).toContain("← Chains");
      expect(listed(session).some((one) => one.act.id === question)).toBe(false);
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

test("a hover on the line of a read shows the text that it read, and on a line that the gate refused shows its error", () =>
  composing(
    async ({ session, screen, frame }) => {
      const root = session.engine.root;
      const wrote = await session.engine.rung({
        word: 'write(Text("hover.txt", "HOVER" + "TEXT first\\nsecond"))\nseen = read("hover.txt")',
        on: root,
      });
      await session.engine.result(wrote);
      await session.refresh();
      const shown = await frame();
      const rows = shown.split("\n");
      const y = rows.findIndex((row) => row.includes('seen = read("hover.txt")'));
      expect(y).toBeGreaterThanOrEqual(0);
      expect(shown).not.toContain("HOVERTEXT");
      await screen.mockMouse.moveTo((rows[y] ?? "").indexOf("seen") + 1, y);
      await until(screen.renderer, () => screen.captureCharFrame().includes("HOVERTEXT first"), "frame");
      keep("hover-read", screen.captureCharFrame());
      await screen.mockMouse.moveTo(1, 1);
      const refused = await session.engine.rung({ word: "ok = 1\nbad = = 2", on: root });
      await session.engine.result(refused).catch(() => undefined);
      await session.refresh();
      const failed = await frame();
      keep("gate", failed);
      const lines = failed.split("\n");
      const row = lines.findIndex((line) => line.includes("bad = = 2"));
      expect(row).toBeGreaterThanOrEqual(0);
      expect(lines[row]).toContain(glyph.failed);
      await screen.mockMouse.moveTo((lines[row] ?? "").indexOf("bad") + 1, row);
      await until(
        screen.renderer,
        () => screen.captureCharFrame().includes("The gate refused line 2"),
        "frame",
      );
      keep("hover-gate", screen.captureCharFrame());
    },
    { ...size, useMouse: true },
    true,
  ));

test("a line that awaits a command lights the line that made it, names it, and shows its time after a second", () =>
  composing(
    async ({ session, frame }) => {
      const rung = await session.engine.rung({
        word: 'build = bash("sleep 30")\nx = 1\nb = await build',
        on: session.engine.root,
      });
      // The time of a line comes after a second of its wait, so the frames are read until it shows.
      let rows: string[] = [];
      const ready = () => {
        const waits = rows.find((row) => row.includes("b = await build")) ?? "";
        return /waits on bash\d+ of line 1/.test(waits) && /\b\d+s\b/.test(waits);
      };
      for (let step = 0; step < 160 && !ready(); step++) {
        await session.refresh();
        rows = (await frame()).split("\n");
        if (!ready()) await Bun.sleep(50);
      }
      keep("awaits", rows.join("\n"));
      expect(ready()).toBe(true);
      expect(rows.find((row) => row.includes("b = await build"))).toContain(glyph.running);
      expect(rows.find((row) => row.includes('build = bash("sleep 30")'))).toContain(glyph.awaited);
      await session.engine.cancel(rung);
    },
    size,
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

test("the Context part of the sidebar holds three lines: its share, its meter and its numbers, and no row of tokens under them", () =>
  composing(
    async (use) => {
      const { session, frame } = use;
      await idle(session);
      const rows = rail(session, await frame()).map((line) => line.trim());
      const at = rows.findIndex((line) => line.startsWith("Context"));
      expect(at).toBeGreaterThan(0);
      expect(rows[at + 1]).toMatch(/^━+$/);
      expect(rows[at + 2]).toContain("prompt");
      expect(rows.slice(at + 3).join("\n")).not.toMatch(/Fresh input|Cache write|Output|Last prompt/);
    },
    size,
    true,
  ));

test("a string of three marks over many lines shows as one string with its escapes, folded to its start and an underlined ..., and its statement keeps its colors", () =>
  composing(
    async ({ session, screen, frame }) => {
      const word = `note = """${"alpha beta ".repeat(10)}\n${"gamma delta ".repeat(10)}ENDMARK"""\nprint(len(note))`;
      const rung = await session.engine.rung({ word, on: session.engine.root });
      await session.engine.result(rung);
      await session.refresh();
      const folded = await frame();
      keep("multiline", folded);
      const rows = folded.split("\n");
      const y = rows.findIndex((row) => row.includes('note = "alpha beta'));
      expect(y).toBeGreaterThanOrEqual(0);
      expect(rows[y]).toContain('..."');
      expect(folded).not.toContain("ENDMARK");
      expect(folded).not.toContain('"""');
      expect(rows[y + 1]).toContain("print(len(note))");
      const fg = (x: number, row: number) => cellAt(screen.captureSpans(), x, row).fg;
      const string = RGBA.fromHex(palettes.ink.syntaxString);
      // The closing mark of the string, after its underlined ..., is a string of the statement, as it is whole.
      const x = (rows[y] ?? "").indexOf('..."');
      await until(screen.renderer, () => Boolean(fg(x + 3, y)?.equals(string)), "frame");
      await screen.mockMouse.moveTo(x + 1, y);
      await until(screen.renderer, () => screen.captureCharFrame().includes("ENDMARK"), "frame");
      await screen.mockMouse.click(x + 1, y);
      await until(session, () => session.folds[`string:${rung}:1:0`] === false);
      const opened = await frame();
      keep("multiline-open", opened);
      expect(opened).toContain("ENDMARK");
      const shownRows = opened.split("\n");
      const first = shownRows.findIndex((row) => row.includes('note = "alpha beta'));
      const end = shownRows.findIndex((row) => row.includes("ENDMARK"));
      expect(end).toBeGreaterThan(first);
      expect(shownRows.slice(first, end + 1).some((row) => row.includes('..."'))).toBe(false);
      expect(shownRows.slice(first, end + 1).join("")).toContain("\\ngamma");
    },
    { ...size, useMouse: true },
    true,
  ));

test("a quote of a word shows as the string that it binds: its name, an equal sign and one string with its escapes", () =>
  composing(
    async ({ session, screen, frame }) => {
      const body = `Fix the largest gap.\nKeep the rest for later passes, and say in one sentence what each gap holds, so that a worker can act on it. ${"More words. ".repeat(8)}`;
      const word = `<s:brief>\n${body}\n</s:brief>\nprint(brief)`;
      const rung = await session.engine.rung({ word, on: session.engine.root });
      await session.engine.result(rung);
      await session.refresh();
      const shown = await frame();
      keep("quote", shown);
      const rows = shown.split("\n");
      const y = rows.findIndex((row) => row.includes("brief = 'Fix the largest gap."));
      expect(y).toBeGreaterThanOrEqual(0);
      expect(rows[y]).toContain("\\n");
      expect(rows[y]).toContain("...'");
      expect(shown).not.toContain("<s:brief>");
      expect(rows[y + 1]).toContain("print(brief)");
      const fg = (x: number, row: number) => cellAt(screen.captureSpans(), x, row).fg;
      const x = (rows[y] ?? "").indexOf("Fix");
      await until(
        screen.renderer,
        () => Boolean(fg(x, y)?.equals(RGBA.fromHex(palettes.ink.syntaxString))),
        "frame",
      );
    },
    size,
    true,
  ));

test("a line that opens wraps at tokens within its column, and each row keeps the colors of its whole statement", () =>
  composing(
    async ({ session, screen, frame }) => {
      const names = Array.from({ length: 24 }, (_, k) => `"n${k}xyz"`).join(", ");
      const rung = await session.engine.rung({ word: `names = [${names}]`, on: session.engine.root });
      await session.engine.result(rung);
      await session.refresh();
      const shut = (await frame()).split("\n");
      const y = shut.findIndex((row) => row.includes("names = ["));
      expect(shut[y]).toContain("…");
      await screen.mockMouse.click((shut[y] ?? "").indexOf("names"), y);
      await until(session, () => session.folds[`line:${rung}:1`] === false);
      const opened = await frame();
      keep("wrapped", opened);
      const rows = opened.split("\n");
      expect(rows[y]).toContain("names = [");
      // Every name is whole on some row, and no row cuts one.
      for (let k = 0; k < 24; k++) expect(opened).toContain(`"n${k}xyz"`);
      expect(opened).not.toMatch(/"n\d+(?!\d|xyz)/);
      expect(opened).not.toMatch(/(?<!n\d{0,2})xyz"/);
      const last = rows.findIndex((row) => row.includes('"n23xyz"'));
      expect(last).toBeGreaterThan(y);
      expect(rows[last]).not.toContain("names = [");
      const fg = (x: number, row: number) => cellAt(screen.captureSpans(), x, row).fg;
      const string = RGBA.fromHex(palettes.ink.syntaxString);
      const at = (rows[last] ?? "").indexOf('"n23xyz"') + 2;
      await until(screen.renderer, () => Boolean(fg(at, last)?.equals(string)), "frame");
    },
    { ...size, useMouse: true },
    true,
  ));

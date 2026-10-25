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

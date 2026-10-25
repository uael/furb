import { expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { RGBA } from "@opentui/core";
import { chainThreads } from "furb/life";
import { until } from "../../bind/typescript/test/until.ts";
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
        }
      },
      { width, height: 36, kittyKeyboard: true },
    ));
}

import { expect, test } from "bun:test";
import { join } from "node:path";
import type { ProgramLine } from "furb/life";
import { chainThreads } from "furb/life";
import { until } from "../../bind/typescript/test/until.ts";
import { openEngine } from "../src/bridge.ts";
import { Session } from "../src/session.ts";
import { composing } from "./composing.ts";
import { idle } from "./idle.ts";

const size = { width: 152, height: 46 };

/** The part of a row of a frame that the feed draws, less the sidebar. */
const feedOf = (session: Session, row: string) => row.slice(0, size.width - session.preferences.sidebarWidth);

/** The threads of the chain shown. */
const listed = (session: Session) =>
  chainThreads(session.ladderIndex.on, session.ladderIndex.ids, new Set(session.host.prompts.keys()));

/** A session on the recorded life of a conductor, which shows its log and runs nothing. */
async function recorded(): Promise<Session> {
  const { engine, host } = await openEngine({
    record: join(import.meta.dir, "..", "fixtures", "conductor.jsonl"),
  });
  return new Session(engine, host, true);
}

test("a thread is one box: its message with a line of its color, its program as code, its answer as markdown with a quiet line, and an input at its end in place of Open again", () =>
  composing(
    async ({ session, app, frame }) => {
      await idle(session);
      let rows = (await frame()).split("\n").map((row) => feedOf(session, row));
      // The markdown of the box draws over a few frames.
      for (
        let step = 0;
        step < 80 && !rows.some((row) => row.includes("Fieldnotes keeps ideas close"));
        step++
      ) {
        await Bun.sleep(25);
        rows = (await frame()).split("\n").map((row) => feedOf(session, row));
      }
      const at = (text: string) => rows.findIndex((row) => row.includes(text));
      // The sections of the box are cards that share one panel, in the order of the box.
      const ids = app.scroll.getChildren().map((node) => node.id);
      const head = ids.find((id) => /^prompt\d+$/.test(id)) ?? "";
      expect(ids.slice(ids.indexOf(head), ids.indexOf(head) + 4)).toEqual([
        head,
        `program-${head}`,
        `${head}-answer`,
        `${head}-reply`,
      ]);
      expect(rows[at("Explore this project")]).toMatch(/┃ Explore this project/);
      // The answer stands as markdown in the box, and the quiet line of the box runs through it.
      expect(rows.some((row) => /┃ (## )?A clear starting point/.test(row))).toBe(true);
      expect(at("Fieldnotes keeps ideas close")).toBeGreaterThan(0);
      // The box has an edge above its message and an edge under its input.
      expect(rows[at("Explore this project") - 1]).toContain("▄▄▄▄");
      expect(rows[at("Reply to open this thread again") + 1]).toContain("▀▀▀▀");
      expect(rows.join("\n")).not.toContain("Open again");
      expect(rows.join("\n")).toContain("▾ Collapse");
    },
    size,
    true,
  ));

test("an input at the end of a thread sends its text to the thread and opens it again, and Escape gives the keys back to the input of the chain", () =>
  composing(
    async ({ session, app, screen, click, frame }) => {
      await idle(session);
      const id = listed(session)[0]?.act.id ?? "";
      expect(listed(session)[0]?.resolved).toBe(true);
      await click("Reply to open this thread again");
      expect(app.replyFocus).toBe(id);
      expect(app.composer.focused).toBe(false);
      await screen.mockInput.typeText("Look at it once more");
      await frame();
      expect(session.drafts[`reply:${id}`]).toBe("Look at it once more");
      expect(await frame()).toContain("Look at it once more");
      screen.mockInput.pressEnter();
      await until(session, () => (listed(session)[0]?.messages.length ?? 0) === 1);
      expect(session.drafts[`reply:${id}`]).toBeUndefined();
      expect(app.replyFocus).toBeUndefined();
      expect(app.composer.focused).toBe(true);
      await click("Reply to");
      expect(app.replyFocus).toBe(id);
      screen.mockInput.pressEscape();
      for (let step = 0; step < 40 && app.replyFocus; step++) {
        await Bun.sleep(25);
        await frame();
      }
      expect(app.replyFocus).toBeUndefined();
      expect(app.composer.focused).toBe(true);
    },
    { ...size, useMouse: true },
    true,
  ));

test("a folded thread marks new work with a dot, and the dot goes once the thread opens", () =>
  composing(
    async ({ session, app, frame }) => {
      await idle(session);
      const id = listed(session)[0]?.act.id ?? "";
      app.foldThread(id, true);
      const folded = async () =>
        (await frame()).split("\n").find((row) => row.includes("Explore this project")) ?? "";
      expect(await folded()).not.toContain("●");
      await session.reply(id, "One more thing");
      await until(session, () => (listed(session)[0]?.messages.length ?? 0) === 1);
      expect(await folded()).toMatch(/┃ ● .*Explore this project/);
      app.foldThread(id, false);
      // The box draws open over a few frames on a slow machine, and the thread counts as seen once it has.
      let open = await frame();
      for (let step = 0; step < 160 && !open.includes("Reply to"); step++) {
        await Bun.sleep(25);
        open = await frame();
      }
      expect(open).toContain("Reply to");
      app.foldThread(id, true);
      let row = await folded();
      for (let step = 0; step < 160 && row.includes("●"); step++) {
        await Bun.sleep(25);
        row = await folded();
      }
      expect(row).not.toContain("●");
    },
    size,
    true,
  ));

test("the work that came before a thread folded counts as seen", () =>
  composing(async ({ app }) => {
    expect(app.unread("p1", "a", false)).toBe(false);
    expect(app.unread("p1", "b", true)).toBe(false);
    expect(app.unread("p1", "b", true)).toBe(false);
    expect(app.unread("p1", "c", true)).toBe(true);
    expect(app.unread("p1", "c", false)).toBe(false);
    expect(app.unread("p1", "c", true)).toBe(false);
  }));

/** Lines of code for a window, the nth of which runs when it is named. */
const code = (count: number, runs = -1): ProgramLine[] =>
  Array.from(
    { length: count },
    (_, at): ProgramLine => ({
      kind: "code",
      step: { id: "rung1" } as ProgramLine["step"],
      number: at + 1,
      display: at + 1,
      text: `x${at + 1} = ${at + 1}`,
      acts: [],
      runs: at + 1 === runs,
      hint: "",
    }),
  );

/** What a window shows, row by row: the number of a line, or ... for a row that stands for lines. */
const rows = (lines: ProgramLine[]) => lines.map((line) => (line.kind === "code" ? line.display : line.text));

test("a window of code shows seven rows: the first line or a ..., the line that runs second to last, and a ... or the row where the model writes", () =>
  composing(async ({ app }) => {
    // Lines that fit stand whole.
    expect(rows(app.rolling(code(5), "w1", false).lines)).toEqual([1, 2, 3, 4, 5]);
    expect(rows(app.rolling(code(7), "w2", false).lines)).toEqual([1, 2, 3, 4, 5, 6, 7]);
    // Lines that are over show the last seven, and a ... stands first.
    expect(rows(app.rolling(code(12), "w3", false).lines)).toEqual(["...", 7, 8, 9, 10, 11, 12]);
    // The line that runs stands second to last, with a ... after it, while lines come after.
    expect(rows(app.rolling(code(12, 8), "w4", false).lines)).toEqual(["...", 4, 5, 6, 7, 8, "..."]);
    // The first line stands first while the line that runs is near it.
    expect(rows(app.rolling(code(12, 3), "w5", false).lines)).toEqual([1, 2, 3, 4, 5, 6, "..."]);
    // The last row is where the model writes.
    expect(rows(app.rolling(code(12), "w6", true).lines)).toEqual(["...", 8, 9, 10, 11, 12]);
    expect(rows(app.rolling(code(6), "w7", true).lines)).toEqual([1, 2, 3, 4, 5, 6]);
    expect(rows(app.rolling(code(7), "w8", true).lines)).toEqual(["...", 3, 4, 5, 6, 7]);
    // An opened window shows its lines, the last four hundred at most.
    app.session.folds["window:w9"] = false;
    expect(app.rolling(code(12), "w9", false)).toMatchObject({ opened: true });
    expect(app.rolling(code(500), "w9", false).lines).toHaveLength(400);
    // A jump to an act of the thread opens each window of the thread.
    app.session.folds["window:w10"] = false;
    expect(app.rolling(code(12), "w10:2", false).opened).toBe(true);
  }));

test("a click on a ... of a window of code opens it in a block that scrolls, and a click on Roll up rolls it again", async () =>
  composing(
    async ({ session, app, screen, frame }) => {
      await session.refresh();
      await session.select(
        session.chains.find((chain) => session.labelOf(chain.id) === "life3/feed")?.id ?? "",
      );
      const gaps = async () => {
        const shown = (await frame()).split("\n").map((row) => feedOf(session, row));
        return shown.flatMap((row, y) =>
          /^[\s┃]+\.\.\.\s*$/.test(row) ? [{ x: row.indexOf("..."), y }] : [],
        );
      };
      const first = (await gaps())[0];
      if (!first) throw new Error("No window is rolled.");
      const card = app.scroll.getChildren().find((node) => node.id.startsWith("program-"));
      const key = `window:${card?.id.slice("program-".length)}:0`;
      expect(session.folds[key]).toBeUndefined();
      await screen.mockMouse.click(first.x, first.y);
      await until(session, () => session.folds[key] === false);
      const opened = (await frame()).split("\n");
      expect(opened.join("\n")).toContain("Roll up");
      expect((await gaps()).length).toBeLessThan(1);
      const up = opened.findIndex((row) => row.includes("Roll up"));
      await screen.mockMouse.click((opened[up] ?? "").indexOf("Roll up"), up);
      await until(session, () => session.folds[key] === undefined);
      expect((await gaps()).length).toBeGreaterThan(0);
    },
    { ...size, useMouse: true },
    await recorded(),
  ));

test("the gutter stands inside the block of code at its left, or right of it when the flag says so", () =>
  composing(
    async ({ session, frame }) => {
      await idle(session);
      const row = async () =>
        feedOf(
          session,
          (await frame()).split("\n").find((one) => one.includes('notes = read("README.md")')) ?? "",
        );
      expect(await row()).toMatch(/^[\s┃]+1 notes = read/);
      await session.submit("/gutter right");
      expect(session.preferences.gutter).toBe("right");
      expect(await row()).toMatch(/^[\s┃]+notes = read\("README\.md"\).* 1\s*$/);
      // Without a value the flag turns over.
      await session.submit("/gutter");
      expect(session.preferences.gutter).toBe("left");
      expect(await row()).toMatch(/^[\s┃]+1 notes = read/);
      expect(String(await session.submit("/gutter up").catch((error) => error))).toContain("/gutter left");
    },
    size,
    true,
  ));

test("the box of a thread starts where the input of the chain starts, and its reply starts where the text of that input starts", () =>
  composing(
    async ({ session, frame }) => {
      await idle(session);
      const rows = (await frame()).split("\n").map((row) => feedOf(session, row));
      const ask = rows.find((row) => row.includes("Ask anything")) ?? "";
      const reply = rows.find((row) => row.includes("Reply to")) ?? "";
      expect(ask).not.toBe("");
      expect(reply).not.toBe("");
      expect(reply.indexOf("Reply to")).toBe(ask.indexOf("Ask anything"));
      expect(reply.indexOf("┃")).toBe(ask.indexOf("┃"));
      // The rows of the box stand on the same line, whether they are code, markdown or the input.
      const bars = rows.filter((row) => row.includes("┃")).map((row) => row.indexOf("┃"));
      expect(new Set(bars).size).toBe(1);
    },
    size,
    true,
  ));

test("the up arrow in the input of a thread takes its last message back to edit it, and the message that replaces it takes its place in the box", () =>
  composing(
    async ({ session, app, screen, click, frame }) => {
      await idle(session);
      const id = listed(session)[0]?.act.id ?? "";
      // A pause holds the message open, as a model that works on it does. A model that answers at once leaves
      // nothing to take back, which a fast machine reaches before the key.
      await session.engine.pause(session.engine.root);
      await session.reply(id, "First wording");
      await until(session, () => (listed(session)[0]?.messages.length ?? 0) === 1);
      let seen = await frame();
      for (let step = 0; step < 60 && !seen.includes("First wording"); step++) {
        await Bun.sleep(25);
        seen = await frame();
      }
      expect(seen).toContain("First wording");
      await click("Reply to");
      // The input has the keys once the click reached it, which a slow machine takes a few frames to do.
      for (let step = 0; step < 160 && app.replyInputs.get(id)?.focused !== true; step++) {
        await Bun.sleep(25);
        await frame();
      }
      expect(app.replyInputs.get(id)?.focused).toBe(true);
      screen.mockInput.pressArrow("up");
      await until(session, () => session.drafts[`reply:${id}`] === "First wording");
      const input = app.replyInputs.get(id);
      if (!input) throw new Error("The thread has no input.");
      input.value = "Second wording";
      screen.mockInput.pressEnter();
      await until(session, () => (listed(session)[0]?.messages.length ?? 0) === 2);
      let shown = await frame();
      for (
        let step = 0;
        step < 240 && (shown.includes("First wording") || !shown.includes("Second wording"));
        step++
      ) {
        await Bun.sleep(25);
        shown = await frame();
      }
      expect(shown).not.toContain("First wording");
      expect(shown).toContain("Second wording");
    },
    { ...size, useMouse: true },
    true,
  ));

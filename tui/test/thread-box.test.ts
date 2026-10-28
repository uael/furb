import { expect, test } from "bun:test";
import { join } from "node:path";
import type { KeyEvent } from "@opentui/core";
import type { ProgramLine } from "furb/life";
import { chainThreads } from "furb/life";
import { until } from "../../bind/typescript/test/until.ts";
import { openEngine } from "../src/bridge.ts";
import { Session } from "../src/session.ts";
import { threadTone } from "../src/theme.ts";
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
      // A row of the program quotes the answer, so the text of the answer is the one that no call of close holds.
      const answered = (row: string) =>
        row.includes("Fieldnotes keeps ideas close") && !row.includes("close(");
      // The markdown of the box draws over a few frames.
      for (let step = 0; step < 80 && !rows.some(answered); step++) {
        await Bun.sleep(25);
        rows = (await frame()).split("\n").map((row) => feedOf(session, row));
      }
      const at = (text: string) =>
        rows.findIndex(
          (row) => row.includes(text) && (text !== "Fieldnotes keeps ideas close" || answered(row)),
        );
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
      // The answer stands as markdown with no line of color, in the column of the text of the message.
      expect(at("Fieldnotes keeps ideas close")).toBeGreaterThan(0);
      expect(rows[at("Fieldnotes keeps ideas close")]).not.toContain("┃");
      expect(rows[at("Fieldnotes keeps ideas close")]?.indexOf("Fieldnotes")).toBe(
        rows[at("Explore this project")]?.indexOf("Explore"),
      );
      // The line of the message starts half a row above it, in the top edge of the box, and ends half a row under it, in
      // the row that parts it from the code, as the line of a panel does, so the line adds no row of its own.
      expect(rows[at("Explore this project") - 1]).toMatch(/^\s*╻▄{4,}\s*$/);
      expect(rows[at("Explore this project") + 1]).toMatch(/^\s*╹\s*$/);
      expect(rows[at("Explore this project") + 2]).not.toContain("┃");
      // The box has an edge under its input.
      expect(rows[at("Reply to open it again") + 1]).toContain("▀▀▀▀");
      expect(rows.join("\n")).not.toContain("Open again");
      expect(rows.join("\n")).toContain("▾ Collapse");
    },
    size,
    true,
  ));

test("a click on Reply makes the thread the target of the input, whose line and meta line take its color and its name, a message then goes to the thread and opens it again, and + New thread sends the next one to a new thread", () =>
  composing(
    async ({ session, app, screen, click, frame }) => {
      await idle(session);
      const id = listed(session)[0]?.act.id ?? "";
      expect(listed(session)[0]?.resolved).toBe(true);
      // A thread that is over is no target until a click asks for it.
      expect(session.target).toBeUndefined();
      await click("Reply to open it again");
      expect(session.target).toBe(id);
      expect(app.composer.focused).toBe(true);
      const shown = await frame();
      expect(shown).toContain("↳ Replying here");
      expect(shown).toContain("to ↳ Explore this");
      expect(shown).toContain("+ New thread");
      expect(app.composeEdges[0]?.borderColor.equals(threadTone(id))).toBe(true);
      await screen.mockInput.typeText("Look at it once more");
      // The thread holds its model back, so that the message stays open to be seen: the model closes a message within
      // the batch of facts that brings it, on a machine that is quick.
      await session.command(`/pause ${id}`);
      screen.mockInput.pressEnter();
      await until(session, () => (listed(session)[0]?.messages.length ?? 0) === 1);
      expect(listed(session)[0]?.resolved).toBe(false);
      await session.command(`/wake ${id}`);
      expect(session.target).toBe(id);
      await click("+ New thread");
      expect(session.target).toBeUndefined();
      expect(await frame()).not.toContain("+ New thread");
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
      for (let step = 0; step < 160 && !open.includes("↳ Reply"); step++) {
        await Bun.sleep(25);
        open = await frame();
      }
      expect(open).toContain("↳ Reply");
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

test("a click on a ... of a window of code opens it in a block that fits its lines and scrolls past them, and a click on Roll up rolls it again", async () =>
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
      // A block of fewer lines than it shows before it scrolls takes their rows alone, with no blank row before Roll up.
      for (const node of app.scroll.getChildren().filter((one) => one.id.startsWith("program-")))
        session.folds[`window:${node.id.slice("program-".length)}`] = false;
      app.renderContent();
      const all = (await frame()).split("\n").map((row) => feedOf(session, row));
      const ups = all.flatMap((row, y) => (row.includes("Roll up") ? [y] : []));
      expect(ups.length).toBeGreaterThan(0);
      for (const y of ups) expect((all[y - 1] ?? "").trim()).not.toBe("");
    },
    { ...size, useMouse: true },
    await recorded(),
  ));

test("the box of a thread starts where the input of the chain starts, and the word of its Reply starts where the text of that input starts", () =>
  composing(
    async ({ session, frame }) => {
      await idle(session);
      const rows = (await frame()).split("\n").map((row) => feedOf(session, row));
      const ask = rows.find((row) => row.includes("Ask anything")) ?? "";
      const reply = rows.find((row) => row.includes("↳ Reply")) ?? "";
      expect(ask).not.toBe("");
      expect(reply).not.toBe("");
      expect(reply.indexOf("Reply")).toBe(ask.indexOf("Ask anything"));
      // The line of the box stands on one column, whatever the row.
      const bars = rows.filter((row) => row.includes("┃")).map((row) => row.indexOf("┃"));
      expect(new Set(bars).size).toBe(1);
      // Only a message of the operator holds a line: each cap of a line stands just above its rows, and no edge of a box
      // holds one.
      const caps = rows.flatMap((row, y) => (row.includes("╻") ? [y] : []));
      expect(caps.length).toBeGreaterThan(0);
      for (const y of caps) expect(rows[y + 1] ?? "").toContain("┃");
    },
    size,
    true,
  ));

test("the up arrow in an empty input takes back the last message to the thread that it targets, to edit it, and the message that replaces it takes its place in the box", () =>
  composing(
    async ({ session, app, frame }) => {
      await idle(session);
      const id = listed(session)[0]?.act.id ?? "";
      // A pause holds the message open, as a model that works on it does. A model that answers at once leaves
      // nothing to take back, which a fast machine reaches before the key.
      await session.engine.pause(session.engine.root);
      await session.reply(id, "First wording");
      await until(session, () => (listed(session)[0]?.messages.length ?? 0) === 1);
      let seen = await frame();
      // The markdown of the answer draws over a few frames too.
      const drawn = (text: string) =>
        text.includes("First wording") && /\n\s*A clear starting point/.test(text);
      for (let step = 0; step < 120 && !drawn(seen); step++) {
        await Bun.sleep(25);
        seen = await frame();
      }
      expect(seen).toContain("First wording");
      // The answer of the thread stands before the message that came after it, so each message stands with its answer.
      const rows = seen.split("\n");
      const answerRow = rows.findIndex((row) => /^\s*A clear starting point/.test(row));
      expect(answerRow).toBeGreaterThan(0);
      expect(answerRow).toBeLessThan(rows.findIndex((row) => row.includes("First wording")));
      app.replyTo(id);
      expect(session.target).toBe(id);
      // The key goes to the handler of the input itself: a key press of the test terminal reaches it later on a slow
      // machine than the test can wait.
      expect(app.walkHistory({ name: "up" } as KeyEvent)).toBe(true);
      await until(session, () => app.composer.plainText === "First wording");
      app.composer.setText("Second wording");
      await app.submit();
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

import { expect, setSystemTime, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { RGBA } from "@opentui/core";
import { until } from "../../binding/typescript/test/until.ts";
import { glyph, palettes, spacing as space } from "../src/theme.ts";
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
const listed = (session: Composing["session"]) => session.threadsOf(session.selected).list;

test("the sidebar lists the workspaces and the threads of the session shown, and holds no tree of chains", () =>
  composing(
    async ({ session, frame }) => {
      await idle(session);
      const shown = await frame();
      keep("sidebar", shown);
      const text = rail(session, shown).join("\n");
      expect(text).toContain("Workspaces");
      expect(text).not.toContain("Chains");
      expect(text).not.toContain("← ");
      const first = listed(session)[0];
      expect(first).toBeDefined();
      expect(text).toContain((first?.title ?? "").slice(0, 12));
    },
    size,
    true,
  ));

test("after ⌥T the keys stand on the thread lit, Space collapses it in the feed to its header line, and Escape gives the keys back", () =>
  composing(
    async (use) => {
      const { session, app, screen, frame } = use;
      await idle(session);
      await frame();
      screen.mockInput.pressKey("t", { meta: true });
      await frame();
      expect(app.railKeys).toBe(true);
      const id = app.session.activeThread?.act.id ?? "";
      expect(id).not.toBe("");
      expect(app.railAt).toBe(`th:${id}`);
      screen.mockInput.pressKey(" ");
      await frame();
      expect(session.folds[`thread:${id}`]).toBe(true);
      const folded = await frame();
      keep("collapsed", folded);
      expect(folded).toContain("Expand");
      screen.mockInput.pressKey(" ");
      await frame();
      expect(session.folds[`thread:${id}`]).toBe(false);
      // The feed lays the thread out again over a few frames, and its folded line goes.
      let shown = await frame();
      for (let step = 0; step < 40 && shown.includes("Expand"); step++) {
        await Bun.sleep(25);
        shown = await frame();
      }
      expect(shown).not.toContain("Expand");
      expect(shown).toContain("Collapse");
      screen.mockInput.pressArrow("left");
      await frame();
      expect(app.railKeys).toBe(false);
    },
    size,
    true,
  ));

test("a thread resolves from the controls under its row in the sidebar, and a message to it opens it again", () =>
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
      await until(session, () => session.thread === id);
      const open = await frame();
      keep("chosen", open);
      // The controls of the thread chosen stand on the row under its row.
      const side = rail(session, open);
      const title = side.findIndex((line) => line.includes("Hold this thread open"));
      expect(side[title + 1]).toContain("↳ Reply");
      await railClick(use, " Resolve ");
      await until(session, () => listed(session).find((one) => one.act.id === id)?.resolved === true);
      expect(await session.engine.result(id)).toBe("resolved");
      // The thread chosen stays the target of the input, which opens it again with its next message.
      expect(await frame()).toContain("↳ Replying here");
      await session.submit("Look at it once more");
      await until(session, () => listed(session).find((one) => one.act.id === id)?.resolved === false);
      keep("reopened", await frame());
    },
    size,
    true,
  ));

// The life of 11-05 showed a prompt of the operator closed as "resolved" that nobody resolved. The sidebar orders the open
// threads by the time of their last message, so a message moves a thread under a still pointer, and a release that
// fell on the Resolve link that the redraw put there took it as a click on it.
test("a redraw that puts Resolve under a still pointer between the press and the release resolves nothing", () =>
  composing(
    async (use) => {
      const { session, screen, frame } = use;
      await idle(session);
      const root = session.engine.root;
      await session.engine.pause(root);
      const id = await session.engine.prompt("str", { message: "Hold this thread open", on: root });
      await session.refresh();
      await until(session, () => listed(session).some((one) => one.act.id === id));
      await railClick(use, "Hold this thread open");
      await until(session, () => session.thread === id);
      const other = await session.engine.prompt("str", { message: "Another thread", on: root });
      await session.refresh();
      await until(session, () => listed(session).some((one) => one.act.id === other));
      const left = size.width - session.preferences.sidebarWidth;
      const before = rail(session, await frame());
      const x = (before.find((line) => line.includes("✓ Resolve")) ?? "").indexOf("Resolve");
      // The press lands on the row of the other thread, under the controls of the thread held open, which the message
      // moves under that row.
      const y = before.findIndex((line) => line.includes("Another thread"));
      expect(before[y - 1]).toContain("✓ Resolve");
      expect(before[y]?.[x]).toBe(" ");
      await screen.mockMouse.pressDown(left + x, y);
      // A message moves the thread held open last, and its Resolve comes under the pointer, which did not move.
      await session.submit("One more word on it");
      await until(
        session,
        () => (listed(session).find((one) => one.act.id === id)?.messages.length ?? 0) > 0,
      );
      let after = before;
      for (let step = 0; step < 40 && !after[y]?.includes("Resolve"); step++) {
        await Bun.sleep(25);
        after = rail(session, await frame());
      }
      expect(after[y]?.slice(x, x + "Resolve".length)).toBe("Resolve");
      await screen.mockMouse.release(left + x, y);
      await frame();
      await Bun.sleep(300);
      expect(listed(session).find((one) => one.act.id === id)?.resolved).toBe(false);
      // A press and a release on Resolve itself still resolve the thread.
      await railClick(use, " Resolve ");
      await until(session, () => listed(session).find((one) => one.act.id === id)?.resolved === true);
      expect(await session.engine.result(id)).toBe("resolved");
    },
    size,
    true,
  ));

test("a line on an open thread is a message of None, over at once, and a line on a resolved thread is a prompt of str", () =>
  composing(
    async ({ session }) => {
      await idle(session);
      const root = session.engine.root;
      await session.engine.pause(root);
      const id = await session.engine.prompt("str", { message: "Hold this thread open", on: root });
      await session.refresh();
      await until(session, () => listed(session).some((one) => one.act.id === id));
      const on = () => session.acts.filter((act) => act.thread === id);
      session.aim(id);
      expect(session.lineShape(id)).toBe("None");
      await session.submit("Look at it once");
      await until(session, () => on().length === 1);
      expect(on()[0]?.words[0]).toBe("None");
      // The message is over at once, and it holds the thread open no more than the prompt of the thread does.
      await until(session, () => on()[0]?.done === true);
      expect(listed(session).find((one) => one.act.id === id)?.resolved).toBe(false);
      await session.reply(id, "And once more");
      await until(session, () => on().length === 2);
      expect(on()[1]?.words[0]).toBe("None");
      await session.resolve(id);
      await until(session, () => listed(session).find((one) => one.act.id === id)?.resolved === true);
      expect(session.lineShape(id)).toBe("str");
      await session.submit("Look at it again");
      await until(session, () => on().length === 3);
      expect(on()[2]?.words[0]).toBe("str");
      await until(session, () => listed(session).find((one) => one.act.id === id)?.resolved === false);
    },
    size,
    true,
  ));

test("a thread whose title is longer than the sidebar ends its row in one ellipsis, and keeps its state at the right", () =>
  composing(
    async ({ session, app, frame }) => {
      await idle(session);
      const first = listed(session)[0];
      expect(first).toBeDefined();
      const title = first?.title ?? "";
      session.preferences.sidebarWidth = 28;
      app.render();
      await frame();
      const rows = (await frame()).split("\n").map((line) => line.slice(size.width - 28));
      const row = rows.find((line) => line.includes(title.slice(0, 8)));
      expect(row).toBeDefined();
      expect(row).toContain("…");
      expect(row).not.toContain("...");
      expect(rows.join("\n")).not.toContain(title);
    },
    size,
    true,
  ));

test("a question that waits shows on top of the sidebar and as a mark on its workspace and its session, and the input goes to the thread that is lit", () =>
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
      expect(rows.some((line) => line.includes("1 needs you"))).toBe(true);
      expect(rows.filter((line) => line.includes("◆ 1")).length).toBeGreaterThan(0);
      expect(listed(session).some((one) => one.act.id === question)).toBe(false);
      // The input sends its message to the open thread that the sidebar lights, unless the operator asks for a new thread,
      // and a thread that is over is no target until he chooses it.
      const active = session.activeThread;
      expect(active).toBeDefined();
      expect(session.openThread).toBe(active && !active.resolved ? active : undefined);
      expect(session.target).toBe(session.openThread?.act.id);
      await session.submit("/newthread");
      expect(session.fresh).toBe(true);
      expect(session.target).toBeUndefined();
    },
    size,
    true,
  ));

test("a string too long for its line folds to its start and an underlined …, which a hover shows whole and a click opens in place", () =>
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
      expect(rows[y]).toContain('…"');
      expect(folded).not.toContain("ENDMARK");
      const x = (rows[y] ?? "").indexOf('…"');
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
          .some((row) => row.includes('…"')),
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
      expect(rows.find((row) => row.includes("b = await build"))).toMatch(
        /(?:\d|[\u2800-\u28ff✻]) {2}b = await build/,
      );
      await session.engine.cancel(rung);
    },
    size,
    true,
  ));

test("a line that does not fit its room lays out in rows, and it and a folded string keep the colors of their whole statement", () =>
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
      // The names stand on the rows under the opening, one step deeper, and none is cut.
      const first = rows.findIndex((row) => row.includes('"n0xyz"'));
      expect(first).toBeGreaterThan(cut);
      expect(rows[cut]).not.toContain("n0xyz");
      expect(rows.some((row) => row.includes('"n15xyz"'))).toBe(true);
      expect(rows[fold]).toContain('…"');
      const palette = palettes.ink;
      const fg = (x: number, y: number) => cellAt(screen.captureSpans(), x, y).fg;
      const keyword = RGBA.fromHex(palette.syntaxKeyword);
      const x = (rows[def] ?? "").indexOf("def");
      await until(screen.renderer, () => Boolean(fg(x, def)?.equals(keyword)), "frame");
      expect(
        fg((rows[first] ?? "").indexOf("n0xyz"), first)?.equals(RGBA.fromHex(palette.syntaxString)),
      ).toBe(true);
      // The closing mark of the folded string, after its underlined ..., is a string.
      expect(fg((rows[fold] ?? "").indexOf('…"') + 1, fold)?.equals(RGBA.fromHex(palette.syntaxString))).toBe(
        true,
      );
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

test("the context stands in the corner beside the input, under a row of space, and says No answer yet before the first answer", () =>
  composing(
    async ({ session, frame }) => {
      const shown = await frame();
      keep("corner-wide", shown);
      const left = 140 - session.preferences.sidebarWidth;
      const rows = shown.split("\n");
      const y = rows.findIndex((row) => row.slice(left).trim().startsWith("Context"));
      expect(y).toBeGreaterThan(0);
      expect(rows[y]).toContain("No answer yet");
      // The text of the input starts on the row of the header of the context.
      expect((rows[y] ?? "").slice(0, left)).toContain("Ask anything");
      expect((rows[y - 2] ?? "").slice(left).trim()).toBe("");
    },
    { width: 140, height: 40 },
  ));

test("the keys of the footer stand under the context, on the row of the footer, while the sidebar stands", () =>
  composing(
    async ({ app, session, frame }) => {
      const rows = (await frame()).split("\n");
      const left = 140 - session.preferences.sidebarWidth;
      const last = rows.findLastIndex((row) => row.trim());
      expect((rows[last] ?? "").slice(left)).toContain("commands");
      expect((rows[last] ?? "").slice(0, left)).not.toContain("commands");
      // The context ends where the input ends, and the row under both is the row of the footer.
      expect(app.cornerKeys.y).toBe(last);
      expect(app.context.y + app.context.height).toBe(app.cornerKeys.y);
      expect(app.cornerKeys.y).toBe(app.lower.y + app.lower.height - 1);
    },
    { width: 140, height: 40 },
  ));

test("the sidebar ends as far above the context as the feed ends above the input, on a feed that fills the screen", () =>
  composing(
    async ({ app, frame }) => {
      await frame();
      app.scroll.scrollTo(0);
      const rows = (await frame()).split("\n");
      expect(app.scroll.scrollHeight).toBeGreaterThan(app.scroll.height);
      const feed = app.lower.y - (app.scroll.y + app.scroll.height);
      const side = app.corner.y - (app.railSlot.y + app.railSlot.height);
      expect(feed).toBe(space.rest);
      expect(side).toBe(space.rest);
      const left = 140 - app.corner.width;
      expect((rows[app.lower.y] ?? "").slice(3, 4)).toBe(glyph.halfTop);
      expect((rows[app.corner.y] ?? "").slice(left + 2, left + 3)).toBe(glyph.halfTop);
    },
    { width: 140, height: 20 },
    true,
  ));

test("the last thread of the feed ends as far above the input as the sidebar ends above the context, on a short feed and on a full one", async () => {
  for (const height of [20, 80])
    await composing(
      async ({ app, frame }) => {
        await frame();
        app.scroll.scrollTo(app.scroll.scrollHeight);
        await frame();
        const last = app.scroll.content.getChildren().at(-1);
        expect(last).toBeDefined();
        const full = app.scroll.scrollHeight > app.scroll.height;
        // A short feed starts at the top of its room, so its room ends where its last thread cannot.
        const end = full ? (last?.y ?? 0) + (last?.height ?? 0) : app.scroll.y + app.scroll.height;
        const feed = app.lower.y - end;
        const side = app.corner.y - (app.railSlot.y + app.railSlot.height);
        expect(feed).toBe(space.rest);
        expect(side).toBe(feed);
      },
      { width: 140, height },
      true,
    );
});

test("on a narrow terminal and with the sidebar hidden, the keys stay in the footer beside its state", async () => {
  for (const [width, hidden] of [
    [80, false],
    [140, true],
  ] as const)
    await composing(
      async ({ session, frame }) => {
        if (hidden) session.preferences.sidebar = false;
        const rows = (await frame()).split("\n");
        expect(rows.findLast((row) => row.trim()) ?? "").toContain("commands");
      },
      { width, height: 40 },
    );
});

test("on a narrow terminal and with the sidebar hidden, the context folds into a line of the footer", async () => {
  for (const [width, hidden] of [
    [80, false],
    [140, true],
  ] as const)
    await composing(
      async ({ session, frame }) => {
        if (hidden) session.preferences.sidebar = false;
        const rows = (await frame()).split("\n");
        expect(rows.join("\n")).not.toContain("Context");
        expect(rows.findLast((row) => row.trim()) ?? "").toContain("No answer yet");
      },
      { width, height: 40 },
    );
});

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
      expect(rows[y]).toContain('…"');
      expect(folded).not.toContain("ENDMARK");
      expect(folded).not.toContain('"""');
      expect(rows[y + 1]).toContain("print(len(note))");
      const fg = (x: number, row: number) => cellAt(screen.captureSpans(), x, row).fg;
      const string = RGBA.fromHex(palettes.ink.syntaxString);
      // The closing mark of the string, after its underlined ..., is a string of the statement, as it is whole.
      const x = (rows[y] ?? "").indexOf('…"');
      await until(screen.renderer, () => Boolean(fg(x + 1, y)?.equals(string)), "frame");
      await screen.mockMouse.moveTo(x, y);
      await until(screen.renderer, () => screen.captureCharFrame().includes("ENDMARK"), "frame");
      await screen.mockMouse.click(x, y);
      await until(session, () => session.folds[`string:${rung}:1:0`] === false);
      const opened = await frame();
      keep("multiline-open", opened);
      expect(opened).toContain("ENDMARK");
      const shownRows = opened.split("\n");
      const first = shownRows.findIndex((row) => row.includes('note = "alpha beta'));
      const end = shownRows.findIndex((row) => row.includes("ENDMARK"));
      expect(end).toBeGreaterThan(first);
      expect(shownRows.slice(first, end + 1).some((row) => row.includes('…"'))).toBe(false);
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
      expect(rows[y]).toContain("…'");
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

test("a line that does not fit lays out in rows at its brackets, and each row keeps the colors of its whole statement", () =>
  composing(
    async ({ session, screen, frame }) => {
      const names = Array.from({ length: 24 }, (_, k) => `"n${k}xyz"`).join(", ");
      const rung = await session.engine.rung({ word: `names = [${names}]`, on: session.engine.root });
      await session.engine.result(rung);
      await session.refresh();
      const opened = await frame();
      keep("wrapped", opened);
      const rows = opened.split("\n");
      const y = rows.findIndex((row) => row.includes("names = ["));
      expect(rows[y]).toContain("names = [");
      expect(rows[y]).not.toContain("…");
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
    size,
    true,
  ));

test("the threads of the chain shown are read again for another chain", () =>
  composing(
    async ({ session }) => {
      await idle(session);
      const root = session.engine.root;
      await session.engine.pause(root);
      const other = await session.engine.chain({ label: "other" });
      await session.engine.pause(other);
      const there = await session.engine.prompt("str", { message: "A thread of the other chain", on: other });
      await session.refresh();
      expect(session.threads.list.some((one) => one.act.id === there)).toBe(false);
      await session.select(other);
      // The list of the other chain holds its thread, whatever the version of the acts.
      expect(session.threads.list.map((one) => one.act.id)).toContain(there);
    },
    size,
    true,
  ));

test("a row of a thread says how long ago the thread last changed, as the session saw it", () =>
  composing(
    async ({ session, frame }) => {
      await idle(session);
      const row = async () =>
        (rail(session, await frame()).find((line) => line.includes("Explore this project")) ?? "").trimEnd();
      // The demo ran in the session, so every act changed just now.
      expect(await row()).not.toMatch(/\d[mhd]$/);
      setSystemTime(new Date(Date.now() + 5 * 60_000));
      try {
        expect(await row()).toMatch(/ 5m$/);
      } finally {
        setSystemTime();
      }
    },
    size,
    true,
  ));

test("an act that is no prompt leaves the list of the threads of the chain shown as it was, and a prompt reads it again", () =>
  composing(
    async ({ session }) => {
      await idle(session);
      const root = session.engine.root;
      const before = session.threads.list;
      const version = session.version;
      await session.engine.result(await session.engine.rung({ word: "step = 1", on: root }));
      await session.refresh();
      // The act came, and the threads read it not.
      expect(session.version).toBeGreaterThan(version);
      expect(session.threads.list).toBe(before);
      const there = await session.engine.prompt("str", { message: "A new thread", on: root });
      await session.refresh();
      expect(session.threads.list).not.toBe(before);
      expect(session.threads.list.some((one) => one.act.id === there)).toBe(true);
    },
    size,
    true,
  ));

test("a welcome that is taller than the screen starts at its top, with the logo, and not at its end", () =>
  composing(
    async ({ session, frame }) => {
      await idle(session);
      const shown = await frame();
      expect(shown).toContain("▀▀██▀▀");
      expect(shown).toContain("The model answers in Python.");
      // The logo stands above its line, and the keys at the end of the welcome are the part that the screen cuts.
      expect(shown.indexOf("▀▀██▀▀")).toBeLessThan(shown.indexOf("The model answers in Python."));
      expect(shown).not.toContain("@ files");
    },
    { width: 100, height: 16 },
    false,
  ));

test("a line that failed says its reason once, under it, and not again beside it", () =>
  composing(
    async ({ session, frame }) => {
      const id = await session.engine.rung({ word: 'read("missing-file.txt")', on: session.selected });
      await until(session, () => session.activity.some((act) => act.id === id && act.done));
      await session.refresh();
      const shown = await frame();
      expect(shown.split("There is no file").length - 1).toBe(1);
    },
    size,
    false,
  ));

test("the agents that a thread hires fold under it to their count in the sidebar, and a click lists each one row deeper", () =>
  composing(
    async (use) => {
      const { session, app, frame } = use;
      app.composer.setText("Hire eight workers.");
      await app.submit();
      // The prompts of the agents stand on chains of their own, so the frames are read until the sidebar folds them.
      let rows: string[] = [];
      for (let step = 0; step < 100 && !rows.some((row) => /Agents +8/.test(row)); step++) {
        await session.refresh();
        rows = rail(session, await frame());
        await Bun.sleep(50);
      }
      const fold = rows.findIndex((row) => /▸ +Agents +8/.test(row));
      expect(fold).toBeGreaterThan(rows.findIndex((row) => row.includes("Hire eight workers")));
      expect(rows.filter((row) => row.includes("Think until I cancel"))).toHaveLength(0);
      await railClick(use, "Agents");
      rows = rail(session, await frame());
      const agents = rows.filter((row) => row.includes("Think until I cancel"));
      expect(agents).toHaveLength(8);
      const lead = rows.find((row) => row.includes("Hire eight workers")) ?? "";
      expect((agents[0] ?? "").search(/\S/)).toBeGreaterThan(lead.search(/\S/));
      await session.submit("/cancel");
    },
    { width: 152, height: 60 },
    true,
  ));

test("the tip of the marks of a workspace says each count in words, in the order of their weight", () =>
  composing(async ({ app }) => {
    const tip = (asks: number, failed: number, paused: number) =>
      app.badgeTip({ asks, failed, paused, unread: 0 })[0]?.[0];
    expect(tip(1, 0, 0)).toBe("1 question waits for you");
    expect(tip(0, 1, 1)).toBe("1 failed  ·  1 paused");
  }, size));

test("a done thread of the operator that no fact seen covers shows ●, and a seen that another window says turns it into a faint ✓", () =>
  composing(
    async ({ session, frame }) => {
      session.folds["thread:prompt1"] = true;
      await idle(session);
      // A window that opens later on the life, with the thread folded, never showed its work. The count of the reads
      // moves with each change of what the window showed, or the rail keeps the marks that it read.
      const inner = session as unknown as { showing: Map<string, number>; readsCount: number };
      inner.showing.clear();
      inner.readsCount++;
      const row = async () =>
        rail(session, await frame()).find((line) => line.includes("Explore this project")) ?? "";
      expect(await row()).toContain("● Explore this project");
      // Another window of the life said that the operator read the thread, and the host gives it to this one.
      session.host.seen = new Map([["prompt1", Number.MAX_SAFE_INTEGER]]);
      session.emit("change");
      let seen = await row();
      for (let step = 0; step < 80 && !seen.includes(`${glyph.done} Explore`); step++) {
        await Bun.sleep(25);
        seen = await row();
      }
      expect(seen).toContain(`${glyph.done} Explore this project`);
      expect(seen).not.toContain("●");
    },
    size,
    true,
  ));

test("after a switch to another chain, the sidebar lights its open thread and shows its controls", () =>
  composing(
    async ({ session, frame }) => {
      await idle(session);
      const root = session.engine.root;
      await session.engine.pause(root);
      const other = await session.engine.chain({ label: "other" });
      await session.engine.pause(other);
      const there = await session.engine.prompt("str", { message: "A thread of the other chain", on: other });
      await session.refresh();
      await frame();
      await session.select(other);
      expect(session.activeThread?.act.id).toBe(there);
      expect(rail(session, await frame()).join("\n")).toMatch(/Resolve(?!d)/);
    },
    size,
    true,
  ));

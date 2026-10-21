import { afterAll, expect, test } from "bun:test";
import type { RowState } from "@furb/engine/life";
import type { App } from "../src/app.ts";
import { removeDemoDirectories } from "../src/demo.ts";
import { idle } from "./idle.ts";
import { looming, soon } from "./looming.ts";

afterAll(removeDemoDirectories);

/** The state of each thread of the loom by its label, as the App reads it now. */
function states(app: App): Record<string, RowState> {
  app.render();
  return Object.fromEntries(app.loom.rows.map((thread) => [thread.label, thread.state]));
}

/** The demo of a conductor once its workers stand: one done, one at work, one paused, and a question to the operator. */
async function settled(app: App): Promise<void> {
  await soon(() => {
    const now = states(app);
    return (
      now["w search"] === "done" &&
      now["w docs"] === "working" &&
      now["w gif"] === "paused" &&
      app.loom.owed.length === 1
    );
  });
}

test("the loom shows a conductor with its four workers under it, each in its state, and the operator owes a prompt", () =>
  looming(
    async ({ app, frame }) => {
      await settled(app);
      const shown = await frame();
      expect(shown).toMatch(/◆ 1\s+◉ \d/);
      expect(shown).toMatch(/◆ you +.*1 owed/);
      for (const row of [/✓ ├ w search/, /◉ ├ w docs|[⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏] ├ w docs/, /◌ └ w gif .* paused/])
        expect(shown).toMatch(row);
      expect(shown).toContain("waits on");
      expect(shown).toContain("now");
    },
    { seed: "conductor" },
  ));

test("Tab answers the oldest prompt that the operator owes, and the answer closes it", () =>
  looming(
    async ({ app, screen, session }) => {
      await settled(app);
      const [owed] = app.loom.owed;
      if (!owed) throw new Error("The operator owes no prompt.");
      screen.mockInput.pressTab();
      await soon(() => app.focus === "word");
      expect([app.mode, app.target]).toEqual(["answer", owed.id]);
      await screen.mockInput.typeText("high");
      screen.mockInput.pressEnter();
      await soon(() => Boolean(session.acts.find((act) => act.id === owed.id)?.done));
      app.render();
      expect(app.loom.owed).toEqual([]);
      expect(session.acts.find((act) => act.id === owed.id)?.value).toBe("high");
    },
    { seed: "conductor" },
  ));

test("i asks the model of the selected chain, and the thread shows the prompt, its rung, and what it closed with", () =>
  looming(async ({ app, screen, session, frame }) => {
    screen.mockInput.pressKey("i");
    await soon(() => app.focus === "word");
    await screen.mockInput.typeText("What is this project?");
    screen.mockInput.pressEnter();
    await soon(() => session.acts.some((act) => act.kind === "prompt"));
    await idle(session);
    const shown = await frame();
    expect(shown).toMatch(/✓ prompt\d+ +What is this project\?/);
    expect(shown).toMatch(/✓ rung\d+ +notes = read\("README\.md"\)/);
    expect(shown).toContain("closed  ## A clear starting point");
  }));

test(": runs a word of python as a rung on the selected chain", () =>
  looming(async ({ app, screen, session, frame }) => {
    await screen.mockInput.typeText(":");
    await soon(() => app.mode === "python");
    await screen.mockInput.typeText("answer = 6 * 7");
    screen.mockInput.pressEnter();
    await soon(() => session.acts.some((act) => act.kind === "rung" && act.done));
    expect((await session.engine.inspect("answer", session.selected)).value).toBe(42);
    expect(await frame()).toMatch(/✓ rung\d+ +answer = 6 \* 7/);
  }));

test("space pauses the selected chain and wakes it again, and x cancels its work", () =>
  looming(
    async ({ app, screen }) => {
      await settled(app);
      const docs = app.loom.rows.findIndex((thread) => thread.label === "w docs");
      await app.move(docs - app.selectedThread);
      expect(app.selection).toBe(app.loom.rows[docs]?.id ?? "");
      screen.mockInput.pressKey(" ");
      await soon(() => states(app)["w docs"] === "paused");
      screen.mockInput.pressKey(" ");
      await soon(() => states(app)["w docs"] === "working");
      screen.mockInput.pressKey("x");
      await soon(() => !["working", "paused"].includes(states(app)["w docs"] ?? ""));
    },
    { seed: "conductor" },
  ));

test("Enter opens the thread of a chain, then the lens of a line, and Esc goes back one level each time", () =>
  looming(
    async ({ app, screen, session, frame }) => {
      await idle(session);
      screen.mockInput.pressEnter();
      await soon(() => app.focus === "thread");
      const rung = app.lines.findIndex((line) => line.type === "rung");
      await app.move(rung - app.selectedLine);
      screen.mockInput.pressEnter();
      await soon(() => app.focus === "lens");
      const shown = await frame();
      expect(shown).toMatch(/rung\d+ +✓ +rung on Main, made by prompt\d+/);
      expect(shown).toContain('check = await bash("printf');
      screen.mockInput.pressEscape();
      await soon(() => app.focus === "thread");
      screen.mockInput.pressEscape();
      await soon(() => app.focus === "loom");
    },
    { seed: "demo" },
  ));

test("Left folds the workers of a conductor into its thread, which counts them by state, and Right shows them again", () =>
  looming(
    async ({ app, screen, session, frame }) => {
      await settled(app);
      const root = app.loom.rows.findIndex((thread) => thread.id === session.engine.root);
      await app.move(root - app.selectedThread);
      screen.mockInput.pressArrow("left");
      await soon(() => app.folds.has(session.engine.root));
      const shown = await frame();
      expect(shown).not.toContain("w docs");
      expect(shown).toMatch(/▸ Main 1✓ 1[◉] 1◌|▸ Main .*1◌/);
      screen.mockInput.pressArrow("right");
      await soon(() => !app.folds.has(session.engine.root));
      expect(await frame()).toContain("w docs");
    },
    { seed: "conductor" },
  ));

test("an empty life offers ways to start, and Enter on one puts its prompt in the word line", () =>
  looming(async ({ app, screen, frame }) => {
    const shown = await frame();
    expect(shown).toContain("Explore a codebase");
    expect(shown).toContain("Each act has a name, a state and a place in time.");
    screen.mockInput.pressEnter();
    await soon(() => app.focus === "thread");
    screen.mockInput.pressArrow("down");
    screen.mockInput.pressEnter();
    await soon(() => app.focus === "word");
    expect(app.word.plainText).toBe("Find one useful improvement in this project and implement it.");
  }));

test("⌃O lists the lives by workspace, and the help lists every key", () =>
  looming(async ({ app, screen, frame, quits }) => {
    screen.mockInput.pressKey("o", { ctrl: true });
    await soon(() => app.focus === "lives");
    const lives = await frame();
    expect(lives).toContain("Lives");
    expect(lives).toMatch(/▾ fieldnotes/);
    expect(lives).toContain("on screen");
    screen.mockInput.pressEscape();
    await soon(() => app.focus === "loom");
    await screen.mockInput.typeText("?");
    await soon(() => app.help);
    expect(await frame()).toContain("Fold the workers of a thread into it");
    screen.mockInput.pressKey("q", { ctrl: true });
    await soon(() => quits() === 1);
  }));

test("[ moves the time cursor into the past, where the loom shows the life as it stood, and b branches the chain there", () =>
  looming(
    async ({ app, screen, session, frame }) => {
      await idle(session);
      const prompt = session.acts.find((act) => act.kind === "prompt");
      if (!prompt?.ended) throw new Error("The demo made no prompt.");
      // The cursor walks back to the moment before the prompt ended, where its rung still runs.
      while (app.cursor === undefined || app.cursor >= prompt.ended) {
        await screen.mockInput.typeText("[");
        await soon(() => app.cursor !== undefined);
      }
      const shown = await frame();
      expect(shown).toMatch(/◷ \d+ of \d+/);
      expect(shown).toContain("then");
      expect(app.lines.find((line) => line.act.id === prompt.id)).toMatchObject({ state: "running" });
      screen.mockInput.pressKey("x");
      await soon(() => session.notice.startsWith("The loom shows the past"));
      const chains = session.acts.filter((act) => act.kind === "chain").length;
      screen.mockInput.pressKey("b");
      await soon(() => session.acts.filter((act) => act.kind === "chain").length === chains + 1);
      await soon(() => app.cursor === undefined && session.selected !== session.engine.root);
      expect(session.label).toBe("Main then");
    },
    { seed: "demo" },
  ));

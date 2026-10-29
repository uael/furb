import { expect, test } from "bun:test";
import { until } from "../../bind/typescript/test/until.ts";
import { composing } from "./composing.ts";

/** A question to the operator, once the console holds it and the session shows it. */
async function asked(
  { session }: { session: import("../src/session.ts").Session },
  shape: string,
  message: string,
  on = session.engine.root,
): Promise<string> {
  const id = await session.engine.prompt(shape, { message, to: "operator", on });
  await until(session.host, () => session.host.prompts.has(id));
  await session.refresh();
  return id;
}

/** Escape reaches the keys after the pause that a terminal takes to tell it from the start of a sequence. */
async function pressEsc(screen: { mockInput: { pressEscape(): void } }): Promise<void> {
  screen.mockInput.pressEscape();
  await new Promise((done) => setTimeout(done, 120));
}

test("Other of a choice opens a field for the words of the operator, and the values come back at Esc and at Up from an empty field", () =>
  composing(
    async (context) => {
      const { session, app, screen, frame } = context;
      const runner = await asked(context, "Literal['vitepress', 'mkdocs', 'plain']", "Which runner?");
      await frame();
      expect(app.composer.visible).toBe(false);
      screen.mockInput.pressKey("4");
      const typing = await frame();
      expect(app.composer.visible).toBe(true);
      expect(typing).toContain("❯ 4  Other  in your own words");
      expect(app.composer.hint).toBe(
        "Your own words in place of vitepress, mkdocs or plain. Esc goes back to the values.",
      );
      expect(typing).toContain("Esc back to the values");
      // An empty field sends nothing, and says what to do.
      screen.mockInput.pressEnter();
      await frame();
      expect(session.notice).toContain("Write your own words");
      expect(session.desk.asks.map((ask) => ask.id)).toEqual([runner]);
      // Esc goes back to the values and keeps the words.
      await screen.mockInput.typeText("Neither, ");
      await pressEsc(screen);
      expect(await frame()).toContain("❯ 4  Other");
      expect(app.composer.visible).toBe(false);
      screen.mockInput.pressEnter();
      expect(await frame()).toContain("Neither, ");
      expect(app.composer.visible).toBe(true);
      // Up from an empty field goes back to the values too.
      app.composer.setText("");
      screen.mockInput.pressArrow("up");
      await frame();
      expect(app.composer.visible).toBe(false);
      screen.mockInput.pressEnter();
      await frame();
      await screen.mockInput.typeText("Neither: keep the Markdown.");
      screen.mockInput.pressEnter();
      await expect(session.engine.result(runner)).rejects.toThrow("Neither: keep the Markdown.");
      await until(session, () => session.notice.startsWith(`Your words went to ${runner}`));
    },
    { width: 150, height: 40 },
  ));

test("a list of a Literal is ticked with Space and sent with Enter as a list, and Other still takes the words", () =>
  composing(
    async (context) => {
      const { session, app, screen, frame } = context;
      const flags = await asked(context, "list[Literal['fast', 'safe', 'quiet']]", "Which flags?");
      const listed = await frame();
      expect(listed).toContain("❯ 1  [ ] fast");
      expect(listed).toContain("  3  [ ] quiet");
      expect(listed).toContain("  4  Other  in your own words");
      expect(listed).toContain("wants any of fast, safe or quiet");
      expect(listed).toContain("Tick below with Space, send with Enter");
      expect(listed).toContain("Space tick");
      expect(listed).toContain("Enter send none");
      screen.mockInput.pressKey(" ");
      screen.mockInput.pressArrow("down");
      screen.mockInput.pressArrow("down");
      screen.mockInput.pressKey(" ");
      const ticked = await frame();
      expect(ticked).toContain("  1  [x] fast");
      expect(ticked).toContain("❯ 3  [x] quiet");
      expect(ticked).toContain("Enter send 2");
      // A number ticks its row too, and ticks it off again.
      screen.mockInput.pressKey("2");
      expect(await frame()).toMatch(/2 {2}\[x\] safe/);
      screen.mockInput.pressKey("2");
      expect(await frame()).toMatch(/2 {2}\[ \] safe/);
      screen.mockInput.pressEnter();
      expect(await session.engine.result(flags)).toEqual(["fast", "quiet"]);
      const again = await asked(context, "list[Literal['fast', 'safe']]", "Which flags again?");
      screen.mockInput.pressKey("3");
      await frame();
      expect(app.composer.visible).toBe(true);
      await screen.mockInput.typeText("None of them.");
      screen.mockInput.pressEnter();
      await expect(session.engine.result(again)).rejects.toThrow("None of them.");
    },
    { width: 132, height: 40 },
  ));

test("a question of a shape that is more than text says what its line is, and takes the line as a value of the shape", () =>
  composing(
    async (context) => {
      const { session, app, frame } = context;
      const names = await asked(context, "list[str]", "Name the files.");
      const shown = await frame();
      expect(shown).toContain("Enter sends the input to");
      expect(shown).toContain('text, as ["a", "b"]');
      expect(app.composer.hint).toContain(`Your answer to ${names}, as list[str]: a JSON list`);
      // A line that is no list of text is refused, and the question stays open.
      app.composer.setText("a, b");
      await app.submit();
      expect(session.desk.asks.map((ask) => ask.id)).toEqual([names]);
      expect(session.acts.find((act) => act.id === names)?.done).toBe(false);
      // The refusal says why, in a dialog, and the line goes back to the input.
      expect(await frame()).toContain('Refused: "a, b" is no list[str]');
      expect(app.composer.plainText).toBe("a, b");
      app.closeOverlay();
      app.composer.setText('["a.rs", "b.rs"]');
      await app.submit();
      expect(await session.engine.result(names)).toEqual(["a.rs", "b.rs"]);
      const count = await asked(context, "int | None", "How many retries?");
      await frame();
      expect(app.composer.hint).toBe(
        `Your answer to ${count}, as int | None: a whole number, or no word for nothing`,
      );
      app.composer.setText("3");
      await app.submit();
      expect(await session.engine.result(count)).toBe(3);
      const none = await asked(context, "int | None", "And now?");
      await frame();
      // A line of no word answers a union that holds None with None.
      app.composer.setText("");
      await app.submit();
      expect(await session.engine.result(none)).toBeNull();
    },
    { width: 120, height: 40 },
  ));

test("the words that the operator writes for a question stay with it when another question is aimed, and go to no other question", () =>
  composing(
    async (context) => {
      const { session, app, frame } = context;
      const one = await asked(context, "str", "First?");
      const two = await asked(context, "str", "Second?");
      await frame();
      expect(session.operatorPrompt?.id).toBe(one);
      app.composer.setText("answer one");
      await session.point(two);
      await frame();
      expect(session.operatorPrompt?.id).toBe(two);
      expect(app.composer.plainText).toBe("");
      app.composer.setText("answer two");
      await session.point(one);
      await frame();
      expect(app.composer.plainText).toBe("answer one");
      await app.submit();
      expect(await session.engine.result(one)).toBe("answer one");
      await frame();
      expect(session.operatorPrompt?.id).toBe(two);
      expect(app.composer.plainText).toBe("answer two");
      await app.submit();
      expect(await session.engine.result(two)).toBe("answer two");
    },
    { width: 120, height: 40 },
  ));

test("the words written for a question that another window answered go to the history of the input, and the next question takes none of them", () =>
  composing(
    async (context) => {
      const { session, app, frame } = context;
      const one = await asked(context, "str", "First?");
      const two = await asked(context, "str", "Second?");
      await frame();
      app.composer.setText("half written for the first");
      await session.host.answer(one, "from the other window");
      await until(session, () => session.operatorPrompt?.id === two);
      await frame();
      expect(app.composer.plainText).toBe("");
      expect(session.notice).toBe(`${one} closed. ↑ brings your words back.`);
      expect(session.histories[session.draftKey]).toContain("half written for the first");
      expect(session.desk.asks.map((ask) => ask.id)).toEqual([two]);
      await app.submit();
      expect(session.desk.asks.map((ask) => ask.id)).toEqual([two]);
    },
    { width: 120, height: 40 },
  ));

test("the footer offers ⌃C to take back your last message while it runs and the input is empty", () =>
  composing(
    async ({ session, app, frame }) => {
      expect(await frame()).not.toContain("take back");
      await session.submit("Think until I cancel.");
      await until(session, () => Boolean(session.lastOwn));
      expect(await frame()).toContain("⌃C take back");
      app.composer.setText("a new message");
      expect(await frame()).not.toContain("⌃C take back");
    },
    { width: 140, height: 44, exitOnCtrlC: false },
  ));

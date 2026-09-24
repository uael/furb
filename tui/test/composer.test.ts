import { afterAll, expect, test } from "bun:test";
import { createTestRenderer, setRendererCapabilities } from "@opentui/core/testing";
import { until } from "../../bind/typescript/test/until.ts";
import { App } from "../src/app.ts";
import { demoSession, removeDemoDirectories } from "../src/demo.ts";
import type { Session } from "../src/session.ts";

afterAll(removeDemoDirectories);

/** A demo session in an App on a test terminal, which a test uses and which ends after it. */
async function composing(
  use: (context: {
    session: Session;
    app: App;
    screen: Awaited<ReturnType<typeof createTestRenderer>>;
    frame: () => Promise<string>;
  }) => Promise<void>,
  options: Parameters<typeof createTestRenderer>[0] = { width: 120, height: 44 },
  seed = false,
): Promise<void> {
  const session = await demoSession(seed);
  const screen = await createTestRenderer(options);
  const app = new App(screen.renderer, session, { quit() {} });
  const frame = async () => {
    app.render();
    await screen.flush();
    return screen.captureCharFrame();
  };
  try {
    await screen.flush();
    app.composer.focus();
    await use({ session, app, screen, frame });
  } finally {
    app.dispose();
    screen.renderer.destroy();
    await session.dispose();
  }
}

test("a terminal with no kitty keyboard protocol reaches each action by a chord it sends, and the help names those chords", () =>
  composing(
    async ({ session, app, screen, frame }) => {
      screen.mockInput.pressKey("3", { meta: true });
      expect(session.view).toBe("changes");
      screen.mockInput.pressKey("1", { meta: true });
      expect(session.view).toBe("feed");
      screen.mockInput.pressKey("m", { meta: true });
      expect(await frame()).toContain("The chain sends its next prompt to this model.");
      app.closeOverlay();
      screen.mockInput.pressKey("r", { ctrl: true });
      expect(session.mode).toBe("python");
      await frame();
      await screen.mockInput.typeText('x = "你好"');
      screen.mockInput.pressKey("j", { ctrl: true });
      await screen.mockInput.typeText("if x:");
      screen.mockInput.pressKey("j", { ctrl: true });
      expect(app.composer.plainText).toBe('x = "你好"\nif x:\n  ');
      screen.mockInput.pressKey("F1");
      let help = await frame();
      expect(help).toContain("Alt+1 through Alt+3");
      expect(help).not.toContain("Ctrl+1 through Ctrl+3");
      expect(help).toContain("Alt+M");
      app.closeOverlay();
      setRendererCapabilities(screen.renderer, { kitty_keyboard: true });
      app.help();
      help = await frame();
      expect(help).toContain("Ctrl+1 through Ctrl+3 or Alt+1 through Alt+3");
      expect(help).toContain("Shift+Enter or Ctrl+J");
    },
    undefined,
    true,
  ));

test("a text that a command puts in the composer keeps the draft one undo away", () =>
  composing(async ({ app, screen, frame }) => {
    await screen.mockInput.typeText("a long question I was writing");
    screen.mockInput.pressKey("n", { ctrl: true });
    expect(app.composer.plainText).toBe("/chain ");
    expect(app.composer.undo()).toBe(true);
    expect(app.composer.plainText).toBe("a long question I was writing");
    app.palette();
    await screen.mockInput.typeText("budget");
    await frame();
    screen.mockInput.pressEnter();
    await frame();
    expect(app.composer.plainText).toBe("/grant ");
    app.composer.undo();
    expect(app.composer.plainText).toBe("a long question I was writing");
  }));

test("Alt+Enter queues a message, and leaves Python input and a program under edit in the composer", () =>
  composing(
    async ({ session, app, screen }) => {
      app.toggleMode();
      await screen.mockInput.typeText("queued_value = 7");
      screen.mockInput.pressEnter({ meta: true });
      expect(app.composer.plainText).toBe("queued_value = 7");
      expect(session.queued).toEqual([]);
      expect(session.notice).toContain("Only a message can wait in the queue.");
      await session.command("/edit");
      const program = app.composer.plainText;
      expect(session.editing).toBeDefined();
      screen.mockInput.pressEnter({ meta: true });
      expect(app.composer.plainText).toBe(program);
      expect(session.queued).toEqual([]);
      screen.mockInput.pressEscape();
      app.toggleMode();
      app.render();
      app.composer.setText("a follow-up");
      screen.mockInput.pressEnter({ meta: true });
      expect(app.composer.plainText).toBe("");
      await until(session, () =>
        session.acts.some((act) => act.kind === "prompt" && act.words[1] === "a follow-up"),
      );
    },
    { width: 120, height: 44, kittyKeyboard: true },
    true,
  ));

test("a command that the text names whole comes first among its suggestions", () =>
  composing(
    async ({ session, app, screen, frame }) => {
      await screen.mockInput.typeText("/edit");
      const rows = (await frame()).split("\n").filter((line) => /\/edit/.test(line) && !line.includes("│"));
      expect(rows[0]).toContain("/edit [prompt id]");
      expect(rows[1]).toContain("/editor");
      screen.mockInput.pressTab();
      expect(app.composer.plainText).toBe("/edit ");
      app.composer.setText("");
      await screen.mockInput.typeText("/edit");
      await frame();
      screen.mockInput.pressEnter();
      await until(session, () => session.editing !== undefined);
      expect(app.composer.plainText).toContain("close(");
    },
    undefined,
    true,
  ));

test("a sent text leaves its draft at once, and a program under edit opens from its door the next time", () =>
  composing(
    async ({ session, app, screen, frame }) => {
      // /edit opens the latest prompt with a program, so the demo settles first: its last prompt may still run.
      await until(
        session,
        () => !session.activity.some((act) => !act.done && ["prompt", "rung"].includes(act.kind)),
      );
      const prompt = session.draftKey;
      await screen.mockInput.typeText("/edit");
      await frame();
      screen.mockInput.pressEnter();
      await until(session, () => session.editing !== undefined);
      const edited = session.draftKey;
      app.composer.setText('close("edited once")');
      await app.submit();
      expect(session.editing).toBeUndefined();
      await frame();
      expect(session.drafts[prompt]).toBeUndefined();
      expect(session.drafts[edited]).toBeUndefined();
      expect(session.histories[edited]).toEqual(['close("edited once")']);
      app.composer.setText("/edit");
      await app.submit();
      expect(app.composer.plainText).toBe('close("edited once")');
      screen.mockInput.pressEscape();
      await frame();
      expect(app.composer.plainText).toBe("");
      // A draft typed while the text is sent stays in the composer, and the sent text enters the history of its draft.
      const submit = session.submit.bind(session);
      let release = () => {};
      const gate = new Promise<void>((resolve) => {
        release = resolve;
      });
      session.submit = async (input) => {
        await gate;
        return submit(input);
      };
      app.composer.setText("a question");
      const sending = app.submit();
      expect(app.composer.plainText).toBe("");
      await screen.mockInput.typeText("the next one");
      release();
      await sending;
      session.submit = submit;
      expect(app.composer.plainText).toBe("the next one");
      expect(session.histories[prompt]).toEqual(["/edit", "a question"]);
      // A text that is not sent goes back to its draft.
      app.composer.setText("");
      app.composer.setText("/model nothing");
      await app.submit();
      app.closeOverlay();
      expect(app.composer.plainText).toBe("/model nothing");
    },
    { width: 120, height: 44, kittyKeyboard: true },
    true,
  ));

test("the completion of a token reads the text before the cursor, whatever the width of the characters before it", () =>
  composing(async ({ session, app, screen, frame }) => {
    await session.projectFiles();
    // A combining mark is inserted with the letter it marks, as a paste or an input method gives it.
    app.composer.insertText("Compare cafe\u0301 with ");
    await screen.mockInput.typeText("@READ");
    await frame();
    screen.mockInput.pressTab();
    expect(app.composer.plainText).toBe("Compare cafe\u0301 with @README.md ");
    app.composer.setText("");
    app.composer.insertText("总结 @READ 的内容");
    for (let step = 0; step < 4; step++) screen.mockInput.pressArrow("left");
    expect(await frame()).toContain("@README.md");
    screen.mockInput.pressTab();
    expect(app.composer.plainText).toBe("总结 @README.md  的内容");
  }));

test("a click on a suggestion completes the token as it stands when it is clicked", () =>
  composing(
    async ({ session, app, screen, frame }) => {
      await session.projectFiles();
      for (const [typed, more, completed] of [
        ["/rewi", "n", "/rewind "],
        ["Read @READ", "M", "Read @README.md "],
      ] as const) {
        app.composer.setText("");
        await screen.mockInput.typeText(typed);
        await frame();
        await screen.mockInput.typeText(more);
        const lines = (await frame()).split("\n");
        const label = completed.trim().split(" ").at(-1) ?? "";
        const row = lines.findIndex((line) => line.includes(label) && !line.includes("│"));
        await screen.mockMouse.click((lines[row] ?? "").indexOf(label) + 1, row);
        expect(app.composer.plainText).toBe(completed);
      }
    },
    { width: 120, height: 44, useMouse: true },
  ));

test("a command that opens a picker opens it in the view, and the session refuses it with no argument", () =>
  composing(async ({ session, app, frame }) => {
    app.composer.setText("/model");
    await app.submit();
    expect(await frame()).toContain("The chain sends its next prompt to this model.");
    app.closeOverlay();
    expect(String(await session.submit("/model").catch((error: unknown) => error))).toContain(
      "Use /model followed by a model name.",
    );
    expect((await frame()).split("\n").findLast((line) => line.trim())).toContain("Ready");
  }));

test("a slash command typed under edit runs as the command it names, and the program stays as its door holds it", () =>
  composing(
    async ({ session, app }) => {
      app.composer.setText("/edit");
      await app.submit();
      await until(session, () => session.editing !== undefined);
      const editing = session.editing;
      if (!editing) throw new Error("No program under edit.");
      const program = app.composer.plainText;
      app.composer.setText("/name renamed");
      await app.submit();
      expect(session.sessionName).toBe("renamed");
      expect(session.editing).toBe(editing);
      const door = (await session.life.read(editing, undefined, session.selected)) as { content: string };
      expect(door.content).toBe(program);
    },
    undefined,
    true,
  ));

test("Up and Down walk the history from the edges of the input, and Up on an empty input takes back a queued message", () =>
  composing(
    async ({ session, app, screen }) => {
      for (const name of ["first", "second"]) {
        app.composer.setText(`/name ${name}`);
        await app.submit();
      }
      app.composer.setText("a draft");
      screen.mockInput.pressArrow("up");
      expect(app.composer.plainText).toBe("/name second");
      screen.mockInput.pressArrow("up");
      expect(app.composer.plainText).toBe("/name first");
      screen.mockInput.pressArrow("down");
      expect(app.composer.plainText).toBe("/name second");
      screen.mockInput.pressArrow("down");
      expect(app.composer.plainText).toBe("a draft");
      app.composer.setText("");
      session.queueHeld = true;
      session.enqueue("a queued message");
      expect(session.queued).toHaveLength(1);
      screen.mockInput.pressArrow("up");
      expect(app.composer.plainText).toBe("a queued message");
      expect(session.queued).toEqual([]);
    },
    { width: 120, height: 44, kittyKeyboard: true },
  ));

test("Ctrl+S puts the input aside, the line under the input shows it, and Ctrl+S brings it back", () =>
  composing(async ({ session, app, screen, frame }) => {
    await screen.mockInput.typeText("a long draft");
    screen.mockInput.pressKey("s", { ctrl: true });
    expect(app.composer.plainText).toBe("");
    expect(session.stashes[session.draftKey]).toBe("a long draft");
    expect(await frame()).toContain("stashed");
    await screen.mockInput.typeText("a quick question");
    screen.mockInput.pressKey("s", { ctrl: true });
    expect(app.composer.plainText).toBe("a long draft");
    expect(session.stashes[session.draftKey]).toBe("a quick question");
    app.composer.setText("");
    screen.mockInput.pressKey("s", { ctrl: true });
    expect(app.composer.plainText).toBe("a quick question");
    expect(session.stashes[session.draftKey]).toBeUndefined();
    expect(await frame()).not.toContain("stashed");
  }));

test("Escape twice opens the rewind tree in the feed, which keys move, fold, and close", () =>
  composing(
    async ({ session, app, screen, frame }) => {
      await until(
        session,
        () => !session.activity.some((act) => !act.done && ["prompt", "rung"].includes(act.kind)),
      );
      await frame();
      screen.mockInput.pressEscape();
      expect(await frame()).toContain("Press Escape again to rewind.");
      screen.mockInput.pressEscape();
      const tree = await frame();
      expect(tree).toContain("Rewind");
      const rows = () => app.scroll.getChildren().map((node) => node.id);
      expect(rows().every((id) => id.startsWith("tree-"))).toBe(true);
      const last = session.activity.findLast((act) => session.isUserPrompt(act));
      // The pointer starts on the last message, and Enter would give it back on a new branch.
      expect(tree).toContain("edit it on a new branch");
      const before = rows().length;
      screen.mockInput.pressArrow("up");
      screen.mockInput.pressArrow("up");
      expect(await frame()).not.toContain("edit it on a new branch");
      // Home goes to the chain at the top, and Left folds it.
      screen.mockInput.pressKey("home");
      screen.mockInput.pressArrow("left");
      await frame();
      expect(rows().length).toBeLessThan(before);
      screen.mockInput.pressArrow("right");
      await frame();
      expect(rows().length).toBe(before);
      screen.mockInput.pressEscape();
      await frame();
      expect(rows().some((id) => id.startsWith("tree-"))).toBe(false);
      expect(session.selected).toBe(last?.on ?? "");
    },
    { width: 120, height: 44, kittyKeyboard: true },
    true,
  ));

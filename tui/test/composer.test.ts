import { afterAll, expect, setDefaultTimeout, test } from "bun:test";
import { createTestRenderer, setRendererCapabilities } from "@opentui/core/testing";
import { until } from "../../bind/typescript/test/until.ts";
import { App } from "../src/app.ts";
import { demoSession, removeDemoDirectories } from "../src/demo.ts";
import type { Session } from "../src/session.ts";

// Each test opens a demo session, with its worker and its record, as the tests of the App do, and so has their time:
// a Windows runner has run such a test four times slower than usual, past the five seconds that bun gives by default.
setDefaultTimeout(30000);
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
      expect(help).toContain("⌥1-3");
      expect(help).not.toContain("⌃1-3");
      expect(help).toContain("⌥M");
      app.closeOverlay();
      setRendererCapabilities(screen.renderer, { kitty_keyboard: true });
      app.help();
      help = await frame();
      expect(help).toContain("⌃1-3 or ⌥1-3");
      expect(help).toContain("⇧Enter or ⌃J");
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
      const door = (await session.engine.read(editing, { on: session.selected })) as { content: string };
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

test("a space after a command whose values are known lists them, and Enter on a value runs the command", () =>
  composing(async ({ session, app, screen, frame }) => {
    await screen.mockInput.typeText("/effort ");
    let shown = await frame();
    expect(shown).toContain("Quick answers to simple work");
    expect(shown).toContain("A balance of speed and depth");
    await screen.mockInput.typeText("hi");
    shown = await frame();
    expect(shown).toContain("More thought for hard work");
    expect(shown).not.toContain("A balance of speed and depth");
    screen.mockInput.pressEnter();
    await until(session, () => session.actorChoice.effort === "high");
    expect(app.composer.plainText).toBe("");
    // A value that no known value holds is sent as it is typed.
    await screen.mockInput.typeText("/theme nothing");
    expect(await frame()).toContain("No known value matches. Enter sends what is typed.");
  }));

test("⌃Tab rolls to the next chain and ⇧⌃Tab to the one before it, round from the last to the first", () =>
  composing(
    async ({ session, screen, frame }) => {
      await session.refresh();
      const chains = session.chains.map((chain) => chain.id);
      expect(chains.length).toBeGreaterThan(1);
      const first = session.selected;
      screen.mockInput.pressKey("TAB", { ctrl: true });
      await until(session, () => session.selected === chains[1]);
      await until(session, () => session.notice.endsWith(`chain 2 of ${chains.length}`));
      expect(await frame()).toContain(`chain 2 of ${chains.length}`);
      screen.mockInput.pressKey("TAB", { ctrl: true, shift: true });
      await until(session, () => session.selected === first);
      screen.mockInput.pressKey("TAB", { ctrl: true, shift: true });
      await until(session, () => session.selected === chains.at(-1));
    },
    { width: 120, height: 44, kittyKeyboard: true },
    true,
  ));

test("the switch of the views fills the view shown with the accent, names its chord, and a click on a view shows it", () =>
  composing(async ({ session, screen, frame }) => {
    const top = (await frame()).split("\n")[0] ?? "";
    expect(top).toContain("Feed");
    expect(top).toContain("⌥1-3");
    await screen.mockMouse.click(top.indexOf("Changes") + 1, 0);
    await until(session, () => session.view === "changes");
    const again = (await frame()).split("\n")[0] ?? "";
    await screen.mockMouse.click(again.indexOf("Feed") + 1, 0);
    await until(session, () => session.view === "feed");
  }));

test("the switch of the input shows Prompt and Python, ⌃R names it, and a click on a mode chooses it", () =>
  composing(async ({ session, screen, frame }) => {
    const lines = (await frame()).split("\n");
    const row = lines.findIndex((line) => line.includes("Python") && line.includes("Prompt"));
    expect(row).toBeGreaterThanOrEqual(0);
    expect(lines[row]).toContain("⌃R");
    await screen.mockMouse.click((lines[row] ?? "").indexOf("Python") + 1, row);
    await until(session, () => session.mode === "python");
    const after = (await frame()).split("\n");
    const again = after.findIndex((line) => line.includes("Python") && line.includes("Prompt"));
    await screen.mockMouse.click((after[again] ?? "").indexOf("Prompt") + 1, again);
    await until(session, () => session.mode === "prompt");
  }));

test("the meter of the context says its share and where the chain pauses when the pointer is over it", () =>
  composing(
    async ({ session, screen, frame }) => {
      await session.submit("/context 0.8");
      await until(session, () =>
        session.activity.some((act) => act.kind === "grant" && act.words[1] === 0.8),
      );
      const lines = (await frame()).split("\n");
      expect(lines.some((line) => line.includes("Pause at"))).toBe(false);
      const row = lines.findIndex((line) => line.includes("━"));
      expect(row).toBeGreaterThan(0);
      await screen.mockMouse.moveTo((lines[row] ?? "").indexOf("━") + 2, row);
      const tip = await frame();
      expect(tip).toContain("of the context window");
      expect(tip).toContain("pauses at 80%");
    },
    { width: 120, height: 44 },
    true,
  ));

test("the usage counts each token once, and the context share shows from the first answer with no grant", () =>
  composing(async ({ session, frame }) => {
    // Each answer of the demo reads a prompt of 3240 tokens, 2800 of them from the cache, and writes 184.
    await session.submit("Explore this project.");
    await until(session, () => session.turns.filter((turn) => turn[0] === "assistant").length >= 2);
    await session.refresh();
    const answers = session.turns.filter((turn) => turn[0] === "assistant").length;
    expect(session.spend).toEqual({
      input: 440 * answers,
      output: 184 * answers,
      cacheRead: 2800 * answers,
      cacheWrite: 0,
      dollars: expect.closeTo(0.0024 * answers, 6),
    });
    expect(session.activity.some((act) => act.kind === "grant")).toBe(false);
    expect(session.filled).toBe(3240 / 1_000_000);
    expect(session.context).toBe(3240);
    const shown = await frame();
    expect(shown).toContain("0.3%");
    expect(shown).toContain("Cache read");
    expect(shown).not.toContain("Cached");
    expect(shown).toContain("━");
  }));

test("a paused chain says so at the end of its feed and in the footer, and Resume wakes it", () =>
  composing(async ({ session, screen, frame }) => {
    await session.submit("/pause");
    await until(session, () => session.paused);
    let shown = await frame();
    expect(shown).toContain("This chain is paused");
    expect(shown).toContain("/wake resume");
    const lines = shown.split("\n");
    const row = lines.findIndex((line) => line.includes("Resume") && line.includes("runs what waits"));
    await screen.mockMouse.click((lines[row] ?? "").indexOf("Resume") + 1, row);
    await until(session, () => !session.paused);
    shown = await frame();
    expect(shown).not.toContain("This chain is paused");
  }));

test("a message sent to a paused chain wakes it, and the answer comes", () =>
  composing(async ({ session, app }) => {
    await session.submit("/pause");
    await until(session, () => session.paused);
    app.composer.setText("Explore this project.");
    await app.submit();
    expect(session.paused).toBe(false);
    expect(session.notice).toBe("The paused chain resumed with this message.");
    await until(session, () => session.turns.some((turn) => turn[0] === "assistant"));
  }));

test("a notice cut to the room of the footer shows whole in a tip while the pointer is over it", () =>
  composing(async ({ session, screen, frame }) => {
    session.notice = `A notice longer than its line ${"and longer ".repeat(12)}to its very end`;
    const lines = (await frame()).split("\n");
    const row = lines.findIndex((line) => line.includes("A notice longer"));
    expect(lines[row]).not.toContain("to its very end");
    await screen.mockMouse.moveTo((lines[row] ?? "").indexOf("A notice") + 2, row);
    expect(await frame()).toContain("to its very end");
  }));

test("a cancelled message keeps its place in the feed, and its cancel reads as a cancel, not as a failure", () =>
  composing(async ({ session, app, frame }) => {
    app.composer.setText("show live progress");
    await app.submit();
    await until(session, () => session.activity.some((act) => act.kind === "rung" && !act.done));
    await session.submit("/cancel");
    await until(
      session,
      () => !session.activity.some((act) => !act.done && ["prompt", "rung"].includes(act.kind)),
    );
    app.composer.setText("A second message.");
    await app.submit();
    await until(session, () => session.turns.some((turn) => turn[0] === "assistant"));
    await session.refresh();
    const shown = await frame();
    expect(shown).not.toContain("failed");
    expect(shown).not.toContain("CancelledError");
    // The first message says its cancel at its right, above the second message, and the rung that the cancel ended
    // before it wrote a word shows nothing.
    const lines = shown.split("\n");
    const first = lines.findIndex((line) => line.includes("show live progress"));
    expect(lines[first]).toContain("⊘ cancelled");
    expect(first).toBeLessThan(lines.findIndex((line) => line.includes("A second message.")));
    expect(lines.filter((line) => line.includes(" cancelled") && line.includes("⊘"))).toHaveLength(1);
    // A message that its answer closed says no state, only the type of its answer.
    expect(lines.find((line) => line.includes("A second message."))).not.toContain("✓");
  }));

test("a word that the gate refused and that a later word of the same prompt replaced folds, and reads as retried", () =>
  composing(
    async ({ session, frame }) => {
      await session.submit("Give me an answer in a fence.");
      await until(session, () =>
        session.turns.some((turn) => turn[0] === "assistant" && turn[1].includes("plain Python")),
      );
      await until(
        session,
        () => !session.activity.some((act) => !act.done && ["prompt", "rung"].includes(act.kind)),
      );
      await session.refresh();
      const shown = await frame();
      expect(shown).toMatch(/▸ ✗ rung\d+ .*retried as rung\d+/);
      expect(shown).not.toContain("failed");
      expect(shown).not.toContain("Got unexpected token");
    },
    { width: 140, height: 44 },
    true,
  ));

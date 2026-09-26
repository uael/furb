import { afterAll, expect, setSystemTime, test } from "bun:test";
import { readFile, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { createTestRenderer } from "@opentui/core/testing";
import { until } from "../../bind/typescript/test/until.ts";
import { App, sessionDetail } from "../src/app.ts";
import { openEngine } from "../src/bridge.ts";
import { demoLibrary, demoSession, removeDemoDirectories, seedDemo } from "../src/demo.ts";
import { Preferences } from "../src/preferences.ts";
import { Session } from "../src/session.ts";
import { type SessionEntry, Workspaces } from "../src/workspaces.ts";
import { composing } from "./composing.ts";
import { idle } from "./idle.ts";
import { transcriptOf } from "./transcript.ts";

afterAll(removeDemoDirectories);

test("the real native life drives the feed, the transcript, the palette, and responsive views", () =>
  composing(
    async ({ session, app, screen }) => {
      expect(screen.captureCharFrame()).toContain("Explore a codebase");
      expect(session.theme).toBe("github");
      expect(app.scroll.x).toBe(2);
      expect(app.scroll.height).toBeGreaterThanOrEqual(36);
      expect(app.scroll.height).toBeLessThanOrEqual(38);
      app.composer.setText("first line\nsecond line");
      app.render();
      await screen.flush();
      expect(app.composer.height).toBeGreaterThanOrEqual(2);
      app.composer.setText("");
      expect(screen.captureCharFrame()).not.toContain("No budget set");
      expect(screen.captureCharFrame()).not.toContain("Session saved");
      await seedDemo(session);
      app.render();
      await screen.flush();
      // The answer of a prompt stands under the name of the model that gave it, once its markdown is drawn.
      expect(screen.captureCharFrame()).toContain("● sonnet");
      for (
        let pass = 0;
        pass < 100 && !screen.captureCharFrame().includes("A clear starting point");
        pass++
      ) {
        await new Promise((done) => setTimeout(done, 20));
        app.render();
        await screen.flush();
      }
      const conversation = screen.captureCharFrame();
      expect(conversation.indexOf("Explore this project")).toBeLessThan(
        conversation.indexOf("A clear starting point"),
      );
      // A command that is over folds to its heading, and its output shows once it opens.
      expect(conversation).not.toContain("└ ✓ capture");
      const command = app.scroll.getChildren().find((node) => /^bash\d+$/.test(node.id));
      const heading = command?.getChildren()[0];
      if (!heading) throw new Error("No command heading.");
      await screen.mockMouse.click(heading.x, heading.y);
      await screen.flush();
      expect(screen.captureCharFrame()).toContain("exit 0");
      expect(screen.captureCharFrame()).toContain("✓ local storage");
      const expanded = app.scroll
        .getChildren()
        .find((node) => node.id === command?.id)
        ?.getChildren()[0];
      expect(expanded?.visible).toBe(true);
      if (expanded) await screen.mockMouse.click(expanded.x, expanded.y);
      await screen.flush();
      expect(screen.captureCharFrame()).not.toContain("exit 0");
      expect(app.scroll.getChildren().some((child) => /^bash\d+$/.test(child.id))).toBe(true);
      expect(screen.captureCharFrame()).not.toContain("failed");
      session.show("transcript");
      app.render();
      await screen.flush();
      expect(screen.captureCharFrame()).toContain('notes = read("README.md")');
      session.show("feed");
      app.palette();
      await screen.flush();
      expect(screen.captureCharFrame()).toContain("Commands");
      await screen.mockInput.typeText("budget");
      await screen.flush();
      expect(screen.captureCharFrame()).toContain("Set budget");
      screen.mockInput.pressEnter();
      await screen.flush();
      expect(app.composer.plainText).toBe("/grant ");
      screen.resize(80, 30);
      app.render();
      await screen.flush();
      // A narrow top line keeps the session, the chain, and the switch of the views, and the sidebar is hidden.
      const top = screen.captureCharFrame().split("\n")[0] ?? "";
      expect(top).toContain(`${session.sessionName} › Main`);
      expect(top).toContain("Transcript");
      expect(screen.captureCharFrame()).not.toContain("Chains");
    },
    { width: 145, height: 45, useMouse: true },
  ));

test("model and effort change independently, a model is named by its id alone, and a theme preference applies to a new session", async () => {
  const first = await demoSession();
  let second: Session | undefined;
  let recovered: Session | undefined;
  try {
    await first.submit("/effort high");
    expect(first.actor).toBe("claude-cli:sonnet/high");
    await first.submit("/model claude-cli:opus");
    expect(first.actor).toBe("claude-cli:opus/high");
    expect(first.roster.map(([name]) => name)).toContain("claude-cli:fable");
    await first.submit("/model fable");
    expect(first.actor).toBe("claude-cli:fable/high");
    await first.submit("/model opus");
    expect(first.actor).toBe("claude-cli:opus/high");
    // On bun 1.3.14 and 1.4.2, rejects hangs on a rejection that a message of the worker brings, so the test
    // catches it.
    expect(String(await first.submit("/model nothing").catch((error: unknown) => error))).toContain(
      "Choose one of",
    );
    await first.submit("/effort low");
    expect(first.actor).toBe("claude-cli:opus/low");
    await first.submit("/theme paper");
    second = await demoSession(false, new Preferences(first.preferences.path));
    expect(second.theme).toBe("paper");
    expect(second.host.record).not.toBe(first.host.record);
    expect(first.preferences.path).toBe(join(dirname(first.host.record ?? ""), "ui-preferences.json"));
    first.roster.push(["claude-cli:org/plain", [], 1000], ["claude-cli:org/high", [], 1000]);
    first.actor = "claude-cli:org/plain";
    expect(first.actorChoice).toEqual({ model: "claude-cli:org/plain", effort: "off" });
    first.actor = "claude-cli:org/high";
    expect(first.actorChoice).toEqual({ model: "claude-cli:org/high", effort: "off" });
    await writeFile(first.preferences.path, "{");
    recovered = await demoSession(false, new Preferences(first.preferences.path));
    expect(recovered.theme).toBe("github");
    expect(recovered.preferences.notice).toContain("Could not read preferences");
    expect(await readFile(first.preferences.path, "utf8")).toBe("{");
  } finally {
    await recovered?.dispose();
    await second?.dispose();
    await first.dispose();
  }
});

test("refreshes keep content in place, act failures stay in the record, and hidden scrollbars still scroll", () =>
  composing(
    async ({ session, app, screen }) => {
      const snapshot = session.host.snapshot.bind(session.host);
      let release = () => {};
      try {
        await idle(session);
        await session.refresh();
        app.render();
        await screen.flush();
        const before = app.scroll.getChildren().map((node) => [node.id, node.y]);
        const gate = new Promise<void>((resolve) => {
          release = resolve;
        });
        session.host.snapshot = async (chain) => {
          await gate;
          return snapshot(chain);
        };
        const reading = session.refresh();
        app.render();
        await screen.flush();
        expect(screen.captureCharFrame()).not.toContain("Loading conversation...");
        expect(app.scroll.getChildren().map((node) => [node.id, node.y])).toEqual(before);
        release();
        await reading;
        session.host.snapshot = snapshot;

        await session.submit("/read missing-review-file.txt");
        await session.refresh();
        app.render();
        await screen.flush();
        expect(session.error).toBe("");
        expect(screen.captureCharFrame()).not.toContain("Refresh view");
        expect(
          session.activity.some(
            (act) => act.done && act.kind === "rung" && JSON.stringify(act.value).includes("Refused"),
          ),
        ).toBe(true);

        session.show("transcript");
        await session.refresh();
        app.render();
        await screen.flush();
        expect(app.scroll.scrollHeight).toBeGreaterThan(app.scroll.height);
        expect(app.scroll.verticalScrollBar.visible).toBe(false);
        app.scroll.scrollBy(5);
        await screen.flush();
        expect(app.scroll.scrollTop).toBeGreaterThan(0);

        session.host.snapshot = async () => {
          throw new Error("Snapshot unavailable");
        };
        await session.refresh().catch(session.fail);
        app.render();
        await screen.flush();
        app.scroll.scrollTo(0);
        await screen.flush();
        expect(screen.captureCharFrame()).toContain("Snapshot unavailable");
        expect(screen.captureCharFrame()).toContain("Refresh view");
      } finally {
        release();
        session.host.snapshot = snapshot;
      }
    },
    { width: 120, height: 40 },
    true,
  ));

test("every view shows an empty result, loading, and an error in its feed", () =>
  composing(
    async ({ session, app, screen }) => {
      for (const view of ["feed", "transcript", "changes"] as const) {
        session.show(view);
        await session.refresh();
        session.search = "nothing matches this";
        app.render();
        await screen.flush();
        expect(screen.captureCharFrame()).toContain("Nothing matches “nothing matches this”");
        const snapshot = session.host.snapshot.bind(session.host);
        let release = () => {};
        const gate = new Promise<void>((resolve) => {
          release = resolve;
        });
        session.host.snapshot = async (chain) => {
          await gate;
          return snapshot(chain);
        };
        const loading = session.refresh();
        app.render();
        await screen.flush();
        expect(screen.captureCharFrame()).toContain(`Loading the ${view}`);
        release();
        await loading;
        session.host.snapshot = snapshot;
        session.fail(new Error(`Could not load ${view}`));
        app.render();
        await screen.flush();
        expect(screen.captureCharFrame()).toContain(`Could not load ${view}`);
        expect(screen.captureCharFrame()).toContain("Refresh view");
        session.error = "";
      }
    },
    { width: 120, height: 42 },
  ));

test("operator answers and program edits act through the binding", () =>
  composing(
    async ({ session, app, screen }) => {
      const id = await session.engine.prompt("bool", {
        message: "Continue with the change?",
        to: "operator",
        on: session.engine.root,
      });
      await until(session.host, () => session.host.prompts.has(id));
      await session.refresh();
      app.render();
      await screen.flush();
      expect(screen.captureCharFrame()).toContain("Continue with the change?");
      await session.submit("yes");
      expect(await session.engine.result(id)).toBe(true);
      await seedDemo(session);
      await session.command("/edit");
      expect(app.composer.plainText).toContain("close(");
      await session.submit('close("Edited answer")');
      expect(session.editing).toBeUndefined();
      await session.command("/run this is invalid python !!!");
      expect(session.error).toBe("");
      expect(session.findings.join("\n")).toContain("line 1");
      app.render();
      await screen.flush();
      expect(screen.captureCharFrame()).toContain("this is invalid python");
      app.composer.setText("keep the session open");
      screen.mockInput.pressCtrlC();
      await screen.flush();
      expect(app.composer.plainText).toBe("");
    },
    { width: 120, height: 40, exitOnCtrlC: false },
  ));

test("resume preserves chains, programs, theme, and input drafts while unfinished work stays paused", async () => {
  const first = await demoSession(true);
  let library = await demoLibrary(first);
  const screen = await createTestRenderer({ width: 120, height: 40 });
  const app = new App(screen.renderer, first, { quit() {}, workspaces: library });
  const fork = first.chains.find((chain) => chain.id !== first.engine.root);
  if (!fork) throw new Error("No fork in the fixture.");
  await first.select(fork.id);
  first.mode = "python";
  first.theme = "paper";
  first.view = "transcript";
  app.render();
  app.composer.setText('draft = "keep this"');
  const pending = await first.engine.wait({ seconds: 10, on: fork.id });
  const ids = first.chains.map((chain) => chain.id);
  const program = { ...first.program };
  const record = first.host.record;
  app.dispose();
  screen.renderer.destroy();
  await library.dispose();
  if (!record) throw new Error("No saved record.");
  library = new Workspaces(first.preferences, { demo: true });
  await library.refresh();
  // The replay of a saved record comes after the list, and the picker shows its state once it lands.
  await until(library, () => library.groups[0]?.sessions[0]?.status === "paused");
  const saved = library.groups[0]?.sessions ?? [];
  expect(saved).toHaveLength(1);
  const detail = sessionDetail(saved[0] as SessionEntry, false);
  expect(detail).toContain("KiB");
  expect(detail).toContain("Paused");
  await library.dispose();
  const opened = await openEngine({ record, demo: true });
  const second = new Session(opened.engine, opened.host, true);
  await second.refresh();
  await composing(
    async ({ app }) => {
      expect(second.chains.map((chain) => chain.id)).toEqual(ids);
      expect(second.program).toEqual(program);
      expect(second.selected).toBe(fork.id);
      expect(second.theme).toBe("paper");
      expect(second.mode).toBe("python");
      expect(app.composer.plainText).toBe('draft = "keep this"');
      expect(second.host.pending.has(pending)).toBe(true);
      expect((await second.engine.outcome(pending)).done).toBe(false);
    },
    { width: 120, height: 40 },
    second,
  );
});

test("rewind is a recorded rung and keeps the selected transcript after reopening", async () => {
  let rewound = "";
  let transcript: string[] = [];
  let record: string | undefined;
  await composing(
    async ({ session, app, screen }) => {
      const source = session.selected;
      await session.submit("/pause");
      const pausedActs = session.activity.map((act) => act.id);
      app.rewind();
      app.rewind();
      await screen.flush();
      expect(screen.captureCharFrame()).toContain("Resume this chain before rewinding");
      await session.refresh();
      expect(session.activity.map((act) => act.id)).toEqual(pausedActs);
      app.closeOverlay();
      await session.submit("/wake");
      // A paragraph told while an ask is in flight goes to the turn after its answer, so the origin is read once it
      // settles.
      await idle(session);
      const original = await transcriptOf(session.engine, source);
      record = session.host.record;
      const message = session.activity.findLast((act) => session.isUserPrompt(act));
      app.rewind();
      await screen.flush();
      // The tree opens in the feed with the pointer on the last message, which Enter gives back on a new branch.
      expect(screen.captureCharFrame()).toContain("Rewind");
      expect(app.scroll.getChildren().some((node) => node.id === `tree-${message?.id}`)).toBe(true);
      const selected = new Promise<void>((resolve, reject) => {
        const changed = () => {
          if (session.selected === source && !session.error) return;
          session.off("change", changed);
          if (session.error) reject(new Error(session.error));
          else resolve();
        };
        session.on("change", changed);
      });
      screen.mockInput.pressEnter();
      await selected;
      await session.refresh();
      expect(session.selected).not.toBe(source);
      // The message returns to the input once the branch is made.
      await until(session, () => app.composer.plainText !== "");
      expect(app.composer.plainText).toBe(String(message?.words[1] ?? ""));
      const continued = await transcriptOf(session.engine, source);
      expect(continued.join("\n")).toContain(original.join("\n"));
      expect(session.turns.length).toBeGreaterThan(0);
      rewound = session.selected;
      transcript = await transcriptOf(session.engine, rewound);
      const maker = session.chains.find((chain) => chain.id === rewound)?.by;
      expect(session.acts.find((act) => act.id === maker)?.kind).toBe("rung");
    },
    { width: 140, height: 42 },
    true,
  );
  const reopened = await openEngine({ record, demo: true });
  try {
    expect(await transcriptOf(reopened.engine, rewound)).toEqual(transcript);
  } finally {
    await reopened.host.dispose();
  }
});

test("a progress tick keeps an in-flight act's card and body in place", () =>
  composing(
    async ({ session, app, screen }) => {
      try {
        const id = await session.engine.wait({ seconds: 60, on: session.engine.root });
        await session.refresh();
        app.render();
        await screen.flush();
        const card = app.scroll.getChildren().find((node) => node.id === id);
        if (!card) throw new Error("No card for the pending wait.");
        const body = card.getChildren().at(-1);
        // The clock moves past a tick of the spinner and a second of the elapsed time, and the card stays.
        setSystemTime(new Date(Date.now() + 1300));
        app.render();
        await screen.flush();
        expect(app.scroll.getChildren().find((node) => node.id === id)).toBe(card);
        expect(card.getChildren().at(-1)).toBe(body);
        expect(screen.captureCharFrame()).toContain("running 1s");
      } finally {
        setSystemTime();
      }
    },
    { width: 120, height: 40 },
  ));

test("a name inside a transcript tag opens the same live inspector as Python code", () =>
  composing(
    async ({ session, app, screen }) => {
      await session.engine.result(
        await session.engine.rung({ word: "answer = 17", on: session.engine.root }),
      );
      await session.refresh();
      session.show("transcript");
      app.render();
      await screen.flush();
      const lines = screen.captureCharFrame().split("\n");
      const row = lines.findIndex((line) => line.includes("answer = 17"));
      expect(row).toBeGreaterThanOrEqual(0);
      const column = lines[row]?.indexOf("answer") ?? -1;
      await screen.mockMouse.click(column + 1, row, 0, { modifiers: { ctrl: true } });
      await screen.waitForFrame((frame) => frame.includes("answer: int"), { maxPasses: 200 });
      expect(screen.captureCharFrame()).toContain("17");
    },
    { width: 120, height: 44, useMouse: true },
  ));

test("a delayed snapshot cannot restore the chain selected before a switch", async () => {
  const session = await demoSession();
  try {
    const child = await session.engine.chain({ label: "next" });
    await session.refresh();
    const original = session.host.snapshot.bind(session.host);
    let release = () => {};
    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    let entered = () => {};
    const started = new Promise<void>((resolve) => {
      entered = resolve;
    });
    let first = true;
    session.host.snapshot = async (chain) => {
      if (first) {
        first = false;
        const before = await original(chain);
        entered();
        await gate;
        return before;
      }
      return original(chain);
    };
    const reading = session.refresh();
    await started;
    const switching = session.select(child);
    release();
    await Promise.all([reading, switching]);
    expect(session.selected).toBe(child);
    expect(session.label).toBe("next");
  } finally {
    await session.dispose();
  }
});

test("editing a prompt program is a durable operator rung", async () => {
  const first = await demoSession(true);
  const prompt = first.activity.find((act) => act.kind === "prompt" && act.by === "operator");
  if (!prompt) throw new Error("No prompt in the fixture.");
  await first.command(`/edit ${prompt.id}`);
  await first.submit('saved_edit = 42\nclose("edited")');
  expect((await first.engine.inspect("saved_edit")).value).toBe(42);
  const record = first.host.record;
  await first.dispose();
  const opened = await openEngine({ record, demo: true });
  const second = new Session(opened.engine, opened.host, true);
  try {
    await second.refresh();
    expect((await second.engine.inspect("saved_edit")).value).toBe(42);
  } finally {
    await second.dispose();
  }
});

test("slash commands and project files are suggested above the input as they are typed, and Ctrl+D twice exits", () =>
  composing(
    async ({ session, app, screen, frame, quits }) => {
      app.composer.setText("draft");
      screen.mockInput.pressKey("d", { ctrl: true });
      screen.mockInput.pressKey("d", { ctrl: true });
      expect(quits()).toBe(0);
      app.composer.setText("");
      screen.mockInput.pressKey("d", { ctrl: true });
      expect(quits()).toBe(0);
      expect(await frame()).toContain("Press ⌃D again to exit.");
      screen.mockInput.pressKey("d", { ctrl: true });
      expect(quits()).toBe(1);

      await screen.mockInput.typeText("/mod");
      expect(app.composer.plainText).toBe("/mod");
      const commands = await frame();
      expect(commands).toContain("/model [model]");
      expect(commands).not.toContain("/exit");
      expect(commands).not.toContain("Commands");
      screen.mockInput.pressTab();
      expect(app.composer.plainText).toBe("/model ");
      expect(await frame()).not.toContain("/model [model]");

      app.composer.setText("");
      await screen.mockInput.typeText("/");
      screen.mockInput.pressArrow("down");
      screen.mockInput.pressArrow("down");
      await screen.mockInput.typeText("e");
      screen.mockInput.pressTab();
      expect(app.composer.plainText).toBe("/effort ");

      app.composer.setText("");
      await screen.mockInput.typeText("Read @READ");
      await session.projectFiles();
      expect(await frame()).toContain("@README.md");
      screen.mockInput.pressEscape();
      expect(await frame()).not.toContain("@README.md");
      await screen.mockInput.typeText("M");
      expect(await frame()).toContain("@README.md");
      screen.mockInput.pressEnter();
      expect(app.composer.plainText).toBe("Read @README.md ");
      expect(
        session.acts.some((act) => act.kind === "prompt" && String(act.words[1]).startsWith("Read")),
      ).toBe(false);
    },
    // In the kitty protocol an Escape is a key of its own, which the parser need not wait on to tell from Alt.
    { width: 120, height: 40, kittyKeyboard: true },
  ));

import { afterAll, expect, setSystemTime, test } from "bun:test";
import { readFile, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { RGBA } from "@opentui/core";
import { createTestRenderer } from "@opentui/core/testing";
import { until } from "../../bind/typescript/test/until.ts";
import { App, sessionDetail } from "../src/app.ts";
import { openEngine } from "../src/bridge.ts";
import { demoLibrary, demoSession, removeDemoDirectories, seedDemo } from "../src/demo.ts";
import { Preferences } from "../src/preferences.ts";
import type { Session } from "../src/session.ts";
import { palettes } from "../src/theme.ts";
import { type SessionEntry, Workspaces } from "../src/workspaces.ts";
import { cellAt, composing, drawn, withDemo } from "./composing.ts";
import { idle } from "./idle.ts";
import { transcriptOf } from "./transcript.ts";

afterAll(removeDemoDirectories);

test("the welcome shows provider checks when they arrive and updates them after a reload", () =>
  composing(async ({ session, frame }) => {
    session.setup = {
      claude: false,
      signed: false,
      providers: [{ provider: "local", env: [], enabled: true, offered: 0 }],
    };
    session.catalog = [];
    session.catalogRead = true;
    session.emit("change");
    expect(await frame()).toContain("No model from the local provider settings.");
    session.setup = {
      claude: false,
      signed: false,
      providers: [{ provider: "local", env: [], enabled: true, offered: 1 }],
    };
    session.catalog = [["local:qwen3", ["medium"], 65536]];
    session.emit("change");
    const shown = await frame();
    expect(shown).toContain("1 model in the provider settings.");
    expect(shown).not.toContain("No model from the local provider settings.");
  }));

test("the real native life drives the feed, the transcript, and responsive views", () =>
  composing(
    async ({ session, app, screen, frame }) => {
      expect(screen.captureCharFrame()).toContain("Explore a codebase");
      expect(session.theme).toBe("ink");
      expect(app.scroll.x).toBe(2);
      expect(app.scroll.height).toBeGreaterThanOrEqual(36);
      expect(app.scroll.height).toBeLessThanOrEqual(38);
      app.composer.setText("first line\nsecond line");
      await frame();
      expect(app.composer.height).toBeGreaterThanOrEqual(2);
      app.composer.setText("");
      expect(screen.captureCharFrame()).not.toContain("No budget set");
      expect(screen.captureCharFrame()).not.toContain("Session saved");
      await seedDemo(session);
      await frame();
      // The answer of a prompt stands under the name of the model that gave it, once its markdown is drawn.
      expect(screen.captureCharFrame()).toMatch(/sonnet {2}high/);
      for (
        let pass = 0;
        pass < 100 &&
        screen.captureCharFrame().lastIndexOf("A clear starting point") <
          screen.captureCharFrame().indexOf("Explore this project");
        pass++
      ) {
        await new Promise((done) => setTimeout(done, 20));
        await frame();
      }
      const conversation = screen.captureCharFrame();
      // The board over the feed says the answer of the chain too, so the feed holds its last mention.
      expect(conversation.indexOf("Explore this project")).toBeLessThan(
        conversation.lastIndexOf("A clear starting point"),
      );
      // A thread is the unit of the feed: its prompt heads it, and the words of its steps stand under it as one
      // program, each line with what its acts came to as a quiet hint. No step and no act stands as a row of its own.
      const ids = app.scroll.getChildren().map((node) => node.id);
      expect(ids).toContain("program-prompt1");
      expect(ids.some((id) => /^(rung|bash|read)\d+$/.test(id))).toBe(false);
      const program = await drawn(frame, screen.renderer.root);
      expect(program).toMatch(/ 1 notes = read\("README\.md"\) +# 7 lines/);
      expect(program).toMatch(/ 2 check = await bash\(.*# exit 0, 3 lines/);
      // A command that is over shows its output no more: its hint says what it came to.
      expect(program).not.toMatch(/# {3}✓ search/);
      // The control on the header of a thread folds it to its header, and a click on that header opens it again.
      const control = async () => {
        const rows = (await frame()).split("\n");
        const at = /▾ Collapse|▸ Expand/;
        const y = rows.findIndex((row) => at.test(row));
        const x = rows[y]?.search(at) ?? -1;
        if (y < 0 || x < 0) throw new Error("No control on the header of the thread.");
        await screen.mockMouse.click(x + 2, y);
        await screen.flush();
        return frame();
      };
      const folded = await control();
      expect(folded).toContain("Explore this project");
      expect(folded).not.toContain('notes = read("README.md")');
      expect(await control()).toContain('notes = read("README.md")');
      expect(screen.captureCharFrame()).not.toContain("failed");
      session.show("transcript");
      // The transcript opens at its start, and the doctrine of the first reply stands before the word, so the test
      // scrolls down a page at a time until the word shows.
      let page = await frame();
      while (
        !page.includes('notes = read("README.md")') &&
        app.scroll.scrollTop < app.scroll.scrollHeight - app.scroll.viewport.height
      ) {
        app.scroll.scrollBy(app.scroll.viewport.height);
        page = await frame();
      }
      expect(page).toContain('notes = read("README.md")');
      session.show("feed");
      screen.resize(80, 30);
      await frame();
      // A narrow top line keeps the session, the chain, and the switch of the views, and the sidebar is hidden.
      const top = screen.captureCharFrame().split("\n")[0] ?? "";
      expect(top).toContain(`${session.sessionName} › Main`);
      expect(top).toContain("Transcript");
      expect(screen.captureCharFrame()).not.toContain("Chains");
    },
    { width: 145, height: 45, useMouse: true },
  ));

test("a click on a line of a program opens it in place, with its whole text and what each of its acts gave, and a second click folds it", () =>
  composing(
    async ({ session, screen, frame }) => {
      await seedDemo(session);
      const row = async () => {
        const rows = (await drawn(frame, screen.renderer.root)).split("\n");
        const y = rows.findIndex((one) => / 2 check = await bash\(/.test(one));
        return { rows, y, x: rows[y]?.indexOf("check") ?? -1 };
      };
      await until(session, () => session.turns.some((turn) => turn[0] === "assistant"));
      const shut = await row();
      expect(shut.y).toBeGreaterThan(0);
      expect(shut.rows.join("\n")).not.toMatch(/^[ ┃]+✓ search\b/m);
      await screen.mockMouse.click(shut.x, shut.y);
      await screen.flush();
      const open = await row();
      expect(open.rows.join("\n")).toMatch(/^[ ┃]+✓ search\b/m);
      // The whole line shows, cut into rows at the column of the code, past the mark where it was cut.
      expect(open.rows[open.y]).not.toContain("…");
      await screen.mockMouse.click(open.x, open.y);
      await screen.flush();
      expect((await row()).rows.join("\n")).not.toMatch(/^[ ┃]+✓ search *$/m);
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
      "Choose one of claude-cli:sonnet, claude-cli:opus, claude-cli:haiku, claude-cli:fable.",
    );
    await first.submit("/effort low");
    expect(first.actor).toBe("claude-cli:opus/low");
    await first.submit("/theme paper");
    second = await demoSession({ preferences: new Preferences(first.preferences.path) });
    expect(second.theme).toBe("paper");
    expect(second.host.record).not.toBe(first.host.record);
    expect(first.preferences.path).toBe(join(dirname(first.host.record ?? ""), "ui-preferences.json"));
    first.roster.push(["claude-cli:org/plain", [], 1000], ["claude-cli:org/high", [], 1000]);
    first.actor = "claude-cli:org/plain";
    expect(first.actorChoice).toEqual({ model: "claude-cli:org/plain", effort: "off" });
    first.actor = "claude-cli:org/high";
    expect(first.actorChoice).toEqual({ model: "claude-cli:org/high", effort: "off" });
    await writeFile(first.preferences.path, "{");
    recovered = await demoSession({ preferences: new Preferences(first.preferences.path) });
    expect(recovered.theme).toBe("ink");
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
    async ({ session, app, screen, frame }) => {
      const snapshot = session.host.snapshot.bind(session.host);
      const { promise: gate, resolve: release } = Promise.withResolvers<void>();
      try {
        await idle(session);
        await session.refresh();
        await frame();
        const before = app.scroll.getChildren().map((node) => [node.id, node.y]);
        session.host.snapshot = async (chain) => {
          await gate;
          return snapshot(chain);
        };
        const reading = session.refresh();
        expect(await frame()).not.toContain("Loading conversation...");
        expect(app.scroll.getChildren().map((node) => [node.id, node.y])).toEqual(before);
        release();
        await reading;
        session.host.snapshot = snapshot;

        await session.submit("/read missing-review-file.txt");
        await session.refresh();
        expect(session.error).toBe("");
        expect(await frame()).not.toContain("Refresh view");
        expect(
          session.activity.some(
            (act) => act.done && act.kind === "rung" && JSON.stringify(act.value).includes("Refused"),
          ),
        ).toBe(true);

        session.show("transcript");
        await session.refresh();
        await frame();
        expect(app.scroll.scrollHeight).toBeGreaterThan(app.scroll.height);
        expect(app.scroll.verticalScrollBar.visible).toBe(false);
        app.scroll.scrollBy(5);
        await screen.flush();
        expect(app.scroll.scrollTop).toBeGreaterThan(0);

        session.host.snapshot = async () => {
          throw new Error("Snapshot unavailable");
        };
        await session.refresh().catch(session.fail);
        await frame();
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
    async ({ session, screen, frame }) => {
      for (const view of ["feed", "transcript", "changes"] as const) {
        session.show(view);
        await session.refresh();
        session.search = "nothing matches this";
        expect(await frame()).toContain("Nothing matches “nothing matches this”");
        const snapshot = session.host.snapshot.bind(session.host);
        const { promise: gate, resolve: release } = Promise.withResolvers<void>();
        session.host.snapshot = async (chain) => {
          await gate;
          return snapshot(chain);
        };
        const loading = session.refresh();
        expect(await frame()).toContain(`Loading the ${view}`);
        release();
        await loading;
        session.host.snapshot = snapshot;
        session.fail(new Error(`Could not load ${view}`));
        expect(await frame()).toContain(`Could not load ${view}`);
        expect(screen.captureCharFrame()).toContain("Refresh view");
        session.error = "";
      }
    },
    { width: 120, height: 42 },
  ));

test("operator answers and program edits act through the binding", () =>
  composing(
    async ({ session, app, screen, frame }) => {
      const id = await session.engine.prompt("bool", {
        message: "Continue with the change?",
        to: "operator",
        on: session.engine.root,
      });
      await until(session.host, () => session.host.prompts.has(id));
      await session.refresh();
      await drawn(frame, screen.renderer.root);
      expect(await frame()).toContain("Continue with the change?");
      await session.submit("yes");
      expect(await session.engine.result(id)).toBe(true);
      await seedDemo(session);
      await session.command("/edit");
      expect(app.composer.plainText).toContain("close(");
      await session.submit('close("Edited answer")');
      expect(session.editing).toBeUndefined();
      app.composer.setText("keep the session open");
      screen.mockInput.pressCtrlC();
      await screen.flush();
      expect(app.composer.plainText).toBe("");
    },
    { width: 120, height: 40, exitOnCtrlC: false },
  ));

test("a question of a choice offers its values as a list in place of the input, and Other takes the words of the operator, which close it as Refused", () =>
  composing(
    async ({ session, app, screen, frame }) => {
      const root = session.engine.root;
      const runner = await session.engine.prompt("Literal['vitepress', 'mkdocs', 'plain']", {
        message: "Which runner for the docs?",
        to: "operator",
        on: root,
      });
      await until(session.host, () => session.host.prompts.has(runner));
      await session.refresh();
      const listed = await frame();
      expect(listed).toContain("❯ 1  vitepress");
      expect(listed).toContain("  2  mkdocs");
      expect(listed).toContain("  4  Other  in your own words");
      expect(app.composer.visible).toBe(false);
      screen.mockInput.pressArrow("down");
      expect(await frame()).toContain("❯ 2  mkdocs");
      screen.mockInput.pressEnter();
      expect(await session.engine.result(runner)).toBe("mkdocs");
      const merge = await session.engine.prompt("bool", { message: "Merge it?", to: "operator", on: root });
      await until(session.host, () => session.host.prompts.has(merge));
      await session.refresh();
      expect(await frame()).toContain("  3  Other");
      screen.mockInput.pressKey("3");
      await frame();
      expect(app.composer.visible).toBe(true);
      await screen.mockInput.typeText("Not yet: the checks run.");
      screen.mockInput.pressEnter();
      await expect(session.engine.result(merge)).rejects.toThrow("Not yet: the checks run.");
      await until(session, () => session.notice.startsWith(`Your words went to ${merge}`));
    },
    { width: 120, height: 40 },
  ));

test("the last row of a thread whose model works shows a loader left of the number of the line that comes, and # thinking until the model shows a thought, then the last line of that thought", () =>
  composing(async ({ session, app, frame }) => {
    app.composer.setText("Think until I cancel.");
    await app.submit();
    await until(session, () => session.activity.some((act) => act.kind === "rung" && !act.done));
    await session.refresh();
    const row = async () => (await frame()).split("\n").find((line) => / 1 # /.test(line)) ?? "";
    expect(await row()).toMatch(/^[ ┃]+1 # thinking/);
    const step = session.activity.find((act) => act.kind === "rung" && !act.done)?.id ?? "";
    // The thought streams in the worker, since each state of the worker replaces the streams that the host holds.
    const think = Reflect.get(session.host, "ask") as (
      target: string,
      method: string,
      thought: string,
    ) => Promise<null>;
    await think.call(session.host, "library", "think", "First I read.\nThen I wait.");
    await until(
      session.host,
      () => session.host.streams.get(step)?.thinking === "First I read.\nThen I wait.",
    );
    // The tick draws the row again from the stream, and no other part of the feed.
    await new Promise((done) => setTimeout(done, 250));
    expect(await row()).toMatch(/^[ ┃]+1 # Then I wait\./);
    await session.submit("/cancel");
  }));

test("a prompt of None to the operator is a notice that settles as a quiet line, which no key dismisses", () =>
  composing(
    async ({ session, screen, frame }) => {
      const id = await session.engine.prompt("None", {
        message: "The build is green.\n\nEach check passed.",
        to: "operator",
        on: session.engine.root,
      });
      expect(await session.engine.result(id)).toBeNull();
      // The session dismisses the notice as the console takes it, so it waits on no desk.
      await until(
        session.host,
        () => !session.host.notices.has(id) && session.acts.some((act) => act.id === id),
      );
      await session.refresh();
      expect(session.operatorPrompt).toBeUndefined();
      expect(session.desk.asks).toEqual([]);
      const shown = await drawn(frame, screen.renderer.root);
      expect(shown).toMatch(/┃ The build is green\. +\+1 line/);
      expect(shown).not.toContain("Each check passed.");
      expect(shown).not.toContain("You answered");
      expect(shown).not.toContain("⌃A");
    },
    { width: 120, height: 40 },
  ));

test("a word that awaits a prompt of None to the operator goes on at once, since a notice blocks no one", () =>
  composing(
    async ({ session, frame }) => {
      const rung = await session.engine.rung({
        word: 'await prompt(None, "The build is green.", to="operator")\nafter = 1',
        on: session.engine.root,
      });
      await until(session, () =>
        session.activity.some((act) => act.id === rung && act.run?.status === "done"),
      );
      await session.refresh();
      // The word ran to its end: no desk holds a question, no dialog waits, and no row of the feed still runs or asks.
      expect(session.operatorPrompt).toBeUndefined();
      expect(session.desk.asks).toEqual([]);
      const notices = session.activity.filter((act) => act.kind === "prompt" && act.by === rung);
      expect(notices.length).toBe(1);
      expect(notices.every((act) => act.done)).toBe(true);
      const shown = await frame();
      expect(shown).toContain("after = 1");
      expect(shown).not.toContain("◉");
      expect(shown).not.toContain("needs input");
      expect(shown).not.toContain("running");
    },
    { width: 120, height: 40 },
  ));

test("many questions wait on the desk, which says which one the input answers, and the dialog goes from one to the next", () =>
  composing(
    async ({ session, app, screen, frame }) => {
      const root = session.engine.root;
      const side = await session.engine.chain({ label: "Side" });
      const base = await session.engine.prompt("str", { message: "Which base?", to: "operator", on: root });
      const merge = await session.engine.prompt("bool", { message: "Merge it?", to: "operator", on: side });
      const count = await session.engine.prompt("int", {
        message: "How many retries?",
        to: "operator",
        on: root,
      });
      await until(session.host, () => session.host.prompts.size === 3);
      await session.refresh();
      const first = await drawn(frame, screen.renderer.root);
      expect(first).toContain("◆ ◆ ◆  Question 1 of 3  from Main");
      expect(first).toContain(`answers ${base}`);
      expect(first).toContain("The input below answers this question, as str");
      expect(first).toContain("Click here to answer this question in the input below");
      await session.submit("main");
      expect(await session.engine.result(base)).toBe("main");
      expect(session.notice).toBe(`Your answer went to ${base} on Main. 2 more wait.`);
      await until(session.host, () => !session.host.prompts.has(base));
      await session.refresh();
      expect(await drawn(frame, screen.renderer.root)).toContain(`answers ${count}`);
      app.question();
      screen.mockInput.pressTab();
      await until(session, () => session.selected === side);
      const next = await drawn(frame, screen.renderer.root);
      expect(next).toContain("Question 1 of 2 from Side");
      expect(next).toContain("Merge it?");
      screen.mockInput.pressEnter();
      expect(await session.engine.result(merge)).toBe(true);
      await until(session.host, () => !session.host.prompts.has(merge));
      await until(session, () => session.selected === root);
      expect(await drawn(frame, screen.renderer.root)).toContain("Question 1 of 1 from Main");
      await screen.mockInput.typeText("3");
      screen.mockInput.pressEnter();
      expect(await session.engine.result(count)).toBe(3);
      await until(session.host, () => !session.host.prompts.size);
      await session.refresh();
      expect(await drawn(frame, screen.renderer.root)).not.toContain("Question 1 of");
    },
    { width: 120, height: 40 },
  ));

test("the markdown of a message draws each element with no marks, and a code block on a surface of its own", () =>
  composing(
    async ({ session, screen, frame }) => {
      const long = `    return ${"note + ".repeat(20)}note`;
      const id = await session.engine.prompt("None", {
        message: [
          "## The shortcut",
          "",
          "Press **Ctrl+K**, as `search.py` binds it, *once*. See [the guide](https://example.com).",
          "",
          "- one",
          "  - nested",
          "",
          "> a quote",
          "",
          "| Key | Action |",
          "| --- | --- |",
          "| Ctrl+K | Find |",
          "",
          "```python",
          "def search(note):",
          long,
          "        pass",
          "```",
          "",
          "```",
          "plain/",
          "    kept",
          "```",
        ].join("\n"),
        to: "operator",
        on: session.engine.root,
      });
      await until(session.host, () => session.acts.some((act) => act.id === id && act.done));
      // A notice settles folded, and opens to its message.
      session.folds[id] = false;
      await session.refresh();
      await drawn(frame, screen.renderer.root);
      const rows = (await frame()).split("\n");
      const shown = rows.join("\n");
      for (const mark of ["##", "**", "`", "[the guide]", "| ---", "> a quote"])
        expect(shown).not.toContain(mark);
      for (const text of [
        "The shortcut",
        "Press Ctrl+K, as search.py binds it, once.",
        "- one",
        "  - nested",
        "│ a quote",
      ])
        expect(shown).toContain(text);
      expect(shown).toMatch(/│Key +│Action +│/);
      const label = rows.findIndex((row) => row.includes(" python "));
      const code = rows[label + 1] ?? "";
      const x = code.indexOf("def search(note):");
      expect(x).toBe(rows[label]?.indexOf("python") ?? -1);
      expect(rows[label + 2]?.slice(x)).toStartWith("    return note + note");
      expect(rows[label + 3]?.slice(x)).toStartWith("        pass");
      const plain = rows.findIndex((row) => row.includes("plain/"));
      expect(rows[plain + 1]?.slice(rows[plain]?.indexOf("plain/"))).toStartWith("    kept");
      const spans = screen.captureSpans();
      const palette = palettes.ink;
      expect(cellAt(spans, x, label + 1).fg?.equals(RGBA.fromHex(palette.syntaxKeyword))).toBe(true);
      // The notice stands on a panel, as a message of the operator does, and its code block on the ground, inset in it.
      expect(cellAt(spans, x - 1, label + 1).bg?.equals(RGBA.fromHex(palette.background))).toBe(true);
      expect(cellAt(spans, x, label - 2).bg?.equals(RGBA.fromHex(palette.panel))).toBe(true);
    },
    { width: 100, height: 60 },
  ));

test("resume preserves chains, programs, theme, and input drafts while unfinished work stays paused", async () => {
  const first = await demoSession({ seed: true });
  let library = await demoLibrary(first);
  const screen = await createTestRenderer({ width: 120, height: 40, exitSignals: [] });
  const app = new App(screen.renderer, first, { quit() {}, workspaces: library });
  const fork = first.chains.find((chain) => chain.id !== first.engine.root);
  if (!fork) throw new Error("No fork in the fixture.");
  await first.select(fork.id);
  first.mode = "python";
  first.theme = "paper";
  first.view = "transcript";
  app.render();
  app.composer.setText('draft = "keep this"');
  const pending = await first.engine.wait([], { seconds: 10, on: fork.id });
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
  const second = await demoSession({ record });
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
      expect(screen.captureCharFrame()).toContain("Wake this chain before rewinding");
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
      screen.mockInput.pressEnter();
      await until(session, () => session.selected !== source || Boolean(session.error));
      expect(session.error).toBe("");
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

test("an act that no word made stands as a lit line of python in a panel, which says its time after a second", () =>
  composing(
    async ({ session, app, screen, frame }) => {
      // The clock stands still from before the wait starts, so the elapsed time is what the test moves it by, however
      // long the frames take.
      const start = Date.now();
      setSystemTime(new Date(start));
      try {
        const id = await session.engine.wait([], { seconds: 60, on: session.engine.root });
        await session.refresh();
        const before = await frame();
        expect(before).toContain(`${id} = wait(60)`);
        expect(before).not.toContain("1.0s");
        const card = app.scroll.getChildren().find((node) => node.id === id);
        if (!card) throw new Error("No panel for the pending wait.");
        // The clock moves past a second of the elapsed time, and the line says it.
        setSystemTime(new Date(start + 1300));
        await frame();
        expect(app.scroll.getChildren().some((node) => node.id === id)).toBe(true);
        expect(screen.captureCharFrame()).toMatch(new RegExp(`${id} = wait\\(60\\) +◉ # waits 60s +1\\.0s`));
      } finally {
        setSystemTime();
      }
    },
    { width: 120, height: 40 },
  ));

test("a name inside a transcript tag opens the same live inspector as Python code", () =>
  composing(
    async ({ session, screen, frame }) => {
      await session.engine.result(
        await session.engine.rung({ word: "answer = 17", on: session.engine.root }),
      );
      await session.refresh();
      session.show("transcript");
      const lines = (await frame()).split("\n");
      const row = lines.findIndex((line) => line.includes("answer = 17"));
      expect(row).toBeGreaterThanOrEqual(0);
      const column = lines[row]?.indexOf("answer") ?? -1;
      await screen.mockMouse.click(column + 1, row, 0, { modifiers: { ctrl: true } });
      await until(screen.renderer, () => screen.captureCharFrame().includes("answer: int"), "frame");
      expect(screen.captureCharFrame()).toContain("17");
    },
    { width: 120, height: 44, useMouse: true },
  ));

test("a delayed snapshot cannot restore the chain selected before a switch", () =>
  withDemo(async (session) => {
    const child = await session.engine.chain({ label: "next" });
    await session.refresh();
    const original = session.host.snapshot.bind(session.host);
    const { promise: gate, resolve: release } = Promise.withResolvers<void>();
    const { promise: started, resolve: entered } = Promise.withResolvers<void>();
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
  }));

test("editing a prompt program is a durable operator rung", async () => {
  const first = await demoSession({ seed: true });
  const prompt = first.activity.find((act) => act.kind === "prompt" && act.by === "operator");
  if (!prompt) throw new Error("No prompt in the fixture.");
  await first.command(`/edit ${prompt.id}`);
  await first.submit('saved_edit = 42\nclose("edited")');
  expect((await first.engine.inspect("saved_edit")).value).toBe(42);
  const record = first.host.record;
  await first.dispose();
  const second = await demoSession({ record });
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

test("each line of a word says what its acts did as a quiet hint: a read its lines, a write the lines it added and removed, and a command its exit", () =>
  composing(
    async ({ session, screen, frame }) => {
      const id = await session.engine.rung({
        word: [
          'write(Text("notes.md", "one\\ntwo\\n"))',
          'write(read("notes.md").replace("two", "three\\nfour"))',
          'await bash("true")',
        ].join("\n"),
        on: session.engine.root,
      });
      await session.engine.result(id);
      await session.refresh();
      session.folds[id] = false;
      const shown = await drawn(frame, screen.renderer.root);
      expect(shown).toMatch(/ 1 write\(Text\("notes\.md".*# \+2 -0/);
      expect(shown).toMatch(/ 2 write\(read\("notes\.md"\).*# 2 lines {2}\+2 -1/);
      expect(shown).toMatch(/ 3 await bash\("true"\) +# exit 0/);
    },
    { width: 120, height: 40 },
  ));

import { expect, test } from "bun:test";
import { readFile, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { createTestRenderer } from "@opentui/core/testing";
import { App } from "../src/app.ts";
import { openEngine } from "../src/bridge.ts";
import { demoWorkspace, seedDemo } from "../src/demo.ts";
import { Preferences } from "../src/preferences.ts";
import { sessionChoices } from "../src/sessions.ts";
import { Workspace } from "../src/workspace.ts";

test("the real native life drives conversation, program, activity, search, and responsive views", async () => {
  const workspace = await demoWorkspace();
  const test = await createTestRenderer({ width: 145, height: 45, useMouse: true });
  const app = new App(test.renderer, workspace, { quit() {} });
  try {
    await test.flush();
    expect(test.captureCharFrame()).toContain("Explore a codebase");
    expect(workspace.theme).toBe("github");
    expect(app.scroll.x).toBe(1);
    expect(app.scroll.height).toBeGreaterThanOrEqual(36);
    expect(app.scroll.height).toBeLessThanOrEqual(38);
    app.composer.setText("first line\nsecond line");
    app.render();
    await test.flush();
    expect(app.composer.height).toBeGreaterThanOrEqual(2);
    app.composer.setText("");
    expect(test.captureCharFrame()).not.toContain("No budget set");
    expect(test.captureCharFrame()).not.toContain("Session saved");
    await seedDemo(workspace);
    app.render();
    await test.flush();
    expect(test.captureCharFrame()).toContain("Result");
    const conversation = test.captureCharFrame();
    expect(conversation.indexOf("Explore this project")).toBeLessThan(
      conversation.indexOf("A clear starting point"),
    );
    expect(conversation).toContain("✓ local storage");
    const command = app.scroll.getChildren().find((node) => node.id.startsWith("bash://"));
    const heading = command?.getChildren()[0];
    if (!heading) throw new Error("No command heading.");
    await test.mockMouse.click(heading.x, heading.y);
    await test.flush();
    expect(test.captureCharFrame()).toContain("command:");
    const expanded = app.scroll
      .getChildren()
      .find((node) => node.id === command?.id)
      ?.getChildren()[0];
    expect(expanded?.visible).toBe(true);
    if (expanded) await test.mockMouse.click(expanded.x, expanded.y);
    await test.flush();
    expect(test.captureCharFrame()).not.toContain("command:");
    workspace.show("program");
    app.render();
    await test.flush();
    expect(test.captureCharFrame()).toContain('notes = read("README.md")');
    workspace.show("activity");
    app.render();
    await test.flush();
    expect(app.scroll.getChildren().some((child) => child.id.startsWith("bash://"))).toBe(true);
    expect(test.captureCharFrame()).not.toContain("failed");
    app.palette();
    await test.flush();
    expect(test.captureCharFrame()).toContain("Commands");
    await test.mockInput.typeText("budget");
    await test.flush();
    expect(test.captureCharFrame()).toContain("Set budget");
    test.mockInput.pressEnter();
    await test.flush();
    expect(app.composer.plainText).toBe("/grant ");
    test.resize(80, 30);
    app.render();
    await test.flush();
    expect(test.captureCharFrame()).toContain("furb");
    expect(test.captureCharFrame()).not.toContain("Chains");
  } finally {
    app.dispose();
    test.renderer.destroy();
    await workspace.dispose();
  }
}, 30000);

test("model and effort change independently and a theme preference applies to a new session", async () => {
  const first = await demoWorkspace();
  let second: Workspace | undefined;
  let recovered: Workspace | undefined;
  try {
    await first.submit("/effort high");
    expect(first.actor).toBe("claude-cli:sonnet/high");
    await first.submit("/model claude-cli:opus");
    expect(first.actor).toBe("claude-cli:opus/high");
    await first.submit("/effort low");
    expect(first.actor).toBe("claude-cli:opus/low");
    await first.submit("/theme paper");
    second = await demoWorkspace(false, new Preferences(first.preferences.path));
    expect(second.theme).toBe("paper");
    expect(second.world.records.path).not.toBe(first.world.records.path);
    expect(first.preferences.path).toBe(join(dirname(first.world.records.path ?? ""), "ui-preferences.json"));
    first.roster.push(["claude-cli:org/plain", [], 1000], ["claude-cli:org/high", [], 1000]);
    first.actor = "claude-cli:org/plain";
    expect(first.actorChoice).toEqual({ model: "claude-cli:org/plain", effort: "off" });
    first.actor = "claude-cli:org/high";
    expect(first.actorChoice).toEqual({ model: "claude-cli:org/high", effort: "off" });
    await writeFile(first.preferences.path, "{");
    recovered = await demoWorkspace(false, new Preferences(first.preferences.path));
    expect(recovered.theme).toBe("github");
    expect(recovered.preferences.notice).toContain("Could not read preferences");
    expect(await readFile(first.preferences.path, "utf8")).toBe("{");
  } finally {
    await recovered?.dispose();
    await second?.dispose();
    await first.dispose();
  }
}, 30000);

test("refreshes keep content in place, act failures stay in the record, and hidden scrollbars still scroll", async () => {
  const workspace = await demoWorkspace(true);
  const screen = await createTestRenderer({ width: 120, height: 40 });
  const app = new App(screen.renderer, workspace, { quit() {} });
  const snapshot = workspace.world.snapshot.bind(workspace.world);
  let release = () => {};
  try {
    await Bun.sleep(300);
    await workspace.refresh();
    app.render();
    await screen.flush();
    const before = app.scroll.getChildren().map((node) => [node.id, node.y]);
    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    workspace.world.snapshot = async (chain) => {
      await gate;
      return snapshot(chain);
    };
    const reading = workspace.refresh();
    app.render();
    await screen.flush();
    expect(screen.captureCharFrame()).not.toContain("Loading conversation...");
    expect(app.scroll.getChildren().map((node) => [node.id, node.y])).toEqual(before);
    release();
    await reading;
    workspace.world.snapshot = snapshot;

    await workspace.submit("/read missing-review-file.txt");
    await workspace.refresh();
    app.render();
    await screen.flush();
    expect(workspace.error).toBe("");
    expect(screen.captureCharFrame()).not.toContain("Refresh view");
    expect(
      workspace.activity.some(
        (act) => act.done && act.kind === "rung" && JSON.stringify(act.value).includes("Refused"),
      ),
    ).toBe(true);

    workspace.show("transcript");
    await workspace.refresh();
    app.render();
    await screen.flush();
    expect(app.scroll.scrollHeight).toBeGreaterThan(app.scroll.height);
    expect(app.scroll.verticalScrollBar.visible).toBe(false);
    app.scroll.scrollBy(5);
    await screen.flush();
    expect(app.scroll.scrollTop).toBeGreaterThan(0);

    workspace.world.snapshot = async () => {
      throw new Error("Snapshot unavailable");
    };
    await workspace.refresh().catch(workspace.fail);
    app.render();
    await screen.flush();
    app.scroll.scrollTo(0);
    await screen.flush();
    expect(screen.captureCharFrame()).toContain("Snapshot unavailable");
    expect(screen.captureCharFrame()).toContain("Refresh view");
  } finally {
    release();
    workspace.world.snapshot = snapshot;
    app.dispose();
    screen.renderer.destroy();
    await workspace.dispose();
  }
}, 30000);

test("every view shows an empty result, loading, and an error in its feed", async () => {
  const workspace = await demoWorkspace();
  const screen = await createTestRenderer({ width: 120, height: 42 });
  const app = new App(screen.renderer, workspace, { quit() {} });
  try {
    for (const view of ["conversation", "program", "activity", "facts", "transcript", "changes"] as const) {
      workspace.show(view);
      await workspace.refresh();
      workspace.query = "nothing matches this";
      app.render();
      await screen.flush();
      expect(screen.captureCharFrame()).toContain(`No matching ${view}`);
      const snapshot = workspace.world.snapshot.bind(workspace.world);
      let release = () => {};
      const gate = new Promise<void>((resolve) => {
        release = resolve;
      });
      workspace.world.snapshot = async (chain) => {
        await gate;
        return snapshot(chain);
      };
      const loading = workspace.refresh();
      app.render();
      await screen.flush();
      expect(screen.captureCharFrame()).toContain(`Loading ${view}`);
      release();
      await loading;
      workspace.world.snapshot = snapshot;
      workspace.fail(new Error(`Could not load ${view}`));
      app.render();
      await screen.flush();
      expect(screen.captureCharFrame()).toContain(`Could not load ${view}`);
      expect(screen.captureCharFrame()).toContain("Refresh view");
      workspace.error = "";
    }
  } finally {
    app.dispose();
    screen.renderer.destroy();
    await workspace.dispose();
  }
}, 30000);

test("operator answers and program edits act through the binding", async () => {
  const workspace = await demoWorkspace();
  const test = await createTestRenderer({ width: 120, height: 40, exitOnCtrlC: false });
  const app = new App(test.renderer, workspace, { quit() {} });
  try {
    const id = await workspace.life.prompt("bool", "Continue with the change?", { to: "operator" });
    await Bun.sleep(60);
    await workspace.refresh();
    app.render();
    await test.flush();
    expect(test.captureCharFrame()).toContain("Continue with the change?");
    await workspace.submit("yes");
    expect(await workspace.life.result(id)).toBe(true);
    await seedDemo(workspace);
    await workspace.command("/edit");
    expect(app.composer.plainText).toContain("close(");
    await workspace.submit('close("Edited answer")');
    expect(workspace.editing).toBeUndefined();
    await workspace.command("/run this is invalid python !!!");
    expect(workspace.error).toBe("");
    expect(workspace.findings.join("\n")).toContain("line 1");
    app.render();
    await test.flush();
    expect(test.captureCharFrame()).toContain("this is invalid python");
    app.composer.setText("keep the session open");
    test.mockInput.pressCtrlC();
    await test.flush();
    expect(app.composer.plainText).toBe("");
  } finally {
    app.dispose();
    test.renderer.destroy();
    await workspace.dispose();
  }
}, 30000);

test("resume preserves chains, programs, theme, and input drafts while unfinished work stays paused", async () => {
  const first = await demoWorkspace(true);
  const screen = await createTestRenderer({ width: 120, height: 40 });
  const app = new App(screen.renderer, first, { quit() {} });
  const fork = first.chains.find((chain) => chain.id !== first.life.root);
  if (!fork) throw new Error("No fork in the fixture.");
  await first.select(fork.id);
  first.mode = "python";
  first.theme = "paper";
  first.view = "program";
  app.render();
  app.composer.setText('draft = "keep this"');
  const pending = await first.life.wait(10, fork.id);
  const ids = first.chains.map((chain) => chain.id);
  const program = { ...first.program };
  const record = first.world.records.path;
  app.dispose();
  screen.renderer.destroy();
  await first.dispose();
  if (!record) throw new Error("No saved record.");
  const choices = await sessionChoices(
    dirname(record),
    async () => {},
    async () => {},
  );
  expect(choices[1]?.detail).toContain("KiB");
  expect(choices[1]?.detail).not.toContain("Paused");
  expect(choices).toHaveLength(2);
  const opened = await openEngine({ record, demo: true });
  const second = new Workspace(opened.life, opened.world, true);
  await second.refresh();
  const next = await createTestRenderer({ width: 120, height: 40 });
  const view = new App(next.renderer, second, { quit() {} });
  try {
    expect(second.chains.map((chain) => chain.id)).toEqual(ids);
    expect(second.program).toEqual(program);
    expect(second.selected).toBe(fork.id);
    expect(second.theme).toBe("paper");
    expect(second.mode).toBe("python");
    expect(view.composer.plainText).toBe('draft = "keep this"');
    expect(second.world.held.has(pending)).toBe(true);
    expect((await second.life.outcome(pending)).done).toBe(false);
  } finally {
    view.dispose();
    next.renderer.destroy();
    await second.dispose();
  }
}, 30000);

test("rewind is a recorded rung and keeps the selected transcript after reopening", async () => {
  const workspace = await demoWorkspace(true);
  const screen = await createTestRenderer({ width: 140, height: 42 });
  const app = new App(screen.renderer, workspace, { quit() {} });
  const source = workspace.selected;
  await workspace.submit("/pause");
  const heldActs = workspace.activity.map((act) => act.id);
  app.rewind();
  app.rewind();
  await screen.flush();
  expect(screen.captureCharFrame()).toContain("Resume this chain before rewinding");
  await workspace.refresh();
  expect(workspace.activity.map((act) => act.id)).toEqual(heldActs);
  app.closeOverlay();
  await workspace.submit("/wake");
  const original = await workspace.life.rendered(source);
  let rewound = "";
  let transcript: string[] = [];
  const record = workspace.world.records.path;
  try {
    app.rewind();
    await screen.flush();
    expect(screen.captureCharFrame()).toContain("Rewind transcript");
    const selected = new Promise<void>((resolve, reject) => {
      const changed = () => {
        if (workspace.selected === source && !workspace.error) return;
        workspace.off("change", changed);
        if (workspace.error) reject(new Error(workspace.error));
        else resolve();
      };
      workspace.on("change", changed);
    });
    screen.mockInput.pressEnter();
    await selected;
    await workspace.refresh();
    expect(workspace.selected).not.toBe(source);
    const continued = await workspace.life.rendered(source);
    expect(continued.join("\n")).toContain(original.join("\n"));
    expect(workspace.turns.length).toBeGreaterThan(0);
    rewound = workspace.selected;
    transcript = await workspace.life.rendered(rewound);
    const maker = workspace.chains.find((chain) => chain.id === rewound)?.by;
    expect(workspace.acts.find((act) => act.id === maker)?.kind).toBe("rung");
  } finally {
    app.dispose();
    screen.renderer.destroy();
    await workspace.dispose();
  }
  const reopened = await openEngine({ record, demo: true });
  try {
    expect(await reopened.life.rendered(rewound)).toEqual(transcript);
  } finally {
    await reopened.world.dispose();
  }
}, 30000);

test("a progress tick keeps an in-flight act's card and body in place", async () => {
  const workspace = await demoWorkspace();
  const screen = await createTestRenderer({ width: 120, height: 40 });
  const app = new App(screen.renderer, workspace, { quit() {} });
  try {
    const id = await workspace.life.wait(60);
    await workspace.refresh();
    workspace.show("activity");
    app.render();
    await screen.flush();
    const card = app.scroll.getChildren().find((node) => node.id === id);
    if (!card) throw new Error("No card for the pending wait.");
    const body = card.getChildren().at(-1);
    await Bun.sleep(300);
    app.render();
    await screen.flush();
    expect(app.scroll.getChildren().find((node) => node.id === id)).toBe(card);
    expect(card.getChildren().at(-1)).toBe(body);
    expect(screen.captureCharFrame()).toContain("since start");
  } finally {
    app.dispose();
    screen.renderer.destroy();
    await workspace.dispose();
  }
}, 30000);

test("a name inside a transcript tag opens the same live inspector as Python code", async () => {
  const workspace = await demoWorkspace();
  const screen = await createTestRenderer({ width: 120, height: 44, useMouse: true });
  const app = new App(screen.renderer, workspace, { quit() {} });
  try {
    await workspace.life.result(await workspace.life.rung("answer = 17"));
    await workspace.refresh();
    workspace.show("transcript");
    app.render();
    await screen.flush();
    const lines = screen.captureCharFrame().split("\n");
    const row = lines.findIndex((line) => line.includes("answer = 17"));
    expect(row).toBeGreaterThanOrEqual(0);
    const column = lines[row]?.indexOf("answer") ?? -1;
    await screen.mockMouse.click(column + 1, row, 0, { modifiers: { ctrl: true } });
    await screen.waitForFrame((frame) => frame.includes("answer · int"), { maxPasses: 200 });
    expect(screen.captureCharFrame()).toContain("17");
  } finally {
    app.dispose();
    screen.renderer.destroy();
    await workspace.dispose();
  }
}, 30000);

test("a delayed snapshot cannot restore the chain selected before a switch", async () => {
  const workspace = await demoWorkspace();
  try {
    const child = await workspace.life.chain("next");
    await workspace.refresh();
    const original = workspace.world.snapshot.bind(workspace.world);
    let release = () => {};
    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    let entered = () => {};
    const started = new Promise<void>((resolve) => {
      entered = resolve;
    });
    let first = true;
    workspace.world.snapshot = async (chain) => {
      if (first) {
        first = false;
        const before = await original(chain);
        entered();
        await gate;
        return before;
      }
      return original(chain);
    };
    const reading = workspace.refresh();
    await started;
    const switching = workspace.select(child);
    release();
    await Promise.all([reading, switching]);
    expect(workspace.selected).toBe(child);
    expect(workspace.label).toBe("next");
  } finally {
    await workspace.dispose();
  }
}, 30000);

test("editing a prompt program is a durable operator rung", async () => {
  const first = await demoWorkspace(true);
  const prompt = first.activity.find((act) => act.kind === "prompt" && act.by === "operator");
  if (!prompt) throw new Error("No prompt in the fixture.");
  await first.command(`/edit ${prompt.id}`);
  await first.submit('saved_edit = 42\nclose("edited")');
  expect((await first.life.inspect("saved_edit")).value).toBe(42);
  const record = first.world.records.path;
  await first.dispose();
  const opened = await openEngine({ record, demo: true });
  const second = new Workspace(opened.life, opened.world, true);
  try {
    await second.refresh();
    expect((await second.life.inspect("saved_edit")).value).toBe(42);
  } finally {
    await second.dispose();
  }
}, 30000);

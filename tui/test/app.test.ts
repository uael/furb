import { expect, test } from "bun:test";
import { dirname } from "node:path";
import { createTestRenderer } from "@opentui/core/testing";
import { App } from "../src/app.ts";
import { openEngine } from "../src/bridge.ts";
import { demoWorkspace, seedDemo } from "../src/demo.ts";
import { sessionChoices } from "../src/sessions.ts";
import { Workspace } from "../src/workspace.ts";

test("the real native life drives conversation, program, activity, search, and responsive views", async () => {
  const workspace = await demoWorkspace();
  const test = await createTestRenderer({ width: 145, height: 45 });
  const app = new App(test.renderer, workspace, { quit() {} });
  try {
    await test.flush();
    expect(test.captureCharFrame()).toContain("A little space for ambitious work.");
    await seedDemo(workspace);
    app.render();
    await test.flush();
    expect(test.captureCharFrame()).toContain("RESULT");
    workspace.show("program");
    app.render();
    await test.flush();
    expect(test.captureCharFrame()).toContain('notes = read("README.md")');
    workspace.show("activity");
    app.render();
    await test.flush();
    expect(app.scroll.getChildren().some((child) => child.id.startsWith("bash://"))).toBe(true);
    app.palette();
    await test.flush();
    expect(test.captureCharFrame()).toContain("COMMANDS");
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
    expect(test.captureCharFrame()).not.toContain("THIS CHAIN");
  } finally {
    app.dispose();
    test.renderer.destroy();
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
    const error = await workspace.command("/run this is invalid python !!!").catch((error: unknown) => error);
    expect(error).toBeInstanceOf(Error);
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
  expect(choices.filter((choice) => choice.detail.includes("Paused"))).toHaveLength(1);
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

test("rewind picks an act and filters the new chain's transcript without changing the source", async () => {
  const workspace = await demoWorkspace(true);
  const screen = await createTestRenderer({ width: 140, height: 42 });
  const app = new App(screen.renderer, workspace, { quit() {} });
  const source = workspace.selected;
  const original = await workspace.life.rendered(source);
  try {
    app.rewind();
    await screen.flush();
    expect(screen.captureCharFrame()).toContain("REWIND TRANSCRIPT");
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
    expect(await workspace.life.rendered(source)).toEqual(original);
    expect(workspace.turns.length).toBeGreaterThan(0);
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

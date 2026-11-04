import { afterAll, expect, setSystemTime, test } from "bun:test";
import { InputRenderable, TextRenderable } from "@opentui/core";
import { createTestRenderer } from "@opentui/core/testing";
import type { Node, Tree, ViewEvent } from "furb/protocol";
import { App } from "../src/app.ts";
import { demoLibrary, demoSession, removeDemoDirectories } from "../src/demo.ts";
import { frame, spin } from "../src/theme.ts";

afterAll(removeDemoDirectories);

/** A view of a test skill that holds each node of the closed set. */
const kit: Node = {
  id: "root",
  node: "box",
  axis: "column",
  gap: 0,
  border: true,
  children: [
    {
      id: "title",
      node: "text",
      runs: [
        { text: "Test skill ", tone: "accent" },
        { text: "ready", tone: "success" },
      ],
    },
    { id: "steps", node: "list", items: [{ id: "one", node: "text", runs: [{ text: "first step" }] }] },
    {
      id: "grid",
      node: "table",
      header: ["name", "state"],
      rows: [
        [
          { id: "a", node: "text", runs: [{ text: "lint" }] },
          { id: "b", node: "text", runs: [{ text: "green" }] },
        ],
      ],
    },
    { id: "ask", node: "input", value: "", hint: "a name" },
    { id: "go", node: "button", label: "Run" },
    { id: "half", node: "progress", done: 0.5 },
    { id: "wait", node: "progress" },
    { id: "src", node: "code", text: "print('hi')", language: "python" },
    { id: "patch", node: "diff", text: "--- a/x\n+++ b/x\n@@ -1 +1 @@\n-old\n+new\n" },
    { id: "doc", node: "markdown", text: "Some **prose**." },
  ],
};

/** An App on the demo, with a test renderer, and each event that its views said. */
async function opened(role: "read" | "write" = "write") {
  const session = await demoSession({ seed: true });
  const library = await demoLibrary(session);
  (session.host as { role: "read" | "write" }).role = role;
  const screen = await createTestRenderer({ width: 100, height: 60, exitSignals: [] });
  const app = new App(screen.renderer, session, { quit() {}, workspaces: library });
  const said: ViewEvent[] = [];
  app.viewEvents = (event) => said.push(event);
  // The end of a load of the parts draws the dock again, and a new input would lose its focus in a test.
  await app.partsReady;
  const tree = (version: number, node = kit, on = session.selected): Tree => ({
    viewer: "v1",
    slot: { slot: "dock", on },
    version,
    tree: node,
  });
  const draw = async () => {
    app.render();
    await screen.renderOnce();
  };
  return {
    session,
    app,
    screen,
    said,
    tree,
    draw,
    find: (id: string) => app.renderer.root.findDescendantById(`view:v1:${id}`),
    close: () => {
      app.dispose();
      screen.renderer.destroy();
    },
  };
}

test("the dock draws each node of a tree on the chain on screen, and no tree of another chain", async () => {
  const life = await opened();
  try {
    life.app.showTree(life.tree(1, kit, "chain999"));
    await life.draw();
    expect(life.find("title")).toBeUndefined();
    life.app.showTree(life.tree(2));
    // Markdown takes a frame or more to parse, so the frame is read once it draws the prose.
    for (let tries = 0; tries < 50 && !life.screen.captureCharFrame().includes("Some prose."); tries++) {
      await Bun.sleep(20);
      await life.draw();
    }
    const frameText = life.screen.captureCharFrame();
    for (const text of ["Test skill ready", "first step", "lint", "green", "[ Run ]", "50%", "Some prose."])
      expect(frameText).toContain(text);
    for (const id of ["title", "steps", "grid", "ask", "go", "half", "wait", "src", "patch", "doc"])
      expect(life.find(id)).toBeDefined();
  } finally {
    life.close();
  }
});

test("a tree takes the place of each tree of a lower version, and a lower version changes nothing", async () => {
  const life = await opened();
  const said = (text: string): Node => ({ id: "title", node: "text", runs: [{ text }] });
  try {
    life.app.showTree(life.tree(2, said("second")));
    life.app.showTree(life.tree(1, said("first")));
    await life.draw();
    expect(life.screen.captureCharFrame()).toContain("second");
    expect(life.screen.captureCharFrame()).not.toContain("first");
    life.app.disposeView("v1");
    await life.draw();
    expect(life.screen.captureCharFrame()).not.toContain("second");
  } finally {
    life.close();
  }
});

test("a press and the keys of an input go to the process of the view under the id of the node", async () => {
  const life = await opened();
  try {
    life.app.showTree(life.tree(1));
    await life.draw();
    const go = life.find("go");
    if (!go) throw new Error("No button.");
    await life.screen.mockMouse.click(go.x + 1, go.y);
    const field = life.find("ask");
    if (!(field instanceof InputRenderable)) throw new Error("No input.");
    await life.draw();
    await life.screen.mockMouse.click(field.x + 1, field.y);
    await life.draw();
    expect(field.focused).toBe(true);
    await life.screen.mockInput.typeText("ab");
    life.screen.mockInput.pressEnter();
    await life.draw();
    expect(life.said).toEqual([
      { viewer: "v1", node: "go", kind: "press" },
      { viewer: "v1", node: "ask", kind: "change", value: "a" },
      { viewer: "v1", node: "ask", kind: "change", value: "ab" },
      { viewer: "v1", node: "ask", kind: "submit", value: "ab" },
    ]);
    expect(field.focused).toBe(false);
  } finally {
    life.close();
  }
});

test("a member who reads only sees each acting node inert", async () => {
  const life = await opened("read");
  try {
    life.app.showTree(
      life.tree(1, {
        ...kit,
        children: kit.children.map((one) => (one.id === "ask" ? ({ ...one, value: "kept" } as Node) : one)),
      } as Node),
    );
    await life.draw();
    const go = life.find("go");
    if (!go) throw new Error("No button.");
    await life.screen.mockMouse.click(go.x + 1, go.y);
    expect(life.find("ask")).toBeInstanceOf(TextRenderable);
    expect(life.screen.captureCharFrame()).toContain("kept");
    expect(life.said).toEqual([]);
  } finally {
    life.close();
  }
});

test("a spinner moves with the clock of the TUI", async () => {
  const life = await opened();
  try {
    setSystemTime(new Date(1_000_000));
    life.app.showTree(life.tree(1));
    await life.draw();
    const wait = life.find("wait");
    if (!(wait instanceof TextRenderable)) throw new Error("No spinner.");
    const first = wait.plainText;
    setSystemTime(new Date(1_000_000 + frame));
    life.app.tickViews();
    expect(wait.plainText).toBe(spin());
    expect(wait.plainText).not.toBe(first);
  } finally {
    setSystemTime();
    life.close();
  }
});

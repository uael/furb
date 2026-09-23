import { afterAll, expect, test } from "bun:test";
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { type Renderable, TextRenderable } from "@opentui/core";
import { createTestRenderer } from "@opentui/core/testing";
import { until } from "../../bind/typescript/test/until.ts";
import { App } from "../src/app.ts";
import { openEngine } from "../src/bridge.ts";
import { demoSession, removeDemoDirectories } from "../src/demo.ts";
import { Session, type View } from "../src/session.ts";

afterAll(removeDemoDirectories);

/** Every text under a node of the view. */
const texts = (node: Renderable): string[] =>
  node
    .getChildren()
    .flatMap((child) => [...(child instanceof TextRenderable ? [child.plainText] : []), ...texts(child)]);

test("a view opens at the offset it was left at, after a shorter view, in a new App, and after a reopen", async () => {
  const session = await demoSession(true);
  const screen = await createTestRenderer({ width: 120, height: 30 });
  let app = new App(screen.renderer, session, { quit() {} });
  const show = async (view: View) => {
    session.show(view);
    await session.refresh();
    app.render();
    await screen.flush();
    await screen.flush();
  };
  try {
    await show("facts");
    expect(app.scroll.scrollHeight).toBeGreaterThan(app.scroll.viewport.height + 20);
    app.scroll.scrollTo(20);
    await show("changes");
    expect(app.scroll.scrollHeight).toBeLessThanOrEqual(app.scroll.viewport.height);
    await show("facts");
    expect(app.scroll.scrollTop).toBe(20);
    app.dispose();
    app = new App(screen.renderer, session, { quit() {} });
    await screen.flush();
    await screen.flush();
    expect(app.scroll.scrollTop).toBe(20);
  } finally {
    app.dispose();
  }
  const record = session.world.records.path;
  await session.dispose();
  const opened = await openEngine({ record, demo: true });
  const reopened = new Session(opened.life, opened.world, true);
  await reopened.refresh();
  app = new App(screen.renderer, reopened, { quit() {} });
  try {
    await screen.flush();
    await screen.flush();
    expect(reopened.view).toBe("facts");
    expect(app.scroll.scrollTop).toBe(20);
  } finally {
    app.dispose();
    screen.renderer.destroy();
    await reopened.dispose();
  }
}, 30000);

test("a command that printed more than a row holds sends the tail and its length, and its open card shows all of it", async () => {
  const session = await demoSession();
  const screen = await createTestRenderer({ width: 120, height: 40, useMouse: true });
  const app = new App(screen.renderer, session, { quit() {} });
  try {
    await session.submit("/bash seq 1 3000");
    const command = session.activity.find((act) => act.kind === "bash");
    if (!command) throw new Error("No command.");
    await session.life.result(command.id);
    await session.refresh();
    const row = session.acts.find((act) => act.id === command.id);
    const printed = Array.from({ length: 3000 }, (_, index) => `${index + 1}\n`).join("");
    const stdout = (row?.value as { stdout: { content: string } }).stdout.content;
    expect(row?.output).toBe(printed.length);
    expect(stdout.length).toBeLessThanOrEqual(2000);
    expect(printed.endsWith(stdout)).toBe(true);
    session.show("activity");
    app.render();
    await screen.flush();
    const card = app.scroll.getChildren().find((node) => node.id === command.id);
    const heading = card?.getChildren()[0];
    if (!card || !heading) throw new Error("No card for the command.");
    await screen.mockMouse.click(heading.x + 1, heading.y);
    await screen.flush();
    const opened = app.scroll.getChildren().find((node) => node.id === command.id);
    if (!opened) throw new Error("No open card for the command.");
    await screen.waitFor(() => texts(opened).includes(printed), { maxPasses: 200 });
    expect(texts(opened)).toContain(printed);
  } finally {
    app.dispose();
    screen.renderer.destroy();
    await session.dispose();
  }
}, 30000);

test("the views say each quantity one way, read a page of changes once, and set a heading only when it changes", async () => {
  const session = await demoSession();
  const screen = await createTestRenderer({ width: 150, height: 40 });
  const app = new App(screen.renderer, session, { quit() {} });
  const reads: number[] = [];
  const readChanges = session.world.readChanges.bind(session.world);
  session.world.readChanges = (start, count) => {
    reads.push(start);
    return readChanges(start, count);
  };
  try {
    await session.submit("/context 0.3");
    await session.submit("/grant 1.5");
    session.show("activity");
    await session.refresh();
    app.render();
    await screen.flush();
    const frame = screen.captureCharFrame();
    expect(frame).toContain("30% context");
    expect(frame).not.toContain("30.000000000000004");
    expect(frame).toContain("$1.50");
    const card = app.scroll.getChildren().find((node) => node.id.startsWith("grant://"));
    const heading = card?.getChildren()[0] as TextRenderable | undefined;
    if (!heading) throw new Error("No card for the grant.");
    const content = heading.content;
    app.render();
    await screen.flush();
    expect(heading.content).toBe(content);
    await session.life.result(await session.life.rung('write(Text("note.txt", "one\\n"))'));
    await until(session.world, () => session.world.changes === 1);
    session.show("changes");
    await session.refresh();
    await session.refresh();
    expect(reads).toEqual([0]);
    expect(session.changes[0]?.patch).toContain("+one");
    await session.life.result(await session.life.rung('write(Text("note.txt", "two\\n"))'));
    await until(session.world, () => session.world.changes === 2);
    await session.refresh();
    expect(reads).toEqual([0, 0]);
    expect(session.changes[1]?.patch).toContain("+two");
  } finally {
    session.world.readChanges = readChanges;
    app.dispose();
    screen.renderer.destroy();
    await session.dispose();
  }
}, 30000);

test("a relative path that the operator types is read from the directory of the selected chain", async () => {
  const session = await demoSession();
  try {
    const directory = join(session.world.directory, "sub");
    await mkdir(directory);
    const pixel =
      "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/l9sAAAAASUVORK5CYII=";
    await writeFile(join(directory, "pixel.png"), Buffer.from(pixel, "base64"));
    await session.submit("/cd sub");
    await until(session, () => session.directory === "sub");
    expect(session.path("out.json")).toBe(join(directory, "out.json"));
    await session.submit("/export out.json");
    await session.submit("/share page.html");
    await session.attachImage("pixel.png");
    expect(JSON.parse(await Bun.file(join(directory, "out.json")).text()).chain).toBe(session.selected);
    expect(await Bun.file(join(directory, "page.html")).exists()).toBe(true);
    expect(session.images[session.selected]?.map((image) => image.name)).toEqual(["pixel.png"]);
  } finally {
    await session.dispose();
  }
}, 30000);

test("a card that the view goes to, or that Details expands, is in view once the view has laid it out", async () => {
  const session = await demoSession(true);
  const screen = await createTestRenderer({ width: 120, height: 24 });
  const app = new App(screen.renderer, session, { quit() {} });
  const laid = async () => {
    app.render();
    await screen.flush();
    await screen.flush();
  };
  const card = (id: string) => {
    const found = app.scroll.getChildren().find((node) => node.id === id);
    if (!found) throw new Error(`No card ${id} in the view.`);
    return found;
  };
  // A card taller than the view is in view when its top is at the top of the view.
  const seen = (id: string) => {
    const { y, height } = card(id);
    const view = app.scroll.viewport;
    return y >= view.y && (y + height <= view.y + view.height || y === view.y);
  };
  try {
    for (let i = 0; i < 24; i++) await session.command(`/run v${i} = ${i}`);
    await session.command("/run target = 1");
    // A rung the World has not run yet stands in the program once it runs.
    await until(session, () => Object.values(session.program).includes("target = 1"));
    const rung = Object.entries(session.program).find(([, word]) => word === "target = 1")?.[0];
    if (!rung) throw new Error("No rung in the program.");
    session.show("conversation");
    await session.refresh();
    await laid();
    await app.inspect("target");
    await laid();
    screen.mockInput.pressEnter();
    await laid();
    expect(session.view).toBe("program");
    expect(app.scroll.scrollHeight).toBeGreaterThan(app.scroll.viewport.height * 2);
    expect(seen(rung)).toBe(true);
    session.show("facts");
    await laid();
    const view = app.scroll.viewport;
    const last = app.scroll.getChildren().find((node) => node.y === view.y + view.height - 1);
    if (!last) throw new Error("No card on the last line of the view.");
    const shut = card(last.id).height;
    app.details();
    const heading = (last.getChildren()[0] as TextRenderable).plainText.replace(/^[▸▾] /, "");
    await screen.mockInput.typeText(heading);
    screen.mockInput.pressEnter();
    await laid();
    expect(card(last.id).height).toBeGreaterThan(shut);
    expect(seen(last.id)).toBe(true);
  } finally {
    app.dispose();
    screen.renderer.destroy();
    await session.dispose();
  }
});

test("the palette lists as many choices as its rows hold, with the selected one among them, inside its border", async () => {
  const session = await demoSession();
  const screen = await createTestRenderer({ width: 120, height: 30 });
  const app = new App(screen.renderer, session, { quit() {} });
  const frame = async () => {
    app.render();
    await screen.flush();
    return screen.captureCharFrame().split("\n");
  };
  try {
    app.palette();
    let lines = await frame();
    const bottom = lines.findIndex((line) => line.includes("╰"));
    expect(lines[bottom]?.trim()).toMatch(/^╰─+╯$/);
    const count = lines.filter((line) => line.includes("│   ") || line.includes("│ ▸ ")).length;
    for (let i = 0; i < count + 4; i++) screen.mockInput.pressArrow("down");
    lines = await frame();
    expect(lines.find((line) => line.includes("╰"))?.trim()).toMatch(/^╰─+╯$/);
    expect(lines.some((line) => line.includes("│ ▸ "))).toBe(true);
  } finally {
    app.dispose();
    screen.renderer.destroy();
    await session.dispose();
  }
});

test("a text that truncates keeps one line and shows where it was cut", async () => {
  const session = await demoSession();
  const screen = await createTestRenderer({ width: 120, height: 30 });
  const app = new App(screen.renderer, session, { quit() {} });
  try {
    session.notice = `A notice longer than its line ${"and longer ".repeat(20)}to its end`;
    app.render();
    await screen.flush();
    const lines = screen.captureCharFrame().split("\n");
    const status = lines.findLast((line) => line.includes("A notice longer"));
    expect(status).toContain("...");
    expect(status).toContain("demo.jsonl");
    expect(lines.filter((line) => line.includes("and longer")).length).toBe(1);
  } finally {
    app.dispose();
    screen.renderer.destroy();
    await session.dispose();
  }
});

test("the conversation left at its end opens at its end, and one left above its end opens where it was", async () => {
  const session = await demoSession(true);
  const screen = await createTestRenderer({ width: 120, height: 30 });
  const app = new App(screen.renderer, session, { quit() {} });
  const show = async (view: View) => {
    session.show(view);
    await session.refresh();
    app.render();
    await screen.flush();
    await screen.flush();
  };
  const end = () => Math.max(0, app.scroll.scrollHeight - app.scroll.viewport.height);
  try {
    await show("conversation");
    expect(app.scroll.scrollTop).toBe(end());
    await show("facts");
    for (let i = 0; i < 12; i++) await session.command(`/run grown${i} = ${i}`);
    await until(session, () => Object.values(session.program).includes("grown11 = 11"));
    await show("conversation");
    expect(end()).toBeGreaterThan(0);
    expect(app.scroll.scrollTop).toBe(end());
    app.scroll.scrollTo(3);
    await show("facts");
    await show("conversation");
    expect(app.scroll.scrollTop).toBe(3);
  } finally {
    app.dispose();
    screen.renderer.destroy();
    await session.dispose();
  }
});

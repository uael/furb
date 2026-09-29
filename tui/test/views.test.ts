import { afterAll, expect, test } from "bun:test";
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { type Renderable, RGBA, TextRenderable } from "@opentui/core";
import { createTestRenderer } from "@opentui/core/testing";
import { until } from "../../bind/typescript/test/until.ts";
import { find } from "../script/stage.ts";
import { App } from "../src/app.ts";
import { demoLibrary, demoSession, removeDemoDirectories } from "../src/demo.ts";
import type { View } from "../src/session.ts";
import { palettes } from "../src/theme.ts";
import { type Composing, cellAt, composing, withDemo } from "./composing.ts";
import { idle } from "./idle.ts";

afterAll(removeDemoDirectories);

/** Every text under a node of the view. */
const texts = (node: Renderable): string[] =>
  node
    .getChildren()
    .flatMap((child) => [...(child instanceof TextRenderable ? [child.plainText] : []), ...texts(child)]);

/** The first text under a node that the view shows. */
const visible = (node: Renderable): TextRenderable | undefined => {
  for (const child of node.getChildren()) {
    const found = child instanceof TextRenderable && child.visible ? child : visible(child);
    if (found) return found;
  }
  return undefined;
};

/** A view shown, read, and laid out, so that it stands where it opens. */
async function show({ session, app, screen }: Pick<Composing, "session" | "app" | "screen">, view: View) {
  session.show(view);
  await session.refresh();
  app.render();
  await screen.flush();
  await screen.flush();
}

test("a view opens at the offset it was left at, after a shorter view, in a new App, and after a reopen", async () => {
  const session = await demoSession({ seed: true });
  const library = await demoLibrary(session);
  const screen = await createTestRenderer({ width: 120, height: 30 });
  let app = new App(screen.renderer, session, { quit() {}, workspaces: library });
  let record: string | undefined;
  try {
    await show({ session, app, screen }, "transcript");
    expect(app.scroll.scrollHeight).toBeGreaterThan(app.scroll.viewport.height + 20);
    app.scroll.scrollTo(20);
    await show({ session, app, screen }, "changes");
    expect(app.scroll.scrollHeight).toBeLessThanOrEqual(app.scroll.viewport.height);
    await show({ session, app, screen }, "transcript");
    expect(app.scroll.scrollTop).toBe(20);
    app.dispose();
    app = new App(screen.renderer, session, app.options);
    await screen.flush();
    await screen.flush();
    expect(app.scroll.scrollTop).toBe(20);
    record = session.host.record;
  } finally {
    app.dispose();
    screen.renderer.destroy();
    await library.dispose();
  }
  const reopened = await demoSession({ record });
  await composing(
    async ({ session, app, screen }) => {
      await screen.flush();
      expect(session.view).toBe("transcript");
      expect(app.scroll.scrollTop).toBe(20);
    },
    { width: 120, height: 30 },
    reopened,
  );
});

test("a command that printed more than a row holds sends the tail and its length, and its open card shows all of it", () =>
  composing(
    async ({ session, app, screen, frame }) => {
      await session.submit("/bash seq 1 3000");
      const command = session.activity.find((act) => act.kind === "bash");
      if (!command) throw new Error("No command.");
      await session.engine.result(command.id);
      await session.refresh();
      const row = session.acts.find((act) => act.id === command.id);
      const printed = Array.from({ length: 3000 }, (_, index) => `${index + 1}\n`).join("");
      const stdout = (row?.value as { stdout: { content: string } }).stdout.content;
      expect(row?.output).toBe(printed.length);
      expect(stdout.length).toBeLessThanOrEqual(2000);
      expect(printed.endsWith(stdout)).toBe(true);
      await frame();
      const card = app.scroll.getChildren().find((node) => node.id === command.id);
      const heading = card?.getChildren()[0];
      if (!card || !heading) throw new Error("No card for the command.");
      await screen.mockMouse.click(heading.x + 1, heading.y);
      await screen.flush();
      const opened = app.scroll.getChildren().find((node) => node.id === command.id);
      if (!opened) throw new Error("No open card for the command.");
      // The line end that closes the output ends its last row, and the card draws no empty row for it.
      const shown = printed.replace(/\n$/, "");
      await screen.waitFor(() => texts(opened).includes(shown), { maxPasses: 200 });
      expect(texts(opened)).toContain(shown);
    },
    { width: 120, height: 40, useMouse: true },
  ));

test("the views say each quantity one way, read a page of changes once, and set a heading only when it changes", () =>
  composing(
    async ({ session, app, frame }) => {
      const reads: number[] = [];
      const readChanges = session.host.readChanges.bind(session.host);
      session.host.readChanges = (start, count) => {
        reads.push(start);
        return readChanges(start, count);
      };
      try {
        // The sidebar says the ceiling of the grant that holds now: a share of the context, then a sum of dollars.
        await session.submit("/context 0.3");
        await session.engine.result(
          await session.engine.rung({ word: "counted = 1", on: session.engine.root }),
        );
        await session.submit("/grant 1.5");
        await session.refresh();
        expect(await frame()).toContain("$1.50");
        const card = app.scroll.getChildren().find((node) => /^rung\d+$/.test(node.id));
        const heading = card?.getChildren()[0] as TextRenderable | undefined;
        if (!heading) throw new Error("No card for the rung.");
        const content = heading.content;
        await frame();
        expect(heading.content).toBe(content);
        await session.engine.result(
          await session.engine.rung({ word: 'write(Text("note.txt", "one\\n"))', on: session.engine.root }),
        );
        await until(session.host, () => session.host.changes === 1);
        session.show("changes");
        await session.refresh();
        await session.refresh();
        expect(reads).toEqual([0]);
        expect(session.changes[0]?.patch).toContain("+one");
        // A view that shows a change says no word of an empty view.
        expect(await frame()).not.toContain("No file changes yet");
        await session.engine.result(
          await session.engine.rung({ word: 'write(Text("note.txt", "two\\n"))', on: session.engine.root }),
        );
        await until(session.host, () => session.host.changes === 2);
        await session.refresh();
        expect(reads).toEqual([0, 0]);
        expect(session.changes[1]?.patch).toContain("+two");
      } finally {
        session.host.readChanges = readChanges;
      }
    },
    { width: 150, height: 40 },
  ));

test("a relative path that the operator types is read from the directory of the selected chain", () =>
  withDemo(async (session) => {
    const directory = join(session.host.directory, "sub");
    await mkdir(directory);
    const pixel =
      "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/l9sAAAAASUVORK5CYII=";
    await writeFile(join(directory, "pixel.png"), Buffer.from(pixel, "base64"));
    await session.submit("/cd sub");
    await until(session, () => session.directory === directory);
    expect(session.path("out.json")).toBe(join(directory, "out.json"));
    await session.submit("/export out.json");
    await session.submit("/share page.html");
    await session.attachImage("pixel.png");
    expect(JSON.parse(await Bun.file(join(directory, "out.json")).text()).chain).toBe(session.selected);
    expect(await Bun.file(join(directory, "page.html")).exists()).toBe(true);
    expect(session.images[session.selected]?.map((image) => image.name)).toEqual(["pixel.png"]);
  }));

test("the standing of a chain is no card of the conversation, though its turns bind its three parts", () =>
  composing(
    async (context) => {
      const { session, app } = context;
      await show(context, "feed");
      const root = session.engine.root;
      const told = session.turns.flatMap(([role, python]) => (role === "user" ? python.split("\n") : []));
      const at = told.indexOf(`#${root} standing`);
      expect(told.slice(at + 1, at + 4).map((line) => line.split(" = ", 1)[0])).toEqual([
        `${root}_roster`,
        `${root}_cwd`,
        `${root}_actor`,
      ]);
      const shown = texts(app.scroll).map((text) => text.replace(/^\S+ /, ""));
      const thread = session.acts.find((act) => act.kind === "thread" && act.by === "operator");
      expect(shown.some((text) => text.includes(String(thread?.words[1])))).toBe(true);
      expect(shown.filter((text) => /^(standing|roster|cwd|actor)\b/.test(text))).toEqual([]);
    },
    { width: 120, height: 30 },
    true,
  ));

test("a card that the view goes to, or that Details expands, is in view once the view has laid it out", () =>
  composing(
    async ({ session, app, screen, frame }) => {
      const laid = async () => {
        await frame();
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
      for (let i = 0; i < 24; i++) await session.command(`/run v${i} = ${i}`);
      await session.command("/run target = 1");
      // A rung the Kernel has not run yet stands in the program once it runs.
      await until(session, () => Object.values(session.program).includes("target = 1"));
      const rung = Object.entries(session.program).find(([, word]) => word === "target = 1")?.[0];
      if (!rung) throw new Error("No rung in the program.");
      session.show("transcript");
      await session.refresh();
      await laid();
      await app.inspect("target");
      await laid();
      screen.mockInput.pressEnter();
      await laid();
      expect(session.view).toBe("feed");
      expect(app.scroll.scrollHeight).toBeGreaterThan(app.scroll.viewport.height * 2);
      expect(seen(rung)).toBe(true);
      // Folded rungs take a row each, so that the heading of one stands at the foot of the view.
      session.preferences.foldRungs = true;
      app.scroll.scrollTo(app.scroll.scrollHeight);
      await laid();
      const view = app.scroll.viewport;
      const last = app.scroll
        .getChildren()
        .filter((node) => /^rung\d+$/.test(node.id) && node.y >= view.y && node.y < view.y + view.height)
        .at(-1);
      if (!last) throw new Error("No folded rung at the foot of the view.");
      const shut = card(last.id).height;
      app.details();
      const heading = (last.getChildren()[0] as TextRenderable).plainText.replace(/^[▸▾] /, "");
      await screen.mockInput.typeText(heading);
      screen.mockInput.pressEnter();
      await laid();
      expect(card(last.id).height).toBeGreaterThan(shut);
      expect(seen(last.id)).toBe(true);
    },
    { width: 120, height: 24 },
    true,
  ));

test("the palette lists as many choices as its rows hold, with the selected one among them, inside its panel", () =>
  composing(
    async ({ app, screen, frame }) => {
      /** The rows of the frame that the palette takes, and the row of its selected choice. */
      const palette = async () => {
        const lines = (await frame()).split("\n");
        const found = (node: Renderable): Renderable | undefined =>
          node.id === "palette" ? node : node.getChildren().map(found).find(Boolean);
        const panel = found(screen.renderer.root);
        if (!panel) throw new Error("No palette.");
        return {
          top: panel.y,
          bottom: panel.y + panel.height,
          // The selected choice has the bar of the selection at its left, inside the panel.
          selected: lines.findIndex(
            (line, row) =>
              row >= panel.y &&
              row < panel.y + panel.height &&
              line.slice(panel.x, panel.x + panel.width).includes("▎ "),
          ),
        };
      };
      app.palette();
      let shown = await palette();
      expect(shown.bottom).toBeLessThanOrEqual(30);
      for (let i = 0; i < shown.bottom - shown.top + 4; i++) screen.mockInput.pressArrow("down");
      shown = await palette();
      expect(shown.bottom).toBeLessThanOrEqual(30);
      expect(shown.selected).toBeGreaterThan(shown.top);
      expect(shown.selected).toBeLessThan(shown.bottom);
    },
    { width: 120, height: 30 },
  ));

test("the feed left at its end opens at its end, and one left above its end opens where it was", () =>
  composing(
    async (context) => {
      const { session, app } = context;
      const end = () => Math.max(0, app.scroll.scrollHeight - app.scroll.viewport.height);
      await show(context, "feed");
      expect(app.scroll.scrollTop).toBe(end());
      await show(context, "transcript");
      for (let i = 0; i < 12; i++) await session.command(`/run grown${i} = ${i}`);
      await until(session, () => Object.values(session.program).includes("grown11 = 11"));
      await show(context, "feed");
      expect(end()).toBeGreaterThan(0);
      expect(app.scroll.scrollTop).toBe(end());
      app.scroll.scrollTo(3);
      await show(context, "transcript");
      await show(context, "feed");
      expect(app.scroll.scrollTop).toBe(3);
    },
    { width: 120, height: 30 },
    true,
  ));

test("the feed at its end stays at its end when the input grows, and a turn of the wheel stays where it went", () =>
  composing(
    async (context) => {
      const { session, app, screen } = context;
      const end = () => Math.max(0, app.scroll.scrollHeight - app.scroll.viewport.height);
      await show(context, "transcript");
      for (let i = 0; i < 12; i++) await session.command(`/run grown${i} = ${i}`);
      await until(session, () => Object.values(session.program).includes("grown11 = 11"));
      await show(context, "feed");
      const tall = app.scroll.viewport.height;
      app.composer.setText("one\ntwo\nthree\nfour\nfive");
      app.render();
      await screen.flush();
      await screen.flush();
      expect(app.scroll.viewport.height).toBeLessThan(tall);
      expect(app.scroll.scrollTop).toBe(end());
      // A render that changes nothing on the screen draws no frame, and the wheel still moves the feed.
      app.render();
      await screen.flush();
      await screen.mockMouse.scroll(10, 5, "up");
      await screen.flush();
      await screen.flush();
      expect(app.scroll.scrollTop).toBeLessThan(end());
    },
    { width: 120, height: 30, useMouse: true },
    true,
  ));

test("the keys of the footer and the palette answer the mouse, and a drag over a heading selects and copies its text", () =>
  composing(
    async ({ session, app, screen, frame, click }) => {
      await idle(session);
      await click("F1 help");
      await frame();
      expect(find(screen, "Keys and commands")[1]).toBeGreaterThan(0);
      // The wheel moves the selection of the palette, and a click on a choice runs it.
      app.closeOverlay();
      app.palette();
      await frame();
      const [x, y] = find(screen, "Transcript view");
      await screen.mockMouse.scroll(x, y, "down");
      await screen.flush();
      await screen.mockMouse.click(x, y);
      expect(session.view).toBe("transcript");
      session.show("feed");
      // The words of a model stand in the feed of their thread.
      const thread = session.acts.find((act) => act.kind === "thread" && act.by === "operator");
      if (!thread) throw new Error("No thread of the operator.");
      await session.open(thread.id);
      // The screen draws the feed, with the palette gone, before the pointer acts on it again.
      await frame();
      // A drag over the first step of a word selects its text and leaves the word as it was.
      const word = () => app.scroll.getChildren().find((node) => /^rung\d+$/.test(node.id));
      const step = () => {
        const card = word();
        return card && visible(card);
      };
      const first = step();
      if (!first) throw new Error("No word in the feed.");
      const text = first.plainText;
      await screen.mockMouse.drag(first.x + 2, first.y, first.x + 8, first.y);
      await frame();
      expect(step()?.plainText).toBe(text);
      const selected = screen.renderer.getSelection()?.getSelectedText() ?? "";
      expect(selected.length).toBeGreaterThan(0);
      expect(text).toContain(selected);
      expect(session.notice).toStartWith("Copied");
      // A word that is over starts folded to its steps, and a click on a step opens it.
      expect(text).toStartWith("▸ ");
      // A click right after a drag is a second click, which selects a word, so the selection goes first.
      screen.renderer.clearSelection();
      await screen.mockMouse.click(first.x + 2, first.y);
      await frame();
      expect(step()?.plainText).toStartWith("▾ ");
    },
    { width: 140, height: 40, useMouse: true },
    true,
  ));

test("the root chain stands in the list of chains when it rests, and the other resting chains fold under Finished", () =>
  composing(
    async ({ session, frame }) => {
      const { engine } = session;
      const side = await engine.chain({ label: "Side", source: engine.root });
      await engine.chain({ label: "Notes", source: engine.root });
      await session.refresh();
      await session.select(side);
      const left = 140 - session.preferences.sidebarWidth;
      const rows = (await frame()).split("\n").map((line) => line.slice(left).trim());
      expect(rows.slice(1, 4)).toEqual(["○ Main", "▎ ○ Side", "▸ Finished  1"]);
    },
    { width: 140, height: 30 },
  ));

test("a title longer than its room keeps its start, and ends with an ellipsis", () =>
  composing(
    async ({ session, frame }) => {
      const left = 120 - session.preferences.sidebarWidth;
      const rows = (await frame()).split("\n").map((line) => line.slice(left));
      const row = rows.find((line) => line.includes("Explore this project")) ?? "";
      expect(row.trimEnd()).toMatch(/Explore this project, .*…$/);
      expect(row).not.toContain("...");
    },
    { width: 120, height: 30 },
    true,
  ));

test("a block of code in an answer stands on the surface of a block, in the colors of its language", () =>
  composing(async ({ session, screen, frame }) => {
    const id = await session.engine.thread("str", {
      markdown: "Show the code.",
      to: "operator",
      on: session.engine.root,
    });
    await until(session.host, () => session.host.threads.has(id));
    await session.refresh();
    await session.submit("```python\nanswer = 42\n```");
    await until(session, () => session.acts.some((act) => act.id === id && act.done));
    await session.open(id);
    const shown = await frame();
    expect(shown).not.toContain("```");
    const [x, y] = find(screen, "answer = 42");
    const surface = RGBA.fromHex(palettes[session.theme].surface2);
    expect(cellAt(screen.captureSpans(), x, y).bg?.equals(surface)).toBe(true);
    // A name keeps the color of text, and a number takes the warm color, as in the Python of a word.
    expect(cellAt(screen.captureSpans(), x, y).fg?.equals(RGBA.fromHex(palettes[session.theme].bright))).toBe(
      true,
    );
    expect(
      cellAt(screen.captureSpans(), x + 9, y).fg?.equals(RGBA.fromHex(palettes[session.theme].warm)),
    ).toBe(true);
  }));

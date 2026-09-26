import { afterAll, expect, test } from "bun:test";
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { type Renderable, TextRenderable } from "@opentui/core";
import { createTestRenderer } from "@opentui/core/testing";
import { until } from "../../bind/typescript/test/until.ts";
import { App } from "../src/app.ts";
import { openEngine } from "../src/bridge.ts";
import { demoLibrary, demoSession, removeDemoDirectories } from "../src/demo.ts";
import { Session, type View } from "../src/session.ts";
import { type Composing, composing } from "./composing.ts";
import { idle } from "./idle.ts";

afterAll(removeDemoDirectories);

/** Every text under a node of the view. */
const texts = (node: Renderable): string[] =>
  node
    .getChildren()
    .flatMap((child) => [...(child instanceof TextRenderable ? [child.plainText] : []), ...texts(child)]);

/** A view shown, read, and laid out, so that it stands where it opens. */
async function show({ session, app, screen }: Pick<Composing, "session" | "app" | "screen">, view: View) {
  session.show(view);
  await session.refresh();
  app.render();
  await screen.flush();
  await screen.flush();
}

test("a view opens at the offset it was left at, after a shorter view, in a new App, and after a reopen", async () => {
  const session = await demoSession(true);
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
  const opened = await openEngine({ record, demo: true });
  const reopened = new Session(opened.engine, opened.host, true);
  await reopened.refresh();
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
    async ({ session, app, screen }) => {
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
      app.render();
      await screen.flush();
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
    async ({ session, app, screen }) => {
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
        await session.refresh();
        app.render();
        await screen.flush();
        let frame = screen.captureCharFrame();
        const lines = frame.split("\n");
        const meter = lines.findIndex((line) => line.includes("━"));
        await screen.mockMouse.moveTo((lines[meter] ?? "").indexOf("━") + 2, meter);
        await screen.flush();
        frame = screen.captureCharFrame();
        expect(frame).toContain("pauses at 30%");
        expect(frame).not.toContain("30.000000000000004");
        await screen.mockMouse.moveTo(0, 0);
        await session.submit("/grant 1.5");
        await session.refresh();
        app.render();
        await screen.flush();
        frame = screen.captureCharFrame();
        expect(frame).toContain("$1.50");
        const card = app.scroll.getChildren().find((node) => /^rung\d+$/.test(node.id));
        const heading = card?.getChildren()[0] as TextRenderable | undefined;
        if (!heading) throw new Error("No card for the rung.");
        const content = heading.content;
        app.render();
        await screen.flush();
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

test("a relative path that the operator types is read from the directory of the selected chain", async () => {
  const session = await demoSession();
  try {
    const directory = join(session.host.directory, "sub");
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
});

test("the standing of a chain is no card of the conversation, though its turns hold its three rows", () =>
  composing(
    async (context) => {
      const { session, app } = context;
      await show(context, "feed");
      const root = session.engine.root;
      const told = session.turns.flatMap(([role, python]) => (role === "user" ? python.split("\n") : []));
      const at = told.findIndex((line) => line.startsWith(`#${root} roster `));
      expect(told.slice(at, at + 3).map((line) => line.split(" ", 2))).toEqual([
        [`#${root}`, "roster"],
        [`#${root}`, "cwd"],
        [`#${root}`, "actor"],
      ]);
      const headings = app.scroll.getChildren().map((card) => (texts(card)[0] ?? "").replace(/^[▸▾] /, ""));
      expect(headings).toContain("You");
      expect(headings.filter((heading) => /^(roster|cwd|actor) · /.test(heading))).toEqual([]);
    },
    { width: 120, height: 30 },
    true,
  ));

test("a card that the view goes to, or that Details expands, is in view once the view has laid it out", () =>
  composing(
    async ({ session, app, screen }) => {
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

test("a text that truncates keeps one line and shows where it was cut", () =>
  composing(
    async ({ session, frame }) => {
      session.notice = `A notice longer than its line ${"and longer ".repeat(20)}to its end`;
      const lines = (await frame()).split("\n");
      const status = lines.findLast((line) => line.includes("A notice longer"));
      expect(status).toContain("…");
      expect(status).toContain("F1 help");
      expect(lines.filter((line) => line.includes("and longer")).length).toBe(1);
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

test("the toggle, the keys of the footer, and the palette answer the mouse, and a drag over a heading selects and copies its text", () =>
  composing(
    async ({ session, app, screen, frame }) => {
      /** The column and the row of the first place in the frame that shows a text. */
      const at = async (text: string): Promise<[number, number]> => {
        const lines = (await frame()).split("\n");
        const row = lines.findIndex((line) => line.includes(text));
        if (row < 0) throw new Error(`No ${text} in the frame.`);
        return [(lines[row] ?? "").indexOf(text) + 1, row];
      };
      await idle(session);
      await screen.mockMouse.click(...(await at("Transcript")));
      expect(session.view).toBe("transcript");
      await screen.mockMouse.click(...(await at("Feed ")));
      expect(session.view).toBe("feed");
      await screen.mockMouse.click(...(await at("F1 help")));
      expect((await at("Keys and commands"))[1]).toBeGreaterThan(0);
      // The wheel moves the selection of the palette, and a click on a choice runs it.
      app.closeOverlay();
      app.palette();
      const [x, y] = await at("Transcript view");
      await screen.mockMouse.scroll(x, y, "down");
      await screen.flush();
      await screen.mockMouse.click(x, y);
      expect(session.view).toBe("transcript");
      session.show("feed");
      // The screen draws the feed, with the palette gone, before the pointer acts on it again.
      app.render();
      await screen.flush();
      // A drag over the heading of a card selects its text and leaves the card as it was.
      const card = app.scroll.getChildren().find((node) => /^rung\d+$/.test(node.id));
      const heading = card?.getChildren()[0];
      if (!card || !heading) throw new Error("No rung in the feed.");
      const open = card.getChildren().length;
      await screen.mockMouse.drag(heading.x, heading.y, heading.x + 6, heading.y);
      app.render();
      await screen.flush();
      expect(
        app.scroll
          .getChildren()
          .find((node) => node.id === card.id)
          ?.getChildren().length,
      ).toBe(open);
      expect(screen.renderer.getSelection()?.getSelectedText()).toBeTruthy();
      expect(session.notice).toStartWith("Copied");
      // A rung that is over starts folded to its heading, and a click on the heading opens it.
      expect(open).toBe(1);
      await screen.mockMouse.click(heading.x + 2, heading.y);
      app.render();
      await screen.flush();
      expect(
        app.scroll
          .getChildren()
          .find((node) => node.id === card.id)
          ?.getChildren().length,
      ).toBeGreaterThan(1);
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

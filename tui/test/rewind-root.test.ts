// The rewind tree of a long life opens scrolled to its pointer, and the operator goes back to the root from there: a
// new branch that starts empty, with his first message back in the input.
import { expect, test } from "bun:test";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { composing } from "./composing.ts";
import { idle } from "./idle.ts";

test(
  "from a scrolled rewind tree the operator rewinds to the root and reads the life there",
  () =>
    composing(
      async ({ app, session, frame, screen }) => {
        for (let k = 0; k < 24; k++) {
          session.aim();
          await session.submit(`Message ${k}.`);
          await idle(session);
        }
        await session.refresh();
        const source = session.selected;
        const before = session.activity.length;
        expect(before).toBeGreaterThan(24);
        app.rewind();
        for (let k = 0; k < 10; k++) await frame();
        // The tree opens scrolled to the last message, so its root stands above the view.
        expect(app.scroll.scrollTop).toBeGreaterThan(0);
        const root = app.tree?.rows[0]?.id;
        expect(root).toBe(source);
        // The pointer goes up to the root, which the scroll follows to the top of the tree.
        for (let k = 0; k < (app.tree?.rows.length ?? 0) && app.treeRow()?.id !== root; k++) {
          screen.mockInput.pressArrow("up");
          await frame();
        }
        for (let k = 0; k < 5; k++) await frame();
        expect(app.scroll.scrollTop).toBe(0);
        expect(await frame()).toContain("start again from here");
        screen.mockInput.pressEnter();
        for (let k = 0; k < 100 && session.selected === source; k++) {
          await Bun.sleep(50);
          await frame();
        }
        expect(session.error).toBe("");
        // The life there: a new branch that reads nothing of the first, with the first message back in the input.
        expect(session.selected).not.toBe(source);
        await session.refresh();
        expect(app.tree).toBeUndefined();
        const turns = await session.engine.turns({ on: session.selected });
        expect(JSON.stringify(turns)).not.toContain("Message 1.");
        expect(JSON.stringify(turns)).not.toContain("Message 0.");
        expect(JSON.stringify(await session.engine.turns({ on: source }))).toContain("Message 23.");
        expect(session.notice).toContain("starts empty");
        expect(app.composer.plainText).toContain("Explore this project");
      },
      { width: 140, height: 44 },
      true,
    ),
  180000,
);

test(
  "the root of the tree asks what comes back of the files when a write changed one, and Both puts the file back",
  () =>
    composing(
      async ({ app, session, frame, screen }) => {
        const { engine } = session;
        const on = engine.root;
        const path = join(session.host.directory, "README.md");
        const readme = await readFile(path, "utf8");
        await engine.result(
          await engine.prompt("str", { message: "Read this project and say what it does.", on }),
        );
        await engine.result(await engine.prompt("str", { message: "Add the search shortcut, Ctrl+K.", on }));
        await idle(session);
        await session.refresh();
        expect(await readFile(path, "utf8")).not.toBe(readme);
        const source = session.selected;
        app.rewind();
        for (let k = 0; k < 10; k++) await frame();
        const root = app.tree?.rows[0]?.id;
        for (let k = 0; k < (app.tree?.rows.length ?? 0) && app.treeRow()?.id !== root; k++) {
          screen.mockInput.pressArrow("up");
          await frame();
        }
        screen.mockInput.pressEnter();
        let shown = await frame();
        for (let tries = 0; tries < 400 && !shown.includes("Restore the files"); tries++) {
          await Bun.sleep(10);
          shown = await frame();
        }
        expect(shown).toContain("Restore the conversation");
        expect(shown).toContain("README.md");
        screen.mockInput.pressArrow("down");
        screen.mockInput.pressArrow("down");
        screen.mockInput.pressEnter();
        for (let tries = 0; tries < 200 && session.selected === source; tries++) {
          await Bun.sleep(50);
          await frame();
        }
        expect(session.error).toBe("");
        expect(session.selected).not.toBe(source);
        expect(await readFile(path, "utf8")).toBe(readme);
        expect(session.notice).toContain("back as it was");
      },
      { width: 140, height: 44 },
      true,
    ),
  180000,
);

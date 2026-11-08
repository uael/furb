// The rewind tree of a long life opens scrolled to its pointer, and the operator goes back to the root from there: a
// new branch that starts empty, with his first message back in the input.
import { expect, test } from "bun:test";
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

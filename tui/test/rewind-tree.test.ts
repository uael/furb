// The rewind tree opens with its root in view while its list fits, says how many rows stand above when it scrolled to
// the pointer, and lists no step of a skill as a point to go back to.
import { expect, test } from "bun:test";
import { composing } from "./composing.ts";
import { idle } from "./idle.ts";

test(
  "a short tree opens with its root in view, and a long one says how many rows stand above",
  () =>
    composing(
      async ({ app, session, frame }) => {
        const open = async () => {
          app.rewind();
          for (let k = 0; k < 10; k++) await frame();
          return await frame();
        };
        let shown = await open();
        expect(app.scroll.scrollTop).toBe(0);
        expect(shown).toMatch(/▾ ✓ Main\s+current/);
        expect(shown).not.toContain("rows above");
        app.closeTree();
        for (let k = 0; k < 24; k++) {
          session.aim();
          await session.submit(`Message ${k}.`);
          await idle(session);
        }
        await session.refresh();
        shown = await open();
        const above = app.scroll.scrollTop;
        expect(above).toBeGreaterThan(0);
        expect(shown).toContain(`↑ ${above} rows above`);
      },
      { width: 140, height: 44 },
      true,
    ),
  120000,
);

test("a step that a skill plays, and what it made, is no point to go back to", () =>
  composing(
    async ({ app, session }) => {
      const rung = {
        id: "rung90",
        kind: "rung",
        by: "extensions",
        chain: session.engine.root,
        words: [],
      } as never;
      const act = {
        id: "rename_session9",
        kind: "rename_session",
        by: "rung90",
        chain: session.engine.root,
      } as never;
      const actOf = session.actOf.bind(session);
      session.actOf = (path: string) => (path === "rung90" ? rung : actOf(path));
      expect(app.isPoint(rung)).toBe(false);
      expect(app.isPoint(act)).toBe(false);
      const mine = session.activity.find((one) => one.kind === "prompt");
      if (mine) expect(app.isPoint(mine)).toBe(true);
    },
    { width: 140, height: 44 },
    true,
  ));

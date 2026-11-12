import { expect, test } from "bun:test";
import { until } from "../../bind/typescript/test/until.ts";
import { demoSession } from "../src/demo.ts";
import { composing, framed, ownPreferences } from "./composing.ts";

// The top line of the sidebar holds a head, and a tail of its parts. The tail drops its last parts, one by one,
// before the head is cut, so that "needs you" stays whole at the least width of the sidebar and the count of active
// prompts, which comes last, goes first.
test("the top line of the sidebar drops the last part of its tail before its head is cut", async () =>
  composing(
    async ({ session, frame }) => {
      const root = session.engine.root;
      const asked = await session.engine.prompt("str", {
        message: "Which runner?",
        to: "operator",
        on: root,
      });
      await until(session.host, () => session.host.prompts.has(asked));
      await session.engine.pause(root);
      await session.engine.prompt("str", { message: "A held thread", on: root });
      await framed(
        async () => {
          await session.refresh();
          return frame();
        },
        (shown) => shown.includes("1 active · 1 paused"),
        "the count of the held thread",
      );
      const width = 132;
      const top = async (sidebar: number): Promise<string> => {
        session.preferences.sidebarWidth = sidebar;
        // The first frame builds the line, and the next one lays it out.
        await frame();
        return ((await frame()).split("\n")[0] ?? "").slice(width - sidebar);
      };
      const wide = await top(56);
      expect(wide).toContain("1 needs you");
      expect(wide).toContain("⌥A");
      expect(wide).toContain("1 active · 1 paused");
      for (const sidebar of [44, 32]) {
        const line = await top(sidebar);
        expect(line, `the top line at ${sidebar} cells`).toContain("1 needs you");
        expect(line, `the top line at ${sidebar} cells`).toContain("⌥A");
        expect(line, `the top line at ${sidebar} cells`).not.toContain("active");
      }
    },
    { width: 132, height: 44 },
    await demoSession({ preferences: ownPreferences() }),
  ));

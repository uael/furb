// The header of a thread that a word opened names its asker as the model with its effort, as the block of a reply does,
// and no rungN: the pinned header, the box of the thread and the frame that draws it read one label.
import { expect, test } from "bun:test";
import { until } from "../../bind/typescript/test/until.ts";
import { composing } from "./composing.ts";

test("the header of a thread that a word opened names the model with its effort, and no rung", () =>
  composing(
    async ({ app, session, frame }) => {
      const model = "claude-cli:sonnet/medium";
      await session.engine.rung({
        word: `prompt(str, "Name the next step of this word.", to="${model}")`,
        on: session.engine.root,
      });
      const asked = () =>
        session.acts.find(
          (act) => act.kind === "prompt" && String(act.words[1]).startsWith("Name the next step"),
        );
      await until(session, () => asked() !== undefined);
      const act = asked();
      if (!act) throw new Error("No prompt of the word.");
      expect(app.threadHead(act, false).sent).toBe("sonnet/medium");
      expect(app.sender(act)).not.toMatch(/rung\d+|asked/);
      const shown = await frame();
      expect(shown).not.toMatch(/rung\d+ asked/);
      // The headers of the word blocks of the feed read the model with its effort, or the operator, with no rungN and no
      // word beside it.
      const feed = shown
        .split("\n")
        .map((row) => row.slice(0, 72))
        .join("\n");
      expect(feed).toContain("sonnet/high");
      expect(feed).toContain("operator");
      expect(feed).not.toMatch(/\brung\d+\b|\bword\b/i);
    },
    undefined,
    true,
  ));

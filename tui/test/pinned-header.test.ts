// The header of a thread names who asked it as a model with its effort, as the block of a word does, and a block of a
// word names the model that wrote it, or the operator: no rungN stands in either.
import { expect, test } from "bun:test";
import { operated } from "furb/life";
import { until } from "../../bind/typescript/test/until.ts";
import { composing } from "./composing.ts";

test("the header of a thread that a word opened names the model with its effort, and no rung", () =>
  composing(
    async ({ app, session, frame }) => {
      await session.engine.rung({
        word: 'prompt(str, "Name the next step of this word.", to="claude-cli:sonnet/medium")',
        on: session.engine.root,
      });
      const asked = () =>
        session.acts.find(
          (act) => act.kind === "prompt" && String(act.words[1]).startsWith("Name the next step"),
        );
      await until(session, () => asked() !== undefined);
      const act = asked();
      if (!act) throw new Error("No prompt of the word.");
      // The operator asked, so the header names the asked model alone.
      expect(app.threadHead(act, false).sent).toBe("sonnet/medium");
      // A word of a model that asks another model still names the asked model alone, as one label.
      const word = session.acts.find((one) => one.kind === "rung" && !operated(one.by) && one.words[2]);
      if (!word) throw new Error("No word of a model in the demo.");
      expect(app.writer(word)).toBe("sonnet/high");
      expect(app.sender({ ...act, by: word.id })).toBe("sonnet/medium");
      expect(
        app.sender({ ...act, by: word.id, words: [act.words[0], act.words[1], "claude-cli:sonnet/high"] }),
      ).toBe("sonnet/high");
      // The block of a word of the model and of the operator: the row that heads each reads its writer, with no rung.
      const rows = (await frame()).split("\n").map((row) => row.slice(0, 72).trim());
      expect(rows.some((row) => /^sonnet\/high\s+\d+ms$/.test(row))).toBe(true);
      expect(rows).toContain("┃       operator");
      expect(rows.filter((row) => /\brung\d+\b/.test(row))).toEqual([]);
    },
    undefined,
    true,
  ));

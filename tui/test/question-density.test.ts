import { expect, test } from "bun:test";
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { until } from "../../bind/typescript/test/until.ts";
import { rasterize } from "../script/raster.ts";
import { paletteOf } from "../src/theme.ts";
import { composing } from "./composing.ts";
import { idle } from "./idle.ts";

for (const width of [80, 160]) {
  test(`many questions keep the program and thread navigation usable at ${width} columns`, () =>
    composing(
      async ({ session, app, screen, frame, click }) => {
        await idle(session);
        const capture = async (name: string) => {
          const out = process.env.FURB_QUESTION_DENSITY_OUT;
          if (!out) return;
          await frame();
          await app.colorsDone();
          const text = await frame();
          await mkdir(out, { recursive: true });
          const path = join(out, `${name}-${screen.renderer.width}x${screen.renderer.height}`);
          await writeFile(`${path}.txt`, text);
          await writeFile(`${path}.png`, rasterize(screen.captureSpans(), paletteOf(session.theme), "furb"));
        };
        const chain = await session.engine.chain({ label: "Review queue" });
        const questions: string[] = [];
        for (let index = 1; index <= 130; index++)
          questions.push(
            await session.engine.prompt("str", {
              message: `Density question ${String(index).padStart(3, "0")}: name the next check.`,
              on: chain,
              to: "operator",
            }),
          );
        const last = questions.at(-1);
        if (!last) throw new Error("The scripted questions did not open.");
        await until(session.host, () => session.host.prompts.has(last));
        await session.refresh();
        app.composer.setText("Keep this unsent draft");
        const shown = await frame();
        expect(shown).toContain("130 questions");
        expect(shown).not.toMatch(/[◆♦](?:\s+[◆♦]){8}/);
        await capture("queue");
        if (width >= 100) {
          expect(shown).toContain("Threads");
          expect(shown).toContain("Find threads");
          expect(shown).toContain("⌃B switch");
          screen.resize(160, 48);
          const resized = await frame();
          expect(resized).toContain("Threads");
          expect(resized).toContain("View all 130 questions");
          await capture("queue");
          await click("View all");
        } else {
          screen.mockInput.pressKey("a", { ctrl: true });
        }
        expect(await frame()).toContain("Questions");
        expect(session.selected).toBe(session.engine.root);
        expect(app.composer.plainText).toBe("Keep this unsent draft");
        await screen.mockInput.typeText("Density question 130");
        expect(await frame()).toContain("Density question 130");
        await capture("chooser");
        screen.mockInput.pressEnter();
        await until(session, () => session.operatorPrompt?.id === last);
        expect(session.selected).toBe(chain);
        expect(session.operatorPrompt?.id).toBe(last);
        expect(session.actOf(last)?.done).toBe(false);
        expect(await frame()).toContain("Question 130 of 130");
        await capture("answer");
        screen.mockInput.pressEscape();
        await session.select(session.engine.root);
        app.render();
        expect(app.composer.plainText).toBe("Keep this unsent draft");
        await capture("restored");
      },
      { width, height: 24, kittyKeyboard: true },
      true,
    ));
}

test("a notice settles without adding an answer target to the question queue", () =>
  composing(
    async ({ session, app, frame, click }) => {
      await idle(session);
      const chain = await session.engine.chain({ label: "Four checks" });
      for (const message of ["First question", "Second question", "Third question", "Fourth question"])
        await session.engine.prompt("str", { message, to: "operator", on: chain });
      const notice = await session.engine.prompt("None", {
        message: "The build is ready to read.",
        to: "operator",
        on: chain,
      });
      await until(session, () => session.actOf(notice)?.done === true);
      await session.refresh();
      const shown = await frame();
      expect(shown).toContain("4 questions");
      expect(shown).toContain("View all 4 questions");
      expect(session.desk.asks).toHaveLength(4);
      expect(session.desk.asks.some((ask) => ask.id === notice)).toBe(false);
      await click("View all");
      expect(await frame()).toContain("Questions");
      expect(await frame()).toContain("Fourth question");
      expect(await frame()).not.toContain("The build is ready to read.");
      expect(app.composer.plainText).toBe("");
    },
    { width: 160, height: 48 },
  ));

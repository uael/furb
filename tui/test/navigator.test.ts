import { expect, test } from "bun:test";
import { until } from "../../bind/typescript/test/until.ts";
import { highlighting } from "../script/stage.ts";
import { composing } from "./composing.ts";
import { idle } from "./idle.ts";

for (const { width, height } of [
  { width: 80, height: 24 },
  { width: 160, height: 48 },
]) {
  test(`scripted navigator at ${width}x${height}`, () =>
    composing(
      async ({ session, app, screen, frame }) => {
        await idle(session);
        const capture = async () => {
          await frame();
          await Promise.all(highlighting(screen.renderer.root));
          await app.colorsDone();
          const text = await frame();
          return text;
        };
        if (width < 100) {
          app.scroll.stickyScroll = false;
          app.scroll.scrollTo(0);
          const codeFrame = await capture();
          expect(codeFrame).toContain("notes = read");
        }
        let parent = session.engine.root;
        for (let index = 1; index <= 22; index++) {
          const made = await session.engine.rung({
            word: `side = chain("Nested ${String(index).padStart(2, "0")}")`,
            on: parent,
          });
          await session.engine.result(made);
          const child = await session.engine.inspect("side", parent);
          if (typeof child.value !== "string") throw new Error("The scripted chain did not open.");
          parent = child.value;
        }
        const targetBeforeInspect = session.thread;
        app.details();
        await frame();
        await screen.mockInput.typeText("Find threads");
        screen.mockInput.pressEnter();
        const threadPicker = await capture();
        expect(threadPicker).toContain("Threads");
        expect(threadPicker.split("\n").find((line) => line.includes("prompt1"))).toMatch(/prompt1\s+done/);
        screen.mockInput.pressArrow("right");
        expect(await capture()).toContain("Reopen and reply");
        expect(session.thread).toBe(targetBeforeInspect);
        app.closeOverlay();
        if (width === 80) {
          app.switchView("loom");
          screen.mockInput.pressKey("g", { meta: true });
          const footer = await capture();
          expect(footer.trimEnd().split("\n").at(-1)).toContain("F1 help");
          app.switchView("main");
        }
        const questions = [];
        for (let index = 1; index <= 3; index++) {
          questions.push(
            await session.engine.prompt("bool", {
              message: `Review scripted navigation question ${index} with a deliberately long subject`,
              on: index === 1 ? session.engine.root : parent,
              to: "operator",
            }),
          );
        }
        const firstQuestion = questions[0];
        if (!firstQuestion) throw new Error("No scripted question.");
        await until(session.host, () => session.host.prompts.has(firstQuestion));
        await session.refresh();
        await session.point(firstQuestion);
        screen.mockMouse.moveTo(0, height - 1);
        app.render();
        expect(session.operatorPrompt?.id).toBe(firstQuestion);
        expect(session.operatorPrompt?.choices).toEqual(["yes", "no"]);
        const text = await capture();
        expect(text).toContain("Review scripted navigation question 1");
        expect(text).toContain("yes");
        expect(text).toContain("Answer");
        if (width >= 100) {
          expect(text).toContain("Needs you");
          expect(text).toContain("Threads");
          expect(text).toContain("⌃B switch");
          expect(text).toContain("Find threads");
          expect(text).toContain("Nested 22");
        }
        app.chains();
        await frame();
        await screen.mockInput.typeText(parent);
        const picker = await capture();
        expect(picker).toContain("Nested 22");
        screen.mockInput.pressArrow("right");
        const path = await capture();
        expect(path).toContain("Chain path");
        expect(path).toContain("Nested 01");
        app.closeOverlay();
        app.chains();
        await frame();
        await screen.mockInput.typeText(parent);
        screen.mockInput.pressEnter();
        await until(session, () => session.selected === parent);
        expect(session.label).toBe("Nested 22");
        await session.select(session.engine.root);
        await session.point(firstQuestion);
        app.details();
        await frame();
        await screen.mockInput.typeText("Context details");
        screen.mockInput.pressEnter();
        expect(await capture()).toContain("3,240 tokens");
        app.closeOverlay();
        const free = await session.engine.prompt("str", {
          message: "Describe the scripted change in your own words",
          on: session.engine.root,
          to: "operator",
        });
        await until(session.host, () => session.host.prompts.has(free));
        await session.refresh();
        await session.point(free);
        app.composer.setText("Keep this answer draft");
        expect(session.operatorPrompt?.id).toBe(free);
        expect(session.operatorPrompt?.shape).toBe("str");
        expect(await capture()).toContain("returns str");
        expect(app.composer.plainText).toBe("Keep this answer draft");
        await session.point(firstQuestion);
        await frame();
        screen.mockInput.pressKey("3");
        await frame();
        app.composer.setText("A different proposal");
        expect(session.operatorPrompt?.id).toBe(firstQuestion);
        expect(session.actOf(firstQuestion)?.done).toBe(false);
        expect(await capture()).toContain("A different proposal");
        expect(app.composer.focused).toBe(true);
      },
      { width, height, exitSignals: [] },
      true,
    ));
}

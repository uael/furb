import { expect, test } from "bun:test";
import type { Session } from "../src/session.ts";
import { looming, soon } from "./looming.ts";

/** The life of many threads once each worker stands where the demo leaves it: two questions wait for the operator,
 * the guide builds, and a grant holds the worker of the animation. */
async function settled(session: Session): Promise<void> {
  const done = (cue: string) =>
    session.acts.some((act) => act.kind === "prompt" && String(act.words[1]).startsWith(cue) && act.done);
  for (const start = Date.now(); Date.now() - start < 25000; await Bun.sleep(50)) {
    await session.refresh();
    if (done("Write the changelog") && done("Close the gap of sessions") && session.host.prompts.size === 2)
      return;
  }
  throw new Error("The life of many threads did not settle.");
}

test("v changes the variant, and the graph shows the tree of the threads beside the map of the programs", () =>
  looming(
    async ({ app, screen, session, frame }) => {
      await settled(session);
      await screen.mockInput.typeText("v");
      await soon(() => app.variant === "graph");
      const shown = await frame();
      expect(shown).toContain("Threads");
      expect(shown).toMatch(/Ship the release 0\.4/);
      expect(shown).toMatch(/◆ you/);
      expect(shown).toMatch(/w search/);
      expect(shown).toMatch(/◆2/);
      await screen.mockInput.typeText("v");
      await soon(() => app.variant === "loom");
    },
    { seed: "threads" },
  ));

test("Enter zooms from a thread to its map, then to the source of a program, a word and an act, and Esc zooms out each time", () =>
  looming(
    async ({ app, screen, session, frame }) => {
      await settled(session);
      await app.graph.move([0, 1]);
      screen.mockInput.pressEnter();
      await soon(() => app.graph.level === "map");
      await soon(() => app.graph.node === session.engine.root);
      screen.mockInput.pressArrow("right");
      await soon(() => app.graph.node !== session.engine.root);
      screen.mockInput.pressEnter();
      await soon(() => app.graph.level === "program");
      expect(await frame()).toMatch(/w search/);
      expect(await frame()).toMatch(/read\d+ ✓/);
      app.graph.line = 2;
      screen.mockInput.pressEnter();
      await soon(() => app.graph.level === "word");
      screen.mockInput.pressEnter();
      await soon(() => app.graph.level === "act" && app.focus === "lens");
      for (const level of ["word", "program", "map", "threads"]) {
        screen.mockInput.pressEscape();
        await soon(() => app.graph.level === level);
      }
    },
    { seed: "threads", variant: "graph" },
  ));

test("n lets the selected chain run one more rung, then a pause holds it again", () =>
  looming(
    async ({ app, screen, session }) => {
      await soon(() =>
        app.loom.threads.some((thread) => thread.label === "w gif" && thread.state === "paused"),
      );
      const gif = app.loom.threads.find((thread) => thread.label === "w gif")?.id ?? "";
      await app.select(gif);
      screen.mockInput.pressKey("n");
      await soon(() => app.stepping.has(gif));
      await soon(() => !app.stepping.has(gif));
      await soon(() => session.notice.includes("paused after one step"));
    },
    { seed: "conductor", variant: "graph" },
  ));

test("with a hundred chains a frame of the map and a key of the graph each take less than a tenth of a second", () =>
  looming(
    async ({ app, session }) => {
      for (const start = Date.now(); Date.now() - start < 25000; await Bun.sleep(50)) {
        await session.refresh();
        if (session.acts.filter((act) => act.kind === "chain").length >= 101) break;
      }
      app.graph.level = "map";
      const frames: number[] = [];
      for (let n = 0; n < 10; n++) {
        const start = performance.now();
        app.render();
        frames.push(performance.now() - start);
      }
      const keys: number[] = [];
      for (let n = 0; n < 10; n++) {
        const start = performance.now();
        await app.graph.move([0, 1]);
        app.render();
        keys.push(performance.now() - start);
      }
      const median = (values: number[]) =>
        [...values].sort((one, other) => one - other)[values.length >> 1] ?? 0;
      console.log(
        `a hundred chains: a frame takes ${median(frames).toFixed(1)} ms, a key ${median(keys).toFixed(1)} ms`,
      );
      expect(median(frames)).toBeLessThan(100);
      expect(median(keys)).toBeLessThan(100);
    },
    { seed: "hundred", variant: "graph" },
  ));

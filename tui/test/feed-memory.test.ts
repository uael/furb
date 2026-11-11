// The feed holds what it shows of a long life: a card of a done act far from the view keeps its place and its height
// alone, the feed keeps the lines of the programs near the view alone, and a scroll back shows what it showed.
import { expect, test } from "bun:test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import type { Renderable } from "@opentui/core";
import { createTestRenderer } from "@opentui/core/testing";
import { App } from "../src/app.ts";
import { demoLibrary, demoSession } from "../src/demo.ts";
import { Preferences } from "../src/preferences.ts";
import { realLife } from "./real-life.ts";

/** How many renderables a node holds, itself among them. */
const count = (node: Renderable): number => 1 + node.getChildren().reduce((sum, one) => sum + count(one), 0);

test("a long feed shelves the cards far from its view and keeps a bounded count of programs, and a scroll back loses nothing", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-feed-memory-"));
  const record = join(directory, "long.jsonl");
  try {
    // A life of many words of the operator, each with a command, which a later window opens again.
    await realLife(
      record,
      Array.from({ length: 160 }, (_, k) => `await bash("echo step_${k}; seq 1 40")`),
    );
    const session = await demoSession({
      record,
      cwd: directory,
      preferences: new Preferences(join(directory, "ui.json")),
    });
    const library = await demoLibrary(session);
    const screen = await createTestRenderer({ width: 140, height: 44, exitSignals: [] });
    const app = new App(screen.renderer, session, { quit() {}, workspaces: library });
    try {
      // The operator opened the earlier steps of the chain, so the feed holds a card for each act.
      for (const key of ["rungs:", "prompts:", "rungs:chain1", "prompts:chain1", "steps:chain1"])
        session.folds[key] = false;
      app.render();
      const frame = async () => {
        await screen.renderOnce();
        return screen.captureCharFrame();
      };
      for (let k = 0, height = -1, still = 0; k < 600 && still < 20; k++) {
        still = app.scroll.scrollHeight === height ? still + 1 : 0;
        height = app.scroll.scrollHeight;
        await frame();
        await Bun.sleep(50);
      }
      expect(app.cards.size).toBeGreaterThan(100);
      const top = async () => {
        for (let k = 0; k < 3000 && app.scroll.scrollTop > 0; k++) {
          app.scroll.scrollBy(-40);
          await frame();
        }
        for (let k = 0; k < 5; k++) await frame();
        return await frame();
      };
      const shown = await top();
      // Each card shown holds about ten renderables, and a shelved one holds one.
      expect(count(app.scroll.content)).toBeLessThan(app.cards.size * 3);
      expect(app.programs.size).toBeLessThanOrEqual(64);
      for (
        let k = 0;
        k < 3000 && app.scroll.scrollTop < app.scroll.scrollHeight - app.scroll.viewport.height;
        k++
      ) {
        app.scroll.scrollBy(40);
        await frame();
      }
      expect(count(app.scroll.content)).toBeLessThan(app.cards.size * 3);
      expect(await top()).toBe(shown);
      expect(shown).toContain("step_0");
    } finally {
      app.dispose();
      screen.renderer.destroy();
      await library.dispose();
    }
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}, 600000);

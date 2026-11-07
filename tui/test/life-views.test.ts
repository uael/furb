import { expect, test } from "bun:test";
import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import type { Loom } from "furb/life";
import { openEngine } from "../src/bridge.ts";
import { LoomPane, questionOf, shortActor } from "../src/overview.ts";
import { Preferences } from "../src/preferences.ts";
import { Session } from "../src/session.ts";
import { plain } from "../src/ui.ts";
import { composing } from "./composing.ts";

/** The recorded life of a conductor, with a score of chains, which shows its log and runs nothing, on preferences of
 * its own, so that the view that a test switches to reaches no test that runs beside it. */
async function recorded(): Promise<Session> {
  const { engine, host } = await openEngine({
    record: join(import.meta.dir, "..", "fixtures", "conductor.jsonl"),
  });
  const folder = await mkdtemp(join(tmpdir(), "furb-views-"));
  return new Session(engine, host, true, new Preferences(join(folder, "ui.json")));
}

test("each row of the loom shows the state, the actor and the cost of its chain, and a row that asks shows its question", () => {
  const loom: Loom = {
    rows: [
      {
        id: "operator",
        label: "you",
        lines: "",
        state: "asks",
        workers: 0,
        folded: false,
        tally: {},
        cells: [],
      },
      {
        id: "chain1",
        label: "Main",
        lines: "",
        state: "asks",
        workers: 0,
        folded: false,
        tally: {},
        cells: [{ mark: "▂", tone: "asks" }],
        measure: { dollars: 1.5, spent: 1.5, answers: 3, share: 0.25, actor: "claude-cli:opus/high" },
      },
    ],
    owed: [
      { id: "prompt2", chain: "chain1", by: "rung1", shape: "str", message: "Which name?\nMore.", began: 4 },
    ],
    folded: 0,
  };
  const rows = new LoomPane()
    .draw({
      loom,
      width: 140,
      height: 10,
      selected: "chain1",
      frozen: false,
      steering: false,
      actor: () => "claude-cli:opus/high",
    })
    .map(plain);
  expect(rows[0]).toContain("waits on");
  expect(rows[0]).toContain("actor");
  expect(rows[1]).toContain("1 owed");
  expect(rows[2]).toContain("◆ Main");
  expect(rows[2]).toContain("◆ Which name?");
  expect(rows[2]).toContain("opus/high");
  expect(rows[2]).toContain("$1.50");
  expect(rows[2]).toContain("25%");
  expect(questionOf("chain1", loom.owed)).toBe("Which name?");
  expect(shortActor("claude-cli:opus/high")).toBe("opus/high");
});

test("⌥V switches the view of the life between main, the loom and the graph, which the setting keeps, ⌥G and /view act in them, and the screens of main stay in each", async () =>
  // One test holds every step on an App, since each test runs in a process of its own and two Apps at once save one
  // setting.
  composing(
    async ({ session, app, screen, frame }) => {
      await session.refresh();
      const soon = async (done: () => boolean) => {
        for (let wait = 0; wait < 150 && !done(); wait++) await Bun.sleep(20);
        expect(done()).toBe(true);
      };
      // The switch.
      expect(app.lifeView).toBe("main");
      expect(await frame()).not.toContain("waits on");
      screen.mockInput.pressKey("v", { meta: true });
      expect(app.lifeView).toBe("loom");
      expect(new Preferences(session.preferences.path).view).toBe("loom");
      const loom = await frame();
      expect(loom).toContain("older ┊");
      expect(loom).toContain("waits on");
      screen.mockInput.pressKey("v", { meta: true });
      expect(app.lifeView).toBe("graph");
      // The title of the map names the open thread of the conductor, whose ladder hired 10 of the 23 chains.
      expect(await frame()).toMatch(/You are the orchestrator of the work on furb.* {3}11 of 23/);
      screen.mockInput.pressKey("v", { meta: true });
      expect(app.lifeView).toBe("main");
      // The command.
      app.composer.setText("/view graph");
      await app.submit();
      expect(app.lifeView).toBe("graph");
      app.composer.setText("/view");
      await app.submit();
      expect(await frame()).toContain("The feed of the selected chain beside the threads");
      screen.mockInput.pressEscape();
      // The keys on the loom.
      app.switchView("loom");
      await frame();
      screen.mockInput.pressKey("g", { meta: true });
      expect(app.steering).toBe(true);
      const first = session.selected;
      screen.mockInput.pressArrow("down");
      await soon(() => session.selected !== first);
      screen.mockInput.pressKey("[");
      expect(app.cursor).toBeDefined();
      expect(await frame()).toContain("then");
      screen.mockInput.pressEscape();
      await soon(() => app.cursor === undefined);
      screen.mockInput.pressEscape();
      await soon(() => !app.steering);
      expect(app.composer.focused).toBe(true);
      // The keys on the map: an arrow selects a chain under the feed, and Enter gives the keys to the input on it.
      app.switchView("graph");
      await frame();
      app.steer(true);
      const before = session.selected;
      screen.mockInput.pressArrow("down");
      await soon(() => session.selected !== before);
      const chosen = session.selected;
      screen.mockInput.pressEnter();
      await soon(() => !app.steering);
      expect(app.composer.focused).toBe(true);
      expect(session.selected).toBe(chosen);
      await frame();
      expect(app.scroll.visible).toBe(true);
      expect(app.renderer.root.findDescendantById("life-stage")?.visible).toBe(true);
      // The screens of main.
      const stage = () => app.renderer.root.findDescendantById("life-stage");
      for (const view of ["loom", "graph"] as const) {
        app.switchView(view);
        session.show("transcript");
        await session.refresh();
        await frame();
        expect(app.scroll.visible).toBe(true);
        expect(stage()?.visible).toBe(false);
        session.show("feed");
        await frame();
        expect(stage()?.visible).toBe(true);
        screen.mockInput.pressKey("p", { ctrl: true });
        expect(await frame()).toContain("Commands");
        await screen.mockInput.typeText("view of");
        expect(await frame()).toContain("View of the life");
        screen.mockInput.pressEscape();
        await frame();
      }
      // A frame of the map of a score of chains takes less than a tenth of a second.
      app.render();
      const frames = Array.from({ length: 5 }, () => {
        const start = performance.now();
        app.graph.rows(116, 24);
        return performance.now() - start;
      }).sort((one, other) => one - other);
      expect(frames[2]).toBeLessThan(100);
      app.switchView("main");
    },
    { width: 152, height: 46 },
    await recorded(),
  ));

test("the map lights the chain of the thread that the sidebar shows and the chains of its agents, and not the chain of the thread over it", async () =>
  composing(
    async ({ session, app }) => {
      await session.refresh();
      // The worker ui-research-tui of the coordinator ui-audit, whose ladder hired four rivals.
      await session.select("chain14");
      expect(session.activeThread?.act.id).toBe("prompt101");
      expect([...(app.stageOf().thread?.chains ?? [])].sort()).toEqual([
        "chain14",
        "chain20",
        "chain21",
        "chain22",
        "chain23",
      ]);
      // In the past the map lights only the agents that had begun: the rival on chain23 begins at the moment 4554.
      app.cursor = 4553;
      const before = app.stageOf().thread?.chains;
      expect([before?.has("chain14"), before?.has("chain23")]).toEqual([true, false]);
      app.cursor = 4554;
      expect(app.stageOf().thread?.chains.has("chain23")).toBe(true);
      app.cursor = undefined;
    },
    { width: 152, height: 46 },
    await recorded(),
  ));

test("a click on a program of the map selects its chain, as a click on a row of the loom does", async () =>
  composing(
    async ({ session, app, screen, frame }) => {
      await session.refresh();
      app.switchView("graph");
      const shown = (await frame()).split("\n");
      const stage = app.renderer.root.findDescendantById("life-stage");
      expect(stage?.visible).toBe(true);
      const top = stage?.y ?? 0;
      const other = session.chains.find(
        (chain) =>
          chain.id !== session.selected &&
          shown
            .slice(top + 2, top + (stage?.height ?? 0))
            .some((line) => line.includes(session.labelOf(chain.id))),
      );
      expect(other).toBeDefined();
      const label = session.labelOf(other?.id ?? "");
      const y = shown.findIndex((line, at) => at >= top + 2 && line.includes(label));
      await screen.mockMouse.click((shown[y] ?? "").indexOf(label) + 1, y);
      for (let tries = 0; tries < 100 && session.selected !== other?.id; tries++) await Bun.sleep(20);
      expect(session.selected).toBe(other?.id ?? "");
      app.switchView("main");
    },
    { width: 152, height: 46 },
    await recorded(),
  ));

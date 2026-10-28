import { expect, test } from "bun:test";
import type { Loom, Row } from "furb/life";
import { until } from "../../bind/typescript/test/until.ts";
import { drawChains, quietFolds } from "../src/chains.ts";
import { plain } from "../src/ui.ts";
import { composing, pressEscape } from "./composing.ts";
import { idle } from "./idle.ts";

const row = (
  id: string,
  label: string,
  state: Row["state"],
  lines = "",
  tone: "work" | "past" = "work",
): Row => ({
  id,
  label,
  lines,
  state,
  workers: 0,
  folded: false,
  tally: {},
  cells: [{ mark: "▂", tone }],
});

/** A chain that asks, a chain that works under it, and two finished chains under it. */
const loom: Loom = {
  rows: [
    row("operator", "you", "asks"),
    row("chain1", "Main", "asks"),
    row("chain2", "w/search", "working", "├─ "),
    row("chain3", "w/types", "done", "├─ ", "past"),
    row("chain4", "w/release", "done", "╰─ ", "past"),
  ],
  owed: [{ id: "prompt9", chain: "chain1", by: "rung1", shape: "str", message: "Which name?", began: 4 }],
  folded: 0,
};

const input = {
  loom,
  width: 120,
  height: 20,
  pointer: "chain1",
  folds: new Set<string>(),
  rested: new Set<string>(),
  top: 0,
  actor: () => "claude-cli:opus/high",
};

test("the panel of the chains draws a thread for each chain, joins the finished chains in one row, and says which chain asks", () => {
  const drawn = drawChains(input);
  const text = drawn.lines.map(plain).join("\n");
  expect(plain(drawn.header)).toBe("Chains  2 shown   ◆ 1 chain asks you");
  expect(text).toContain("◆ ▾ Main");
  expect(text).toContain("w/search");
  expect(text).toContain("2 finished");
  expect(text).not.toContain("w/types");
  expect(text).toContain("↑↓ move");
  expect(drawn.entries.map((entry) => entry.id)).toEqual(["chain1", "chain2", "rest:chain1"]);
});

test("the pointer on the finished chains of a chain names them, and showing them lists each one", () => {
  const joined = drawChains({ ...input, pointer: "rest:chain1" });
  expect(joined.lines.map(plain).join("\n")).toContain("2 finished chains of Main");
  const shown = drawChains({ ...input, rested: new Set(["chain1"]) });
  expect(shown.entries.map((entry) => entry.id)).toEqual([
    "chain1",
    "chain2",
    "chain3",
    "chain4",
    "rest:chain1",
  ]);
  expect(shown.lines.map(plain).join("\n")).toContain("w/types");
});

test("a fresh panel folds a branch that is all finished, unless the pointer is on its way", () => {
  const nested: Loom = {
    ...loom,
    rows: [row("operator", "you", "asks"), row("a", "A", "done"), row("b", "B", "done", "├─ ")],
  };
  expect([...quietFolds(nested, "other")]).toEqual(["a"]);
  expect([...quietFolds(nested, "b")]).toEqual([]);
});

const size = { width: 152, height: 46 };

test("ctrl+b opens the panel of the chains over the feed, Enter opens the chain under the pointer and closes the panel, and Esc closes it", () =>
  composing(
    async ({ session, screen, frame }) => {
      await idle(session);
      screen.mockInput.pressKey("b", { ctrl: true });
      const shown = await frame();
      expect(shown).toContain("shown");
      expect(shown).toContain("↑↓ move");
      screen.mockInput.pressArrow("down");
      screen.mockInput.pressEnter();
      await until(session, () => session.selected !== session.engine.root);
      // The panel closes over a few frames on a slow machine.
      let closed = await frame();
      for (let step = 0; step < 160 && closed.includes("↑↓ move"); step++) {
        await Bun.sleep(25);
        closed = await frame();
      }
      expect(closed).not.toContain("↑↓ move");
      screen.mockInput.pressKey("b", { ctrl: true });
      expect(await frame()).toContain("↑↓ move");
      await pressEscape(screen);
      expect(await frame()).not.toContain("↑↓ move");
    },
    size,
    true,
  ));

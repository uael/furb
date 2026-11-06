import { expect, test } from "bun:test";
import { frame, markOf, marks, motion, palettes, setTheme, theme } from "../src/theme.ts";

test("one table gives each state its one mark, its color and its weight, and the spinner stands still as ✻", () => {
  setTheme("ink");
  motion.still = true;
  const shown = Object.fromEntries(
    (Object.keys(marks) as (keyof typeof marks)[]).map((state) => {
      const [mark, color, attributes] = markOf(state);
      return [state, [mark, color.toString(), attributes]];
    }),
  );
  const role = (name: keyof typeof theme) => theme[name].toString();
  expect(shown).toEqual({
    asks: ["◆", role("warning"), 1],
    unread: ["●", role("accent"), 1],
    failed: ["✗", role("danger"), 0],
    paused: ["◌", role("warning"), 0],
    thinks: ["✻", role("accent"), 0],
    runs: ["◉", role("accent"), 2],
    delegates: ["◎", role("faint"), 0],
    done: ["✓", role("success"), 0],
    seen: ["✓", role("faint"), 0],
    cancelled: ["⊘", role("faint"), 0],
    idle: ["○", role("faint"), 0],
  });
  // ● keeps one meaning: no other state takes it.
  expect(Object.values(shown).filter(([mark]) => mark === "●")).toHaveLength(1);
  expect(palettes.ink).toBeDefined();
  motion.still = false;
});

test("only the states that move move: a model that thinks spins, work that runs turns, and each other state stands still", () => {
  motion.still = false;
  const at = (state: keyof typeof marks) =>
    Array.from({ length: 12 }, (_, step) => markOf(state, step * frame)[0]);
  expect(new Set(at("thinks")).size).toBe(10);
  expect([...new Set(at("runs"))]).toEqual(["◐", "◓", "◑", "◒"]);
  // The two motions are apart, so a turn never reads as a thought.
  expect(at("runs").some((one) => at("thinks").includes(one))).toBe(false);
  for (const state of [
    "asks",
    "unread",
    "failed",
    "paused",
    "delegates",
    "done",
    "seen",
    "cancelled",
    "idle",
  ] as const)
    expect(new Set(at(state)).size).toBe(1);
});

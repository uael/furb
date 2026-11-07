// The marks of work that runs and of a model that thinks move on the real view: the mark in the sidebar, the cell of the
// number of the line that works, and the footer each change between two moments half a second apart, each state turns
// its own frames, and a session shows the mark of its thread.
import { expect } from "bun:test";
import { scenario, test, type Window } from "./harness.ts";

const runs = /[◐◓◑◒]/;
const thinks = /[⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏]/;

/** What a frame shows of the work: the mark of each row of the sidebar that names one of the texts, the cell of the
 * number of the row of the feed that holds the line, and the footer. */
async function read(window: Window, line: string, rows: string[]) {
  const frame = await window.frame();
  const lines = frame.split("\n");
  const left = window.width - window.session.preferences.sidebarWidth;
  const rail = rows.map((text) => {
    const row = lines.find((one) => one.slice(left).includes(text)) ?? "";
    return row.slice(left).match(/▎ (\S) /)?.[1] ?? "";
  });
  const cell = (lines.find((one) => one.slice(0, left).includes(line)) ?? "").slice(0, 7);
  return { rail, cell, footer: lines.at(-2)?.trim() ?? "", frame };
}

test("the marks of a command that runs and of a thought turn in the sidebar, the cell of the number and the footer", () =>
  scenario(
    [
      { cue: "Scenario: run long.", word: 'got = await bash("sleep 30", timeout=60)\nclose("Ran.")' },
      {
        cue: "Scenario: think.",
        thought: "I read the notes.\nThe last notes name the tests.",
        word: 'close("Thought.")',
        hold: "go",
      },
    ],
    async (window, yard) => {
      await window.type("Scenario: run long.");
      await window.shows((frame) => frame.includes('bash("sleep 30"'), "the command that runs");
      const ran = [];
      for (let n = 0; n < 7; n++) {
        await Bun.sleep(500);
        ran.push(await read(window, 'bash("sleep 30"', ["life", "Scenario: run long."]));
      }
      for (const [at, one] of ran.entries()) {
        // The session and its thread show the one mark of work that runs.
        expect(one.rail[0]).toMatch(runs);
        expect(one.rail[1]).toBe(one.rail[0]);
        expect(one.footer).toMatch(at ? /^[◐◓◑◒] Working · [\d.]+s / : /^[◐◓◑◒] Working/);
        const next = ran[at + 1];
        if (next) {
          expect(next.rail).not.toEqual(one.rail);
          expect(next.footer).not.toBe(one.footer);
        }
      }
      // The cell of the number of the line that runs turns the loader of work that runs and comes back to its number.
      const cells = ran.map((one) => one.cell.trim());
      expect(cells.some((cell) => cell === "1")).toBe(true);
      expect(cells.some((cell) => runs.test(cell))).toBe(true);

      await window.type("/newthread");
      await window.type("Scenario: think.");
      await window.shows((frame) => frame.includes("The last notes"), "the thought");
      const first = await read(window, "The last notes", ["Scenario: think."]);
      await Bun.sleep(500);
      const second = await read(window, "The last notes", ["Scenario: think."]);
      for (const one of [first, second]) {
        // The cell of the number of the line that a model writes turns the loader of a thought, which no still mark of
        // a thought stands for, and the line says no word that the loader already says.
        expect(one.cell).toMatch(thinks);
        expect(one.rail[0]).toMatch(thinks);
        expect(one.footer).toMatch(/^[⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏] Thinking/);
        expect(one.frame).not.toContain("✻");
        expect(one.frame).not.toContain("· thinking");
      }
      expect(second.cell).not.toBe(first.cell);
      expect(second.rail).not.toEqual(first.rail);
      expect(second.footer).not.toBe(first.footer);
      await yard.release("go");
      await window.shows((frame) => frame.includes("Thought."), "the answer after the thought");
    },
  ));

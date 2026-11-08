// The state and the age of a thread in the sidebar never join: a cancelled thread reads "cancelled 5m" where the row
// has the room, and "cancelled" alone where the title would else have too little.
import { expect } from "bun:test";
import { scenario, test, until, type Window } from "./harness.ts";

/** The row of the sidebar that holds a text, in the frame as it is drawn. */
const row = (window: Window, frame: string, text: string) =>
  frame
    .split("\n")
    .map((line) => line.slice(window.width - window.session.preferences.sidebarWidth))
    .find((line) => line.includes(text)) ?? "";

test("a cancelled thread reads its state and its age parted, and its state alone in a narrow sidebar", () =>
  scenario([], async (window) => {
    const { engine, session } = window;
    const still = process.env.FURB_STILL_CLOCK;
    await engine.pause(window.root);
    const id = await engine.prompt("str", {
      message: "Cancel this thread, whose title is long",
      on: window.root,
    });
    await until(
      () => session.threadsOf(session.selected).list.some((one) => one.act.id === id),
      "the thread",
    );
    await window.type(`/cancel ${id}`);
    await until(async () => (await engine.peek(id)) !== null, "the cancel");
    await window.shows((frame) => row(window, frame, "cancelled") !== "", "cancelled");
    // Five minutes pass: the clock of the view stands five minutes on, and the sidebar draws the age again at the minute.
    process.env.FURB_STILL_CLOCK = String(Date.now() + 5 * 60000);
    try {
      for (const width of [56, 44, 32]) {
        session.preferences.sidebarWidth = width;
        // The first frame builds the row, and the next one lays it out.
        await window.frame();
        const line = row(window, await window.frame(), "cancelled");
        expect(line, `the row at ${width} cells`).not.toMatch(/cancelled\d/);
        if (width === 56) expect(line).toMatch(/cancelled 5m/);
        if (width === 32) expect(line).toMatch(/cancelled\s*$/);
      }
    } finally {
      if (still === undefined) delete process.env.FURB_STILL_CLOCK;
      else process.env.FURB_STILL_CLOCK = still;
    }
  }));

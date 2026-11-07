// A stall that the watch of the daemon tells is a notice of furb, never a thread of the operator: it stands as an aside
// in the feed, no row of the sidebar names it, and the input keeps its target on the thread of the operator.
import { expect } from "bun:test";
import { writeFile } from "node:fs/promises";
import { join } from "node:path";
import { test, Window, Yard } from "./harness.ts";

test("the stall notice of the watch stands as an aside, in no row of the sidebar, and the input keeps its target", async () => {
  const yard = await Yard.make([{ cue: "Stall: hold.", word: 'close("Held, then done.")', hold: "go" }]);
  let window: Window | undefined;
  try {
    // The watch tells a stall after 2 seconds with no fact, in place of a minute.
    await writeFile(join(yard.root, "config", "host.json"), JSON.stringify({ stall: 2 }));
    window = await Window.open(yard);
    const { session } = window;
    const root = window.root;
    await window.type("Stall: hold.");
    const frame = await window.shows((shown) => shown.includes("had no change"), "the stall notice", 60);
    const mine = session.threadsOf(root).list.map((one) => String(one.act.words[1]));
    expect(mine).toEqual(["Stall: hold."]);
    const left = window.width - session.preferences.sidebarWidth;
    const sidebar = frame.split("\n").map((line) => line.slice(left));
    expect(sidebar.some((line) => line.includes("had no change"))).toBe(false);
    expect(sidebar.some((line) => line.includes("Stall: hold."))).toBe(true);
    const target = session.target;
    expect(target && String(session.threadsOf(root).by.get(target)?.act.words[1])).toBe("Stall: hold.");
    expect(frame).toContain("to ↳ Stall: hold.");
    await yard.release("go");
  } finally {
    await window?.close(true).catch(() => {});
    await yard.remove();
  }
});

// The sidebar titles the session by its name and each thread by its own first message: a thread never takes the name
// of its session.
import { expect } from "bun:test";
import { scenario, test } from "./harness.ts";

test("the sidebar shows the name of the session on its row and the message of its first thread on the row of the thread", () =>
  scenario([{ cue: "Explore this project", word: 'close("Explored.")' }], async (window) => {
    expect(await window.session.rename("Release 0.4")).toBe(true);
    await window.type("Explore this project, run its checks.");
    const frame = await window.shows((shown) => shown.includes("Explored."), "the answer");
    const left = window.width - window.session.preferences.sidebarWidth;
    const sidebar = frame.split("\n").map((line) => line.slice(left));
    expect(sidebar.filter((line) => line.includes("Release 0.4"))).toHaveLength(1);
    const thread = sidebar.find((line) => line.includes("Explore this project"));
    expect(thread).toBeDefined();
    expect(thread).not.toContain("Release 0.4");
  }));

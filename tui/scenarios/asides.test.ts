// What the owner sees of a notice of the round: it stands in the feed as an aside of its chain, and never as a thread of
// the sidebar beside his own threads.
import { expect } from "bun:test";
import { scenario, test, until } from "./harness.ts";

test("a notice that a word says stands as an aside in the feed, and no thread of the sidebar holds it", () =>
  scenario([], async (window) => {
    const { engine, session } = window;
    const root = window.root;
    await engine.result(await engine.rung({ word: 'prompt(None, "A note of the round")', on: root }));
    await engine.pause(root);
    const mine = await engine.prompt("str", { message: "A thread of mine", on: root });
    const threads = () => session.threadsOf(root);
    // The prompt of mine comes to the window after the notice, so the list of threads is read once both stand.
    await until(
      () =>
        [...threads().asides.values()].some((one) =>
          String(one.act.words[1]).includes("A note of the round"),
        ) && threads().by.has(mine),
      "the notice as an aside and the thread of mine",
    );
    expect(threads().list.map((one) => one.act.id)).toContain(mine);
    expect(threads().list.some((one) => String(one.act.words[1]).includes("A note of the round"))).toBe(
      false,
    );
    const frame = await window.shows(
      (shown) => shown.includes("A note of the round"),
      "the notice in the feed",
    );
    const left = window.width - session.preferences.sidebarWidth;
    const sidebar = frame.split("\n").map((line) => line.slice(left));
    expect(sidebar.some((line) => line.includes("A thread of mine"))).toBe(true);
    expect(sidebar.some((line) => line.includes("A note of the round"))).toBe(false);
  }));

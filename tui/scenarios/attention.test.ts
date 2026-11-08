// What marks the row of the session in the sidebar: what concerns the operator as the crate counts it, and nothing
// that a chain of the round does by itself.
import { expect } from "bun:test";
import { scenario, test, until, type Window } from "./harness.ts";

/** The marks of the row of the session of the window, and of the total of the sidebar, as the sidebar counts them. */
const marks = (window: Window) => {
  const entries = window.library.groups.flatMap((group) => group.sessions).filter((one) => one.session);
  return window.app.attentionOf(entries);
};

const NONE = { asks: 0, paused: 0, failed: 0, unread: 0 };

test("a sub-chain that fails marks no row of the session", () =>
  scenario([], async (window) => {
    const { engine } = window;
    const sub = await engine.chain({ label: "w/sub", on: engine.root });
    const word = await engine.rung({ word: 'raise Refused("no")', on: sub });
    await engine.result(word).catch(() => {});
    await until(
      () => window.session.acts.some((act) => act.chain === sub && act.state === "failed"),
      "the failure",
    );
    await window.session.readThreads(engine.root);
    expect(marks(window)).toEqual(NONE);
  }));

test("a word that wakes another chain marks no row of the session", () =>
  scenario([], async (window) => {
    const { engine } = window;
    const sub = await engine.chain({ label: "w/sub", on: engine.root });
    await engine.pause(sub);
    await engine.wake(sub);
    await window.session.readThreads(engine.root);
    expect(marks(window)).toEqual(NONE);
  }));

test("a thread that the operator started and that fails marks the row as failed", () =>
  scenario([], async (window) => {
    const { engine } = window;
    const mine = await engine.prompt("str", { message: "A thread of mine", on: engine.root });
    await engine.result(await engine.rung({ word: `close(Refused("no"), "${mine}")`, on: engine.root }));
    await until(() => (window.session.attention?.failed ?? 0) === 1, "the failed thread in the crate");
    expect(marks(window)).toEqual({ ...NONE, failed: 1 });
  }));

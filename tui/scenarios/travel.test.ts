// The keys that travel or clear change the window alone. Up and Down walk the history of the input and end no ask of
// the operator, and ⌃C on an empty input names the act that a second ⌃C cancels, so no press reaches the act before it.
import { expect } from "bun:test";
import { operated } from "furb/life";
import { scenario, test, until, type Window } from "./harness.ts";

/** The asks of the operator on the root, each id, message and whether it is over. */
async function asks(window: Window) {
  const facts = await window.engine.transcript({ on: window.root, of: "prompt" });
  return Promise.all(
    facts
      .filter((fact) => operated(String(fact[2])))
      .map(async (fact) => ({
        id: String(fact[1]),
        message: String(fact[5]),
        over: (await window.engine.peek(String(fact[1]))) !== null,
      })),
  );
}

/** The cancels that the record holds on the root. */
const cancels = (window: Window) => window.engine.transcript({ on: window.root, of: "cancel" });

/** The script: a first message that its model answers, then two messages on its thread that stay open, the first of
 * them with a worker under it whose model never answers, as the asks of the owner that carried his lanes. */
const script = [
  { cue: "Travel: begin.", word: 'close("Begun.")' },
  { cue: "Travel: hire.", word: 'prompt(str, "Travel: work.", on=chain("travel worker"))' },
  { cue: "Travel: ask.", word: "x = 1" },
];

/** The three messages sent, and the two last open on the thread of the first, each in the session of the window. */
async function sent(window: Window): Promise<{ hire: string; ask: string }> {
  await window.type("Travel: begin.");
  await window.shows((frame) => frame.includes("Begun."), "the answer of the first message");
  await window.type("Travel: hire.");
  // The prompt of the worker stands on the chain of the worker, which the session reads with the rest of the life.
  await until(async () => {
    await window.session.refresh();
    return window.session.acts.some((act) => act.kind === "prompt" && act.words[1] === "Travel: work.");
  }, "the worker under the second message");
  await window.type("Travel: ask.");
  let open: { hire: string; ask: string } | undefined;
  await until(async () => {
    const all = await asks(window);
    const hire = all.find((one) => one.message === "Travel: hire." && !one.over)?.id;
    const ask = all.find((one) => one.message === "Travel: ask." && !one.over)?.id;
    await window.session.refresh();
    if (hire && ask && window.session.acts.some((act) => act.id === ask)) open = { hire, ask };
    return open;
  }, "two open messages on the thread");
  return open as { hire: string; ask: string };
}

// 11-12: two presses of Up, 460 ms apart, to find a line sent before, cancelled the two last open asks of the owner.
test("Up two times on an empty input walks the history and ends no ask of the operator", () =>
  scenario(script, async (window) => {
    const { hire, ask } = await sent(window);
    expect(window.app.composer.plainText).toBe("");
    // Both presses come inside one refresh of the session, as the keys of the owner did.
    window.screen.mockInput.pressArrow("up");
    window.screen.mockInput.pressArrow("up");
    await until(
      () => window.app.composer.plainText === "Travel: hire.",
      "the second line back of the history",
    );
    // A cancel goes to the daemon and back: the time of a refresh and more lets one land if a key made one.
    await Bun.sleep(1500);
    await window.session.refresh();
    expect(await cancels(window)).toEqual([]);
    const all = await asks(window);
    expect(all.find((one) => one.id === hire)?.over).toBe(false);
    expect(all.find((one) => one.id === ask)?.over).toBe(false);
    expect(window.app.composer.plainText).toBe("Travel: hire.");
  }));

test("⌃C on an empty input names the act that a second ⌃C cancels, and cancels that act alone", () =>
  scenario(script, async (window) => {
    const { hire, ask } = await sent(window);
    window.screen.mockInput.pressCtrlC();
    await until(
      () => window.session.notice.includes(`take back ${ask}`),
      "the notice that names the last ask",
    );
    await window.shows((frame) => frame.includes(`Press ⌃C again to take back ${ask}`), "the notice drawn");
    await Bun.sleep(1500);
    expect(await cancels(window)).toEqual([]);
    window.screen.mockInput.pressCtrlC();
    await until(
      async () => (await asks(window)).find((one) => one.id === ask)?.over,
      "the cancel of the named ask",
    );
    await until(() => window.app.composer.plainText === "Travel: ask.", "the message back in the input");
    expect((await cancels(window)).map((fact) => fact[1])).toEqual([ask]);
    expect((await asks(window)).find((one) => one.id === hire)?.over).toBe(false);
  }));

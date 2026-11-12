import { expect, test } from "bun:test";
import { createTestRenderer } from "@opentui/core/testing";
import { isDone, outcomeOf } from "furb/life";
import { until } from "../../bind/typescript/test/until.ts";
import { App } from "../src/app.ts";
import { demoLibrary, demoSession } from "../src/demo.ts";

// A message of a thread closes with its answer. The session reads the close into its index at once, and the crate
// folds the thread of the message on another task, a step later, so a render can fall between the two. The box keys
// the program of a thread on the fold that the program is made of, so the answer stands once the fold holds the close.
test("the box of a thread shows the answer of a message that closed before the crate folded its thread again", async () => {
  const session = await demoSession({ seed: false });
  const library = await demoLibrary(session);
  const screen = await createTestRenderer({ exitSignals: [], width: 152, height: 46 });
  const app = new App(screen.renderer, session, { quit() {}, workspaces: library });
  const render = async (): Promise<string> => {
    for (let pass = 0; pass < 3; pass++) {
      app.render();
      await session.laddersRead();
    }
    app.render();
    await screen.flush();
    return screen.captureCharFrame();
  };
  const feedOf = (frame: string) =>
    frame
      .split("\n")
      .map((line) => line.slice(0, screen.renderer.width - session.preferences.sidebarWidth))
      .join("\n");
  const root = session.engine.root;
  const host = session.host as unknown as { threads: (...a: unknown[]) => Promise<unknown> };
  const real = host.threads.bind(host);
  // The fold of the crate as the feed reads it, held at the one it last gave, so a render reads the thread as it stood
  // when the hold began and the index as it stands now, which is the order of the failure.
  const kept = new Map<string, unknown>();
  let hold = false;
  host.threads = async (...a: unknown[]) => {
    const on = String(a[0]);
    if (hold && kept.has(on)) return kept.get(on);
    const got = await real(...a);
    kept.set(on, got);
    return got;
  };
  try {
    // A head that a message stands on, answered.
    await session.submit("Look around.");
    await until(session, () => session.acts.some((act) => session.isUserPrompt(act) && isDone(act)));
    // The message. The demo asks the operator and closes nothing, so the message stays open until the operator answers,
    // which is the moment the test owns.
    await session.submit("Ask me first, then answer.");
    await until(session, () => session.operatorPrompt !== undefined);
    const ask = session.operatorPrompt;
    const message = session.acts.findLast((act) => session.isUserPrompt(act))?.id ?? "";
    // The box draws the thread with the message open, so it keeps the program of the thread as it stands then, and the
    // crate holds that fold from here on.
    await render();
    expect(isDone(session.acts.find((x) => x.id === message))).toBe(false);
    hold = true;
    // The operator answers, so the word of the message closes it: the index takes the close, and the held fold does not.
    if (ask) await session.answer(ask, "Go ahead.");
    await until(session, () => isDone(session.acts.find((x) => x.id === message)));
    const answer = String(outcomeOf(session.acts.find((x) => x.id === message)) ?? "");
    await render();
    // The fold catches up, and the box draws the thread again: the message closed, so its answer stands under it.
    hold = false;
    await session.readThreads(root);
    const shown = feedOf(await render());
    expect(answer.length).toBeGreaterThan(0);
    expect(shown).toContain(answer.split("\n")[0] ?? "");
  } finally {
    app.dispose();
    screen.renderer.destroy();
    await library.dispose();
    await session.dispose();
  }
});

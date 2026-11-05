// What the owner does in the window: he closes, cancels and resolves threads with the keys and the mouse, a redraw
// between a press and its release resolves nothing, and "thinking" shows only while a reply of a model runs.
import { expect } from "bun:test";
import { scenario, test, until, type Window } from "./harness.ts";

/** The threads of the chain shown, as the sidebar lists them. */
const listed = (window: Window) => window.session.threadsOn(window.session.selected);

/** The part of each row of a frame that the sidebar shows. */
const rail = (window: Window, frame: string) =>
  frame.split("\n").map((line) => line.slice(window.width - window.session.preferences.sidebarWidth));

/** A click on the first row of the sidebar that shows a text. */
async function railClick(window: Window, text: string): Promise<void> {
  const left = window.width - window.session.preferences.sidebarWidth;
  const rows = rail(window, await window.frame());
  const y = rows.findIndex((line) => line.includes(text));
  expect(y).toBeGreaterThanOrEqual(0);
  await window.screen.mockMouse.click(left + (rows[y] ?? "").indexOf(text) + 1, y);
  await window.frame();
}

// #1253: the sidebar orders the open threads by their last message, so a message moved a thread under a still
// pointer, and a release on the Resolve that the redraw put there resolved a thread that nobody resolved.
test("a redraw between a press and its release resolves nothing, and a click on Resolve resolves the thread", () =>
  scenario([], async (window) => {
    const { engine } = window;
    const root = window.root;
    await engine.pause(root);
    const id = await engine.prompt("str", { message: "Hold this thread open", on: root });
    await until(() => listed(window).some((one) => one.act.id === id), "the first thread");
    await railClick(window, "Hold this thread open");
    await until(() => window.session.thread === id, "the thread chosen");
    const other = await engine.prompt("str", { message: "Another thread", on: root });
    await until(() => listed(window).some((one) => one.act.id === other), "the second thread");
    const left = window.width - window.session.preferences.sidebarWidth;
    const before = rail(window, await window.frame());
    const x = (before.find((line) => line.includes("✓ Resolve")) ?? "").indexOf("Resolve");
    const y = before.findIndex((line) => line.includes("Another thread")) + 1;
    expect(x).toBeGreaterThan(0);
    expect(before[y]?.[x]).toBe(" ");
    await window.screen.mockMouse.pressDown(left + x, y);
    // A message moves the thread held open, and its Resolve comes under the pointer, which did not move.
    await window.session.submit("One more word on it");
    await until(
      () => (listed(window).find((one) => one.act.id === id)?.messages.length ?? 0) > 0,
      "the message",
    );
    let after = before;
    for (let step = 0; step < 80 && !after[y]?.includes("Resolve"); step++) {
      await Bun.sleep(25);
      after = rail(window, await window.frame());
    }
    expect(after[y]?.slice(x, x + "Resolve".length)).toBe("Resolve");
    await window.screen.mockMouse.release(left + x, y);
    await window.frame();
    await Bun.sleep(300);
    expect(listed(window).find((one) => one.act.id === id)?.resolved).toBe(false);
    expect(await engine.peek(id)).toBeNull();
    await railClick(window, " Resolve ");
    await until(() => listed(window).find((one) => one.act.id === id)?.resolved === true, "the resolve");
    expect(await engine.peek(id)).toBe("resolved");
  }));

test("a thread closes and an act cancels from the commands of the window", () =>
  scenario(
    [{ cue: "Scenario: run long.", word: 'slow = bash("sleep 600", timeout=700)\nclose("Started.")' }],
    async (window) => {
      const { engine } = window;
      await window.type("Scenario: run long.");
      await window.shows((frame) => frame.includes("Started."), "the answer");
      const facts = await engine.transcript({ on: window.root, of: "bash" });
      const command = facts.find((fact) => /^bash\d+$/.test(fact[1]))?.[1] ?? "";
      expect(command).not.toBe("");
      await window.type(`/cancel ${command}`);
      await until(async () => (await engine.peek(command)) !== null, "the cancel of the command");
      await engine.pause(window.root);
      const open = await engine.prompt("int", { message: "Give a number", on: window.root });
      await until(() => listed(window).some((one) => one.act.id === open), "the thread of the number");
      await window.type(`/close ${open} 7`);
      await until(async () => (await engine.peek(open)) === 7, "the close of the thread");
    },
  ));

// #1258: a rung whose word had not come showed "thinking" while no reply of it ran, as one that waits for its turn
// behind the reply of another rung.
test('"thinking" shows only while a reply of the rung runs, and a rung that waits for its turn says so', () =>
  scenario(
    [
      { cue: "Scenario: first.", word: 'close("First done.")', hold: "go" },
      { cue: "Scenario: second.", word: 'close("Second done.")' },
    ],
    async (window, yard) => {
      await window.type("Scenario: first.");
      await window.shows((frame) => frame.includes("thinking"), "the first rung thinking");
      await window.type("/newthread");
      await window.type("Scenario: second.");
      const second = await window.shows(
        (frame) => frame.includes("waits for its turn"),
        "the second rung waiting for its turn",
      );
      expect(second.match(/thinking/g)?.length ?? 0).toBeLessThanOrEqual(1);
      await yard.release("go");
      await window.shows((frame) => frame.includes("Second done."), "the second answer");
      const done = await window.frame();
      expect(done).not.toContain("thinking");
    },
  ));

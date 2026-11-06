// What the owner does in the window: he closes, cancels, folds and resolves threads with the keys and the mouse, a
// redraw between a press and its release resolves and folds nothing, and "thinking" shows only while a reply of a model
// runs.
import { expect } from "bun:test";
import { scenario, test, until, type Window } from "./harness.ts";

/** The threads of the chain shown, as the sidebar lists them. */
const listed = (window: Window) => window.session.threadsOf(window.session.selected).list;

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
    // The press lands on the row of the other thread, under the controls of the thread held open, which the message
    // moves under that row.
    const y = before.findIndex((line) => line.includes("Another thread"));
    expect(x).toBeGreaterThan(0);
    expect(before[y - 1]).toContain("✓ Resolve");
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

// A thread is folded by one toggle alone: the Collapse or the Expand of its header, which stands pinned at the top of
// the feed while the header of a thread taller than the screen scrolled above it. A click in the body of a thread
// once folded it, and a click on its header resolved it beside that toggle.
test("the toggle of a thread taller than the screen stays in view and folds it, and a click in its body folds and resolves nothing", () =>
  scenario([], async (window) => {
    const { engine, app, session } = window;
    const root = window.root;
    await engine.pause(root);
    const long = (word: string) => Array.from({ length: 60 }, (_, n) => `${word} ${n + 1}.`).join("\n\n");
    const id = await engine.prompt("str", { message: long("Row"), on: root });
    const other = await engine.prompt("str", { message: long("Line"), on: root });
    await until(() => listed(window).some((one) => one.act.id === other), "the threads");
    const feed = (frame: string) =>
      frame.split("\n").map((line) => line.slice(0, window.width - session.preferences.sidebarWidth));
    const { viewport } = app.scroll;
    const headOf = (thread: string) => () => app.cards.get(thread)?.node;
    const head = headOf(id);
    await until(async () => {
      await window.frame();
      return [id, other].every((one) => (headOf(one)()?.height ?? 0) > 2 * viewport.height);
    }, "the whole messages");
    // Two frames: the first lays the feed out, and the pin reads that layout and shows in the second.
    const settle = async () => {
      await window.frame();
      return window.frame();
    };
    const scrollTo = async (row: number, of = head) => {
      const node = of();
      if (!node) throw new Error("no header of the thread");
      app.scroll.scrollTo(app.scroll.scrollTop + node.y + row - viewport.y);
      return settle();
    };
    // The middle of the thread stands at the top of the feed, and its header far above it.
    const middle = feed(await scrollTo(40));
    expect(head()?.y ?? 0).toBeLessThan(viewport.y);
    const top = middle[viewport.y] ?? "";
    expect(top).toContain("Row 1.");
    expect(top).toContain("▾ Collapse");
    expect(middle.filter((line) => line.includes("Collapse"))).toHaveLength(1);
    const toggle = top.indexOf("Collapse");
    // A click on the line at the left of the body and a click on its text fold nothing and resolve nothing.
    const y = middle.findIndex((line, at) => at > viewport.y + 5 && /Row \d+\./.test(line));
    const body = middle[y] ?? "";
    expect(y).toBeGreaterThan(0);
    for (const x of [body.indexOf("┃"), body.search(/Row/) + 2, toggle]) {
      expect(x).toBeGreaterThanOrEqual(0);
      await window.screen.mockMouse.click(x, y);
      await settle();
    }
    expect(session.folds[`thread:${id}`]).not.toBe(true);
    expect(listed(window).find((one) => one.act.id === id)?.resolved).toBe(false);
    expect(await engine.peek(id)).toBeNull();
    // #1253: a press on a row of the feed, then a redraw that puts the toggle under the still pointer, then the
    // release: nothing folds.
    const shown = feed(await scrollTo(0));
    expect(shown[viewport.y]).not.toContain("Collapse");
    // The header of the thread in the feed holds the toggle alone: the sidebar and /close resolve a thread.
    expect(shown.some((line) => line.includes("▾ Collapse"))).toBe(true);
    expect(shown.some((line) => line.includes("Resolve"))).toBe(false);
    await window.screen.mockMouse.pressDown(toggle, viewport.y);
    const pinned = feed(await scrollTo(20));
    expect(pinned[viewport.y]?.slice(toggle, toggle + "Collapse".length)).toBe("Collapse");
    await window.screen.mockMouse.release(toggle, viewport.y);
    await settle();
    expect(session.folds[`thread:${id}`]).not.toBe(true);
    // A press on the pinned toggle of one thread, a scroll that pins the next thread under the still pointer, and the
    // release fold neither thread: the toggle under the pointer at the release is another control.
    expect(feed(await scrollTo(40))[viewport.y]).toContain("Row 1.");
    await window.screen.mockMouse.pressDown(toggle, viewport.y);
    const next = feed(await scrollTo(20, headOf(other)))[viewport.y] ?? "";
    expect(next).toContain("Line 1.");
    expect(next.slice(toggle, toggle + "Collapse".length)).toBe("Collapse");
    await window.screen.mockMouse.release(toggle, viewport.y);
    await settle();
    expect(session.folds[`thread:${id}`]).not.toBe(true);
    expect(session.folds[`thread:${other}`]).not.toBe(true);
    await scrollTo(40);
    // The toggle folds the thread, and the feed then shows its header.
    await window.screen.mockMouse.click(toggle, viewport.y);
    const folded = feed(await window.shows((frame) => frame.includes("▸ Expand"), "the folded thread"));
    expect(session.folds[`thread:${id}`]).toBe(true);
    const node = head();
    expect(node && node.y >= viewport.y && node.y < viewport.y + viewport.height).toBe(true);
    expect(folded.filter((line) => line.includes("Expand"))).toHaveLength(1);
    expect(listed(window).find((one) => one.act.id === id)?.resolved).toBe(false);
    expect(await engine.peek(id)).toBeNull();
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

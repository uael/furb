import { expect, test } from "bun:test";
import { until } from "../../bind/typescript/test/until.ts";
import { find } from "../script/stage.ts";
import type { Session } from "../src/session.ts";
import { composing } from "./composing.ts";

/** A life as a conductor makes it: a message of the operator, the thread, whose model hires two workers, each with a
 * prompt on a chain of its own that thinks until a cancel, under a wait that holds the thread open. It gives the
 * thread and the prompts of the workers. */
async function conductor(session: Session): Promise<{ thread: string; workers: string[] }> {
  await session.submit("Hire two workers.");
  const workers = () =>
    session.acts.filter((act) => act.kind === "prompt" && act.on !== session.engine.root && !act.done);
  const thinking = () =>
    session.acts.filter((act) => act.kind === "rung" && act.on !== session.engine.root && !act.done);
  await until(session, () => workers().length === 2 && thinking().length === 2);
  const thread = session.activity.find((act) => session.isUserPrompt(act))?.id ?? "";
  return { thread, workers: workers().map((act) => act.id) };
}

/** Whether each act lives on. */
const live = (session: Session, ids: string[]) =>
  ids.every((id) => session.acts.find((act) => act.id === id)?.done === false);

/** The last message of the operator on the chain, once its model thinks on it. */
async function sent(session: Session, before: string): Promise<string> {
  await until(session, () => {
    const last = session.activity.findLast((act) => session.isUserPrompt(act));
    return Boolean(
      last &&
        last.id !== before &&
        session.activity.some((act) => act.kind === "rung" && act.by === last.id && !act.done),
    );
  });
  return session.activity.findLast((act) => session.isUserPrompt(act))?.id ?? "";
}

test("⌃C cancels the last message of the owner alone, and never the thread or the chain that it stands on", () =>
  composing(
    async ({ session, app, screen, frame }) => {
      const { thread, workers } = await conductor(session);
      app.composer.setText("Think until I cancel.");
      await app.submit();
      const message = await sent(session, thread);
      screen.mockInput.pressCtrlC();
      await until(session, () => session.acts.find((act) => act.id === message)?.done === true);
      // No work ran under the message but its own answer, so the cancel asked nothing.
      expect(await frame()).not.toContain("Keep");
      expect(live(session, [thread, ...workers])).toBe(true);
      // A message on the thread itself is the last message too, and its cancel leaves the thread.
      app.composer.setText("");
      const reply = await session.engine.prompt("str", { message: "Think until I cancel.", on: thread });
      await until(session, () =>
        session.activity.some((act) => act.kind === "rung" && act.by === reply && !act.done),
      );
      expect(session.lastOwn?.id).toBe(reply);
      screen.mockInput.pressCtrlC();
      await until(session, () => session.acts.find((act) => act.id === reply)?.done === true);
      expect(live(session, [thread, ...workers])).toBe(true);
      // The last message of the owner that is open is now the thread, which reaches the workers, so ⌃C asks first, and
      // ⌃C again closes the question and ends nothing.
      app.composer.setText("");
      screen.mockInput.pressCtrlC();
      await until(session, () => session.lastOwn?.id === thread);
      expect(await frame()).toContain(`Take back ${thread}, and end 5 running acts on 3 chains?`);
      screen.mockInput.pressCtrlC();
      expect(await frame()).not.toContain("Take back");
      await session.refresh();
      expect(live(session, [thread, ...workers])).toBe(true);
    },
    { width: 140, height: 44, exitOnCtrlC: false },
  ));

test("a cancel that reaches running work says how many acts it ends and on which chains, and ends them only once the owner confirms", () =>
  composing(
    async ({ session, frame, click }) => {
      const { thread, workers } = await conductor(session);
      // The wait of the thread, and the prompt and the rung of each worker.
      expect(session.reach(thread).acts).toHaveLength(5);
      await session.command(`/cancel ${thread}`);
      const asked = await frame();
      expect(asked).toContain(`Cancel ${thread}, and end 5 running acts on 3 chains?`);
      expect(asked).toContain(`It ends work on ${session.engine.root} (Main), `);
      expect(asked).toContain("(w/one)");
      expect(asked).toContain("(w/two)");
      await click("Keep");
      await session.refresh();
      expect(live(session, [thread, ...workers])).toBe(true);
      await session.command(`/cancel ${thread}`);
      await click("Cancel it");
      await until(session, () =>
        session.notice.startsWith(`${thread} is cancelled, and 5 acts under it on `),
      );
      await session.refresh();
      expect([thread, ...workers].some((id) => live(session, [id]))).toBe(false);
    },
    { width: 140, height: 44 },
  ));

test("the owner takes back his last message and writes it again, and no other work ends", () =>
  composing(
    async ({ session, app, screen, frame }) => {
      const { thread, workers } = await conductor(session);
      app.composer.setText("Think until I cancel.");
      await app.submit();
      const message = await sent(session, thread);
      // The menu of the message offers to take it back, and the menu of the thread offers its cancel.
      await session.refresh();
      for (const [id, choice] of [
        [message, "Take back"],
        [thread, "Cancel"],
      ] as const) {
        const text = String(session.acts.find((act) => act.id === id)?.words[1]);
        await frame();
        const [column, row] = find(screen, `┃ ${text}`);
        await screen.mockMouse.click(column + 4, row, 2);
        expect(await frame()).toContain(choice);
        screen.mockInput.pressCtrlC();
      }
      app.composer.setText("");
      await frame();
      screen.mockInput.pressCtrlC();
      await until(session, () => app.composer.plainText === "Think until I cancel.");
      await until(session, () => session.acts.find((act) => act.id === message)?.done === true);
      await session.refresh();
      expect(session.notice).toBe(`You took back ${message}. Change it in the input, and send it again.`);
      expect(await frame()).toContain("You took back");
      app.composer.setText("Think until I cancel, and say when.");
      await app.submit();
      await until(session, () =>
        session.activity.some((act) => act.words[1] === "Think until I cancel, and say when."),
      );
      expect(live(session, [thread, ...workers])).toBe(true);
      expect(session.acts.find((act) => act.id === message)?.done).toBe(true);
    },
    { width: 140, height: 60, exitOnCtrlC: false },
  ));

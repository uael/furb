import { expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { nestThreads, orderThreads, threadOpen } from "furb/life";
import { until } from "../../bind/typescript/test/until.ts";
import { openEngine } from "../src/bridge.ts";
import { Session } from "../src/session.ts";
import { type Composing, composing, ownPreferences } from "./composing.ts";

/** A frame that the QA of a change reads, when FURB_QA names a folder. */
const keep = (name: string, frame: string) => {
  if (process.env.FURB_QA) writeFileSync(`${process.env.FURB_QA}/${name}.txt`, frame);
};

/** A session on the recorded life of a conductor, which shows its log and runs nothing. */
async function recorded(): Promise<Session> {
  const { engine, host } = await openEngine({
    record: join(import.meta.dir, "..", "fixtures", "conductor.jsonl"),
  });
  return new Session(engine, host, true, ownPreferences());
}

/** The rows of a frame that the sidebar shows. */
const rail = ({ session }: Composing, frame: string, width: number) =>
  frame.split("\n").map((line) => line.slice(width - session.preferences.sidebarWidth));

/** A click on the first row of the sidebar that shows a text. */
async function railClick(use: Composing, text: string, width: number): Promise<void> {
  const rows = rail(use, await use.frame(), width);
  const y = rows.findIndex((line) => line.includes(text));
  expect(y).toBeGreaterThanOrEqual(0);
  const left = width - use.session.preferences.sidebarWidth;
  await use.screen.mockMouse.click(left + (rows[y] ?? "").indexOf(text) + 1, y);
}

/** Frames until the feed stands still, since it lays a thread out over a few frames. */
async function settled({ frame }: Composing): Promise<string> {
  let shown = await frame();
  for (let step = 0; step < 40; step++) {
    await Bun.sleep(25);
    const next = await frame();
    if (next === shown) return next;
    shown = next;
  }
  return shown;
}

/** The threads of the session shown, as the sidebar orders them. */
const ordered = (app: Composing["app"], session: Session) =>
  orderThreads(nestThreads(app.railThreads(), session.boardIndex.ids));

const wide = { width: 152, height: 46 };
const short = { width: 152, height: 30 };
const tall = { width: 152, height: 80 };

// Before, main listed the open threads first and the resolved ones under them, each in the order they began.
test("the sidebar lists the closed threads at the top and the open ones last, nearest the input, each by the time of its last message", async () =>
  composing(
    async (use) => {
      const { session, app } = use;
      await session.refresh();
      await session.select("chain1");
      const tree = ordered(app, session);
      // prompt95 began after prompt1, and prompt1 had its last message after prompt45: the closed ones come first.
      expect(tree.map((one) => one.thread.act.id)).toEqual([
        "prompt29",
        "prompt34",
        "prompt40",
        "prompt45",
        "prompt95",
        "prompt97",
        "prompt98",
        "prompt99",
        "prompt1",
      ]);
      expect(tree.map(threadOpen)).toEqual([false, false, false, false, false, false, false, false, true]);
      const shown = await settled(use);
      keep("order-sidebar", shown);
      const rows = rail(use, shown, tall.width);
      const at = (text: string) => rows.findIndex((line) => line.includes(text));
      // The threads stand under the row of the session that holds them, which bears the name of its record.
      expect(at("conductor")).toBeGreaterThan(0);
      expect(at("Have an agent loading")).toBeGreaterThan(at("conductor"));
      expect(at("This one is a bit more")).toBeGreaterThan(at("Have an agent loading"));
      expect(at("You are the orchestrator")).toBeGreaterThan(at("This one is a bit more"));
    },
    tall,
    await recorded(),
  ));

// Before, main folded a thread from a click on the left edge of any line of it, so a click in its body folded it.
test("a click on the left edge of a line in the middle of a thread folds nothing, and ⌥C folds the thread and its header then shows", async () =>
  composing(
    async (use) => {
      const { session, app, screen } = use;
      await session.refresh();
      await session.select("chain1");
      await settled(use);
      const middle = () => {
        const cards = app.threadCards().filter((one) => one.thread === "prompt1");
        expect(cards.length).toBeGreaterThan(2);
        return cards[Math.floor(cards.length / 2)]?.node;
      };
      const card = middle();
      if (!card) throw new Error("no card in the middle of prompt1");
      app.scroll.scrollChildIntoView(card.id);
      await settled(use);
      expect(session.folds["thread:prompt1"]).not.toBe(true);
      await screen.mockMouse.click(card.x + 1, card.y + Math.floor(card.height / 2));
      keep("click-on-a-line", await settled(use));
      expect(session.folds["thread:prompt1"]).not.toBe(true);
      // ⌥C folds the thread from the line in the middle of the feed.
      const { viewport } = app.scroll;
      const again = middle();
      if (!again) throw new Error("no card in the middle of prompt1");
      app.scroll.scrollTo(
        app.scroll.scrollTop + again.y - (viewport.y + Math.floor(viewport.height / 2)) + 1,
      );
      await settled(use);
      expect(app.threadInView()).toBe("prompt1");
      screen.mockInput.pressKey("c", { meta: true });
      const folded = await settled(use);
      keep("fold-from-a-line", folded);
      expect(session.folds["thread:prompt1"]).toBe(true);
      const head = app.cards.get("prompt1")?.node;
      expect(head && head.y >= viewport.y && head.y < viewport.y + viewport.height).toBe(true);
      expect(folded).toContain("▸ Expand");
    },
    wide,
    await recorded(),
  ));

// Before, main brought the header of the thread into view, and the end of a long thread stayed under the view.
test("a click on a thread in the sidebar moves the feed to the end of that thread, its last message in view", async () =>
  composing(
    async (use) => {
      const { session, app } = use;
      await session.refresh();
      await session.select("chain1");
      await settled(use);
      app.scroll.scrollTo(0);
      await settled(use);
      const { viewport } = app.scroll;
      const bottom = () => {
        const end = app.threadEnd("prompt1");
        return end ? end.y + end.height : Number.NaN;
      };
      expect(bottom()).toBeGreaterThan(viewport.y + viewport.height);
      await railClick(use, "You are the orchestrator", short.width);
      await until(session, () => session.thread === "prompt1");
      const shown = await settled(use);
      keep("click-to-the-end", shown);
      expect(bottom()).toBe(viewport.y + viewport.height);
      expect(shown).toContain("↳ Replying here");
    },
    short,
    await recorded(),
  ));

import { expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { chainSorted } from "furb/life";
import { openEngine } from "../src/bridge.ts";
import { Session } from "../src/session.ts";
import { composing } from "./composing.ts";

const size = { width: 152, height: 46 };

/** A frame that the QA of a change reads, when FURB_QA names a folder. */
const keep = (name: string, frame: string) => {
  if (process.env.FURB_QA) writeFileSync(`${process.env.FURB_QA}/${name}.txt`, frame);
};

/** A session on the recorded life of a conductor, which shows its log and runs nothing. */
async function recorded(): Promise<Session> {
  const { engine, host } = await openEngine({
    record: join(import.meta.dir, "..", "fixtures", "conductor.jsonl"),
  });
  return new Session(engine, host, true);
}

/** The threads of a chain of the session, and its asides with what each is, as the feed and the sidebar read them. */
const sorted = (session: Session, chain: string) => {
  const on = session.acts.filter((act) => act.on === chain);
  const rows = new Map(session.acts.map((act) => [act.id, act]));
  const { threads, asides } = chainSorted(on, rows, new Set(session.host.prompts.keys()));
  return {
    threads: threads.map((one) => one.act.id),
    asides: asides.map((one) => [one.act.id, one.aside]),
  };
};

// Before, main listed each prompt of these asides as a thread of its chain: a notice that the ladder of a thread made,
// and a question that a worker asked its coordinator. The suite of the host proves the same on the same record, with
// a wake and a word of the operator, in host/src/view/threads.rs.
test("a notice and a question of another chain stand as asides of the chain, and its threads are its conversations alone", async () =>
  composing(
    async ({ session, frame }) => {
      await session.refresh();
      await session.select("chain1");
      const question = (from: string) => ({ aside: "question" as const, from });
      expect(sorted(session, "chain1")).toEqual({
        threads: ["prompt1", "prompt29", "prompt34", "prompt95", "prompt97"],
        asides: [
          ["prompt43", { aside: "notice" }],
          ["prompt46", question("chain8")],
          ["prompt52", question("chain8")],
          ["prompt63", { aside: "notice" }],
          ["prompt83", { aside: "notice" }],
          ["prompt87", { aside: "notice" }],
          ["prompt92", question("chain9")],
          ["prompt93", question("chain10")],
        ],
      });
      expect(sorted(session, "chain7")).toEqual({
        threads: ["prompt31"],
        asides: [["prompt36", { aside: "notice" }]],
      });
      expect(sorted(session, "chain2")).toEqual({
        threads: ["prompt3"],
        asides: [
          ["prompt47", { aside: "notice" }],
          ["prompt64", { aside: "notice" }],
        ],
      });
      // The sidebar lists no aside: no notice of the ladder, and no question of a worker.
      const shown = await frame();
      keep("sidebar-chain1", shown);
      const rail = shown
        .split("\n")
        .map((line) => line.slice(size.width - session.preferences.sidebarWidth))
        .join("\n");
      expect(rail).not.toContain("The worker fabricated");
      expect(rail).not.toContain("I am chain8");
    },
    size,
    await recorded(),
  ));

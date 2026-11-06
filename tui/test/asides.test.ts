import { expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { chainNotes, chainSorted } from "furb/life";
import { openEngine } from "../src/bridge.ts";
import { Session } from "../src/session.ts";
import { composing, ownPreferences } from "./composing.ts";

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
  return new Session(engine, host, true, ownPreferences());
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
// a wake and a word of the operator, in src/view/threads.rs.
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

// Before, each note of the round drew as a card of its own, with an empty band between two of them. The host names the
// notes of a chain (the asides that are no question), and the notes that follow one another now draw as one block:
// one header that counts them and the ones not yet seen, and one panel with a row for each.
test("the notes that follow one another draw as one block under one header, with no empty band between them", async () =>
  composing(
    async ({ session, frame }) => {
      await session.refresh();
      await session.select("chain2");
      // The host names the two notes of the chain, and nothing else stands between them in its feed.
      expect(
        sorted(session, "chain2").asides.map(([id, aside]) => [id, (aside as { aside: string }).aside]),
      ).toEqual([
        ["prompt47", "notice"],
        ["prompt64", "notice"],
      ]);
      const shown = await frame();
      keep("notes-one-block-chain2", shown);
      const lines = shown.split("\n").map((line) => line.slice(0, 100).trimEnd());
      const headers = lines.filter((line) => line.includes("Notes"));
      expect(headers).toHaveLength(1);
      expect(headers[0]).toMatch(/Notes\s+2\s+● 2 unread/);
      const at = lines.findIndex((line) => line.includes("Notes"));
      // The header, then the top edge of one panel, the two rows, and its bottom edge: no blank row between the rows.
      const rows = lines.slice(at + 2, at + 4);
      expect(rows.every((line) => line.includes("┃"))).toBe(true);
      expect(lines[at + 1]).toContain("▄");
      expect(lines[at + 4]).toContain("▀");
    },
    size,
    await recorded(),
  ));

test("a note that no other note follows draws alone, with no header", async () =>
  composing(
    async ({ session, frame }) => {
      await session.refresh();
      await session.select("chain7");
      expect(sorted(session, "chain7").asides).toHaveLength(1);
      const shown = await frame();
      keep("notes-one-block-chain7", shown);
      expect(shown).not.toContain("› Notes");
    },
    size,
    await recorded(),
  ));

/** The runs of notes of a chain of the session, each as the ids of its prompts, as the feed draws them. */
const noted = (session: Session, chain: string) => {
  const on = session.acts.filter((act) => act.on === chain);
  const rows = new Map(session.acts.map((act) => [act.id, act]));
  const found = chainSorted(on, rows, new Set(session.host.prompts.keys()));
  return chainNotes(on, rows, found).map((run) => ({
    prompts: run.prompts.map((one) => one.id),
    count: run.count,
  }));
};

// The runs come from the order of the acts and not from the order of the cards, as src/view/threads.rs proves on the
// same record: the program of a thread stands between the two notices of chain2 and ends no run, the notices that the
// ladder of a thread made on chain1 stand in that ladder, and a lone notice is a run of one.
test("the notes of a chain that follow one another are one run, which the feed draws and does not find", async () =>
  composing(
    async ({ session }) => {
      await session.refresh();
      expect(noted(session, "chain2")).toEqual([{ prompts: ["prompt47", "prompt64"], count: 2 }]);
      expect(noted(session, "chain7")).toEqual([{ prompts: ["prompt36"], count: 1 }]);
      expect(noted(session, "chain1")).toEqual([]);
    },
    size,
    await recorded(),
  ));

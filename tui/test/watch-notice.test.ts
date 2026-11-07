// A notice of furb is never a thread of the operator: the stall that the watch of the daemon tells and the naming of
// the session stand as asides in the feed, no row of the sidebar names them, and the input never targets them.
import { expect, test } from "bun:test";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { openEngine } from "../src/bridge.ts";
import { Session } from "../src/session.ts";
import { composing, ownPreferences } from "./composing.ts";

const WIDTH = 140;

test("a stall notice of the watch and the naming of the session are no threads of the operator and no target of the input", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-watch-"));
  try {
    const record = join(directory, "watch.jsonl");
    const facts = [
      ["chain", "chain1", "operator", "", "root", ""],
      ["rung", "rung12", "chain1", "chain1", "name_session()", "", ""],
      ["name_session", "name_session1", "rung12", "chain1"],
      ["prompt", "prompt1", "operator", "chain1", "str", "Reply with pong.", ""],
      ["chain", "chain2", "name_session1", "chain1", "naming", ""],
      ["prompt", "prompt2", "name_session1", "chain2", "str", "Give a name to the session.", ""],
      ["prompt", "prompt5", "watch", "chain1", "None", "The chain chain1 had no change for 1 minute.", ""],
    ];
    // A recorded life, as a fixture keeps it: it shows its log and runs nothing.
    const lines = [{ recorded: true }, ...facts].map((one) => JSON.stringify(one));
    await writeFile(record, `${lines.join("\n")}\n`);
    const { engine, host } = await openEngine({ record });
    const session = new Session(engine, host, true, ownPreferences());
    await composing(
      async ({ session, frame }) => {
        await session.select("chain1");
        await session.readThreads("chain1");
        await session.readRail();
        const shown = await frame();
        const left = WIDTH - session.preferences.sidebarWidth;
        const sidebar = shown.split("\n").map((line) => line.slice(left));
        expect(session.threadsOf("chain1").list.map((one) => one.act.id)).toEqual(["prompt1"]);
        expect(sidebar.some((line) => line.includes("Reply with pong."))).toBe(true);
        expect(sidebar.some((line) => line.includes("had no change"))).toBe(false);
        expect(sidebar.some((line) => line.includes("Give a name"))).toBe(false);
        expect(session.target).toBe("prompt1");
        expect(shown).toContain("had no change for 1 minute");
      },
      { width: WIDTH, height: 44 },
      session,
    );
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

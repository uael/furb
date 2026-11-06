import { expect, test } from "bun:test";
import { appendFileSync, copyFileSync } from "node:fs";
import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { isDone } from "furb/life";
import { demoSession } from "../src/demo.ts";
import { Preferences } from "../src/preferences.ts";
import { composing } from "./composing.ts";

/** The recorded life of the fixture, in a file of its own that a test may add facts to. */
async function life() {
  const root = await mkdtemp(join(tmpdir(), "furb-reads-"));
  const record = join(root, "life.jsonl");
  copyFileSync(join(import.meta.dir, "..", "fixtures", "conductor.jsonl"), record);
  const session = await demoSession({
    record,
    cwd: root,
    preferences: new Preferences(join(root, "ui.json")),
  });
  const add = async (fact: unknown[], seen: () => boolean) => {
    appendFileSync(record, `${JSON.stringify(fact)}\n`);
    const until = Date.now() + 10_000;
    while (!seen()) {
      if (Date.now() > until) throw new Error(`the life heard not ${JSON.stringify(fact)} in 10s`);
      await new Promise<void>((done) => {
        const wake = setTimeout(done, 1000);
        session.once("change", () => {
          clearTimeout(wake);
          done();
        });
      });
    }
  };
  return { session, add };
}

test("the footer finds a model that writes on a chain from the acts that are not done, as each act of the life says", async () => {
  const { session, add } = await life();
  const every = (chain: string) =>
    session.acts.some(
      (act) => act.kind === "rung" && !isDone(act) && act.chain === chain && !session.program[act.id],
    );
  const same = () => {
    const chains = [...new Set(session.acts.map((act) => act.chain))];
    expect(chains.map((chain) => session.writing(chain))).toEqual(chains.map(every));
  };
  same();
  await add(["rung", "rung9001", "operator", "chain1", "", "", ""], () => Boolean(session.actOf("rung9001")));
  await add(["started", "rung9001", "chain1"], () => Boolean(session.actOf("rung9001")?.began));
  expect(session.writing("chain1")).toBe(true);
  same();
  await add(["done", "rung9001", "chain1", null], () =>
    Boolean(session.actOf("rung9001") !== undefined && isDone(session.actOf("rung9001"))),
  );
  same();
});

test("a token that a model writes reads no thread for the marks of the sidebar, and a read that moves reads them again", async () => {
  const { session } = await life();
  await composing(
    async ({ app, frame }) => {
      await frame();
      const threads = app.railThreads();
      expect(threads.length).toBeGreaterThan(1);
      let reads = 0;
      const unread = app.unreadThread.bind(app);
      app.unreadThread = (thread) => {
        reads++;
        return unread(thread);
      };
      session.host.streams.set("rung9002", { chain: "chain1", text: "a token", thinking: "" });
      await frame();
      expect(reads).toBe(0);
      session.host.seen = new Map([[threads[0]?.act.id ?? "", Number.MAX_SAFE_INTEGER]]);
      await frame();
      expect(reads).toBeGreaterThanOrEqual(threads.length);
    },
    { width: 152, height: 46 },
    session,
  );
});

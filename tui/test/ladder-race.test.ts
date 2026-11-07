// A read of the crate that ends while the session changed again must leave the next draw free to ask for the ladder
// again: the change that the read brings is emitted once the read is over, and not while it still holds its place.
import { expect, test } from "bun:test";
import { demoSession } from "../src/demo.ts";
import { idle } from "./idle.ts";

test("a ladder read that ends after a newer change lets the draw of its change ask again", async () => {
  const session = await demoSession({ seed: true });
  try {
    await idle(session);
    const chain = session.engine.root;
    await session.laddersRead();
    const real = session.host.ladder.bind(session.host);
    let calls = 0;
    let release: () => void = () => {};
    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    session.host.ladder = async (...args: Parameters<typeof real>) => {
      calls++;
      if (calls === 1) await gate;
      return real(...args);
    };
    const mark = (session as unknown as { held: number }).held;
    // The acts change, so the ladder is stale: a read starts and waits.
    (session as unknown as { held: number }).held = mark + 1;
    session.ladderOf(chain, []);
    expect(calls).toBe(1);
    // A newer change comes while it waits, and the read ends: the draw that its change brings asks again.
    (session as unknown as { held: number }).held = mark + 2;
    session.on("change", () => void session.ladderOf(chain, []));
    release();
    for (let wait = 0; wait < 100 && calls < 2; wait++) await Bun.sleep(10);
    expect(calls).toBeGreaterThanOrEqual(2);
  } finally {
    await session.dispose();
  }
});

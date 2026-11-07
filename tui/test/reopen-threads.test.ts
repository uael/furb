// A reopen of a window shows each thread with the same title and the same count as before the quit.
import { expect, test } from "bun:test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { until } from "../../bind/typescript/test/until.ts";
import { demoSession } from "../src/demo.ts";
import type { Session } from "../src/session.ts";

const titles = async (session: Session) => {
  for (let tries = 0; tries < 200; tries++) {
    await session.refresh();
    await session.readThreads(session.engine.root);
    await session.readRail();
    const rows = session.rail.map((node) => [node.thread.act.id, node.thread.title, node.thread.state]);
    if (rows.length === 3 && rows.every((row) => row[1])) return rows;
    await Bun.sleep(25);
  }
  return session.rail.map((node) => [node.thread.act.id, node.thread.title, node.thread.state]);
};

test("a reopen lists the threads by their messages, with the same rows and states as before the quit", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-reopen-"));
  const record = join(directory, "life.jsonl");
  let session: Session | undefined;
  try {
    session = await demoSession({ record });
    for (const message of ["First thread.", "Second thread.", "Third thread."]) {
      // Each message opens a thread of its own.
      session.aim();
      await session.submit(message);
    }
    await until(
      session,
      () => session?.threadsOf(session.engine.root).list.every((one) => one.over) ?? false,
    );
    const before = await titles(session);
    await session.dispose();
    session = await demoSession({ record });
    const after = await titles(session);
    expect(before.map((row) => row[1])).toEqual(["First thread.", "Second thread.", "Third thread."]);
    expect(after).toEqual(before);
  } finally {
    await session?.dispose().catch(() => {});
    await rm(directory, { recursive: true, force: true });
  }
});

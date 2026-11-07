import { expect, mock, test } from "bun:test";
import { basename, join } from "node:path";

const scan: { give?: (said: string | undefined) => void } = {};
// The scan of the worker waits for the test.
mock.module("../src/records.ts", () => ({
  inspectRecords: (paths: string[], _signal: AbortSignal, replay: string[] = paths) =>
    new Promise((resolve) => {
      expect(replay).toEqual([]);
      scan.give = (said) =>
        resolve(
          Object.fromEntries(paths.map((path) => [path, { state: "saved", name: said, replayed: false }])),
        );
    }),
}));

const { openEngine } = await import("../src/bridge.ts");
const { Session } = await import("../src/session.ts");

const opened = async () => {
  const { engine, host } = await openEngine({
    record: join(import.meta.dir, "..", "fixtures", "conductor.jsonl"),
  });
  return new Session(engine, host, true);
};

const until = async (done: () => boolean) => {
  for (let tries = 0; !done() && tries < 400; tries++) await Bun.sleep(5);
};

test("the constructor of a session reads no record, and the name that the worker scan finds shows when it comes", async () => {
  const session = await opened();
  try {
    const before = session.sessionName;
    expect(before).not.toBe("pf-worker");
    scan.give?.("pf-worker");
    await until(() => session.sessionName === "pf-worker");
    expect(session.sessionName).toBe("pf-worker");
  } finally {
    await session.dispose();
  }
});

test("an empty name that the worker scan finds gives the default name again", async () => {
  const session = await opened();
  try {
    scan.give?.("");
    await until(() => session.recordName !== undefined);
    expect(session.recordName).toBe(basename(session.host.directory));
  } finally {
    await session.dispose();
  }
});

import { afterAll, expect, mock, test } from "bun:test";
import { basename } from "node:path";
import { isDone } from "furb/life";
import { until } from "../../bind/typescript/test/until.ts";
import * as records from "../src/records.ts";

const real = { ...records };
let failing = true;
let failed = 0;
// The module of the read of a record is the boundary: while `failing` holds, the read fails as a read of a record that
// the store has not written yet does, and the session is what stands under test.
mock.module("../src/records.ts", () => ({
  ...real,
  inspectRecords: (...args: Parameters<typeof real.inspectRecords>) => {
    if (!failing) return real.inspectRecords(...args);
    failed++;
    return Promise.reject(new Error("The read of the record failed."));
  },
}));
const { demoSession, removeDemoDirectories } = await import("../src/demo.ts");

afterAll(removeDemoDirectories);

test("a read of the record that failed when the extension named the session is read again, and the name shows", async () => {
  const session = await demoSession({ skills: true });
  try {
    expect(session.sessionName).toBe(basename(session.host.directory));
    await session.submit("Fix the login form.\nIt fails with a 500.");
    const named = () =>
      session.activityByChain.get(session.engine.root)?.find((one) => one.kind === "name_session");
    // The act is done and the reads that followed it failed, so the record has said nothing of the name yet.
    await until(session, () => {
      const act = named();
      return act !== undefined && isDone(act) && failed >= 2;
    });
    expect(session.sessionName).toBe(basename(session.host.directory));
    failing = false;
    await until(session, () => session.sessionName === "Fix the login form");
    expect(session.sessionName).toBe("Fix the login form");
  } finally {
    await session.dispose();
  }
});

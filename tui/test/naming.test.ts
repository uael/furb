import { afterAll, expect, test } from "bun:test";
import { basename } from "node:path";
import { demoSession, removeDemoDirectories } from "../src/demo.ts";
import type { Session } from "../src/session.ts";
import { Workspaces } from "../src/workspaces.ts";

afterAll(removeDemoDirectories);

/** A demo session that runs the official extensions, in a list of sessions: the session and its row. */
async function listed() {
  const session = await demoSession({ extensions: true });
  const library = new Workspaces(session.preferences, { demo: true });
  return { session, library, entry: library.adopt(session, await library.add(session.host.directory)) };
}

/** The act of the extension naming on the root, once it is done. */
async function named(session: Session) {
  for (let tries = 0; tries < 400; tries++) {
    await session.refresh();
    const act = session.activityByChain.get(session.engine.root)?.find((one) => one.kind === "name_session");
    if (act?.done) return act;
    await Bun.sleep(25);
  }
  throw new Error("The session was not named.");
}

test("the first message of the operator names the session and its row, and the session knows the chain that names it, which the sidebar hides", async () => {
  const { session, entry } = await listed();
  try {
    expect(session.sessionName).toBe(basename(session.host.directory));
    await session.submit("Fix the login form.\nIt fails with a 500.");
    const act = await named(session);
    for (let tries = 0; tries < 400 && !session.sessionNamed; tries++) await Bun.sleep(25);
    expect([act.value, session.sessionName, entry.name]).toEqual([
      "Fix the login form",
      "Fix the login form",
      "Fix the login form",
    ]);
    const chains = [...session.activityByChain.keys()].filter((chain) => session.naming(chain));
    expect(chains.length).toBe(1);
  } finally {
    await session.dispose();
  }
});

test("a name that the operator gave the session stays over the name of the first message", async () => {
  const { session, library, entry } = await listed();
  try {
    library.rename(entry, "Mine");
    await session.submit("Fix the login form.");
    await named(session);
    await Bun.sleep(100);
    expect([session.sessionName, entry.name]).toEqual(["Mine", "Mine"]);
  } finally {
    await session.dispose();
  }
});

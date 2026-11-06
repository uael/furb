import { afterAll, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import { isDone, outcomeOf } from "furb/life";
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
    if (act !== undefined && isDone(act)) return act;
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
    for (let tries = 0; tries < 400 && session.sessionName !== "Fix the login form"; tries++)
      await Bun.sleep(25);
    expect([outcomeOf(act), session.sessionName, entry.name]).toEqual([
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
    await library.rename(entry, "Mine");
    await session.submit("Fix the login form.");
    await named(session);
    await Bun.sleep(100);
    expect([session.sessionName, entry.name]).toEqual(["Mine", "Mine"]);
  } finally {
    await session.dispose();
  }
});

test("a rename of the operator is a fact of the record, which a second session on the record shows with no file of the window", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-name-"));
  const record = join(directory, "life.jsonl");
  let session: Session | undefined;
  try {
    session = await demoSession({ record, extensions: true });
    await session.rename("Mine");
    await session.rename("Mine again");
    await session.dispose();
    await rm(`${record}.ui.json`, { force: true });
    session = await demoSession({ record, extensions: true });
    for (let tries = 0; tries < 400 && session.sessionName !== "Mine again"; tries++) await Bun.sleep(25);
    expect(session.sessionName).toBe("Mine again");
    await session.rename("");
    expect(session.sessionName).toBe(basename(session.host.directory));
  } finally {
    await session?.dispose();
    await rm(directory, { recursive: true, force: true });
  }
});
/** The values of the acts rename_session on the root of a session, in the order of the record. */
function renames(session: Session) {
  return (session.activityByChain.get(session.engine.root) ?? [])
    .filter((one) => one.kind === "rename_session")
    .map((one) => outcomeOf(one));
}

test("a rename of the operator is a fact of the record whatever extension plays, with none playing", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-name-"));
  const record = join(directory, "life.jsonl");
  let session: Session | undefined;
  try {
    session = await demoSession({ record });
    expect(await session.rename("Mine")).toBe(true);
    expect([session.sessionName, session.notice]).toEqual(["Mine", ""]);
    await session.dispose();
    await rm(`${record}.ui.json`, { force: true });
    session = await demoSession({ record });
    for (let tries = 0; tries < 400 && session.sessionName !== "Mine"; tries++) await Bun.sleep(25);
    expect([session.sessionName, renames(session)]).toEqual(["Mine", ["Mine"]]);
  } finally {
    await session?.dispose();
    await rm(directory, { recursive: true, force: true });
  }
});

test("a rename that the record does not take keeps the old name and says why in a notice", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-name-"));
  const record = join(directory, "life.jsonl");
  let session: Session | undefined;
  try {
    session = await demoSession({ record });
    await session.rename("First");
    await session.dispose();
    expect(await session.rename("Second")).toBe(false);
    expect(session.sessionName).toBe("First");
    expect(session.notice).toStartWith("The name was not kept: ");
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

// Before, the threads of a chain were read again only when a prompt or a question changed, so a rename of a session
// that rests kept the old title on its first thread.
test("a rename of a session that rests gives its first thread the new name, with no prompt more", async () => {
  const session = await demoSession({ extensions: true });
  try {
    await session.submit("Look at the readme.");
    const root = session.engine.root;
    const first = async () => {
      await session.refresh();
      await session.readThreads(root);
      return session.threadsOf(root).list.find((one) => one.origin === "operator");
    };
    for (let tries = 0; tries < 400 && !(await first())?.over; tries++) await Bun.sleep(25);
    const prompts = session.promptsVersion;
    expect(await session.rename("New name")).toBe(true);
    let title: string | undefined;
    for (let tries = 0; tries < 400 && title !== "New name"; tries++) {
      await Bun.sleep(25);
      await session.refresh();
      title = session.threadsOf(root).list.find((one) => one.origin === "operator")?.title;
    }
    expect(title).toBe("New name");
    expect(session.promptsVersion).toBe(prompts);
  } finally {
    await session.dispose();
  }
});

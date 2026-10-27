import { expect, test } from "bun:test";
import { execFileSync } from "node:child_process";
import { mkdir, mkdtemp, realpath, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { demoSession } from "../src/demo.ts";
import { workspaceRoot } from "../src/history.ts";
import type { Session } from "../src/session.ts";
import { composing } from "./composing.ts";

/** A session of the demo on a record of its own in a folder, or on a record that a test names. */
let made = 0;
async function demoIn(cwd: string, record?: string): Promise<Session> {
  const sessions = join(cwd, ".furb", "sessions");
  await mkdir(sessions, { recursive: true });
  return demoSession({ cwd, record: record ?? join(sessions, `${++made}.jsonl`) });
}

const git = (cwd: string, ...args: string[]) =>
  execFileSync("git", ["-c", "user.name=test", "-c", "user.email=test@example.com", ...args], {
    cwd,
    stdio: "pipe",
  });

/** A repository with a worktree and a folder in it, another repository, and a folder that no repository holds, in a
 * temporary directory that a test removes after it. */
async function repositories() {
  const base = await realpath(await mkdtemp(join(tmpdir(), "furb-history-")));
  const main = join(base, "project");
  const worktree = join(base, "project-w-feature");
  const other = join(base, "elsewhere");
  const loose = join(base, "loose");
  for (const directory of [main, other]) {
    await mkdir(directory);
    git(directory, "init", "-q");
    git(directory, "commit", "-q", "--allow-empty", "-m", "start");
  }
  await mkdir(loose);
  git(main, "worktree", "add", "-q", worktree, "-b", "w/feature");
  await mkdir(join(worktree, "tui/src"), { recursive: true });
  return { base, main, worktree, other, loose };
}

test("a worktree and its subfolders belong to the workspace of their repository, and another repository to its own", async () => {
  const { base, main, worktree, other, loose } = await repositories();
  try {
    expect(workspaceRoot(main)).toBe(main);
    expect(workspaceRoot(worktree)).toBe(main);
    expect(workspaceRoot(join(worktree, "tui/src"))).toBe(main);
    expect(workspaceRoot(other)).toBe(other);
    expect(workspaceRoot(loose)).toBe(loose);
  } finally {
    await rm(base, { recursive: true, force: true });
  }
}, 60_000);

test("the history of a workspace persists across sessions and lives, the open session comes last, and no other workspace shares it", async () => {
  const { base, main, worktree, other } = await repositories();
  const done = new Set<Session>();
  const opened: Session[] = [];
  const open = async (cwd: string, record?: string) => {
    const session = await demoIn(cwd, record);
    opened.push(session);
    return session;
  };
  // The App of a test ends its session, which saves its view.
  const history = async (session: Session) => {
    let lines: string[] = [];
    let python: string[] = [];
    await composing(
      async ({ app }) => {
        lines = app.history();
        session.mode = "python";
        python = app.history();
        session.mode = "prompt";
      },
      undefined,
      session,
    );
    done.add(session);
    return { lines, python };
  };
  try {
    const first = await open(main);
    const second = await open(main);
    const inWorktree = await open(worktree);
    const elsewhere = await open(other);
    expect(new Set(opened.map((session) => session.host.record)).size).toBe(4);
    const key = first.draftKey;
    expect(second.draftKey).toBe(key);
    first.remember(key, "first one");
    inWorktree.remember(key, "from the worktree");
    elsewhere.remember(key, "from elsewhere");
    first.remember(key, "shared");
    first.remember(key, "first two");
    first.remember(key.replace(":prompt", ":python"), "x = 1");
    second.remember(key, "second one");
    second.remember(key, "shared");

    // The open session comes last, after the other sessions of the workspace and its worktrees, and a line that came
    // before comes once, at its latest place.
    const seen = await history(second);
    expect(seen.lines).toEqual(["first one", "from the worktree", "first two", "second one", "shared"]);
    expect(seen.python).toEqual(["x = 1"]);

    // A later life on the record, a new session of the workspace and a new session of its worktree read the same.
    const again = await open(main, second.host.record);
    const third = await open(main);
    const inWorktreeAgain = await open(worktree);
    const expected = ["first one", "from the worktree", "first two", "second one", "shared"];
    expect((await history(again)).lines).toEqual(expected);
    expect((await history(third)).lines).toEqual(expected);
    expect((await history(inWorktreeAgain)).lines).toEqual(expected);
    // The session of the worktree that sent a line has it last, after those of the rest of the workspace.
    expect((await history(inWorktree)).lines).toEqual([
      "first one",
      "first two",
      "second one",
      "shared",
      "from the worktree",
    ]);

    // Another workspace shares nothing, and its lines reach none of the first.
    expect((await history(elsewhere)).lines).toEqual(["from elsewhere"]);
    expect((await history(await open(other))).lines).toEqual(["from elsewhere"]);
  } finally {
    for (const session of opened) if (!done.has(session)) await session.dispose();
    await rm(base, { recursive: true, force: true });
  }
}, 120_000);

test("a session that kept lines before its workspace did gives them to the workspace when it opens again", async () => {
  const { base, loose } = await repositories();
  const opened: Session[] = [];
  try {
    const old = await demoIn(loose);
    const record = old.host.record;
    old.histories[old.draftKey] = ["an old line"];
    await old.dispose();
    const again = await demoIn(loose, record);
    opened.push(again);
    const next = await demoIn(loose);
    opened.push(next);
    expect(next.workspaceHistory(":prompt")).toEqual(["an old line"]);
    expect(again.workspaceHistory(":prompt")).toEqual([]);
  } finally {
    for (const session of opened) await session.dispose();
    await rm(base, { recursive: true, force: true });
  }
}, 60_000);

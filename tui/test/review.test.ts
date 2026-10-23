import { afterAll, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import { until } from "../../bind/typescript/test/until.ts";
import { removeDemoDirectories } from "../src/demo.ts";
import { branchChanges, commentStart, lines, net, revertWord } from "../src/review.ts";
import type { Session } from "../src/session.ts";
import { composing } from "./composing.ts";
import { idle } from "./idle.ts";

afterAll(removeDemoDirectories);

/** A write of the operator on a chain, which the test waits for until the journal holds it. */
async function written(
  session: Session,
  path: string,
  content: string,
  on = session.engine.root,
): Promise<void> {
  const count = session.host.changes;
  await session.engine.result(
    await session.engine.rung({
      word: `write(Text(${JSON.stringify(path)}, ${JSON.stringify(content)}))`,
      on,
    }),
  );
  await until(session.host, () => session.host.changes > count);
}

/** The files that the review shows in a scope, by the name of each, with the lines that each added and removed. */
async function reviewed(
  session: Session,
  scope: "turn" | "chain" | "life",
): Promise<[string, number, number][]> {
  session.review(scope);
  await session.refresh();
  return session.changes.map((change) => [change.path.split("/").at(-1) ?? "", change.added, change.removed]);
}

test("the net change of a file runs from its text before its first change to its text after its last", () => {
  expect(
    net([
      { path: "a", before: "1\n", after: "2\n" },
      { path: "b", before: "", after: "x\n" },
      { path: "a", before: "2\n", after: "3\n" },
      { path: "b", before: "x\n", after: "" },
    ]),
  ).toEqual([{ path: "a", before: "1\n", after: "3\n" }]);
});

test("each scope of the review holds the files of its writes: this turn, this chain, and the life", () =>
  composing(async ({ session }) => {
    await written(session, "early.txt", "one\n");
    await session.submit("Look at the notes.");
    await idle(session);
    await written(session, "late.txt", "a\nb\n");
    const other = String(
      await session.engine.result(
        await session.engine.rung({ word: 'chain("other")', on: session.engine.root }),
      ),
    );
    await session.refresh();
    const chain = session.chains.find(
      (one) => one.id !== session.engine.root && one.words[0] === "other",
    )?.id;
    expect(chain ?? other).toBeTruthy();
    await written(session, "elsewhere.txt", "z\n", chain ?? other);
    await session.refresh();
    expect(await reviewed(session, "turn")).toEqual([["late.txt", 2, 0]]);
    expect(await reviewed(session, "chain")).toEqual([
      ["early.txt", 1, 0],
      ["late.txt", 2, 0],
    ]);
    expect(await reviewed(session, "life")).toEqual([
      ["early.txt", 1, 0],
      ["late.txt", 2, 0],
      ["elsewhere.txt", 1, 0],
    ]);
  }));

test("the branch against main holds each file that differs from the base of the branch, an untracked file among them", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-review-"));
  const git = (...args: string[]) =>
    spawnSync("git", ["-C", directory, "-c", "user.name=t", "-c", "user.email=t@t", ...args], {
      encoding: "utf8",
    });
  try {
    git("init", "-q", "-b", "main");
    await writeFile(join(directory, "kept.txt"), "one\n");
    await writeFile(join(directory, "gone.txt"), "old\n");
    git("add", ".");
    git("commit", "-q", "-m", "base");
    git("checkout", "-q", "-b", "work");
    await writeFile(join(directory, "kept.txt"), "two\n");
    git("rm", "-q", "gone.txt");
    git("commit", "-q", "-am", "work");
    await writeFile(join(directory, "new.txt"), "fresh\n");
    const changes = branchChanges(directory).map(({ path, before, after }) => [
      basename(path),
      before,
      after,
    ]);
    expect(changes.sort()).toEqual([
      ["gone.txt", "old\n", ""],
      ["kept.txt", "one\n", "two\n"],
      ["new.txt", "", "fresh\n"],
    ]);
    expect(() => branchChanges(tmpdir())).toThrow("is not in a git repository");
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("a revert of a hunk writes its old lines back through the World, and refuses once the file moved on", () =>
  composing(async ({ session }) => {
    const path = join(session.host.directory, "notes.txt");
    await writeFile(path, "a\nb\nc\nd\ne\nf\ng\nh\ni\nj\n");
    await written(session, "notes.txt", "a\nB\nc\nd\ne\nf\ng\nh\ni\nJ\n");
    session.review("chain");
    await session.refresh();
    const [change] = session.changes;
    if (!change) throw new Error("No change to review.");
    expect(change.hunks.length).toBe(2);
    const [first, second] = change.hunks;
    if (!first || !second) throw new Error("No hunks.");
    await session.revert(change, first);
    expect(await readFile(path, "utf8")).toBe("a\nb\nc\nd\ne\nf\ng\nh\ni\nJ\n");
    // The revert is a write of the operator, so the record keeps it and the review shows what is left.
    expect(session.changes.map((one) => one.hunks.length)).toEqual([1]);
    await writeFile(path, "moved\n");
    expect(session.revert(change, second)).rejects.toThrow("changed since the review");
    expect(await readFile(path, "utf8")).toBe("moved\n");
  }));

test("the word of a revert of a hunk that only added lines takes them out", () => {
  const word = revertWord("x.txt", {
    oldStart: 0,
    oldLines: 0,
    newStart: 1,
    newLines: 2,
    lines: ["+a", "+b"],
  });
  expect(word).toContain('if held.lines[0:2] != ["a","b"]:');
  expect(word).toContain('write(held.edit(1, 2, ""))');
});

test("a comment on a line puts the start of a message that names the line in the input, which Enter sends to the chain", () =>
  composing(
    async ({ session, app, screen, frame, click }) => {
      await writeFile(join(session.host.directory, "notes.txt"), "a\nb\nc\n");
      await written(session, "notes.txt", "a\nB\nc\n");
      session.review("chain");
      await session.refresh();
      await frame();
      await click("Comment a line");
      await screen.flush();
      // The pointer of the list stands on the first line that the hunk changed.
      screen.mockInput.pressEnter();
      await screen.flush();
      const hunk = session.changes[0]?.hunks[0];
      if (!hunk) throw new Error("No hunk.");
      const line = lines(hunk).find((one) => one.sign !== " ");
      if (!line) throw new Error("No changed line.");
      expect(line).toEqual({ number: 2, sign: "-", text: "b" });
      expect(app.composer.plainText).toBe(commentStart("notes.txt", line));
      expect(app.composer.plainText).toBe("On notes.txt, the line 2 that the change removed:\n> b\n\n");
      app.composer.insertText("Keep it lower case.");
      const before = session.activity.filter((act) => session.isUserPrompt(act)).length;
      await session.submit(app.composer.plainText);
      expect(session.activity.filter((act) => session.isUserPrompt(act)).length).toBe(before + 1);
      expect(String(session.activity.findLast((act) => session.isUserPrompt(act))?.words[1])).toContain(
        "> b\n\nKeep it lower case.",
      );
    },
    { width: 120, height: 44, useMouse: true },
  ));

test("a diff shows its two sides side by side at 140 columns or more, and one under the other below", async () => {
  for (const [width, side] of [
    [150, true],
    [139, false],
  ] as const)
    await composing(
      async ({ session, frame }) => {
        await writeFile(join(session.host.directory, "notes.txt"), "a\nold line\nc\n");
        await written(session, "notes.txt", "a\nnew line\nc\n");
        session.review("chain");
        await session.refresh();
        const rows = (await frame()).split("\n");
        expect(rows.some((row) => row.includes("old line") && row.includes("new line"))).toBe(side);
        expect(rows.some((row) => row.includes("old line"))).toBe(true);
      },
      { width, height: 40 },
    );
});

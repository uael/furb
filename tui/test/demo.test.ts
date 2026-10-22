import { expect, test } from "bun:test";
import { existsSync } from "node:fs";
import { dirname } from "node:path";
import { demoSession, removeDemoDirectories, tour } from "../src/demo.ts";

test("the temporary directory of a demo goes when the process that made it removes its demo directories", async () => {
  const session = await demoSession();
  const directory = session.host.directory;
  expect(directory).toContain("furb-demo-");
  await session.dispose();
  expect(existsSync(directory)).toBe(true);
  await removeDemoDirectories();
  expect(existsSync(dirname(directory))).toBe(false);
});

test("the life that the operator watches asks three models in one word, branches, and spends nothing", async () => {
  const session = await demoSession({ watch: true });
  try {
    await tour(session, 0);
    const prompts = session.acts.filter((act) => act.kind === "prompt");
    expect(prompts.map((act) => act.words[2] || "")).toEqual([
      "",
      "claude-cli:haiku",
      "claude-cli:sonnet",
      "claude-cli:fable",
      "",
    ]);
    expect(prompts.slice(1, 4).map((act) => act.value)).toEqual([
      "Add a test for a search that finds nothing.",
      "Save the notes to a file, so they stay after a restart.",
      "Give search a shortcut, Ctrl+K, so it is one key away.",
    ]);
    expect(session.chains.map((chain) => chain.words[0])).toEqual(["root", "Another way"]);
    expect(session.host.cost).toBe(0);
  } finally {
    await session.dispose();
    await removeDemoDirectories();
  }
});

test("the moment that the feed shows steps back through the chain, and a branch there keeps what came before", async () => {
  const session = await demoSession({ watch: true });
  try {
    await session.engine.result(
      await session.engine.prompt("str", {
        message: "Read this project and say what it does.",
        on: session.selected,
      }),
    );
    await session.refresh();
    const now = session.at(session.activity);
    session.travel(-1);
    const moment = session.moment ?? 0;
    expect(moment).toBeGreaterThan(0);
    const then = session.at(session.activity);
    expect(then.every((act) => act.began < moment)).toBe(true);
    expect(then.some((act) => !act.done)).toBe(true);
    session.travel(1);
    expect(session.moment).toBeUndefined();
    expect(session.at(session.activity)).toEqual(now);
    session.travel(-1);
    session.travel(-1);
    const root = session.selected;
    const branch = await session.branchHere();
    expect(branch).not.toBe(root);
    expect(session.moment).toBeUndefined();
    expect(session.selected).toBe(branch);
  } finally {
    await session.dispose();
    await removeDemoDirectories();
  }
});

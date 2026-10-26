import { expect, test } from "bun:test";
import { existsSync } from "node:fs";
import { dirname } from "node:path";
import { demoSession, removeDemoDirectories, seedRichDemo, tour } from "../src/demo.ts";

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
  const pace = process.env.FURB_DEMO_PACE;
  // The test checks the streamed result, not the delay that lets a person read each piece.
  process.env.FURB_DEMO_PACE = "1";
  const session = await demoSession({ watch: true });
  try {
    await tour(session, 0);
    const prompts = session.acts.filter((act) => act.kind === "prompt");
    expect(prompts.slice(0, 5).map((act) => act.words[2] || "")).toEqual([
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
    expect(session.chains.map((chain) => chain.words[0])).toContain("Another way");
    expect(session.chains.filter((chain) => String(chain.words[0]).startsWith("Fieldwork "))).toHaveLength(
      12,
    );
    const threads = session.acts.filter((act) => act.kind === "prompt");
    expect(threads.length).toBeGreaterThan(100);
    expect(threads.filter((act) => act.thread)).toHaveLength(130);
    expect(session.host.cost).toBe(0);
  } finally {
    await session.dispose();
    await removeDemoDirectories();
    if (pace === undefined) delete process.env.FURB_DEMO_PACE;
    else process.env.FURB_DEMO_PACE = pace;
  }
}, 20_000);

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

test("the rich scripted life has many real chains, nested threads, and long control-flow programs", async () => {
  const session = await demoSession({ watch: true });
  try {
    const rootThread = await seedRichDemo(session);
    const rootPrompt = session.acts.find((act) => act.id === rootThread);
    expect(rootPrompt?.kind).toBe("prompt");
    expect(rootPrompt?.value).toBe("The fieldwork report is ready.");
    const rootProgram = session.acts.find(
      (act) => act.kind === "rung" && act.by === rootThread && String(act.words[0]).includes("index_notes"),
    );
    expect(rootProgram?.done).toBe(true);
    expect(rootProgram?.on).toBe(session.engine.root);
    const source = String(rootProgram?.words[0]);
    expect(source.split(/\r?\n/).length).toBeGreaterThanOrEqual(50);
    for (const control of ["def classify_row", "while cursor", "for part in range", "try:", "await bash"])
      expect(source).toContain(control);
    expect(session.acts.some((act) => act.kind === "bash" && act.by === rootProgram?.id && act.done)).toBe(
      true,
    );
    expect(session.chains.filter((chain) => String(chain.words[0]).startsWith("Fieldwork "))).toHaveLength(
      12,
    );
    const prompts = session.acts.filter((act) => act.kind === "prompt");
    expect(prompts).toHaveLength(143);
    expect(prompts.filter((act) => act.thread)).toHaveLength(130);
    for (const chain of session.chains.filter((act) => String(act.words[0]).startsWith("Fieldwork "))) {
      await session.select(chain.id);
      expect(
        Object.values(session.program).some(
          (word) => word.includes("for item in batch") && word.includes("try"),
        ),
      ).toBe(true);
      expect(session.activity.filter((act) => act.kind === "rung").length).toBeGreaterThan(1);
    }
    expect(session.host.cost).toBe(0);
  } finally {
    await session.dispose();
    await removeDemoDirectories();
  }
}, 30_000);

import { expect, test } from "bun:test";
import { appendFile, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { until } from "../../bind/typescript/test/until.ts";
import { openEngine } from "../src/bridge.ts";
import { Session } from "../src/session.ts";
import { compactionFacts } from "./compaction-fixture.ts";
import { composing } from "./composing.ts";

test("selecting any continuation targets its latest head and leaves an ordinary branch separate", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-compact-"));
  const record = join(directory, "inert.jsonl");
  await writeFile(
    record,
    [
      JSON.stringify({ recorded: true }),
      ...compactionFacts(directory).map((fact) => JSON.stringify(fact)),
      "",
    ].join("\n"),
  );
  const { engine, host } = await openEngine({ demo: true, record, cwd: directory });
  const sent: [string, string | undefined][] = [];
  const words: string[] = [];
  const send = (kind: string, on?: string) => {
    sent.push([kind, on]);
    return `${kind}999`;
  };
  const replies = new Map<PropertyKey, unknown>([
    ["prompt", async (_shape: unknown, options?: { on?: string }) => send("prompt", options?.on)],
    [
      "rung",
      async (options?: { on?: string; word?: string }) => {
        words.push(options?.word ?? "");
        return send("rung", options?.on);
      },
    ],
    ["bash", async (_word: string, options?: { on?: string }) => send("bash", options?.on)],
    ["grant", async (options?: { on?: string }) => send("grant", options?.on)],
    ["cwd", async () => directory],
    ["result", async () => null],
    ["outcome", async () => ({ done: false, value: null })],
  ]);
  const session = new Session(
    new Proxy(engine, {
      get: (target, key) => replies.get(key) ?? Reflect.get(target, key),
    }),
    host,
    true,
  );
  await session.refresh();
  let composed = false;
  try {
    session.drafts["chain3:prompt"] = "A draft for the current head";
    for (const id of ["chain1", "chain2", "chain3"]) {
      await session.select(id);
      expect(session.selected).toBe("chain3");
      expect(session.drafts[session.draftKey]).toBe("A draft for the current head");
      expect(session.activity.map((act) => act.id)).toContain("rung1");
      expect(session.activity.map((act) => act.id)).toContain("rung5");
    }
    for (const id of ["chain1", "chain2", "chain3"]) {
      await session.select(id);
      await session.submit("Continue the work.");
      await session.command("/run answer = 42");
      await session.command("/bash printf test");
      await session.command("/grant 1");
      await session.command("/context 0.4");
    }
    expect(sent).toEqual(
      Array.from({ length: 3 }, (): [string, string][] => [
        ["prompt", "chain3"],
        ["rung", "chain3"],
        ["bash", "chain3"],
        ["grant", "chain3"],
        ["grant", "chain3"],
      ]).flat(),
    );
    session.thread = "prompt1";
    await session.submit("This is an explicit thread reply.");
    expect(sent.at(-1)).toEqual(["prompt", "prompt1"]);
    expect(session.actOf("rung1")?.on).toBe("chain1");
    expect(session.chains.map((chain) => chain.id)).toEqual(["chain3", "chain4", "chain7"]);
    await session.select("chain4");
    expect(session.selected).toBe("chain4");
    expect(session.activity.map((act) => act.id)).not.toContain("rung5");
    expect(session.compactions).toEqual([]);
    for (const id of ["chain5", "chain6", "chain7"]) {
      await session.select(id);
      expect(session.selected).toBe("chain7");
      expect(session.activity.map((act) => act.id)).toContain("rung7");
      expect(session.activity.map((act) => act.id)).not.toContain("rung1");
      await session.submit("Continue the nested work.");
      await session.command("/run nested = 2");
      await session.command("/bash printf nested");
      await session.command("/grant 2");
      expect(sent.slice(-4)).toEqual([
        ["prompt", "chain7"],
        ["rung", "chain7"],
        ["bash", "chain7"],
        ["grant", "chain7"],
      ]);
    }
    composed = true;
    await composing(
      async ({ app, frame, screen }) => {
        await screen.waitFor(async () => {
          await Bun.sleep(10);
          return app.parts.command("compact") !== undefined;
        });
        await session.select("chain1");
        const shown = await frame();
        expect(shown).toContain("# compacted in place");
        expect(shown).toContain("# compacted from chain1; continue on chain3");
        expect(shown).toContain('1 summary = "Keep the exact value');
        app.composer.setText("/compact Keep the full summary.");
        await app.submit();
        await until(session, () => words.at(-1) === 'compact(summary="Keep the full summary.")');
        expect(sent.at(-1)).toEqual(["rung", "chain3"]);
        app.chains();
        expect(await frame()).not.toContain("Middle");
        screen.mockInput.pressEscape();
        await frame();
        expect(app.composer.focused).toBe(true);
      },
      { width: 160, height: 48, kittyKeyboard: true, useMouse: true },
      session,
    );
  } finally {
    if (!composed) await session.dispose();
    await rm(directory, { recursive: true, force: true });
  }
});

test("a recorded App offers no disabled built-in, then loads a late enable without a transcript read", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-compact-disabled-"));
  const record = join(directory, "inert.jsonl");
  const facts = compactionFacts(directory).filter((fact) => fact[0] !== "enable");
  await writeFile(
    record,
    [JSON.stringify({ recorded: true }), ...facts.map((fact) => JSON.stringify(fact)), ""].join("\n"),
  );
  const { engine, host } = await openEngine({ demo: true, record, cwd: directory });
  const session = new Session(
    new Proxy(engine, {
      get: (target, key) =>
        key === "transcript"
          ? () => {
              throw new Error("No transcript read is needed.");
            }
          : Reflect.get(target, key),
    }),
    host,
    true,
  );
  await session.refresh();
  try {
    await composing(
      async ({ app, frame, screen }) => {
        await app.parts.load();
        expect(app.parts.command("compact")).toBeUndefined();
        await appendFile(
          record,
          `${JSON.stringify(["enable", "chain1", "extensions", "compaction", "def compact(): pass", ""])}\n`,
        );
        await until(session, () => Boolean(session.actOf("chain1")?.extensions?.includes("compaction")));
        await frame();
        await screen.waitFor(() => app.parts.command("compact") !== undefined);
        expect(app.parts.command("compact")?.detail).toBe("Keep the needed context and continue this chain");
      },
      { width: 80, height: 24 },
      session,
    );
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("each independent thread starts at one and appended steps keep their thread numbers", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-thread-numbers-"));
  const record = join(directory, "inert.jsonl");
  const facts = [
    ...compactionFacts(directory),
    ["prompt", "prompt2", "operator", "chain3", "str", "First independent thread", "codex:gpt-6.1-sol/high"],
    ["rung", "rung10", "prompt2", "chain3", "first = 10", "", ""],
    ["run", "run10", "chain3", "chain3", "rung10", "first = 10", ""],
    ["done", "rung10", "", null],
    ["rung", "rung11", "prompt2", "chain3", "appended = 11", "", ""],
    ["run", "run11", "chain3", "chain3", "rung11", "appended = 11", ""],
    ["done", "rung11", "", null],
    ["done", "prompt2", "", null],
    ["prompt", "prompt3", "operator", "chain3", "str", "Second independent thread", "codex:gpt-6.1-sol/high"],
    ["rung", "rung12", "prompt3", "chain3", "second = 12", "", ""],
    ["run", "run12", "chain3", "chain3", "rung12", "second = 12", ""],
    ["done", "rung12", "", null],
    ["done", "prompt3", "", null],
    ["rung", "rung13", "prompt1", "chain3", "compacted_append = 13", "", ""],
    ["run", "run13", "chain3", "chain3", "rung13", "compacted_append = 13", ""],
    ["done", "rung13", "", null],
  ];
  await writeFile(
    record,
    [JSON.stringify({ recorded: true }), ...facts.map((fact) => JSON.stringify(fact)), ""].join("\n"),
  );
  const { engine, host } = await openEngine({ demo: true, record, cwd: directory });
  const session = new Session(engine, host, true);
  await session.refresh();
  try {
    await session.select("chain1");
    session.folds = { "thread:prompt1": false, "thread:prompt2": false, "thread:prompt3": false };
    await composing(
      async ({ frame }) => {
        const shown = await frame();
        expect(shown).toMatch(/\b1 first = 10/);
        expect(shown).toMatch(/\b2 appended = 11/);
        expect(shown).toMatch(/\b2 compacted_append = 13/);
        expect(shown).toMatch(/\b1 second = 12/);
      },
      { width: 160, height: 60 },
      session,
    );
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("a compacted feed hears appends and a host reset replaces equal-count rows and built-ins", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-compact-reset-"));
  const record = join(directory, "first.jsonl");
  const facts = compactionFacts(directory);
  const save = (path: string, entries: unknown[]) =>
    writeFile(
      path,
      [JSON.stringify({ recorded: true }), ...entries.map((fact) => JSON.stringify(fact)), ""].join("\n"),
    );
  await save(record, facts);
  const first = await openEngine({ demo: true, record, cwd: directory });
  let snapshot = first.host.snapshot.bind(first.host);
  const host = new Proxy(first.host, {
    get: (target, key) => {
      if (key === "snapshot") return (...args: Parameters<typeof snapshot>) => snapshot(...args);
      const value = Reflect.get(target, key);
      return typeof value === "function" ? value.bind(target) : value;
    },
  });
  const session = new Session(first.engine, host, true);
  await session.refresh();
  let second: Awaited<ReturnType<typeof openEngine>> | undefined;
  let composed = false;
  try {
    await session.select("chain1");
    const before = session.activity;
    const heads = session.chains;
    expect(session.chains).toBe(heads);
    facts.push(["prompt", "prompt20", "rung1", "chain1", "str", "Keep this old-link question", "operator"]);
    await appendFile(record, `${JSON.stringify(facts.at(-1))}\n`);
    await until(session, () => Boolean(session.actOf("prompt20")));
    const prompts = session.host.prompts;
    Object.defineProperty(session.host, "prompts", {
      configurable: true,
      get: () =>
        new Map([
          ...prompts,
          ["prompt20", { id: "prompt20", message: "Keep this old-link question", shape: "str" }],
        ]),
    });
    expect(session.chainStatus("chain3")).toBe("blocked");
    expect(session.operatorPrompt?.id).toBe("prompt20");
    expect(session.operatorPrompt?.chain).toBe("chain3");
    expect(session.open.some((act) => act.id === "prompt20")).toBe(true);
    expect(session.actOf("prompt20")?.on).toBe("chain1");
    Object.defineProperty(session.host, "prompts", { configurable: true, get: () => prompts });
    const appended = [
      ["rung", "rung10", "operator", "chain3", "after_cut = 10", "", ""],
      ["run", "run10", "chain3", "chain3", "rung10", "after_cut = 10", ""],
      ["done", "rung10", "", null],
    ];
    await appendFile(record, appended.map((fact) => JSON.stringify(fact)).join("\n") + "\n");
    await until(session, () => Boolean(session.actOf("rung10")));
    expect(session.activity).toBe(before);
    expect(session.chains).toBe(heads);
    expect(session.activity.find((act) => act.id === "rung10")?.word).toBe("after_cut = 10");
    expect(session.actOf("rung1")?.on).toBe("chain1");
    const unrelated = ["rung", "rung11", "operator", "chain4", "other_append = 11", "", ""];
    await appendFile(record, JSON.stringify(unrelated) + "\n");
    await until(session, () => Boolean(session.actOf("rung11")));
    expect(session.activity).toBe(before);
    expect(session.chains).toBe(heads);
    expect(session.activity.some((act) => act.id === "rung11")).toBe(false);
    const replacement = join(directory, "second.jsonl");
    await save(
      replacement,
      [...facts, ...appended, unrelated].map((fact) => {
        if (fact[0] === "enable" || fact[0] === "prefix") return ["tell", fact[1], "operator", []];
        if (fact[0] === "rung" && String(fact[4]).startsWith("restore("))
          return [...fact.slice(0, 4), "ordinary = 1", ...fact.slice(5)];
        if (fact[0] === "run" && String(fact[5]).startsWith("restore("))
          return [...fact.slice(0, 5), "ordinary = 1", ...fact.slice(6)];
        return fact;
      }),
    );
    second = await openEngine({ demo: true, record: replacement, cwd: directory });
    snapshot = second.host.snapshot.bind(second.host);
    composed = true;
    await composing(
      async ({ app, frame, screen }) => {
        await app.parts.load();
        expect(app.parts.command("compact")).toBeDefined();
        first.host.emit("reset");
        await session.refresh();
        await frame();
        await screen.waitFor(() => app.parts.command("compact") === undefined);
        expect(session.selected).toBe("chain3");
        expect(session.headOf("chain1")).toBe("chain1");
        expect(session.activity.some((act) => act.id === "rung1")).toBe(false);
        expect(session.actOf("chain1")?.extensions).toBeUndefined();
      },
      { width: 80, height: 24 },
      session,
    );
  } finally {
    if (!composed) await session.dispose();
    if (second) await second.host.dispose();
    await rm(directory, { recursive: true, force: true });
  }
});

test("ordinary chain lists stay stable across an append and a chain phase update", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-ordinary-append-"));
  const record = join(directory, "inert.jsonl");
  await writeFile(
    record,
    [
      JSON.stringify({ recorded: true }),
      JSON.stringify(["chain", "chain1", "operator", "", "First", ""]),
      JSON.stringify(["chain", "chain2", "operator", "", "Second", ""]),
      "",
    ].join("\n"),
  );
  const { engine, host } = await openEngine({ demo: true, record, cwd: directory });
  const session = new Session(engine, host, true);
  try {
    await session.refresh();
    const chains = session.chains;
    await appendFile(
      record,
      `${JSON.stringify(["rung", "rung1", "operator", "chain2", "answer = 42", "", ""])}\n`,
    );
    await until(session, () => Boolean(session.actOf("rung1")));
    expect(session.chains).toBe(chains);
    expect(session.chains.map((chain) => chain.id)).toEqual(["chain1", "chain2"]);
    await appendFile(record, `${JSON.stringify(["pause", "chain2", "operator"])}\n`);
    await until(session, () => session.actOf("chain2")?.paused === true);
    expect(session.chains).toBe(chains);
    expect(session.chains[1]?.paused).toBe(true);
    expect(session.headOf("chain2")).toBe("chain2");
  } finally {
    await session.dispose();
    await rm(directory, { recursive: true, force: true });
  }
});

import { expect, test } from "bun:test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { type Engine, Session } from "@furb/engine";
import { until } from "../../bind/typescript/test/until.ts";
import { defaultModel, hostModels } from "../src/models.ts";
import { Snapshots } from "../src/snapshots.ts";

test("idle snapshots add no facts or sandbox calls as the act table grows, and streamed output needs no peek", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-snapshot-"));
  const session = new Session({ cwd });
  try {
    const engine = session.open();
    const calls: string[] = [];
    const traced = new Proxy(engine, {
      get(target, key) {
        const value = Reflect.get(target, key, target);
        return typeof value === "function"
          ? (...args: unknown[]) => {
              calls.push(String(key));
              return Reflect.apply(value, target, args);
            }
          : value;
      },
    }) as Engine;
    const snapshots = new Snapshots(traced, session);
    snapshots.take(engine.root);
    for (let index = 0; index < 100; index++) engine.wait({ seconds: 60, on: engine.root });
    await Promise.resolve();
    expect(snapshots.take(engine.root).acts).toHaveLength(101);
    await Promise.resolve();
    const before = [...calls],
      facts = session.facts.length;
    for (let index = 0; index < 100; index++) {
      snapshots.take(engine.root);
      await Promise.resolve();
    }
    expect(calls).toEqual(before);
    expect(session.facts.length).toBe(facts);
    expect(calls.filter((name) => ["get", "outcome", "peek"].includes(name))).toEqual([]);

    const command = engine.bash('printf first; read line; printf "$line"; read hold', {
      fed: true,
      on: engine.root,
    }).id;
    const output = () =>
      (session.activity.acts.get(command)?.value as { stdout?: { content: string } })?.stdout?.content;
    await until(session, () => output() === "first");
    snapshots.take(engine.root);
    await Promise.resolve();
    const streamed = [...calls];
    engine.write({ path: `${command}/stdin`, content: "second\n" }, { on: engine.root });
    await until(session, () => output() === "firstsecond");
    const snapshot = snapshots.take(engine.root);
    expect(snapshot.acts.find((act) => act.id === command)?.value).toMatchObject({
      stdout: { content: "firstsecond" },
    });
    expect(calls).toEqual(streamed);
  } finally {
    await session.dispose();
    await rm(cwd, { recursive: true, force: true });
  }
}, 30000);

test("fact-derived act state matches native outcomes, controls, and rung results while view queries follow actual changes", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-state-"));
  // The scripted answer replaces the request of the model the provider offers, so the prompt goes to a model.
  const host = hostModels();
  const session = new Session({
    cwd,
    models: host.models,
    model: defaultModel,
    answer: async () => ["assistant", 'close("answered")', null, null],
  });
  try {
    const engine = session.open();
    const snapshots = new Snapshots(engine, session);
    snapshots.take(engine.root);
    const one = engine.wait({ seconds: 60, on: engine.root }).id,
      two = engine.wait({ seconds: 60, on: engine.root }).id;
    engine.pause(engine.root);
    expect(session.isPaused(one)).toBe(true);
    engine.wake(one);
    expect(session.isPaused(one)).toBe(false);
    expect(session.isPaused(two)).toBe(true);
    engine.wake(engine.root);
    engine.close(null, { id: one });
    engine.cancel(two);
    await engine.rung({ word: "answer = 17", on: engine.root });
    await Promise.resolve();
    let view = snapshots.take(engine.root);
    expect(Object.values(view.program)).toContain("answer = 17");
    expect(view.turns.map(([, python]) => python).join("\n")).toContain("answer = 17");
    // A cd the operator asks is of the moment, and the engine keeps no answer of it, so the cd is a rung, as /cd is.
    await engine.rung({ word: 'cd("another-directory")', on: engine.root });
    await Promise.resolve();
    expect(snapshots.take(engine.root).directory).toBe("another-directory");
    const failed = engine.rung({ word: "x = 1 / 0", on: engine.root });
    await failed.then(
      () => {},
      () => {},
    );
    const refused = engine.rung({ word: "this is invalid python !!!", on: engine.root });
    await refused.then(
      () => {},
      () => {},
    );
    await engine.prompt("str", { message: "Answer this", on: engine.root });
    await Promise.resolve();
    view = snapshots.take(engine.root);
    for (const row of view.acts) {
      expect(row.done).toBe(engine.outcome(row.id).done);
      if (row.done) expect(row.value).toEqual(engine.outcome(row.id).value);
    }
    expect(view.acts.find((act) => act.id === failed.id)?.run).toEqual({
      status: "failed",
      reason: "ZeroDivisionError: division by zero",
    });
    expect(view.acts.find((act) => act.id === refused.id)?.run?.reason).toContain("line 1");
    expect(
      view.acts
        .filter((act) => act.kind === "rung" && /^prompt\d+$/.test(act.by))
        .every((act) => act.run?.status === "done"),
    ).toBe(true);
  } finally {
    await session.dispose();
    host.dispose();
    await rm(cwd, { recursive: true, force: true });
  }
}, 30000);

test("an act of a kind an extension defines joins the act table, and what it tells reaches the view", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-extension-"));
  const session = new Session({ cwd });
  try {
    const engine = session.open();
    const snapshots = new Snapshots(engine, session);
    snapshots.take(engine.root);
    await engine.rung({
      word: [
        "def noted(name):",
        '  yield ("tell", name, [f"#{name} said", "# hello"])',
        '  yield ("done", name, "noted")',
        'note = act("note", "", noted)',
      ].join("\n"),
      on: engine.root,
    });
    await until(session, () => [...session.activity.acts.values()].some((act) => act.kind === "note"));
    const view = snapshots.take(engine.root);
    const note = view.acts.find((act) => act.kind === "note");
    expect(note).toMatchObject({ on: engine.root, done: true, value: "noted" });
    expect(view.turns.map(([, python]) => python).join("\n")).toContain("# hello");
    expect(view.turns).toEqual(engine.turns({ on: engine.root }));
  } finally {
    await session.dispose();
    await rm(cwd, { recursive: true, force: true });
  }
}, 30000);

test("a take after a change of the chain asks its turns by one question", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-rendering-"));
  const session = new Session({ cwd });
  try {
    const engine = session.open();
    const calls: string[] = [];
    const traced = new Proxy(engine, {
      get(target, key) {
        const value = Reflect.get(target, key, target);
        return typeof value === "function"
          ? (...args: unknown[]) => {
              calls.push(String(key));
              return Reflect.apply(value, target, args);
            }
          : value;
      },
    }) as Engine;
    const snapshots = new Snapshots(traced, session);
    snapshots.take(engine.root);
    await engine.result(engine.rung({ word: "changed = 1", on: engine.root }).id);
    calls.length = 0;
    const snapshot = snapshots.take(engine.root);
    expect(calls.filter((name) => name === "turns")).toEqual(["turns"]);
    expect(snapshot.turns.map(([, python]) => python).join("\n")).toContain("changed = 1");
    expect(snapshot.turns).toEqual(engine.turns({ on: engine.root }));
  } finally {
    await session.dispose();
    await rm(cwd, { recursive: true, force: true });
  }
}, 30000);

test("a take carries the acts that changed after the count it is given, and every act after the table is derived again", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-delta-"));
  const session = new Session({ cwd });
  try {
    const engine = session.open();
    const snapshots = new Snapshots(engine, session);
    const first = snapshots.take(engine.root);
    expect(first.acts.map((act) => act.id)).toEqual([...session.activity.acts.keys()]);
    const waiting = engine.wait({ seconds: 60, on: engine.root }).id;
    await Promise.resolve();
    const second = snapshots.take(engine.root, first.count);
    expect(second.acts.map((act) => act.id)).toEqual([waiting]);
    engine.cancel(waiting);
    await until(session, () => session.activity.acts.get(waiting)?.done === true);
    const third = snapshots.take(engine.root, second.count);
    expect(third.acts.map((act) => [act.id, act.done])).toEqual([[waiting, true]]);
    expect(snapshots.take(engine.root, third.count).acts).toEqual([]);
    await engine.rung({
      word: ["def noted(name):", '  yield ("done", name, "noted")', 'note = act("note", "", noted)'].join(
        "\n",
      ),
      on: engine.root,
    });
    await until(session, () => [...session.activity.acts.values()].some((act) => act.kind === "note"));
    const fourth = snapshots.take(engine.root, third.count);
    expect(fourth.acts.map((act) => act.kind)).toEqual(["rung", "note"]);
  } finally {
    await session.dispose();
    await rm(cwd, { recursive: true, force: true });
  }
}, 30000);

test("a new standing drops the roster, the directory and the actor that the view of a chain read", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-standing-"));
  const host = hostModels();
  const session = new Session({ cwd, models: host.models, model: defaultModel });
  try {
    const engine = session.open();
    const snapshots = new Snapshots(engine, session);
    const before = snapshots.take(engine.root);
    expect([before.roster.map(([name]) => name), before.directory, before.actor]).toEqual([
      [defaultModel, "operator"],
      cwd,
      `${defaultModel}/low`,
    ]);
    // A stand that an ear of the word takes is answered by whoever says its done, here the operator.
    await engine.rung({
      word: 'def takes(id):\n  yield "started", id\n  while True:\n    yield\nasked = act("stand", "", takes)',
      on: engine.root,
    });
    const asked = String(engine.inspect("asked", engine.root).value);
    engine.say("done", asked, [[[["operator", [], 200000]], join(cwd, "elsewhere"), "operator"]]);
    await Promise.resolve();
    const after = snapshots.take(engine.root);
    expect([after.roster, after.directory, after.actor]).toEqual([
      [["operator", [], 200000]],
      join(cwd, "elsewhere"),
      "operator",
    ]);
  } finally {
    await session.dispose();
    await rm(cwd, { recursive: true, force: true });
  }
});

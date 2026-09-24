import { expect, test } from "bun:test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { type Life, World } from "@furb/engine";
import { until } from "../../bind/typescript/test/until.ts";
import { defaultModel, hostModels } from "../src/models.ts";
import { Snapshots } from "../src/snapshots.ts";

test("idle snapshots add no facts or sandbox calls as the act table grows, and streamed output needs no peek", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-snapshot-"));
  const world = new World({ cwd });
  try {
    const life = world.open();
    const calls: string[] = [];
    const traced = new Proxy(life, {
      get(target, key) {
        const value = Reflect.get(target, key, target);
        return typeof value === "function"
          ? (...args: unknown[]) => {
              calls.push(String(key));
              return Reflect.apply(value, target, args);
            }
          : value;
      },
    }) as Life;
    const snapshots = new Snapshots(traced, world);
    snapshots.take(life.root);
    for (let index = 0; index < 100; index++) life.wait(60);
    await Promise.resolve();
    expect(snapshots.take(life.root).acts).toHaveLength(101);
    await Promise.resolve();
    const before = [...calls],
      facts = world.facts.length;
    for (let index = 0; index < 100; index++) {
      snapshots.take(life.root);
      await Promise.resolve();
    }
    expect(calls).toEqual(before);
    expect(world.facts.length).toBe(facts);
    expect(calls.filter((name) => ["get", "outcome", "scope", "peek"].includes(name))).toEqual([]);

    const command = life.call<string>("bash", ['printf first; read line; printf "$line"; read hold'], {
      fed: true,
      on: life.root,
    });
    const output = () =>
      (world.activity.acts.get(command)?.value as { stdout?: { content: string } })?.stdout?.content;
    await until(world, () => output() === "first");
    snapshots.take(life.root);
    await Promise.resolve();
    const streamed = [...calls];
    life.call("ask", ["write", life.root, `${command}/stdin`, "second\n"], {});
    await until(world, () => output() === "firstsecond");
    const snapshot = snapshots.take(life.root);
    expect(snapshot.acts.find((act) => act.id === command)?.value).toMatchObject({
      stdout: { content: "firstsecond" },
    });
    expect(calls).toEqual(streamed);
  } finally {
    await world.dispose();
    await rm(cwd, { recursive: true, force: true });
  }
}, 30000);

test("fact-derived act state matches native outcomes, controls, and rung results while view queries follow actual changes", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-state-"));
  // The scripted answer replaces the request of the model the World offers, so the prompt goes to a model.
  const host = hostModels();
  const world = new World({
    cwd,
    models: host.models,
    model: defaultModel,
    answer: async () => ["assistant", 'close("answered")', null, null],
  });
  try {
    const life = world.open();
    const snapshots = new Snapshots(life, world);
    snapshots.take(life.root);
    const one = life.wait(60).id,
      two = life.wait(60).id;
    life.pause(life.root);
    expect(world.isPaused(one)).toBe(true);
    life.wake(one);
    expect(world.isPaused(one)).toBe(false);
    expect(world.isPaused(two)).toBe(true);
    life.wake(life.root);
    life.close(null, one);
    life.cancel(two);
    await life.rung("answer = 17");
    await Promise.resolve();
    let view = snapshots.take(life.root);
    expect(Object.values(view.program)).toContain("answer = 17");
    expect(view.turns.map(([, python]) => python).join("\n")).toContain("answer = 17");
    // A cd the operator asks is of the moment, and the World keeps no answer of it, so the cd is a rung, as /cd is.
    await life.rung('cd("another-directory")');
    await Promise.resolve();
    expect(snapshots.take(life.root).directory).toBe("another-directory");
    const failed = life.rung("x = 1 / 0");
    await failed.then(
      () => {},
      () => {},
    );
    const refused = life.rung("this is invalid python !!!");
    await refused.then(
      () => {},
      () => {},
    );
    await life.prompt("str", "Answer this");
    await Promise.resolve();
    view = snapshots.take(life.root);
    for (const row of view.acts) {
      expect(row.done).toBe(life.outcome(row.id).done);
      if (row.done) expect(row.value).toEqual(life.outcome(row.id).value);
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
    await world.dispose();
    host.dispose();
    await rm(cwd, { recursive: true, force: true });
  }
}, 30000);

test("an act of a kind an extension defines joins the act table, and what it tells reaches the view", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-extension-"));
  const world = new World({ cwd });
  try {
    const life = world.open();
    const snapshots = new Snapshots(life, world);
    snapshots.take(life.root);
    await life.rung(
      [
        "def noted(name):",
        '  yield ("tell", name, [f"#{name} said", "# hello"])',
        '  yield ("done", name, "noted")',
        'note = act("note", "", noted)',
      ].join("\n"),
    );
    await until(world, () => [...world.activity.acts.values()].some((act) => act.kind === "note"));
    const view = snapshots.take(life.root);
    const note = view.acts.find((act) => act.kind === "note");
    expect(note).toMatchObject({ on: life.root, done: true, value: "noted" });
    expect(view.turns.map(([, python]) => python).join("\n")).toContain("# hello");
    expect(view.turns).toEqual(life.turns(life.root));
  } finally {
    await world.dispose();
    await rm(cwd, { recursive: true, force: true });
  }
}, 30000);

test("a take after a change of the chain asks its turns by one question", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-rendering-"));
  const world = new World({ cwd });
  try {
    const life = world.open();
    const calls: string[] = [];
    const traced = new Proxy(life, {
      get(target, key) {
        const value = Reflect.get(target, key, target);
        return typeof value === "function"
          ? (...args: unknown[]) => {
              calls.push(String(key));
              return Reflect.apply(value, target, args);
            }
          : value;
      },
    }) as Life;
    const snapshots = new Snapshots(traced, world);
    snapshots.take(life.root);
    await life.result(life.rung("changed = 1").id);
    calls.length = 0;
    const snapshot = snapshots.take(life.root);
    expect(calls.filter((name) => name === "turns")).toEqual(["turns"]);
    expect(snapshot.turns.map(([, python]) => python).join("\n")).toContain("changed = 1");
    expect(snapshot.turns).toEqual(life.turns(life.root));
  } finally {
    await world.dispose();
    await rm(cwd, { recursive: true, force: true });
  }
}, 30000);

test("a take carries the acts that changed after the count it is given, and every act after the table is derived again", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-delta-"));
  const world = new World({ cwd });
  try {
    const life = world.open();
    const snapshots = new Snapshots(life, world);
    const first = snapshots.take(life.root);
    expect(first.acts.map((act) => act.id)).toEqual([...world.activity.acts.keys()]);
    const waiting = life.wait(60).id;
    await Promise.resolve();
    const second = snapshots.take(life.root, first.count);
    expect([second.whole, second.acts.map((act) => act.id)]).toEqual([false, [waiting]]);
    life.cancel(waiting);
    await until(world, () => world.activity.acts.get(waiting)?.done === true);
    const third = snapshots.take(life.root, second.count);
    expect(third.acts.map((act) => [act.id, act.done])).toEqual([[waiting, true]]);
    expect(snapshots.take(life.root, third.count).acts).toEqual([]);
    await life.rung(
      ["def noted(name):", '  yield ("done", name, "noted")', 'note = act("note", "", noted)'].join("\n"),
    );
    await until(world, () => [...world.activity.acts.values()].some((act) => act.kind === "note"));
    const fourth = snapshots.take(life.root, third.count);
    expect(fourth.whole).toBe(true);
    expect(fourth.acts.map((act) => act.id)).toEqual([...world.activity.acts.keys()]);
  } finally {
    await world.dispose();
    await rm(cwd, { recursive: true, force: true });
  }
}, 30000);

test("a stood about a chain drops the roster, the directory and the actor that its view read", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-stood-"));
  const host = hostModels();
  const world = new World({ cwd, models: host.models, model: defaultModel });
  try {
    const life = world.open();
    const snapshots = new Snapshots(life, world);
    const before = snapshots.take(life.root);
    expect([before.roster.map(([name]) => name), before.directory, before.actor]).toEqual([
      [defaultModel, "operator"],
      cwd,
      `${defaultModel}/low`,
    ]);
    life.send("stood", life.root, [[[["operator", [], 200000]], join(cwd, "elsewhere"), "operator"]]);
    await Promise.resolve();
    const after = snapshots.take(life.root);
    expect([after.roster, after.directory, after.actor]).toEqual([
      [["operator", [], 200000]],
      join(cwd, "elsewhere"),
      "operator",
    ]);
  } finally {
    await world.dispose();
    await rm(cwd, { recursive: true, force: true });
  }
});

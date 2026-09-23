import { expect, test } from "bun:test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { type Life, World } from "@furb/engine";
import { until } from "../../bind/typescript/test/until.ts";
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

    const command = life.bash('printf first; read line; printf "$line"; read hold', { fed: true }).id;
    const output = () =>
      (world.activity.acts.get(command)?.value as { stdout?: { content: string } })?.stdout?.content;
    await until(world, () => output() === "first");
    snapshots.take(life.root);
    await Promise.resolve();
    const streamed = [...calls];
    life.write({ path: `${command}/stdin`, content: "second\n" });
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
  const world = new World({ cwd, answer: async () => ["assistant", ['close("answered")'], null, null] });
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
    expect(view.rendered.join("\n")).toContain("answer = 17");
    life.cd("another-directory");
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
        .filter((act) => act.kind === "rung" && act.by.startsWith("prompt://"))
        .every((act) => act.run?.status === "done"),
    ).toBe(true);
  } finally {
    await world.dispose();
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
        '  yield ("tell", name, [("note", [("id", name)], "hello")])',
        '  yield ("done", name, "noted")',
        'note = act("note", "", noted)',
      ].join("\n"),
    );
    await until(world, () => [...world.activity.acts.values()].some((act) => act.kind === "note"));
    const view = snapshots.take(life.root);
    const note = view.acts.find((act) => act.kind === "note");
    expect(note).toMatchObject({ on: life.root, done: true, value: "noted" });
    expect(view.rendered.join("\n")).toContain("hello");
    expect(view.rendered).toEqual(life.rendered(life.root));
  } finally {
    await world.dispose();
    await rm(cwd, { recursive: true, force: true });
  }
}, 30000);

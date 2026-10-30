import { expect, test } from "bun:test";
import { mkdir, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { type Engine, Session, type SessionOptions } from "furb";
import { until } from "../../bind/typescript/test/until.ts";
import { Snapshots } from "../src/snapshots.ts";

/** An engine that names each of its methods in a list as they are called, and then calls them. */
function traced(engine: Engine, calls: string[]): Engine {
  return new Proxy(engine, {
    get(target, key) {
      const value = Reflect.get(target, key, target);
      return typeof value === "function"
        ? (...args: unknown[]) => {
            calls.push(String(key));
            return Reflect.apply(value, target, args);
          }
        : value;
    },
  });
}

/** A life in a temporary directory of its own, and the snapshots of its engine, which name in `calls` each method of
 * the engine that they call. A test uses them, and all of it ends after the test. */
async function withLife(
  use: (context: {
    session: Session;
    engine: Engine;
    snapshots: Snapshots;
    calls: string[];
    cwd: string;
  }) => Promise<void>,
  options: SessionOptions = {},
): Promise<void> {
  const cwd = await mkdtemp(join(tmpdir(), "furb-snapshots-"));
  const session = new Session({ cwd, ...options });
  try {
    const engine = session.open();
    const calls: string[] = [];
    await use({ session, engine, snapshots: new Snapshots(traced(engine, calls), session), calls, cwd });
  } finally {
    await session.dispose();
    await rm(cwd, { recursive: true, force: true });
  }
}

test("idle snapshots add no facts or sandbox calls as the act table grows, and streamed output needs no peek", () =>
  withLife(
    async ({ session, engine, snapshots, calls }) => {
      // The chain holds itself, the rung of the read of the doctrine that the World plays on it and that read, then
      // the rung of the stand and the cd that the chain plays on itself.
      expect(snapshots.take(engine.root).acts.map((act) => act.kind)).toEqual([
        "chain",
        "rung",
        "read",
        "rung",
      ]);
      for (let index = 0; index < 100; index++) engine.wait([], { seconds: 60, on: engine.root });
      await Promise.resolve();
      expect(snapshots.take(engine.root).acts).toHaveLength(104);
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
        stdin: null,
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
    },
    // The acts of an extension would stand in the act table beside those the test counts.
    { extensions: false },
  ));

test("a take reads the program, the turns and the directory of a chain again once a rung changes them", () =>
  withLife(async ({ engine, snapshots, calls, cwd }) => {
    snapshots.take(engine.root);
    await engine.rung({ word: "answer = 17", on: engine.root });
    await Promise.resolve();
    calls.length = 0;
    const view = snapshots.take(engine.root);
    expect(calls.filter((name) => name === "turns")).toEqual(["turns"]);
    expect(Object.values(view.program?.words ?? {})).toContain("answer = 17");
    expect(view.turns.list.map(([, python]) => python).join("\n")).toContain("answer = 17");
    expect(view.turns).toEqual({ from: 0, list: engine.turns({ on: engine.root }) });
    // A cd the operator asks is of the moment, and the engine keeps no answer of it, so the cd is a rung, as /cd is.
    await mkdir(join(cwd, "sub"));
    await engine.rung({ word: 'cd("sub")', on: engine.root });
    await Promise.resolve();
    expect(snapshots.take(engine.root).directory).toBe(join(cwd, "sub"));
  }));

test("a take for the host that holds the last snapshot carries only what changed of the chain since then", () =>
  withLife(async ({ engine, snapshots }) => {
    const first = snapshots.take(engine.root);
    await engine.rung({ word: "answer = 17", on: engine.root });
    await Promise.resolve();
    const second = snapshots.take(engine.root, first.count, first.mark);
    const turns = engine.turns({ on: engine.root });
    expect(second.roster).toBeUndefined();
    expect(second.program?.whole).toBe(false);
    expect(Object.values(second.program?.words ?? {})).toContain("answer = 17");
    expect({ ...first.program?.words, ...second.program?.words }).toEqual(
      engine.program({ on: engine.root }) as Record<string, string>,
    );
    expect([...first.turns.list.slice(0, second.turns.from), ...second.turns.list]).toEqual(turns);
    const third = snapshots.take(engine.root, second.count, second.mark);
    expect(third.program).toBeUndefined();
    expect(third.turns).toEqual({ from: turns.length, list: [] });
    // A host that does not hold the last snapshot takes the chain whole.
    const again = snapshots.take(engine.root, second.count, first.mark);
    expect(again.roster).toBeDefined();
    expect(again.program?.whole).toBe(true);
    expect(again.turns).toEqual({ from: 0, list: turns });
  }));

test("an act of a kind an extension defines joins the act table, and what it tells reaches the view", () =>
  withLife(async ({ session, engine, snapshots }) => {
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
    expect(view.turns.list.map(([, python]) => python).join("\n")).toContain("# hello");
    expect(view.turns).toEqual({ from: 0, list: engine.turns({ on: engine.root }) });
  }));

test("a take carries the acts that changed after the count it is given, and every act after the table is derived again", () =>
  withLife(async ({ session, engine, snapshots }) => {
    const first = snapshots.take(engine.root);
    expect(first.acts.map((act) => act.id)).toEqual([...session.activity.acts.keys()]);
    const waiting = engine.wait([], { seconds: 60, on: engine.root }).id;
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
  }));

test("a stand drops the roster, the directory and the actor that the view of a chain read, so the view reads them again", () =>
  withLife(
    async ({ engine, snapshots, cwd }) => {
      const before = snapshots.take(engine.root);
      expect([before.roster?.map(([name]) => name), before.directory, before.actor]).toEqual([
        ["claude-cli:sonnet", "operator"],
        cwd,
        "claude-cli:sonnet/high",
      ]);
      // A host that holds the last snapshot takes the roster again only once the view dropped it.
      const quiet = snapshots.take(engine.root, 0, before.mark);
      expect(quiet.roster).toBeUndefined();
      // A stand that an ear of the word takes is answered by whoever says its done, here the operator. Its standing
      // names no doctrine, so the chain plays no stand of its own on it, and its directory stays where its cd moved it.
      await engine.rung({
        word: 'def takes(id):\n  yield "started", id\n  while True:\n    yield\nasked = act("stand", "", takes)',
        on: engine.root,
      });
      const asked = String(engine.inspect("asked", engine.root).value);
      engine.say("done", asked, [[[["operator", [], 200000]], join(cwd, "elsewhere"), "operator"]]);
      await Promise.resolve();
      const after = snapshots.take(engine.root, 0, quiet.mark);
      expect([after.roster, after.directory, after.actor]).toEqual([
        [["operator", [], 200000]],
        cwd,
        "operator",
      ]);
    },

    // The roster is named, so the catalog offers no model of the claude command line that the machine holds.
    { model: "claude-cli:sonnet", roster: ["claude-cli:sonnet"] },
  ));

test("a row of a read or a write carries the count of the lines of its text and leaves the text out, a write the lines that its change added and removed, and an act that the life ran its time", () =>
  withLife(async ({ engine, snapshots }) => {
    const before = snapshots.take(engine.root).count;
    engine.write({ path: "notes.md", content: "one\ntwo\n" }, { on: engine.root });
    engine.read("notes.md", { on: engine.root });
    const command = engine.bash("sleep 0.2", { on: engine.root }).id;
    await engine.result(command);
    engine.write({ path: "notes.md", content: "one\nthree\nfour\n" }, { on: engine.root });
    await Promise.resolve();
    const rows = snapshots.take(engine.root, before).acts;
    expect(rows.filter((act) => act.kind === "write").map((act) => act.lines)).toEqual([
      { added: 2, removed: 0 },
      { added: 2, removed: 1 },
    ]);
    expect(rows.find((act) => act.id === command)?.took).toBeGreaterThanOrEqual(150);
    const read = rows.find((act) => act.kind === "read");
    expect([read?.count, (read?.value as { content?: string }).content]).toEqual([2, ""]);
  }));

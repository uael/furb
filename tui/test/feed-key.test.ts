import { expect, test } from "bun:test";
import { until } from "../../bind/typescript/test/until.ts";
import type { ActRow } from "../src/session.ts";
import { composing } from "./composing.ts";

test("a late change of any field of an act draws its step again, and a snapshot that changes none of its acts leaves its lines as they were", () =>
  composing(async ({ session, app, frame }) => {
    const id = await session.engine.rung({
      word: 'write(Text("notes.md", "one\\ntwo\\n"))',
      on: session.engine.root,
    });
    await session.engine.result(id);
    await session.refresh();
    session.folds[id] = false;
    await until(session, () => session.acts.some((act) => act.kind === "write" && act.lines));
    const target = session.acts.find((act) => act.kind === "write")?.id ?? "";
    expect(target).not.toBe("");

    // A snapshot gives the acts that changed, and the test gives the write a change of its own in place of the ones
    // that the life would make: the field that it names, as it comes late.
    let change: Partial<ActRow> | undefined;
    const real = session.host.snapshot.bind(session.host);
    Object.assign(session.host, {
      snapshot: async (...asked: Parameters<typeof real>) => {
        const given = await real(...asked);
        const rest = given.acts.filter((act) => act.id !== target);
        const now = session.acts.find((act) => act.id === target);
        return { ...given, acts: change && now ? [...rest, { ...now, ...change }] : rest };
      },
    });
    const lines = () => app.programs.get(id)?.lines;
    await frame();
    let before = lines();
    expect(before).toBeDefined();

    // A snapshot that changes no act of the step leaves the lines that were made.
    await session.refresh();
    await frame();
    expect(lines()).toBe(before);

    const changes: Partial<ActRow>[] = [
      { count: 7 },
      { lines: { added: 9, removed: 4 } },
      { output: 5 },
      { value: "late" },
      { paused: true },
      { pause: { by: "operator" } },
      { cost: 0.5 },
      { took: 12 },
      { words: ["late"] },
      { unfinished: true },
      { asking: true },
      { ended: 99 },
      { done: false },
    ];
    for (const one of changes) {
      change = one;
      await session.refresh();
      await frame();
      expect(lines(), Object.keys(one).join()).not.toBe(before);
      before = lines();
      change = undefined;
      await session.refresh();
      await frame();
      expect(lines(), `${Object.keys(one).join()} kept`).toBe(before);
    }
  }));

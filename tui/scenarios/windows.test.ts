// Several windows on one life: a second window on the life of a first one is a second view of it, and neither ends,
// takes over or refuses the other, and a click on a workspace never lands on a session that another window holds.
import { expect } from "bun:test";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { scenario, test, until, Window } from "./harness.ts";

test("a second window on the life of a first one shows the same life, and the first goes on after it closes", () =>
  scenario([{ cue: "Scenario: say one.", word: 'close("One said.")' }], async (first, yard) => {
    // The second window is a process of its own, with its own connection to the daemon, as a second terminal is.
    const second = Bun.spawn(
      [
        process.execPath,
        join(import.meta.dir, "second.ts"),
        yard.root,
        yard.work,
        yard.model,
        yard.record,
        yard.cache,
        "Scenario: say one.",
        "One said.",
      ],
      // The yard named its daemon in the environment of this process, which a spawn does not read unless it is given.
      { stdout: "pipe", stderr: "pipe", env: { ...process.env } },
    );
    const code = await second.exited;
    if (code !== 0)
      throw new Error(`The second window ended with ${code}: ${await new Response(second.stderr).text()}`);
    await first.shows((frame) => frame.includes("One said."), "the reply in the first window");
    const { engine } = first;
    await engine.result(await engine.rung({ word: "still_here = 7", on: engine.root }));
    expect((await engine.inspect("still_here", first.session.selected)).value).toBe(7);
  }));

test("a click on a workspace opens a session that no other window holds, and never the session of another window", () =>
  scenario([], async (first, yard) => {
    const own = join(yard.work, ".furb", "sessions", "a-own.jsonl");
    const second = await Window.open(yard, own);
    try {
      const elsewhere = join(yard.root, "elsewhere");
      await mkdir(elsewhere, { recursive: true });
      const away = await second.library.add(elsewhere);
      await second.library.create(away);
      const work = second.library.groups.find((group) => group.directory === yard.work);
      if (!work) throw new Error("The window lists no workspace of the yard.");
      const mine = work.sessions.find((entry) => entry.path === own);
      if (!mine) throw new Error("The window lists no session of its own record.");
      // The second window lets its own session of the workspace go, so the newest session there is the first one's.
      await second.library.delete(mine);
      await second.library.refresh();
      expect(work.sessions.find((entry) => !entry.archived)?.path).toBe(yard.record);
      await second.app.focusWorkspace(work);
      await until(() => second.library.groupOf() === work, "the workspace in the second window");
      expect(second.library.current?.path).not.toBe(yard.record);
      expect(second.library.current?.path.startsWith(join(yard.work, ".furb", "sessions"))).toBe(true);
    } finally {
      await second.close();
    }
    const { engine } = first;
    await engine.result(await engine.rung({ word: "untouched = 3", on: engine.root }));
    expect((await engine.inspect("untouched", first.session.selected)).value).toBe(3);
  }));

import { afterAll, expect, test } from "bun:test";
import { join } from "node:path";
import { until } from "../../bind/typescript/test/until.ts";
import { removeDemoDirectories } from "../src/demo.ts";
import { withDemo } from "./composing.ts";
import { idle } from "./idle.ts";

afterAll(removeDemoDirectories);

test("the read of the files at a point reads the journal one read at a time, waits on the facts of the host for the change of a write that the journal takes late, and keeps the first change of each path", () =>
  withDemo(async (session) => {
    const { engine } = session;
    const on = engine.root;
    await engine.result(
      await engine.prompt("str", { message: "Read this project and say what it does.", on }),
    );
    await session.refresh();
    const asked = session.activity.find((act) => session.isUserPrompt(act))?.id;
    const point = session.acts.find((act) => act.kind === "rung" && act.by === asked)?.id ?? "";
    await engine.result(await engine.prompt("str", { message: "Add the search shortcut, Ctrl+K.", on }));
    await idle(session);
    await session.refresh();
    await until(session.host, () => session.host.changes > 0, "facts");
    const path = join(session.host.directory, "README.md");
    const first = (await session.host.readChanges(0, 100)).find((change) => change.path === path);
    if (!first) throw new Error("The write of the README made no change.");
    const later = {
      ...first,
      before: "a text that no write replaced",
      after: "a later change of the same path",
    };
    // A journal of two pages: the changes of other writes come first, so the change of the README stands on the second.
    const others = Array.from({ length: 120 }, (_, n) => ({
      path: `/other/${n}`,
      before: "",
      after: "",
      act: "other",
    }));
    let ready = false;
    let running = 0;
    let most = 0;
    let reads = 0;
    session.host.readChanges = async (start, count) => {
      running++;
      most = Math.max(most, running);
      reads++;
      await Bun.sleep(40);
      const journal = [...others, ...(ready ? [first, later] : [])];
      running--;
      return journal.slice(start, start + count);
    };
    // The journal takes the change of the write late, and the host tells its facts several times while a read runs.
    setTimeout(() => {
      ready = true;
      for (let n = 0; n < 5; n++) session.host.emit("facts");
    }, 100);
    const found = await session.filesAt(point);
    expect(most).toBe(1);
    expect(reads).toBeGreaterThan(2);
    expect(found.files).toEqual([first]);
  }));

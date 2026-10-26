import { expect, test } from "bun:test";
import { appendFile, mkdir, mkdtemp, readdir, readFile, realpath, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { createTestRenderer } from "@opentui/core/testing";
import { furbDirectory } from "furb";
import { until } from "../../bind/typescript/test/until.ts";
import { App } from "../src/app.ts";
import { openEngine } from "../src/bridge.ts";
import { demoSession } from "../src/demo.ts";
import { Preferences } from "../src/preferences.ts";
import type { Session } from "../src/session.ts";
import { Workspaces } from "../src/workspaces.ts";
import { composing } from "./composing.ts";

const banner = "The journal of file changes is damaged";

/** A directory of its own, which ends with the test, and the record of a life that wrote three files, whose journal
 * of file changes holds a line that is no change in the middle. */
async function withDamaged(
  use: (context: { directory: string; project: string; record: string; journal: string }) => Promise<void>,
): Promise<void> {
  const directory = await realpath(await mkdtemp(join(tmpdir(), "furb-journal-")));
  const project = join(directory, "project");
  try {
    await mkdir(project, { recursive: true });
    const record = join(furbDirectory(project, "sessions"), "life.jsonl");
    const journal = `${record}.changes.jsonl`;
    const { engine, host } = await openEngine({ cwd: project, record, demo: true });
    for (const name of ["a", "b", "c"])
      await engine.result(
        await engine.rung({ word: `write(Text("${name}.txt", "${name}\\n"))`, on: engine.root }),
      );
    await until(host, () => host.changes === 3);
    await host.dispose();
    const lines = (await readFile(journal, "utf8")).split("\n");
    lines[1] = "{ damaged }";
    await writeFile(journal, lines.join("\n"));
    await use({ directory, project, record, journal });
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

/** The view of changes of a session, drawn in an App on a test terminal. */
async function changesFrame(session: Session, library: Workspaces): Promise<string> {
  const screen = await createTestRenderer({ width: 150, height: 40, exitSignals: [] });
  const app = new App(screen.renderer, session, { quit() {}, workspaces: library });
  try {
    session.show("changes");
    await session.refresh().catch(session.fail);
    app.render();
    await screen.flush();
    await screen.flush();
    return screen.captureCharFrame();
  } finally {
    app.dispose();
    screen.renderer.destroy();
  }
}

/** The text that each change of the journal of a session holds after it, in order. */
async function afters(session: Session): Promise<string[]> {
  return (await session.host.readChanges(0, 20)).map((change) => change.after);
}

test("a damaged journal in the middle opens with --resume and from the list, and the view of changes shows its banner", () =>
  withDamaged(async ({ directory, project, record }) => {
    const preferences = () => new Preferences(join(directory, "config/ui.json"));
    // --resume imports the record into the workspace of its folder.
    const resumed = new Workspaces(preferences(), { demo: true });
    try {
      await resumed.import(record, await resumed.add(project));
      const session = resumed.current?.session;
      if (!session) throw new Error("The record did not open.");
      expect(session.host.changes).toBe(3);
      const frame = await changesFrame(session, resumed);
      expect(frame).toContain(banner);
      expect(frame).toContain("Repair the journal");
      expect(frame).toContain("Start a new journal");
      expect(frame).not.toContain("JSON");
      expect(session.damagedJournal).toEndWith("is damaged: change 2 cannot be read.");
      // The view that shows the damage fails, and the session that the list marks does not.
      expect(session.status(session.selected)).toBe("error");
      expect(session.status()).not.toBe("error");
      // The rewind reads the same journal, and it reads each change that it can.
      const first = session.acts.find((act) => act.kind === "rung");
      expect((await session.filesAt(first?.id ?? "")).files.length).toBeGreaterThan(0);
    } finally {
      await resumed.dispose();
    }
    // The list of the workspace holds the session, and a pick of it opens it.
    const listed = new Workspaces(preferences(), { demo: true });
    try {
      await listed.refresh();
      const entry = listed.groups.flatMap((group) => group.sessions).find((one) => one.path === record);
      if (!entry) throw new Error("The list does not hold the session.");
      await listed.select(entry);
      const session = entry.session;
      if (!session) throw new Error("The session did not open.");
      expect(session.host.changes).toBe(3);
      const frame = await changesFrame(session, listed);
      expect(frame).toContain(banner);
      expect(frame).toContain("Repair the journal");
      expect(session.damagedJournal).toEndWith("is damaged: change 2 cannot be read.");
    } finally {
      await listed.dispose();
    }
  }));

test("a repair keeps both good changes and says 2 changes kept, 1 line removed", () =>
  withDamaged(async ({ project, record, journal }) => {
    const opened = await demoSession({ record, cwd: project });
    await composing(
      async ({ session, frame, click }) => {
        session.show("changes");
        await session.refresh().catch(session.fail);
        expect(await frame()).toContain("Repair the journal");
        await click("Repair the journal");
        await until(
          session,
          () =>
            session.notice.startsWith("The journal is repaired") ||
            Boolean(session.error && !session.damagedJournal),
        );
        expect(session.error).toBe("");
        expect(session.notice).toBe("The journal is repaired: 2 changes kept, 1 line removed.");
        expect(await afters(session)).toEqual(["a\n", "c\n"]);
        expect((await readFile(journal, "utf8")).split("\n").filter(Boolean)).toHaveLength(2);
        expect(session.error).toBe("");
        expect(await frame()).not.toContain(banner);
      },
      { width: 150, height: 40 },
      opened,
    );
  }));

test("a change written while the session is open, by it or by another program, is not lost by a repair", () =>
  withDamaged(async ({ project, record, journal }) => {
    const opened = await demoSession({ record, cwd: project });
    await composing(
      async ({ session, frame, click }) => {
        session.show("changes");
        await session.refresh().catch(session.fail);
        expect(await frame()).toContain(banner);
        const { engine } = session;
        await engine.result(await engine.rung({ word: 'write(Text("d.txt", "d\\n"))', on: engine.root }));
        await until(session.host, () => session.host.changes === 4);
        await appendFile(
          journal,
          `${JSON.stringify({ path: join(project, "e.txt"), before: "", after: "e\n" })}\n`,
        );
        await frame();
        await click("Repair the journal");
        await until(
          session,
          () =>
            session.notice.startsWith("The journal is repaired") ||
            Boolean(session.error && !session.damagedJournal),
        );
        expect(session.error).toBe("");
        expect(session.notice).toBe("The journal is repaired: 4 changes kept, 1 line removed.");
        expect(await afters(session)).toEqual(["a\n", "c\n", "d\n", "e\n"]);
      },
      { width: 150, height: 40 },
      opened,
    );
  }));

test("repair and a new journal have entries of the palette, with their keys", () =>
  withDamaged(async ({ project, record }) => {
    const opened = await demoSession({ record, cwd: project });
    await composing(
      async ({ session, app, screen, frame }) => {
        session.show("changes");
        await session.refresh().catch(session.fail);
        app.palette();
        await screen.mockInput.typeText("journal");
        const shown = await frame();
        expect(shown).toContain("Repair the journal");
        expect(shown).toContain("Start a new journal");
        expect(shown).toContain("⌥R");
        expect(shown).toContain("⌥J");
      },
      { width: 150, height: 40 },
      opened,
    );
  }));

test("the key of a repair acts only on a journal found damaged, and repairs it", () =>
  withDamaged(async ({ project, record }) => {
    const opened = await demoSession({ record, cwd: project });
    await composing(
      async ({ session, screen, frame }) => {
        // A key that finds no damaged journal goes on to the input.
        screen.mockInput.pressKey("r", { meta: true });
        await frame();
        expect(session.notice).toBe("");
        session.show("changes");
        await session.refresh().catch(session.fail);
        expect(await frame()).toContain("⌥R");
        screen.mockInput.pressKey("r", { meta: true });
        await until(
          session,
          () =>
            session.notice.startsWith("The journal is repaired") ||
            Boolean(session.error && !session.damagedJournal),
        );
        expect(session.error).toBe("");
        expect(session.notice).toBe("The journal is repaired: 2 changes kept, 1 line removed.");
        await until(session.host, () => session.host.changes === 2);
        await expect(session.renewJournal()).rejects.toThrow("The journal of file changes is not damaged.");
      },
      { width: 150, height: 40 },
      opened,
    );
  }));

test("a new journal keeps the damaged one beside it, by its key", () =>
  withDamaged(async ({ project, record }) => {
    const opened = await demoSession({ record, cwd: project });
    await composing(
      async ({ session, screen, frame }) => {
        session.show("changes");
        await session.refresh().catch(session.fail);
        expect(await frame()).toContain("Start a new journal");
        screen.mockInput.pressKey("j", { meta: true });
        await until(session, () => session.notice.startsWith("A new journal is started"));
        expect(session.notice).toStartWith("A new journal is started. The damaged one is kept at");
        await until(session.host, () => session.host.changes === 0);
        expect((await readdir(dirname(record))).filter((name) => name.includes(".damaged-"))).toHaveLength(1);
      },
      { width: 150, height: 40 },
      opened,
    );
  }));

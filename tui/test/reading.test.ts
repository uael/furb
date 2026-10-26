import { expect, test } from "bun:test";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createTestRenderer } from "@opentui/core/testing";
import { App } from "../src/app.ts";
import { openEngine } from "../src/bridge.ts";
import { Preferences } from "../src/preferences.ts";
import { Workspaces } from "../src/workspaces.ts";

test("a record whose fact drifts opens to read, with what it kept, and a dialog that names its version", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-drift-"));
  const record = join(directory, "drifts.jsonl");
  try {
    const first = await openEngine({ cwd: directory, record, demo: true });
    await first.engine.result(
      await first.engine.rung({ word: 'await bash("echo hi")', on: first.engine.root }),
    );
    await first.host.dispose();
    // The record keeps the command with a word that this engine does not make, as an earlier engine kept it.
    const kept = await readFile(record, "utf8");
    expect(kept).toContain('"echo hi",""');
    await writeFile(record, kept.replace('"echo hi",""', '"echo hi",false'));
    const log = await readFile(`${record}.log`, "utf8");
    const library = new Workspaces(new Preferences(), { demo: true });
    try {
      await library.import(record, await library.add(directory));
      const session = library.current?.session;
      if (!session) throw new Error("The record did not open.");
      const reading = session.host.reading;
      expect(reading?.why).toStartWith("Drift: bash1 drifts");
      expect(reading?.made).toBe(reading?.engine ?? "");
      expect(reading?.version).toBe(reading?.current ?? -1);
      // The replay that drifted leaves the log of the life before it as it was.
      expect(await readFile(`${record}.log`, "utf8")).toBe(log);
      const bash = session.acts.find((act) => act.id === "bash1");
      expect(bash?.done).toBe(true);
      expect(session.acts.find((act) => act.id === "rung1")?.by).toBe("operator");
      expect(await session.engine.bash("ls").catch((error: Error) => error.message)).toBe(
        "This record is open to read, so nothing runs on it. Start a new life from it to go on.",
      );
      const screen = await createTestRenderer({ width: 120, height: 40, exitSignals: [] });
      const app = new App(screen.renderer, session, { quit() {}, workspaces: library });
      try {
        // The dialog opens with the view, before any frame.
        await screen.renderOnce();
        const frame = screen.captureCharFrame();
        expect(frame).toContain("This record does not replay");
        expect(frame).toContain("Read it");
        expect(frame).toContain("Start a new life from it");
        expect(frame).toContain(`Made by: version ${reading?.version}`);
        expect(frame).toContain(`This build: version ${reading?.current}`);
        expect(frame).toContain("its replay drifts");
      } finally {
        app.dispose();
        screen.renderer.destroy();
      }
    } finally {
      await library.dispose();
    }
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

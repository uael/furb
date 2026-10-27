import { expect, test } from "bun:test";
import { mkdir, mkdtemp, open, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createTestRenderer } from "@opentui/core/testing";
import { follow } from "../src/app.ts";
import { Preferences } from "../src/preferences.ts";
import { Workspaces } from "../src/workspaces.ts";

/** How long the first frame of a new life may take, from the list of workspaces to the frame that holds the composer.
 * The open of a life takes seconds, and no part of it is in this time. */
const FIRST_FRAME = 2000;
/** The records of the project, each large enough that a read of it would show in the time of the first frame. */
const RECORDS = 8;
const LARGE = 150 * 2 ** 20;

/** A project whose folder of sessions holds many large records, a library on it, and a test terminal. */
async function withProject(
  roster: string[],
  use: (context: {
    directory: string;
    library: Workspaces;
    screen: Awaited<ReturnType<typeof createTestRenderer>>;
  }) => Promise<void>,
): Promise<void> {
  const directory = await mkdtemp(join(tmpdir(), "furb-instant-"));
  const sessions = join(directory, "project/.furb/sessions");
  await mkdir(sessions, { recursive: true });
  for (let one = 0; one < RECORDS; one++) {
    const file = await open(join(sessions, `2026-10-2${one}T00-00-00.000Z-0000000${one}.jsonl`), "w");
    await file.truncate(LARGE);
    await file.close();
  }
  const library = new Workspaces(
    new Preferences(join(directory, "config/ui.json")),
    { demo: true, roster },
    join(directory, "config/workspaces.json"),
  );
  const screen = await createTestRenderer({ width: 152, height: 42, useMouse: true, exitSignals: [] });
  try {
    await use({ directory, library, screen });
  } finally {
    screen.renderer.destroy();
    await library.dispose();
    await rm(directory, { recursive: true, force: true });
  }
}

test("a new life draws its first frame with the composer before its life opens, with many large records present", () =>
  withProject(["claude-cli:sonnet"], async ({ directory, library, screen }) => {
    const begun = performance.now();
    const group = await library.add(join(directory, "project"), false);
    const entry = await library.create(group, undefined, undefined, false, group.directory, true);
    const app = follow(screen.renderer, { quit() {}, workspaces: library });
    await screen.flush();
    const first = performance.now() - begun;
    let opened = false;
    void entry.ready?.then(() => {
      opened = true;
    });
    expect(screen.captureCharFrame()).toContain("Ask anything, or type / for commands");
    expect(library.current).toBe(entry);
    expect(opened).toBe(false);
    expect(first).toBeLessThan(FIRST_FRAME);
    // The records stand in the list after the first frame, with their sizes, and the life fills in when it opens.
    expect(group.sessions.filter((one) => one.size === LARGE)).toHaveLength(0);
    await library.refresh();
    expect(group.sessions.filter((one) => one.size === LARGE)).toHaveLength(RECORDS);
    expect(await entry.ready).toBe(true);
    expect(entry.session?.host.actor).not.toBe("");
    app().dispose();
  }));

test("a life that fails to open after its window drew it leaves its row with the error and tells so", () =>
  withProject(["claude-cli:nope"], async ({ directory, library, screen }) => {
    const group = await library.add(join(directory, "project"), false);
    const failed = new Promise<unknown>((resolve) =>
      library.once("failed", (_entry, error) => resolve(error)),
    );
    const entry = await library.create(group, undefined, undefined, false, group.directory, true);
    const app = follow(screen.renderer, { quit() {}, workspaces: library });
    expect(entry.session).toBeDefined();
    expect(String(await failed)).toContain("No model is claude-cli:nope");
    expect(await entry.ready).toBe(false);
    expect(entry.status).toBe("error");
    expect(entry.error).toContain("No model is claude-cli:nope");
    expect(entry.session).toBeUndefined();
    app().dispose();
  }));

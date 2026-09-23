import { expect, test } from "bun:test";
import { mkdir, mkdtemp, readFile, rm, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import { createTestRenderer } from "@opentui/core/testing";
import { App } from "../src/app.ts";
import { openEngine } from "../src/bridge.ts";
import { seedDemoFiles } from "../src/demo.ts";
import { Preferences } from "../src/preferences.ts";
import { Session } from "../src/session.ts";
import { type SessionEntry, Workspaces } from "../src/workspaces.ts";

function until(library: Workspaces, predicate: () => boolean): Promise<void> {
  if (predicate()) return Promise.resolve();
  return new Promise((resolve, reject) => {
    const done = () => {
      if (!predicate()) return;
      clearTimeout(timer);
      library.off("change", done);
      resolve();
    };
    const timer = setTimeout(() => {
      library.off("change", done);
      reject(new Error("Session status did not change."));
    }, 10000);
    library.on("change", done);
  });
}

test("workspaces keep sessions alive, report background completion and input, and reopen saved records paused", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-spaces-"));
  await mkdir(join(directory, "alpha"));
  await mkdir(join(directory, "beta"));
  await seedDemoFiles(join(directory, "alpha"));
  await seedDemoFiles(join(directory, "beta"));
  const preferences = new Preferences(join(directory, "config/ui.json"));
  let library = new Workspaces(preferences, { demo: true });
  let first: SessionEntry;
  try {
    const alpha = await library.add(join(directory, "alpha"));
    const beta = await library.add(join(directory, "beta"));
    first = await library.create(alpha, "Checks");
    const second = await library.create(alpha, "Notes");
    const third = await library.create(beta, "Release");
    expect(alpha.sessions).toHaveLength(2);
    expect(beta.sessions).toHaveLength(1);
    if (!first.session || !second.session || !third.session) throw new Error("Sessions did not open.");
    const worker = first.session.life;
    const command = await worker.bash("sleep 0.6; printf 'background finished'", { on: worker.root });
    await until(library, () => first.status === "working");
    expect(library.groupStatus(alpha)).toBe("working");
    expect(library.current).toBe(third);
    await worker.result(command);
    await until(library, () => first.status === "done");
    await library.select(first);
    expect(first.session.life).toBe(worker);
    expect(first.status).toBe("idle");
    expect(first.unread).toBe(false);

    const question = await second.session.life.prompt("str", "Name the release", { to: "operator" });
    await until(library, () => second.status === "blocked");
    expect(library.groupStatus(alpha)).toBe("blocked");
    await second.session.world.answer(question, "v1");
    await until(library, () => second.status === "done");

    await first.session.submit("/pause");
    await until(library, () => first.status === "paused");
    await first.session.submit("/wake");
    await until(library, () => first.status === "idle");
    const waiting = await first.session.life.wait(60);
    await until(library, () => first.status === "working");
    preferences.sidebar = false;
    preferences.save("paper");
    expect(second.session.theme).toBe("paper");
    library.toggle(beta);
    const record = first.path;
    await library.dispose();
    library = new Workspaces(new Preferences(preferences.path), { demo: true });
    await library.refresh();
    expect(library.groups.map((group) => group.name)).toEqual(["alpha", "beta"]);
    expect(library.groups[1]?.collapsed).toBe(true);
    expect(library.preferences.sidebar).toBe(false);
    expect(library.preferences.theme).toBe("paper");
    expect(
      library.groups
        .flatMap((group) => group.sessions)
        .every((entry) => !entry.session && ["saved", "paused"].includes(entry.status)),
    ).toBe(true);
    const restored = library.groups[0]?.sessions.find((entry) => entry.path === record);
    if (!restored) throw new Error("The saved session is missing.");
    await library.select(restored);
    expect(restored.status).toBe("paused");
    expect(restored.session?.world.held.has(waiting)).toBe(true);
    const metadata = JSON.parse(await readFile(`${record}.world.json`, "utf8"));
    expect(metadata.held).toBeUndefined();
  } finally {
    await library.dispose();
    await rm(directory, { recursive: true, force: true });
  }
}, 30000);

test("deleting a session moves its record and state to trash, keeps other sessions alive, and refuses another owner", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-delete-"));
  const preferences = new Preferences(join(directory, "config/ui.json"));
  const library = new Workspaces(preferences, { demo: true });
  let external: Session | undefined;
  try {
    const group = await library.add(directory);
    const first = await library.create(group, "Remove me");
    const second = await library.create(group, "Keep me");
    if (!first.session || !second.session) throw new Error("Missing live sessions.");
    await first.session.life.result(await first.session.life.rung("kept_value = 41"));
    const otherLife = second.session.life;
    const trash = await library.delete(first);
    expect(library.current).toBe(second);
    expect(second.session.life).toBe(otherLife);
    await otherLife.result(await otherLife.rung("still_alive = 17"));
    expect(await otherLife.held("modules", [second.session.selected, "still_alive"], "at")).toBe(17);
    expect(await stat(first.path).catch(() => null)).toBeNull();
    const archived = join(trash, basename(first.path));
    expect((await stat(archived)).isFile()).toBe(true);
    const restored = await library.import(archived, group);
    expect(
      await restored.session?.life.held("modules", [restored.session.selected, "kept_value"], "at"),
    ).toBe(41);
    const locked = join(group.directory, ".furb/sessions/locked.jsonl");
    const opened = await openEngine({ cwd: directory, record: locked, demo: true });
    external = new Session(opened.life, opened.world, true, preferences);
    await library.refresh();
    const entry = group.sessions.find((entry) => entry.path === locked);
    if (!entry) throw new Error("The locked session was not listed.");
    const error = await library.delete(entry).catch((error: unknown) => error);
    expect(error).toBeInstanceOf(Error);
    expect(String(error)).toContain("Another process owns");
    expect((await stat(locked)).isFile()).toBe(true);
  } finally {
    await external?.dispose();
    await library.dispose();
    await rm(directory, { recursive: true, force: true });
  }
}, 30000);

test("the left tree groups sessions, switches by mouse, collapses and toggles without losing a draft", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-tree-"));
  await mkdir(join(directory, "project"));
  const library = new Workspaces(new Preferences(join(directory, "ui.json")), { demo: true });
  const screen = await createTestRenderer({ width: 152, height: 42, useMouse: true });
  let app: App | undefined;
  try {
    const group = await library.add(join(directory, "project"));
    const first = await library.create(group, "foo");
    const second = await library.create(group, "bar");
    if (!first.session || !second.session) throw new Error("The session did not open.");
    const options = {
      quit() {},
      workspaces: library,
      newSession: async () => {
        await library.create();
      },
    };
    app = new App(screen.renderer, second.session, options);
    library.on("select", (session) => {
      app?.dispose();
      app = new App(screen.renderer, session, options);
    });
    app.composer.setText("keep this draft");
    await screen.flush();
    const lines = screen.captureCharFrame().split("\n");
    const projectRow = lines.findIndex((line) => line.includes("▾") && line.includes("project"));
    const fooRow = lines.findIndex((line) => /[○●◌] foo\b/.test(line.slice(0, 28)));
    const barRow = lines.findIndex((line) => /[○●◌] bar\b/.test(line.slice(0, 28)));
    expect(fooRow).toBeGreaterThan(projectRow);
    expect(barRow).toBeGreaterThan(projectRow);
    await screen.mockMouse.click(5, fooRow);
    await until(library, () => library.current === first);
    await screen.flush();
    expect(app.session).toBe(first.session);
    await library.select(second);
    await screen.flush();
    expect(app.composer.plainText).toBe("keep this draft");
    await screen.mockMouse.click(2, projectRow);
    app.render();
    await screen.flush();
    expect(group.collapsed).toBe(true);
    expect(
      screen
        .captureCharFrame()
        .split("\n")
        .some((line) => line.slice(0, 28).includes("foo")),
    ).toBe(false);
    screen.mockInput.pressKey("\\", { ctrl: true });
    await screen.flush();
    app.render();
    await screen.flush();
    expect(library.preferences.sidebar).toBe(false);
    expect(app.scroll.x).toBe(1);
    expect(app.composer.plainText).toBe("keep this draft");
    screen.mockInput.pressKey("w", { ctrl: true });
    // The picker refreshes the workspaces before it opens, so the frame is read until it shows, for two seconds.
    for (
      let tries = 0;
      tries < 100 && !screen.captureCharFrame().includes("Workspaces & sessions");
      tries++
    ) {
      await Bun.sleep(20);
      await screen.flush();
    }
    expect(screen.captureCharFrame()).toContain("Workspaces & sessions");
    app.closeOverlay();
    library.toggle();
    screen.resize(82, 32);
    app.render();
    await screen.flush();
    expect(app.scroll.x).toBe(1);
    expect(app.composer.plainText).toBe("keep this draft");
  } finally {
    app?.dispose();
    screen.renderer.destroy();
    await library.dispose();
    await rm(directory, { recursive: true, force: true });
  }
}, 30000);

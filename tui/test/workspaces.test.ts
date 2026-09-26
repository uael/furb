import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { mkdir, mkdtemp, readFile, realpath, rm, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, join } from "node:path";
import { type NativeEar, store } from "@furb/engine";
import type { CapturedFrame, RGBA } from "@opentui/core";
import { createTestRenderer } from "@opentui/core/testing";
import { until } from "../../bind/typescript/test/until.ts";
import { App } from "../src/app.ts";
import { openEngine } from "../src/bridge.ts";
import { seedDemoFiles } from "../src/demo.ts";
import { Preferences } from "../src/preferences.ts";
import { Session, savedView } from "../src/session.ts";
import { type SessionEntry, Workspaces } from "../src/workspaces.ts";

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
    const worker = first.session.engine;
    const command = await worker.bash("sleep 0.6; printf 'background finished'", { on: worker.root });
    await until(
      library,
      () => first.status === "working",
      "change",
      () => ({
        status: first.status,
        error: first.error,
        errors: first.session?.errors,
        acts: first.session?.acts.map((act) => [act.kind, act.id, act.done, act.paused]),
      }),
    );
    expect(library.groupStatus(alpha)).toBe("working");
    expect(library.current).toBe(third);
    await worker.result(command);
    await until(library, () => first.status === "done");
    await library.select(first);
    expect(first.session.engine).toBe(worker);
    expect(first.status).toBe("idle");
    expect(first.unread).toBe(false);

    const question = await second.session.engine.prompt("str", {
      message: "Name the release",
      to: "operator",
      on: second.session.engine.root,
    });
    await until(library, () => second.status === "blocked");
    expect(library.groupStatus(alpha)).toBe("blocked");
    await second.session.host.answer(question, "v1");
    await until(library, () => second.status === "done");

    await first.session.submit("/pause");
    await until(library, () => first.status === "paused");
    await first.session.submit("/wake");
    await until(library, () => first.status === "idle");
    const waiting = await first.session.engine.wait({ seconds: 60, on: first.session.engine.root });
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
    expect(restored.session?.host.pending.has(waiting)).toBe(true);
    const metadata = JSON.parse(await readFile(`${record}.session.json`, "utf8"));
    expect(metadata.pending).toBeUndefined();
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
    await first.session.engine.result(
      await first.session.engine.rung({ word: "kept_value = 41", on: first.session.engine.root }),
    );
    const otherEngine = second.session.engine;
    const trash = await library.delete(first);
    expect(library.current).toBe(second);
    expect(second.session.engine).toBe(otherEngine);
    await otherEngine.result(await otherEngine.rung({ word: "still_alive = 17", on: otherEngine.root }));
    expect((await otherEngine.inspect("still_alive", second.session.selected)).value).toBe(17);
    expect(await stat(first.path).catch(() => null)).toBeNull();
    expect(await stat(`${first.path}.lock`).catch(() => null)).toBeNull();
    const archived = join(trash, basename(first.path));
    expect((await stat(archived)).isFile()).toBe(true);
    expect((await stat(`${archived}.lock`)).isFile()).toBe(true);
    const restored = await library.import(archived, group);
    expect((await restored.session?.engine.inspect("kept_value", restored.session.selected))?.value).toBe(41);
    const locked = join(group.directory, ".furb/sessions/locked.jsonl");
    const opened = await openEngine({ cwd: directory, record: locked, demo: true });
    external = new Session(opened.engine, opened.host, true, preferences);
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

test("the sidebar tree groups sessions, switches by mouse, collapses and toggles without losing a draft", async () => {
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
    // The sidebar is the last columns of the screen.
    const left = 152 - library.preferences.sidebarWidth;
    const lines = screen.captureCharFrame().split("\n");
    const projectRow = lines.findIndex((line) => /▾ project/.test(line.slice(left)));
    const fooRow = lines.findIndex((line) => /[○●◌] foo\b/.test(line.slice(left)));
    const barRow = lines.findIndex((line) => /[○●◌] bar\b/.test(line.slice(left)));
    expect(fooRow).toBeGreaterThan(projectRow);
    expect(barRow).toBeGreaterThan(projectRow);
    await screen.mockMouse.click(left + 6, fooRow);
    await until(library, () => library.current === first);
    await screen.flush();
    expect(app.session).toBe(first.session);
    await library.select(second);
    await screen.flush();
    expect(app.composer.plainText).toBe("keep this draft");
    await screen.mockMouse.click(left + 3, projectRow);
    app.render();
    await screen.flush();
    expect(group.collapsed).toBe(true);
    expect(
      screen
        .captureCharFrame()
        .split("\n")
        .some((line) => line.slice(left).includes("foo")),
    ).toBe(false);
    screen.mockInput.pressKey("\\", { ctrl: true });
    await screen.flush();
    app.render();
    await screen.flush();
    expect(library.preferences.sidebar).toBe(false);
    expect(app.scroll.x).toBe(2);
    expect(app.composer.plainText).toBe("keep this draft");
    // The picker reads the workspaces again before it opens: the test awaits the picker the key opened.
    const picker = app.workspacePicker;
    let opened: Promise<void> | undefined;
    app.workspacePicker = () => {
      opened = picker();
      return opened;
    };
    screen.mockInput.pressKey("w", { ctrl: true });
    await opened;
    await screen.flush();
    expect(screen.captureCharFrame()).toContain("Workspaces and sessions");
    app.closeOverlay();
    library.toggle();
    screen.resize(82, 32);
    app.render();
    await screen.flush();
    expect(app.scroll.x).toBe(2);
    expect(app.composer.plainText).toBe("keep this draft");
  } finally {
    app?.dispose();
    screen.renderer.destroy();
    await library.dispose();
    await rm(directory, { recursive: true, force: true });
  }
}, 30000);

test("the list of saved sessions comes before their replays, and a row shows its state when its replay lands", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-inspect-"));
  const preferences = new Preferences(join(directory, "config/ui.json"));
  let library = new Workspaces(preferences, { demo: true });
  try {
    const entry = await library.create(await library.add(directory), "Held");
    const engine = entry.session?.engine;
    await engine?.wait({ seconds: 60, on: engine.root });
    await library.dispose();
    library = new Workspaces(preferences, { demo: true });
    const row = (await library.add(directory)).sessions.find((candidate) => candidate.path === entry.path);
    // The replay runs in a worker that loads the native engine first, so the list is there before it.
    expect(row?.status).toBe("saved");
    await until(library, () => row?.status === "paused");
  } finally {
    await library.dispose();
    await rm(directory, { recursive: true, force: true });
  }
}, 30000);

test("a session that opens keeps its row through a refresh that comes before its record", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-create-"));
  const library = new Workspaces(new Preferences(join(directory, "config/ui.json")), { demo: true });
  try {
    const group = await library.add(directory);
    await library.create(group, "First");
    const creating = library.create(group, "Second");
    await until(library, () =>
      group.sessions.some((entry) => entry.name === "Second" && entry.status === "opening"),
    );
    const row = group.sessions.find((entry) => entry.name === "Second");
    expect(await stat(row?.path ?? "").catch(() => null)).toBeNull();
    await library.refresh();
    const created = await creating;
    expect(created).toBe(row as SessionEntry);
    expect(group.sessions.filter((entry) => entry.path === created.path)).toEqual([created]);
    expect(library.current).toBe(created);
    expect(created.session?.sessionName).toBe("Second");
  } finally {
    await library.dispose();
    await rm(directory, { recursive: true, force: true });
  }
}, 30000);

test("deleting the current session moves to the next session that opens, and to a new one when none does", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-delete-current-"));
  const preferences = new Preferences(join(directory, "config/ui.json"));
  let library = new Workspaces(preferences, { demo: true });
  let lease: NativeEar | undefined;
  try {
    const held = await library.create(await library.add(directory), "Held elsewhere");
    await library.dispose();
    library = new Workspaces(preferences, { demo: true });
    const group = await library.add(directory);
    const current = await library.create(group, "Delete me");
    lease = store(held.path).ear;
    const sibling = group.sessions.find((entry) => entry.path === held.path);
    expect(group.sessions.map((entry) => entry.name)).toEqual(["Delete me", "Held elsewhere"]);
    await library.delete(current);
    expect(sibling?.status).toBe("error");
    expect(sibling?.error).toContain("Another process owns");
    expect(library.current?.session).toBeDefined();
    expect(library.current).not.toBe(current);
    expect(library.current).not.toBe(sibling);
    expect(await stat(current.path).catch(() => null)).toBeNull();
  } finally {
    lease?.dispose();
    await library.dispose();
    await rm(directory, { recursive: true, force: true });
  }
}, 30000);

test("the workspace list keeps what each instance saves, and a list that cannot be read stays for the user to repair", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-list-"));
  const preferences = new Preferences(join(directory, "config/ui.json"));
  const path = join(directory, "config/workspaces.json");
  const libraries: Workspaces[] = [];
  const open = () => {
    const library = new Workspaces(preferences, { demo: true });
    libraries.push(library);
    return library;
  };
  const listed = async () =>
    (JSON.parse(await readFile(path, "utf8")) as { workspaces: { directory: string; collapsed: boolean }[] })
      .workspaces;
  try {
    const project = async (name: string) => {
      await mkdir(join(directory, name));
      return realpath(join(directory, name));
    };
    const alpha = await project("alpha");
    const beta = await project("beta");
    const gamma = await project("gamma");
    const damaged = `{"workspaces": [{"directory": ${JSON.stringify(alpha)}},]}`;
    await mkdir(dirname(path), { recursive: true });
    await writeFile(path, damaged);
    const first = open();
    expect(first.notice).toContain(`Could not read the workspace list at ${path}`);
    await first.add(beta);
    expect(await readFile(path, "utf8")).toBe(damaged);
    await writeFile(path, JSON.stringify({ workspaces: [{ directory: alpha, records: [] }] }));
    const betaGroup = first.groups.find((group) => group.directory === beta);
    if (!betaGroup) throw new Error("No beta workspace.");
    first.toggle(betaGroup);
    expect(first.notice).toBe("");
    expect((await listed()).map((group) => [group.directory, group.collapsed])).toEqual([
      [alpha, false],
      [beta, true],
    ]);
    const second = open();
    await second.add(gamma);
    first.toggle(betaGroup);
    expect((await listed()).map((group) => [group.directory, group.collapsed])).toEqual([
      [alpha, false],
      [beta, false],
      [gamma, false],
    ]);
    expect(open().groups.map((group) => group.directory)).toEqual([alpha, beta, gamma]);
  } finally {
    for (const library of libraries) await library.dispose();
    await rm(directory, { recursive: true, force: true });
  }
}, 30000);

test("a save of the workspace list that fails names its file", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-list-save-"));
  const library = new Workspaces(new Preferences(join(directory, "config/ui.json")), { demo: true });
  try {
    await mkdir(join(directory, "project"));
    // A directory where the save writes its file makes the save fail on every system.
    await mkdir(join(directory, "config/workspaces.json.tmp"), { recursive: true });
    expect(String(await library.add(join(directory, "project")).catch((error: unknown) => error))).toContain(
      "workspaces.json.tmp",
    );
  } finally {
    await library.dispose();
    await rm(directory, { recursive: true, force: true });
  }
});

test("the .furb that the TUI makes in a project keeps itself out of version control", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-ignore-"));
  const library = new Workspaces(new Preferences(join(directory, "config/ui.json")), { demo: true });
  const git = (...args: string[]) =>
    Bun.spawnSync(["git", ...args], { cwd: join(directory, "project") }).stdout.toString();
  try {
    await mkdir(join(directory, "project"));
    git("init", "-q");
    await writeFile(join(directory, "project/app.txt"), "tracked\n");
    const entry = await library.create(await library.add(join(directory, "project")), "Ignored");
    await entry.session?.submit("/share");
    await library.delete(entry);
    expect(await readFile(join(directory, "project/.furb/.gitignore"), "utf8")).toBe("*\n");
    expect(git("status", "--porcelain", "--untracked-files=all")).toBe("?? app.txt\n");
  } finally {
    await library.dispose();
    await rm(directory, { recursive: true, force: true });
  }
}, 30000);

test("a session row archives a session through its remove button, which folds it under Archived until a click brings it back", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-archive-"));
  await mkdir(join(directory, "project"));
  const library = new Workspaces(new Preferences(join(directory, "ui.json")), { demo: true });
  const screen = await createTestRenderer({ width: 152, height: 42, useMouse: true });
  let app: App | undefined;
  try {
    const group = await library.add(join(directory, "project"));
    const first = await library.create(group, "foo");
    const second = await library.create(group, "bar");
    if (!first.session || !second.session) throw new Error("The session did not open.");
    const options = { quit() {}, workspaces: library };
    app = new App(screen.renderer, second.session, options);
    library.on("select", (session) => {
      app?.dispose();
      app = new App(screen.renderer, session, options);
    });
    const left = 152 - library.preferences.sidebarWidth;
    const frame = async () => {
      app?.render();
      await screen.flush();
      return screen.captureCharFrame().split("\n");
    };
    let lines = await frame();
    const fooRow = lines.findIndex((line) => / foo\b/.test(line.slice(left)));
    const remove = (lines[fooRow] ?? "").lastIndexOf("×");
    expect(remove).toBeGreaterThan(left);
    await screen.mockMouse.moveTo(remove, fooRow);
    await screen.mockMouse.click(remove, fooRow);
    lines = await frame();
    expect(lines.join("\n")).toContain("Remove the session “foo”?");
    const archive = lines.findIndex((line) => line.includes("Archive") && line.includes("Fold it under"));
    await screen.mockMouse.click((lines[archive] ?? "").indexOf("Archive") + 1, archive);
    await until(library, () => first.archived === true);
    expect(library.current).toBe(second);
    lines = await frame();
    expect(lines.some((line) => / foo\b/.test(line.slice(left)))).toBe(false);
    const folded = lines.findIndex((line) => /▸ +Archived +1/.test(line.slice(left)));
    expect(folded).toBeGreaterThan(0);
    const saved = JSON.parse(await readFile(library.path, "utf8")) as {
      workspaces: { archived: string[] }[];
    };
    expect(saved.workspaces[0]?.archived).toEqual([first.path]);
    await screen.mockMouse.click(left + 4, folded);
    lines = await frame();
    const archivedRow = lines.findIndex((line) => / foo\b/.test(line.slice(left)));
    expect(archivedRow).toBeGreaterThan(folded);
    await screen.mockMouse.click(left + 8, archivedRow);
    await until(library, () => library.current === first);
    expect(first.archived).toBe(false);
  } finally {
    app?.dispose();
    screen.renderer.destroy();
    await library.dispose();
    await rm(directory, { recursive: true, force: true });
  }
});

/** The text, the color and the ground of the cell at a column of a row of the screen. */
function cellAt(frame: CapturedFrame, x: number, y: number): { text: string; fg?: RGBA; bg?: RGBA } {
  let at = 0;
  for (const span of frame.lines[y]?.spans ?? []) {
    if (x < at + span.width) return { text: span.text, fg: span.fg, bg: span.bg };
    at += span.width;
  }
  return { text: "" };
}

test("a session row lights under the pointer with its buttons, tells its state on its mark alone, and goes dark when the pointer leaves", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-rows-"));
  await mkdir(join(directory, "project"));
  const library = new Workspaces(new Preferences(join(directory, "ui.json")), { demo: true });
  const screen = await createTestRenderer({ width: 152, height: 42, useMouse: true });
  let app: App | undefined;
  try {
    const group = await library.add(join(directory, "project"));
    await library.create(group, "foo");
    const second = await library.create(group, "bar");
    if (!second.session) throw new Error("The session did not open.");
    app = new App(screen.renderer, second.session, { quit() {}, workspaces: library });
    await screen.flush();
    const left = 152 - library.preferences.sidebarWidth;
    const lines = screen.captureCharFrame().split("\n");
    const fooRow = lines.findIndex((line) => / foo\b/.test(line.slice(left)));
    const pencil = (lines[fooRow] ?? "").lastIndexOf("✎");
    const cross = (lines[fooRow] ?? "").lastIndexOf("×");
    const mark = left + (lines[fooRow] ?? "").slice(left).indexOf("○");
    const name = (lines[fooRow] ?? "").indexOf("foo");
    const hidden = (x: number) => {
      const cell = cellAt(screen.captureSpans(), x, fooRow);
      return Boolean(cell.fg?.equals(cell.bg));
    };
    const ground = () => cellAt(screen.captureSpans(), name - 2, fooRow).bg;
    expect(pencil).toBeGreaterThan(name);
    expect(cross).toBe(pencil + 2);
    expect(hidden(pencil) && hidden(cross)).toBe(true);
    const rest = ground();
    await screen.mockMouse.moveTo(name + 1, fooRow);
    await screen.flush();
    expect(hidden(pencil) || hidden(cross)).toBe(false);
    expect(ground()?.equals(rest)).toBe(false);
    // A tip stands on the row under the pointer.
    const under = () => screen.captureCharFrame().split("\n")[fooRow + 1] ?? "";
    expect(under()).not.toContain("Ready");
    await screen.mockMouse.moveTo(mark, fooRow);
    await screen.flush();
    expect(under()).toContain("○ Ready");
    await screen.mockMouse.moveTo(pencil, fooRow);
    await screen.flush();
    expect(screen.captureCharFrame()).toContain("Rename this session");
    await screen.mockMouse.moveTo(cross, fooRow);
    await screen.flush();
    expect(screen.captureCharFrame()).toContain("Archive or remove this session");
    await screen.mockMouse.moveTo(10, 10);
    await screen.flush();
    expect(ground()?.equals(rest)).toBe(true);
    expect(hidden(pencil) && hidden(cross)).toBe(true);
    expect(screen.captureCharFrame()).not.toContain("Archive or remove this session");
  } finally {
    app?.dispose();
    screen.renderer.destroy();
    await library.dispose();
    await rm(directory, { recursive: true, force: true });
  }
});

test("a right click on a session row opens its menu at the pointer, and Rename takes a new name in the row", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-rename-"));
  await mkdir(join(directory, "project"));
  const library = new Workspaces(new Preferences(join(directory, "ui.json")), { demo: true });
  const screen = await createTestRenderer({ width: 152, height: 42, useMouse: true });
  let app: App | undefined;
  try {
    const group = await library.add(join(directory, "project"));
    const first = await library.create(group, "foo");
    const second = await library.create(group, "bar");
    if (!first.session || !second.session) throw new Error("The session did not open.");
    app = new App(screen.renderer, second.session, { quit() {}, workspaces: library });
    const left = 152 - library.preferences.sidebarWidth;
    const frame = async () => {
      app?.render();
      await screen.flush();
      return screen.captureCharFrame().split("\n");
    };
    let lines = await frame();
    const fooRow = lines.findIndex((line) => / foo\b/.test(line.slice(left)));
    await screen.mockMouse.click(left + 8, fooRow, 2);
    lines = await frame();
    const menu = lines.slice(fooRow + 1).map((line) => line.slice(left + 8).trim());
    expect(menu.slice(0, 8)).toEqual([
      "",
      "foo        Esc",
      "",
      "❯ Open",
      "Rename",
      "Archive",
      "Move to trash",
      "",
    ]);
    const renameRow = lines.findIndex((line) => line.slice(left).includes("Rename"));
    await screen.mockMouse.click((lines[renameRow] ?? "").indexOf("Rename") + 1, renameRow);
    lines = await frame();
    expect(lines.join("\n")).toContain("Enter keeps the new name, and Esc leaves it as it was.");
    for (let index = 0; index < 3; index++) screen.mockInput.pressBackspace();
    await screen.mockInput.typeText("notes");
    screen.mockInput.pressEnter();
    lines = await frame();
    expect(first.name).toBe("notes");
    expect(savedView(first.path).view.sessionName).toBe("notes");
    expect(lines.some((line) => / notes\b/.test(line.slice(left)))).toBe(true);
    // Escape leaves the name as it was.
    const notesRow = lines.findIndex((line) => / notes\b/.test(line.slice(left)));
    await screen.mockMouse.moveTo(left + 8, notesRow);
    await screen.mockMouse.click((lines[notesRow] ?? "").lastIndexOf("✎"), notesRow);
    await frame();
    await screen.mockInput.typeText(" draft");
    screen.mockInput.pressEscape();
    // A lone Escape reaches the app once the parser knows that no sequence follows it.
    for (let tries = 0; tries < 100 && !app.composer.focused; tries++) await Bun.sleep(10);
    lines = await frame();
    expect(first.name).toBe("notes");
    expect(app.composer.focused).toBe(true);
    // A session that is not open keeps its new name in its saved view.
    await library.archive(first);
    library.rename(first, "old notes");
    expect(savedView(first.path).view.sessionName).toBe("old notes");
  } finally {
    app?.dispose();
    screen.renderer.destroy();
    await library.dispose();
    await rm(directory, { recursive: true, force: true });
  }
});

test("a workspace row renames its workspace in place, and its remove button takes it off the list and leaves its folder", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-group-"));
  await mkdir(join(directory, "project"));
  await mkdir(join(directory, "other"));
  const library = new Workspaces(new Preferences(join(directory, "ui.json")), { demo: true });
  const screen = await createTestRenderer({ width: 152, height: 42, useMouse: true });
  let app: App | undefined;
  try {
    const project = await library.add(join(directory, "project"));
    const other = await library.add(join(directory, "other"));
    const first = await library.create(project, "foo");
    if (!first.session) throw new Error("The session did not open.");
    const options = { quit() {}, workspaces: library };
    app = new App(screen.renderer, first.session, options);
    library.on("select", (session) => {
      app?.dispose();
      app = new App(screen.renderer, session, options);
    });
    const left = 152 - library.preferences.sidebarWidth;
    const frame = async () => {
      app?.render();
      await screen.flush();
      return screen.captureCharFrame().split("\n");
    };
    let lines = await frame();
    const otherRow = lines.findIndex((line) => /▾ other/.test(line.slice(left)));
    await screen.mockMouse.moveTo(left + 5, otherRow);
    await screen.mockMouse.click((lines[otherRow] ?? "").lastIndexOf("✎"), otherRow);
    await frame();
    for (let index = 0; index < 5; index++) screen.mockInput.pressBackspace();
    await screen.mockInput.typeText("docs");
    screen.mockInput.pressEnter();
    lines = await frame();
    expect(other.name).toBe("docs");
    expect(other.collapsed).toBe(false);
    const listed = () =>
      (
        JSON.parse(readFileSync(library.path, "utf8")) as {
          workspaces: { directory: string; name: string }[];
        }
      ).workspaces;
    expect(listed().find((one) => one.directory === other.directory)?.name).toBe("docs");
    const projectRow = lines.findIndex((line) => /▾ project/.test(line.slice(left)));
    await screen.mockMouse.moveTo(left + 5, projectRow);
    await screen.mockMouse.click((lines[projectRow] ?? "").lastIndexOf("×"), projectRow);
    lines = await frame();
    expect(lines.join("\n")).toContain("Remove the workspace “project” from the list?");
    const remove = lines.findIndex((line) => line.includes("Remove from the list"));
    await screen.mockMouse.click((lines[remove] ?? "").indexOf("Remove from the list") + 1, remove);
    await until(library, () => library.groups.length === 1);
    expect(library.groups).toEqual([other]);
    expect(library.groupOf()).toBe(other);
    expect(first.session).toBeUndefined();
    expect(listed().map((one) => one.directory)).toEqual([other.directory]);
    expect((await stat(project.directory)).isDirectory()).toBe(true);
    // The last workspace stays, since the current session needs one.
    await expect(library.remove(other)).rejects.toThrow("This is the only workspace.");
  } finally {
    app?.dispose();
    screen.renderer.destroy();
    await library.dispose();
    await rm(directory, { recursive: true, force: true });
  }
});

test("a rename in a row ends when a dialog opens or another row is clicked, and the dialog and the click still work", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-rename-end-"));
  await mkdir(join(directory, "project"));
  const library = new Workspaces(new Preferences(join(directory, "ui.json")), { demo: true });
  const screen = await createTestRenderer({ width: 152, height: 42, useMouse: true });
  let app: App | undefined;
  try {
    const group = await library.add(join(directory, "project"));
    const first = await library.create(group, "foo");
    const second = await library.create(group, "bar");
    if (!first.session || !second.session) throw new Error("The session did not open.");
    const options = { quit() {}, workspaces: library };
    app = new App(screen.renderer, second.session, options);
    library.on("select", (session) => {
      app?.dispose();
      app = new App(screen.renderer, session, options);
    });
    const left = 152 - library.preferences.sidebarWidth;
    const frame = async () => {
      app?.render();
      await screen.flush();
      return screen.captureCharFrame().split("\n");
    };
    const rename = async (name: string) => {
      const lines = await frame();
      const row = lines.findIndex((line) => new RegExp(` ${name}\\b`).test(line.slice(left)));
      await screen.mockMouse.moveTo(left + 8, row);
      await screen.mockMouse.click((lines[row] ?? "").lastIndexOf("✎"), row);
      await frame();
    };
    // A dialog that opens during a rename keeps the name typed so far, and takes the keys.
    await rename("bar");
    await screen.mockInput.typeText("2");
    app.palette();
    await screen.mockInput.typeText("them");
    await frame();
    expect(second.name).toBe("bar2");
    expect(app.composer.plainText).toBe("");
    expect(screen.captureCharFrame()).toContain("them");
    app.closeOverlay();
    // One click on another row keeps the name and opens that row.
    await rename("bar2");
    await screen.mockInput.typeText("x");
    const lines = await frame();
    const fooRow = lines.findIndex((line) => / foo\b/.test(line.slice(left)));
    await screen.mockMouse.click(left + 8, fooRow);
    await until(library, () => library.current === first);
    expect(second.name).toBe("bar2x");
  } finally {
    app?.dispose();
    screen.renderer.destroy();
    await library.dispose();
    await rm(directory, { recursive: true, force: true });
  }
});

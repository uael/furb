import { expect, test } from "bun:test";
import { mkdir, mkdtemp, readFile, realpath, rm, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, join } from "node:path";
import { RecordLock } from "@furb/engine";
import { createTestRenderer } from "@opentui/core/testing";
import { until } from "../../bind/typescript/test/until.ts";
import { App } from "../src/app.ts";
import { openEngine } from "../src/bridge.ts";
import { seedDemoFiles } from "../src/demo.ts";
import { Preferences } from "../src/preferences.ts";
import { Session } from "../src/session.ts";
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
    expect(await stat(`${first.path}.lock`).catch(() => null)).toBeNull();
    const archived = join(trash, basename(first.path));
    expect((await stat(archived)).isFile()).toBe(true);
    expect((await stat(`${archived}.lock`)).isFile()).toBe(true);
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

test("the list of saved sessions comes before their replays, and a row shows its state when its replay lands", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-inspect-"));
  const preferences = new Preferences(join(directory, "config/ui.json"));
  let library = new Workspaces(preferences, { demo: true });
  try {
    const entry = await library.create(await library.add(directory), "Held");
    await entry.session?.life.wait(60);
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
  let lease: RecordLock | undefined;
  try {
    const held = await library.create(await library.add(directory), "Held elsewhere");
    await library.dispose();
    library = new Workspaces(preferences, { demo: true });
    const group = await library.add(directory);
    const current = await library.create(group, "Delete me");
    lease = new RecordLock(held.path);
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

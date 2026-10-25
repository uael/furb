import { expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { mkdir, mkdtemp, readFile, realpath, rm, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, join } from "node:path";
import { createTestRenderer } from "@opentui/core/testing";
import { type NativeEar, scratch, store } from "furb";
import { until } from "../../bind/typescript/test/until.ts";
import { rasterize } from "../script/raster.ts";
import { click as clickStage } from "../script/stage.ts";
import { type App, follow } from "../src/app.ts";
import { openEngine } from "../src/bridge.ts";
import { seedDemoFiles } from "../src/demo.ts";
import { Preferences } from "../src/preferences.ts";
import { Session, savedView } from "../src/session.ts";
import { paletteOf } from "../src/theme.ts";
import { type SessionEntry, Workspaces } from "../src/workspaces.ts";

/** A library in the sidebar of an App on a test terminal: the terminal, the App that shows the session that the
 * library selects now, the first column of the sidebar, the rows of the frame that the App draws anew, and the row of
 * the sidebar that names a workspace or a session in the frame that the terminal shows now. */
interface Sidebar {
  screen: Awaited<ReturnType<typeof createTestRenderer>>;
  app: () => App;
  left: number;
  frame: () => Promise<string[]>;
  row: (name: string) => number;
}

/** A test in a temporary directory of its own, with the list of the workspaces of the TUI in it. `open` reads the
 * list into a library, as each run of the TUI does, and `sidebar` shows a library in an App, which follows the
 * session that the library selects. All of it ends after the test, with the directory. */
async function withLibrary(
  use: (context: {
    directory: string;
    open: () => Workspaces;
    sidebar: (library: Workspaces, width?: number, height?: number) => Promise<Sidebar>;
  }) => Promise<void>,
): Promise<void> {
  const directory = await mkdtemp(join(tmpdir(), "furb-workspaces-"));
  const libraries: Workspaces[] = [];
  const sidebars: Sidebar[] = [];
  const open = () => {
    const library = new Workspaces(new Preferences(join(directory, "config/ui.json")), { demo: true });
    libraries.push(library);
    return library;
  };
  const sidebar = async (library: Workspaces, width = 152, height = 42): Promise<Sidebar> => {
    const screen = await createTestRenderer({ width, height, useMouse: true, exitSignals: [] });
    const app = follow(screen.renderer, { quit() {}, workspaces: library });
    const left = width - library.preferences.sidebarWidth;
    const shown = {
      screen,
      app,
      left,
      frame: async () => {
        app().render();
        await screen.flush();
        return screen.captureCharFrame().split("\n");
      },
      row: (name: string) =>
        screen
          .captureCharFrame()
          .split("\n")
          .findIndex((line) => new RegExp(` ${name}\\b`).test(line.slice(left))),
    };
    sidebars.push(shown);
    await screen.flush();
    return shown;
  };
  try {
    await use({ directory, open, sidebar });
  } finally {
    for (const { app, screen } of sidebars) {
      app().dispose();
      screen.renderer.destroy();
    }
    for (const library of libraries) await library.dispose();
    await rm(directory, { recursive: true, force: true });
  }
}

/** A library with the workspace `project`, which holds the open sessions `foo` and `bar`. */
async function fooAndBar(directory: string, open: () => Workspaces) {
  await mkdir(join(directory, "project"));
  const library = open();
  const group = await library.add(join(directory, "project"));
  const first = await library.create(group, "foo");
  const second = await library.create(group, "bar");
  if (!first.session || !second.session) throw new Error("The session did not open.");
  return { library, group, first, second };
}

test("workspaces keep sessions alive, report background completion and input, and reopen saved records paused", () =>
  withLibrary(async ({ directory, open }) => {
    await mkdir(join(directory, "alpha"));
    await mkdir(join(directory, "beta"));
    await seedDemoFiles(join(directory, "alpha"));
    await seedDemoFiles(join(directory, "beta"));
    let library = open();
    const alpha = await library.add(join(directory, "alpha"));
    const beta = await library.add(join(directory, "beta"));
    const first = await library.create(alpha, "Checks");
    const second = await library.create(alpha, "Notes");
    const third = await library.create(beta, "Release");
    expect(alpha.sessions).toHaveLength(2);
    expect(beta.sessions).toHaveLength(1);
    if (!first.session || !second.session || !third.session) throw new Error("Sessions did not open.");
    const worker = first.session.engine;
    // The command works until the test feeds it a line, so the library sees it work however late its view comes.
    const command = await worker.bash("read line; printf 'background finished'", {
      stdin: null,
      on: worker.root,
    });
    await until(library, () => first.status === "working");
    expect(library.groupStatus(alpha)).toBe("working");
    expect(library.current).toBe(third);
    await worker.write({ path: `${command}/stdin`, content: "\n" }, { on: worker.root });
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
    const waiting = await first.session.engine.wait([], { seconds: 60, on: first.session.engine.root });
    await until(library, () => first.status === "working");
    library.preferences.sidebar = false;
    library.preferences.save("paper");
    expect(second.session.theme).toBe("paper");
    library.toggle(beta);
    const record = first.path;
    await library.dispose();
    library = open();
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
  }));

test("deleting a session moves its record and state to trash, removes its scratch directory, keeps other sessions alive, and refuses another owner", () =>
  withLibrary(async ({ directory, open }) => {
    const library = open();
    let external: Session | undefined;
    try {
      const group = await library.add(directory);
      const first = await library.create(group, "Remove me");
      const second = await library.create(group, "Keep me");
      if (!first.session || !second.session) throw new Error("Missing live sessions.");
      const word = 'kept_value = 41\nwrite(Text("scratch://plan.md", "plan"))';
      await first.session.engine.result(
        await first.session.engine.rung({ word, on: first.session.engine.root }),
      );
      const notes = scratch(first.path);
      if (!notes) throw new Error("The record has no scratch directory.");
      expect(readFileSync(join(notes, "plan.md"), "utf8")).toBe("plan");
      const otherEngine = second.session.engine;
      const trash = await library.delete(first);
      expect(await stat(notes).catch(() => null)).toBeNull();
      expect(library.current).toBe(second);
      expect(second.session.engine).toBe(otherEngine);
      await otherEngine.result(await otherEngine.rung({ word: "still_alive = 17", on: otherEngine.root }));
      expect((await otherEngine.inspect("still_alive", second.session.selected)).value).toBe(17);
      expect(await stat(first.path).catch(() => null)).toBeNull();
      expect((await stat(`${first.path}.lock`)).size).toBe(0);
      const archived = join(trash, basename(first.path));
      expect((await stat(archived)).isFile()).toBe(true);
      expect(await stat(`${archived}.lock`).catch(() => null)).toBeNull();
      const restored = await library.import(archived, group);
      expect((await restored.session?.engine.inspect("kept_value", restored.session.selected))?.value).toBe(
        41,
      );
      expect(readFileSync(join(scratch(archived) ?? "", "plan.md"), "utf8")).toBe("plan");
      const locked = join(group.directory, ".furb/sessions/locked.jsonl");
      const opened = await openEngine({ cwd: directory, record: locked, demo: true });
      external = new Session(opened.engine, opened.host, true, library.preferences);
      await library.refresh();
      const entry = group.sessions.find((entry) => entry.path === locked);
      if (!entry) throw new Error("The locked session was not listed.");
      const error = await library.delete(entry).catch((error: unknown) => error);
      expect(error).toBeInstanceOf(Error);
      expect(String(error)).toContain("Another process owns");
      expect((await stat(locked)).isFile()).toBe(true);
    } finally {
      await external?.dispose();
    }
  }));

test("the workspace picker switches sessions by mouse and preserves drafts with the sidebar hidden", () =>
  withLibrary(async ({ directory, open, sidebar }) => {
    const { library, first, second } = await fooAndBar(directory, open);
    const use = await sidebar(library);
    const { screen, app, frame } = use;
    app().composer.setText("keep this draft");
    await app().workspacePicker();
    await frame();
    await clickStage(screen, "foo");
    await until(library, () => library.current === first);
    expect(first.session).toBe(app().session);
    await library.select(second);
    await frame();
    expect(app().composer.plainText).toBe("keep this draft");
    screen.mockInput.pressKey("\\", { ctrl: true });
    await frame();
    expect(library.preferences.sidebar).toBe(false);
    expect(app().scroll.x).toBe(2);
    await app().workspacePicker();
    expect((await frame()).join("\n")).toContain("Workspaces and sessions");
    app().closeOverlay();
    screen.resize(82, 32);
    await app().workspacePicker();
    expect((await frame()).join("\n")).toContain("Workspaces and sessions");
    app().closeOverlay();
    expect(app().composer.plainText).toBe("keep this draft");
  }));

test("a session whose operator names no roster offers every model that the catalog offers", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-workspaces-"));
  // A claude command line that the session names is one the catalog offers, and no model is asked.
  const library = new Workspaces(new Preferences(join(directory, "config/ui.json")), {
    claude: join(directory, "claude"),
  });
  try {
    const entry = await library.create(await library.add(directory), "Models");
    const names = entry.session?.roster.map(([name]) => name) ?? [];
    expect(names.slice(0, 4)).toEqual([
      "claude-cli:opus",
      "claude-cli:sonnet",
      "claude-cli:haiku",
      "claude-cli:fable",
    ]);
    expect(names.at(-1)).toBe("operator");
    await entry.session?.submit("/model sonnet");
    expect(entry.session?.actor).toBe("claude-cli:sonnet/high");
    expect(entry.session?.models).toEqual([]);
  } finally {
    await library.dispose();
    await rm(directory, { recursive: true, force: true });
  }
});

test("a model of the catalog that the roster does not hold joins the session, which opens again on it", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-workspaces-"));
  // A claude command line that the session names is one the catalog offers, and no model is asked.
  const library = new Workspaces(new Preferences(join(directory, "config/ui.json")), {
    claude: join(directory, "claude"),
    roster: ["claude-cli:opus"],
  });
  try {
    const entry = await library.create(await library.add(directory), "Models");
    const first = entry.session;
    expect(first?.roster.map(([name]) => name)).toEqual(["claude-cli:opus", "operator"]);
    await until(library, () => first?.catalog.some(([name]) => name === "claude-cli:sonnet") ?? false);
    // A name that neither the roster nor the catalog holds is refused with the models of the roster.
    expect(String(await first?.submit("/model nothing").catch((error: unknown) => error))).toContain(
      "Choose one of claude-cli:opus.",
    );
    await first?.submit("/model sonnet");
    await until(library, () => entry.session !== first && entry.session?.actor === "claude-cli:sonnet/high");
    expect(entry.session?.roster.map(([name]) => name)).toEqual([
      "claude-cli:opus",
      "claude-cli:sonnet",
      "operator",
    ]);
    expect(entry.session?.models).toEqual(["claude-cli:sonnet"]);
  } finally {
    await library.dispose();
    await rm(directory, { recursive: true, force: true });
  }
});

test("the model picker keeps a roster model distinct from the same id in the catalog", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-workspaces-"));
  const config = join(directory, "config");
  await mkdir(config);
  await mkdir(join(directory, ".furb"));
  await writeFile(
    join(config, "config.json"),
    JSON.stringify({
      providers: {
        openai: { key_env: "FURB_MISSING_MODEL_PICKER_KEY", models: { "gpt-6.1-sol": { window: 1000000 } } },
      },
    }),
  );
  await writeFile(
    join(directory, ".furb/config.json"),
    JSON.stringify({
      providers: {
        subscription: { url: "http://127.0.0.1:8741/v1", models: { "gpt-6.1-sol": { window: 500000 } } },
      },
    }),
  );
  const library = new Workspaces(new Preferences(join(config, "ui.json")), {
    config,
    claude: join(directory, "claude"),
    host: false,
    roster: ["claude-cli:opus", "subscription:gpt-6.1-sol"],
  });
  const screen = await createTestRenderer({ width: 130, height: 44, exitSignals: [] });
  let app: (() => App) | undefined;
  try {
    const entry = await library.create(await library.add(directory), "Models");
    const session = entry.session;
    if (!session) throw new Error("The session did not open.");
    await until(library, () => session.catalogRead);
    app = follow(screen.renderer, { workspaces: library, quit() {} });
    app().models();
    await screen.flush();
    await screen.mockInput.typeText("gpt-6.1-sol");
    screen.mockInput.pressEnter();
    await until(
      screen.renderer,
      () =>
        session.actor === "subscription:gpt-6.1-sol" ||
        screen.captureCharFrame().includes("Could not complete action"),
      "frame",
    );
    expect(screen.captureCharFrame()).not.toContain("Could not complete action");
    expect(session.actor).toBe("subscription:gpt-6.1-sol");
  } finally {
    app?.().dispose();
    screen.renderer.destroy();
    await library.dispose();
    await rm(directory, { recursive: true, force: true });
  }
});

test("a provider added during a life can be selected by its full model name", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-workspaces-"));
  const library = new Workspaces(new Preferences(join(directory, "config/ui.json")), {
    claude: join(directory, "claude"),
  });
  try {
    const entry = await library.create(await library.add(directory), "Providers");
    const session = entry.session;
    if (!session) throw new Error("The session did not open.");
    await session.submit("/provider add subscription http://127.0.0.1:8741/v1 gpt-6.1-sol 500000");
    expect(session.roster.map(([name]) => name)).toContain("subscription:gpt-6.1-sol");
    session.host.model = async () => null;
    await session.submit("/model subscription:gpt-6.1-sol");
    expect(session.actor).toBe("subscription:gpt-6.1-sol");
  } finally {
    await library.dispose();
    await rm(directory, { recursive: true, force: true });
  }
});

test("the list of saved sessions comes before their replays, and a row shows its state when its replay lands", () =>
  withLibrary(async ({ directory, open }) => {
    let library = open();
    const entry = await library.create(await library.add(directory), "Held");
    const engine = entry.session?.engine;
    await engine?.wait([], { seconds: 60, on: engine.root });
    await library.dispose();
    library = open();
    const row = (await library.add(directory)).sessions.find((candidate) => candidate.path === entry.path);
    // The replay runs in a worker that loads the native engine first, so the list is there before it.
    expect(row?.status).toBe("saved");
    await until(library, () => row?.status === "paused");
  }));

test("a session that opens keeps its row through a refresh that comes before its record", () =>
  withLibrary(async ({ directory, open }) => {
    const library = open();
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
  }));

test("a session whose record a build of another version made opens to read before any replay, and names both versions", () =>
  withLibrary(async ({ directory, open }) => {
    let library = open();
    const made = await library.create(await library.add(directory), "Made elsewhere");
    await library.dispose();
    const text = await readFile(made.path, "utf8");
    const found = /\["made","chain1","store","([0-9a-f]{64})",(\d+)\]/.exec(text);
    if (!found?.[1] || !found[2]) throw new Error("The record names no build.");
    const [named, engine, version] = [found[0], found[1], Number(found[2])];
    const other = text.replace(named, `["made","chain1","store","${"0".repeat(64)}",${version + 1}]`);
    await writeFile(made.path, other);
    library = open();
    const group = await library.add(directory);
    const row = group.sessions.find((entry) => entry.path === made.path);
    if (!row) throw new Error("No row for the record.");
    const said = `was made by a build of version ${version + 1}, and this build is of version ${version}, so a replay would drift. Open the record with the build of version ${version + 1}, which stands at `;
    const then = ` once a build of that version ran on this machine, and which script/builds.sh ${version + 1} makes there. Or start a new life.`;
    await library.select(row);
    const reading = row.session?.host.reading;
    expect(reading?.made).toBe("0".repeat(64));
    expect(reading?.engine).toBe(engine);
    expect(reading?.version).toBe(version + 1);
    expect(reading?.current).toBe(version);
    expect(reading?.why).toStartWith("Drift: the record ");
    expect(reading?.why).toContain(said);
    expect(reading?.why).toEndWith(then);
    expect(row.error).toBeUndefined();
    expect(await readFile(made.path, "utf8")).toBe(other);
  }));

test("deleting the current session moves to the next session that opens, and to a new one when none does", () =>
  withLibrary(async ({ directory, open }) => {
    let library = open();
    let lease: NativeEar | undefined;
    try {
      const held = await library.create(await library.add(directory), "Held elsewhere");
      await library.dispose();
      library = open();
      const group = await library.add(directory);
      const current = await library.create(group, "Delete me");
      lease = store(held.path)[1];
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
    }
  }));

test("the workspace list keeps what each instance saves, and a list that cannot be read stays for the user to repair", () =>
  withLibrary(async ({ directory, open }) => {
    const path = join(directory, "config/workspaces.json");
    const listed = async () =>
      (
        JSON.parse(await readFile(path, "utf8")) as {
          workspaces: { directory: string; collapsed: boolean }[];
        }
      ).workspaces;
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
  }));

test("a save of the workspace list that fails names its file", () =>
  withLibrary(async ({ directory, open }) => {
    const library = open();
    await mkdir(join(directory, "project"));
    // A directory where the save writes its file makes the save fail on every system.
    await mkdir(join(directory, "config/workspaces.json.tmp"), { recursive: true });
    expect(String(await library.add(join(directory, "project")).catch((error: unknown) => error))).toContain(
      "workspaces.json.tmp",
    );
  }));

/** Open the actions of a visible picker row, through the same key as a terminal user. */
async function actions(use: Sidebar, name: string): Promise<void> {
  await use.app().workspacePicker();
  await use.frame();
  await use.screen.mockInput.typeText(name);
  await use.frame();
  use.screen.mockInput.pressArrow("right");
  await use.frame();
}
async function pick(use: Sidebar, name: string): Promise<void> {
  await use.frame();
  await clickStage(use.screen, name);
  await use.frame();
}

test("the picker archives and restores a session without losing its row or its saved archive state", () =>
  withLibrary(async ({ directory, open, sidebar }) => {
    const { library, first, second } = await fooAndBar(directory, open);
    const use = await sidebar(library);
    await actions(use, "foo");
    await pick(use, "Archive");
    await until(library, () => first.archived === true);
    expect(library.current).toBe(second);
    const saved = JSON.parse(await readFile(library.path, "utf8")) as {
      workspaces: { archived: string[] }[];
    };
    expect(saved.workspaces[0]?.archived).toEqual([first.path]);
    await use.app().workspacePicker();
    const shown = (await use.frame()).join("\n");
    expect(shown).toContain("foo");
    expect(shown).toContain("Archived");
    await actions(use, "foo");
    await pick(use, "Restore");
    expect(first.archived).toBe(false);
    await use.app().workspacePicker();
    await pick(use, "foo");
    await until(library, () => library.current === first);
  }));

test("the picker exposes session actions by mouse and by keyboard without changing its session", () =>
  withLibrary(async ({ directory, open, sidebar }) => {
    const { library, second } = await fooAndBar(directory, open);
    const use = await sidebar(library);
    await use.app().workspacePicker();
    await use.screen.mockInput.typeText("foo");
    let lines = await use.frame();
    const y = lines.findIndex((line) => line.includes("foo") && line.includes("⋯"));
    const x = (lines[y] ?? "").indexOf("foo");
    await use.screen.mockMouse.moveTo(x, y);
    await use.screen.mockMouse.click(x, y, 2);
    lines = await use.frame();
    for (const label of ["Open", "Rename", "Archive", "Move to trash"])
      expect(lines.join("\n")).toContain(label);
    expect(library.current).toBe(second);
    use.app().closeOverlay();
    await actions(use, "foo");
    expect((await use.frame()).join("\n")).toContain("Rename");
    expect(library.current).toBe(second);
  }));

test("Rename from the picker keeps its input focus, saves the name, and Escape cancels a second rename", () =>
  withLibrary(async ({ directory, open, sidebar }) => {
    const { library, first } = await fooAndBar(directory, open);
    const use = await sidebar(library);
    use.app().composer.setText("unsent draft");
    await actions(use, "foo");
    await pick(use, "Rename");
    await use.screen.mockInput.typeText(" notes");
    await use.frame();
    use.app().render();
    await use.frame();
    expect(use.app().composer.focused).toBe(false);
    use.screen.mockInput.pressEnter();
    await until(library, () => first.name === "foo notes");
    expect(savedView(first.path).view.sessionName).toBe("foo notes");
    await actions(use, "foo notes");
    await pick(use, "Rename");
    await use.screen.mockInput.typeText(" draft");
    use.screen.mockInput.pressEscape();
    await until(use.screen.renderer, () => use.app().composer.focused, "frame");
    expect(first.name).toBe("foo notes");
    expect(use.app().composer.plainText).toBe("unsent draft");
    await library.archive(first);
    library.rename(first, "old notes");
    expect(savedView(first.path).view.sessionName).toBe("old notes");
  }));

test("Move to trash from the picker and /delete both ask before moving a session", () =>
  withLibrary(async ({ directory, open, sidebar }) => {
    const { library, group, first } = await fooAndBar(directory, open);
    const use = await sidebar(library);
    await actions(use, "foo");
    await pick(use, "Move to trash");
    expect((await use.frame()).join("\n")).toContain("Remove the session “foo”?");
    expect(group.sessions).toContain(first);
    use.app().closeOverlay();
    use.app().composer.setText("/delete");
    await use.app().submit();
    await use.screen.mockInput.typeText("foo");
    use.screen.mockInput.pressEnter();
    await use.frame();
    expect((await use.frame()).join("\n")).toContain("Remove the session “foo”?");
    await pick(use, "Move to trash");
    await until(library, () => !group.sessions.includes(first));
  }));

test("workspace actions rename and remove a workspace while its folder stays on disk", () =>
  withLibrary(async ({ directory, open, sidebar }) => {
    await mkdir(join(directory, "project"));
    await mkdir(join(directory, "other"));
    const library = open();
    const project = await library.add(join(directory, "project"));
    const other = await library.add(join(directory, "other"));
    const first = await library.create(project, "foo");
    if (!first.session) throw new Error("The session did not open.");
    const use = await sidebar(library);
    await actions(use, "other");
    await pick(use, "Rename");
    await use.screen.mockInput.typeText(" docs");
    use.screen.mockInput.pressEnter();
    await until(library, () => other.name === "other docs");
    const listed = () =>
      (
        JSON.parse(readFileSync(library.path, "utf8")) as {
          workspaces: { directory: string; name: string }[];
        }
      ).workspaces;
    expect(listed().find((one) => one.directory === other.directory)?.name).toBe("other docs");
    await actions(use, "project");
    await pick(use, "Remove from the list");
    expect((await use.frame()).join("\n")).toContain("Remove the workspace “project” from the list?");
    await pick(use, "Remove from the list");
    await until(library, () => library.groups.length === 1);
    expect(library.groups).toEqual([other]);
    expect(library.groupOf()).toBe(other);
    expect(first.session).toBeUndefined();
    expect(listed().map((one) => one.directory)).toEqual([other.directory]);
    expect((await stat(project.directory)).isDirectory()).toBe(true);
    await expect(library.remove(other)).rejects.toThrow("This is the only workspace.");
  }));

test("a dialog opened during Rename keeps the name and receives the keys without changing the composer draft", () =>
  withLibrary(async ({ directory, open, sidebar }) => {
    const { library, second } = await fooAndBar(directory, open);
    const use = await sidebar(library);
    use.app().composer.setText("keep draft");
    await actions(use, "bar");
    await pick(use, "Rename");
    await use.screen.mockInput.typeText("2");
    use.app().palette();
    await use.screen.mockInput.typeText("them");
    await use.frame();
    expect(second.name).toBe("bar2");
    expect(use.app().composer.plainText).toBe("keep draft");
    expect(use.screen.captureCharFrame()).toContain("them");
  }));

for (const [width, height] of [
  [80, 24],
  [160, 48],
] as const) {
  test(`workspace controls with many sessions at ${width}x${height}`, () =>
    withLibrary(async ({ directory, open, sidebar }) => {
      const library = open();
      for (const folder of ["One", "Two", "Three"]) {
        await mkdir(join(directory, folder));
        const group = await library.add(join(directory, folder));
        for (let index = 1; index <= 4; index++) await library.create(group, `${folder} session ${index}`);
      }
      const current = library.current;
      const session = current?.session;
      if (!session || !current) throw new Error("No scripted session.");
      const question = await session.engine.prompt("bool", {
        on: session.engine.root,
        to: "operator",
        message: "Keep this exact question open while renaming",
      });
      await until(session.host, () => session.host.prompts.has(question));
      await session.refresh();
      await session.point(question);
      const use = await sidebar(library, width, height);
      const { screen, app, frame } = use;
      const capture = async (name: string) => {
        await screen.mockMouse.moveTo(0, height - 1);
        const text = (await frame()).join("\n");
        const folder = "/tmp/furb-tui-sidebar-evidence";
        mkdirSync(folder, { recursive: true });
        writeFileSync(join(folder, `${name}-${width}x${height}.txt`), text);
        writeFileSync(
          join(folder, `${name}-${width}x${height}.png`),
          rasterize(screen.captureSpans(), paletteOf(session.theme), "furb"),
        );
        return text;
      };
      await app().workspacePicker();
      expect(await capture("workspaces")).toContain("Workspaces and sessions");
      await actions(use, current.name);
      expect(await capture("session-menu")).toContain("Move to trash");
      await pick(use, "Rename");
      await screen.mockInput.typeText(" renamed");
      expect(await capture("session-rename")).toContain("Rename session");
      expect(app().composer.focused).toBe(false);
      expect(session.operatorPrompt?.id).toBe(question);
      screen.mockInput.pressEnter();
      await until(library, () => current.name.endsWith(" renamed"));
      await actions(use, current.name);
      await pick(use, "Archive");
      await until(library, () => current.archived === true);
      await app().workspacePicker();
      await screen.mockInput.typeText(current.name);
      expect(await capture("archived-session")).toContain("Archived");
      await actions(use, current.name);
      expect(await capture("restore-menu")).toContain("Restore");
      await pick(use, "Restore");
      expect(current.archived).toBe(false);
      await actions(use, current.name);
      await pick(use, "Move to trash");
      expect(await capture("trash-confirm")).toContain("Remove the session");
      expect(library.groups.flatMap((group) => group.sessions)).toContain(current);
      app().closeOverlay();
      expect(session.operatorPrompt?.id).toBe(question);
    }));
}

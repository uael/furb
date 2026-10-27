import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { mkdir, mkdtemp, readFile, realpath, rm, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, join } from "node:path";
import { createTestRenderer } from "@opentui/core/testing";
import { type NativeEar, scratch, store } from "furb";
import { until } from "../../bind/typescript/test/until.ts";
import { type App, follow } from "../src/app.ts";
import { openEngine } from "../src/bridge.ts";
import { seedDemoFiles } from "../src/demo.ts";
import { Preferences } from "../src/preferences.ts";
import { Session, savedView, statusLabels } from "../src/session.ts";
import { type SessionEntry, Workspaces } from "../src/workspaces.ts";
import { cellAt } from "./composing.ts";

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
    sidebar: (library: Workspaces) => Promise<Sidebar>;
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
  const sidebar = async (library: Workspaces): Promise<Sidebar> => {
    const screen = await createTestRenderer({ width: 152, height: 42, useMouse: true, exitSignals: [] });
    const app = follow(screen.renderer, { quit() {}, workspaces: library });
    const left = 152 - library.preferences.sidebarWidth;
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

test("the sidebar tree groups sessions, switches by mouse, collapses and toggles without losing a draft", () =>
  withLibrary(async ({ directory, open, sidebar }) => {
    const { library, group, first, second } = await fooAndBar(directory, open);
    const { screen, app, left, row } = await sidebar(library);
    app().composer.setText("keep this draft");
    await screen.flush();
    const projectRow = row("project");
    const fooRow = row("foo");
    const barRow = row("bar");
    expect(fooRow).toBeGreaterThan(projectRow);
    expect(barRow).toBeGreaterThan(projectRow);
    await screen.mockMouse.click(left + 6, fooRow);
    await until(library, () => library.current === first);
    await screen.flush();
    expect(first.session).toBe(app().session);
    await library.select(second);
    await screen.flush();
    expect(app().composer.plainText).toBe("keep this draft");
    await screen.mockMouse.click(left + 3, projectRow);
    app().render();
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
    app().render();
    await screen.flush();
    expect(library.preferences.sidebar).toBe(false);
    expect(app().scroll.x).toBe(2);
    expect(app().composer.plainText).toBe("keep this draft");
    // The picker reads the workspaces again before it opens: the test awaits the picker the key opened.
    const picker = app().workspacePicker;
    let opened: Promise<void> | undefined;
    app().workspacePicker = () => {
      opened = picker();
      return opened;
    };
    screen.mockInput.pressKey("w", { ctrl: true });
    await opened;
    await screen.flush();
    expect(screen.captureCharFrame()).toContain("Workspaces and sessions");
    app().closeOverlay();
    library.toggle();
    screen.resize(82, 32);
    app().render();
    await screen.flush();
    expect(app().scroll.x).toBe(2);
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

test("a saved record that a build of another version made shows as read only before and after it opens, and no error", () =>
  withLibrary(async ({ directory, open }) => {
    let library = open();
    const made = await library.create(await library.add(directory), "Made elsewhere");
    await library.dispose();
    const text = await readFile(made.path, "utf8");
    const found = /\["made","chain1","store","([0-9a-f]{64})",(\d+)\]/.exec(text);
    if (!found?.[1] || !found[2]) throw new Error("The record names no build.");
    await writeFile(
      made.path,
      text.replace(found[0], `["made","chain1","store","${"0".repeat(64)}",${Number(found[2]) + 1}]`),
    );
    library = open();
    const row = (await library.add(directory)).sessions.find((entry) => entry.path === made.path);
    if (!row) throw new Error("No row for the record.");
    await until(library, () => row.status === "reading");
    expect(row.error).toBeUndefined();
    expect(statusLabels[row.status]).toBe("Read only");
    await library.select(row);
    expect(row.status).toBe("reading");
    expect(row.error).toBeUndefined();
    expect(library.groupStatus(library.groupOf(row) as never)).toBe("reading");
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

test("a session row archives a session through its remove button, which folds it under Archived until a click brings it back", () =>
  withLibrary(async ({ directory, open, sidebar }) => {
    const { library, first, second } = await fooAndBar(directory, open);
    const { screen, left, frame, row } = await sidebar(library);
    let lines = await frame();
    const fooRow = row("foo");
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
    expect(row("foo")).toBe(-1);
    const folded = lines.findIndex((line) => /▸ +Archived +1/.test(line.slice(left)));
    expect(folded).toBeGreaterThan(0);
    const saved = JSON.parse(await readFile(library.path, "utf8")) as {
      workspaces: { archived: string[] }[];
    };
    expect(saved.workspaces[0]?.archived).toEqual([first.path]);
    await screen.mockMouse.click(left + 4, folded);
    lines = await frame();
    const archivedRow = row("foo");
    expect(archivedRow).toBeGreaterThan(folded);
    await screen.mockMouse.click(left + 8, archivedRow);
    await until(library, () => library.current === first);
    expect(first.archived).toBe(false);
  }));

test("a session row lights under the pointer with its buttons, tells its state on its mark alone, and goes dark when the pointer leaves", () =>
  withLibrary(async ({ directory, open, sidebar }) => {
    const { library } = await fooAndBar(directory, open);
    const { screen, left, row } = await sidebar(library);
    const lines = screen.captureCharFrame().split("\n");
    const fooRow = row("foo");
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
  }));

test("a right click on a session row opens its menu at the pointer, and Rename takes a new name in the row", () =>
  withLibrary(async ({ directory, open, sidebar }) => {
    const { library, first } = await fooAndBar(directory, open);
    const { screen, app, left, frame, row } = await sidebar(library);
    let lines = await frame();
    const fooRow = row("foo");
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
    expect(row("notes")).toBeGreaterThan(0);
    // Escape leaves the name as it was.
    const notesRow = row("notes");
    await screen.mockMouse.moveTo(left + 8, notesRow);
    await screen.mockMouse.click((lines[notesRow] ?? "").lastIndexOf("✎"), notesRow);
    await frame();
    await screen.mockInput.typeText(" draft");
    screen.mockInput.pressEscape();
    // A lone Escape reaches the app once the parser knows that no sequence follows it.
    for (let tries = 0; tries < 100 && !app().composer.focused; tries++) await Bun.sleep(10);
    lines = await frame();
    expect(first.name).toBe("notes");
    expect(app().composer.focused).toBe(true);
    // A session that is not open keeps its new name in its saved view.
    await library.archive(first);
    library.rename(first, "old notes");
    expect(savedView(first.path).view.sessionName).toBe("old notes");
  }));

test("Move to trash in the menu of a session row asks what its remove button asks, as /delete does, and moves nothing before a choice", () =>
  withLibrary(async ({ directory, open, sidebar }) => {
    const { library, group, first } = await fooAndBar(directory, open);
    const { screen, app, left, frame, row } = await sidebar(library);
    await screen.mockMouse.click(left + 8, row("foo"), 2);
    let lines = await frame();
    const menu = lines.findIndex((line) => line.slice(left).includes("Move to trash"));
    await screen.mockMouse.click((lines[menu] ?? "").indexOf("Move to trash") + 1, menu);
    lines = await frame();
    expect(lines.join("\n")).toContain("Remove the session “foo”?");
    expect(group.sessions).toContain(first);
    app().closeOverlay();
    app().composer.setText("/delete");
    await app().submit();
    await screen.mockInput.typeText("foo");
    screen.mockInput.pressEnter();
    lines = await frame();
    expect(lines.join("\n")).toContain("Remove the session “foo”?");
    const trash = lines.findIndex(
      (line) => line.includes("Move to trash") && line.includes("Stop this session"),
    );
    await screen.mockMouse.click((lines[trash] ?? "").indexOf("Move to trash") + 1, trash);
    await until(library, () => !group.sessions.includes(first));
  }));

test("a workspace row renames its workspace in place, and its remove button takes it off the list and leaves its folder", () =>
  withLibrary(async ({ directory, open, sidebar }) => {
    await mkdir(join(directory, "project"));
    await mkdir(join(directory, "other"));
    const library = open();
    const project = await library.add(join(directory, "project"));
    const other = await library.add(join(directory, "other"));
    const first = await library.create(project, "foo");
    if (!first.session) throw new Error("The session did not open.");
    const { screen, left, frame, row } = await sidebar(library);
    let lines = await frame();
    const otherRow = row("other");
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
    const projectRow = row("project");
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
  }));

test("a rename in a row ends when a dialog opens or another row is clicked, and the dialog and the click still work", () =>
  withLibrary(async ({ directory, open, sidebar }) => {
    const { library, first, second } = await fooAndBar(directory, open);
    const { screen, app, left, frame, row } = await sidebar(library);
    const rename = async (name: string) => {
      const lines = await frame();
      const at = row(name);
      await screen.mockMouse.moveTo(left + 8, at);
      await screen.mockMouse.click((lines[at] ?? "").lastIndexOf("✎"), at);
      await frame();
    };
    // A dialog that opens during a rename keeps the name typed so far, and takes the keys.
    await rename("bar");
    await screen.mockInput.typeText("2");
    app().palette();
    await screen.mockInput.typeText("them");
    await frame();
    expect(second.name).toBe("bar2");
    expect(app().composer.plainText).toBe("");
    expect(screen.captureCharFrame()).toContain("them");
    app().closeOverlay();
    // One click on another row keeps the name and opens that row.
    await rename("bar2");
    await screen.mockInput.typeText("x");
    await frame();
    const fooRow = row("foo");
    await screen.mockMouse.click(left + 8, fooRow);
    await until(library, () => library.current === first);
    expect(second.name).toBe("bar2x");
  }));

/** A repository `project` with a commit, and two worktrees of it, `wt-a` and `wt-b`, beside it. */
async function withWorktrees(directory: string) {
  const main = join(directory, "project");
  await mkdir(main);
  const git = (cwd: string, ...args: string[]) => {
    const done = Bun.spawnSync(["git", "-c", "user.name=t", "-c", "user.email=t@t", ...args], { cwd });
    if (done.exitCode !== 0) throw new Error(done.stderr.toString());
  };
  git(main, "init", "-q", "-b", "main");
  await writeFile(join(main, "a.txt"), "a\n");
  git(main, "add", ".");
  git(main, "commit", "-q", "-m", "first");
  const a = join(directory, "wt-a");
  const b = join(directory, "wt-b");
  git(main, "worktree", "add", "-q", "-b", "a", a);
  git(main, "worktree", "add", "-q", "-b", "b", b);
  return { main: await realpath(main), a: await realpath(a), b: await realpath(b) };
}

test("a worktree belongs to the workspace of its main checkout, with its sessions, and is no workspace of its own", () =>
  withLibrary(async ({ directory, open }) => {
    const { main, a, b } = await withWorktrees(directory);
    const library = open();
    const group = await library.add(a);
    expect(group.directory).toBe(main);
    expect(await library.add(b)).toBe(group);
    expect(await library.add(main)).toBe(group);
    expect(library.groups.length).toBe(1);
    const inA = await library.create(group, "in a", undefined, false, a);
    const inB = await library.create(group, "in b", undefined, false, b);
    const inMain = await library.create(group, "in main");
    expect(dirname(inA.path)).toBe(join(a, ".furb/sessions"));
    expect(dirname(inB.path)).toBe(join(b, ".furb/sessions"));
    expect(dirname(inMain.path)).toBe(join(main, ".furb/sessions"));
    expect(inA.session?.host.directory).toBe(a);
    expect(group.sessions).toEqual(expect.arrayContaining([inA, inB, inMain]));
    // A later run reads one workspace, with the three sessions, whichever folder it starts in.
    await library.dispose();
    const later = open();
    expect(later.groups.map((one) => one.directory)).toEqual([main]);
    const again = await later.add(b);
    await later.scan(again);
    expect(again.sessions.map((entry) => entry.path).sort()).toEqual(
      [inA.path, inB.path, inMain.path].sort(),
    );
    // The settings of the workspace are one: the name that the operator gave it reaches it from a worktree.
    later.rename(again, "Furb");
    expect((await later.add(a)).name).toBe("Furb");
  }));

test("a list of an earlier version that holds a worktree as a workspace folds it into its main checkout", () =>
  withLibrary(async ({ directory, open }) => {
    const { main, a, b } = await withWorktrees(directory);
    await mkdir(join(directory, "config"), { recursive: true });
    const one = join(main, ".furb/sessions/one.jsonl");
    const two = join(a, ".furb/sessions/two.jsonl");
    const three = join(b, ".furb/sessions/three.jsonl");
    await writeFile(
      join(directory, "config/workspaces.json"),
      JSON.stringify({
        workspaces: [
          { directory: a, name: "wt-a", collapsed: true, records: [two], archived: [two] },
          { directory: main, name: "project", collapsed: false, records: [one], archived: [] },
          { directory: b, name: "Review", collapsed: true, records: [three], archived: [] },
        ],
      }),
    );
    const library = open();
    expect(library.groups.length).toBe(1);
    const [group] = library.groups;
    expect(group?.directory).toBe(main);
    expect(group?.name).toBe("Review");
    expect(group?.collapsed).toBe(false);
    expect(group?.sessions.map((entry) => [entry.path, entry.archived])).toEqual([
      [two, true],
      [one, false],
      [three, false],
    ]);
  }));

test("the sidebar shows a checkout with its two worktrees as one workspace, and names the worktree of a session", () =>
  withLibrary(async ({ directory, open, sidebar }) => {
    const { a, b } = await withWorktrees(directory);
    const library = open();
    const group = await library.add(a);
    await library.add(b);
    await library.create(group, "main work");
    await library.create(group, "fix", undefined, false, a);
    await library.create(group, "review", undefined, false, b);
    const { frame, left } = await sidebar(library);
    const lines = (await frame()).map((line) => line.slice(left).trimEnd());
    const text = lines.join("\n");
    expect(lines.filter((line) => line.includes("▾ project")).length).toBe(1);
    expect(lines.filter((line) => /▾ /.test(line)).length).toBe(1);
    expect(text).not.toMatch(/▾ wt-/);
    expect(text).toMatch(/fix\s+wt-a/);
    expect(text).toMatch(/review\s+wt-b/);
    expect(text).toMatch(/main work(?!\s+wt-)/);
  }));

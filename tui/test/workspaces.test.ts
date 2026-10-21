import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { mkdir, mkdtemp, readFile, realpath, rm, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, join } from "node:path";
import { type NativeEar, scratch, store } from "@furb/engine";
import { until } from "../../bind/typescript/test/until.ts";
import { openEngine } from "../src/bridge.ts";
import { seedDemoFiles } from "../src/demo.ts";
import { Preferences } from "../src/preferences.ts";
import { Session } from "../src/session.ts";
import { type SessionEntry, Workspaces } from "../src/workspaces.ts";

/** A test in a temporary directory of its own, with the list of the workspaces of the TUI in it. `open` reads the
 * list into a library, as each run of the TUI does. All of it ends after the test, with the directory. */
async function withLibrary(
  use: (context: { directory: string; open: () => Workspaces }) => Promise<void>,
): Promise<void> {
  const directory = await mkdtemp(join(tmpdir(), "furb-workspaces-"));
  const libraries: Workspaces[] = [];
  const open = () => {
    const library = new Workspaces(new Preferences(join(directory, "config/ui.json")), { demo: true });
    libraries.push(library);
    return library;
  };
  try {
    await use({ directory, open });
  } finally {
    for (const library of libraries) await library.dispose();
    await rm(directory, { recursive: true, force: true });
  }
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
    library.preferences.save("paper");
    expect(second.session.theme).toBe("paper");
    library.toggle(beta);
    const record = first.path;
    await library.dispose();
    library = open();
    await library.refresh();
    expect(library.groups.map((group) => group.name)).toEqual(["alpha", "beta"]);
    expect(library.groups[1]?.collapsed).toBe(true);
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

test("a model of the catalog that the roster does not hold joins the session, which opens again on it", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-workspaces-"));
  // A claude command line that the session names is one the catalog offers, and no model is asked.
  const library = new Workspaces(new Preferences(join(directory, "config/ui.json")), {
    claude: join(directory, "claude"),
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

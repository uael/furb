import { existsSync } from "node:fs";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { openEngine } from "./bridge.ts";
import type { Preferences } from "./preferences.ts";
import { Session } from "./session.ts";
import { Workspaces } from "./workspaces.ts";

/** The files of the demo project: a small notes app with the three parts that its answers name. */
const demoFiles: Record<string, string> = {
  "README.md":
    "# Fieldnotes\n\nA small place to keep ideas.\n\n- Capture a thought\n- Find it when it matters\n- Keep everything local\n",
  "package.json": '{\n  "name": "fieldnotes",\n  "type": "module",\n  "scripts": { "test": "bun test" }\n}\n',
  "src/capture.ts":
    'import { save } from "./storage.ts";\n\nexport function capture(text: string): void {\n  save({ text, at: Date.now() });\n}\n',
  "src/search.ts":
    'import { load } from "./storage.ts";\n\nexport function search(query: string) {\n  return load().filter((note) => note.text.includes(query));\n}\n',
  "src/storage.ts":
    "export interface Note {\n  text: string;\n  at: number;\n}\n\nconst notes: Note[] = [];\n\nexport const save = (note: Note) => notes.push(note);\nexport const load = () => notes;\n",
  "test/search.test.ts":
    'import { expect, test } from "bun:test";\nimport { capture } from "../src/capture.ts";\nimport { search } from "../src/search.ts";\n\ntest("search finds a captured note", () => {\n  capture("small ideas");\n  expect(search("ideas")).toHaveLength(1);\n});\n',
};

export async function seedDemoFiles(directory: string): Promise<void> {
  if (existsSync(join(directory, "README.md"))) return;
  for (const [path, content] of Object.entries(demoFiles)) {
    await mkdir(dirname(join(directory, path)), { recursive: true });
    await writeFile(join(directory, path), content, { flag: "wx" });
  }
}

/** The temporary directories that the demos of this process made. */
const temporary = new Set<string>();

/** A new demo project with its files, in a temporary directory that stays until removeDemoDirectories. */
export async function demoDirectory(): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), "furb-demo-"));
  temporary.add(root);
  const directory = join(root, "fieldnotes");
  await seedDemoFiles(directory);
  return directory;
}

/** Remove every temporary directory that a demo of this process made, once no demo of it runs. */
export async function removeDemoDirectories(): Promise<void> {
  for (const root of temporary) await rm(root, { recursive: true, force: true });
  temporary.clear();
}

/** A session of the demo: on a project of its own, a new demo project by default, or on the record of a demo session,
 * in the folder of its record by default, and with the conversation of the demo when it is seeded. */
export async function demoSession({
  seed = false,
  record,
  cwd,
  preferences,
}: {
  seed?: boolean;
  record?: string;
  cwd?: string;
  preferences?: Preferences;
} = {}): Promise<Session> {
  const { engine, host } = await openEngine({
    demo: true,
    record,
    cwd: cwd ?? (record ? undefined : await demoDirectory()),
  });
  const session = new Session(engine, host, true, preferences);
  await session.refresh();
  if (seed) await seedDemo(session);
  return session;
}

/** A library that holds a demo session in the workspace of its directory, with the session selected, as the command
 * line holds the session that it opens. The library keeps its list beside the preferences of the demo. */
export async function demoLibrary(session: Session): Promise<Workspaces> {
  const library = new Workspaces(session.preferences, { demo: true });
  await library.select(library.adopt(session, await library.add(session.host.directory)));
  return library;
}

export async function seedDemo(session: Session): Promise<void> {
  const { engine } = session;
  const on = engine.root;
  await engine.grant({ usd: 2, on });
  const prompt = await engine.prompt("str", {
    message: "Explore this project, run its checks, and suggest a useful next step.",
    on,
  });
  await engine.result(prompt);
  const fork = await engine.chain({ label: "Search shortcut", source: on });
  await engine.result(await engine.rung({ word: 'shortcut = "Ctrl+K"\nquery = "small ideas"', on: fork }));
  await engine.chain({ label: "Review notes" });
  await session.refresh();
}

/** The words of a conductor on the root of the demo: it hires a worker for each item on a chain of its own, and
 * waits for the first that ends, as a conductor loop does. */
export const conducting = `briefs = [
  ("search", "Add the search shortcut and cover it with a test."),
  ("docs", "Think until I cancel."),
  ("ui", "Ask me first, then tidy the layout of the notes."),
  ("gif", "Record the animation of the search."),
]
lanes = {}
for key, brief in briefs:
  lane = chain("w " + key)
  if key == "gif":
    pause(lane)
  lanes[key] = prompt(str, brief, on=lane)
first = await wait(600, *lanes.values())`;

/** A demo life of many workers: the conversation of the demo, then a conductor on the root that hires four workers.
 * One does its item, one thinks until a cancel, one asks the operator, and a pause holds the last before it starts. */
export async function seedConductor(session: Session): Promise<void> {
  const { engine } = session;
  await seedDemo(session);
  await engine.rung({ word: conducting, on: engine.root });
  await session.refresh();
}

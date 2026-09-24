import { existsSync } from "node:fs";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { openEngine } from "./bridge.ts";
import type { Preferences } from "./preferences.ts";
import { Session } from "./session.ts";

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

export async function demoSession(seed = false, preferences?: Preferences): Promise<Session> {
  const { life, world } = await openEngine({ demo: true, cwd: await demoDirectory() });
  const session = new Session(life, world, true, preferences);
  await session.refresh();
  if (seed) await seedDemo(session);
  return session;
}

export async function seedDemo(session: Session): Promise<void> {
  const { life } = session;
  await life.call("grant", [2], { on: life.root });
  const prompt = await life.prompt(
    "str",
    "Explore this project, run its checks, and suggest a useful next step.",
    { on: life.root },
  );
  await life.result(prompt);
  const fork = await life.chain("Search shortcut", life.root);
  await life.result(await life.rung('shortcut = "Ctrl+K"\nquery = "small ideas"', { on: fork }));
  await life.chain("Review notes");
  await session.refresh();
}

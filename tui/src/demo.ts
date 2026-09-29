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
  "README.md": `# Fieldnotes

A small place to keep ideas.

- Capture a thought
- Find it when it matters
- Keep everything local

## Develop

Run the checks with \`bun test\`.
`,
  "package.json": `{
  "name": "fieldnotes",
  "type": "module",
  "scripts": { "test": "bun test", "check": "bun scripts/check.ts", "index": "bun scripts/index.ts" }
}
`,
  "scripts/check.ts": `import { readdirSync, readFileSync } from "node:fs";

// The checks of the project: each test that the files of test/ name, file by file, then how many there are.
const files = readdirSync("test").filter((name) => name.endsWith(".test.ts")).sort();
let count = 0;
for (const file of files) {
  console.log(\`test/\${file}:\`);
  for (const [, name] of readFileSync(\`test/\${file}\`, "utf8").matchAll(/test\\("([^"]+)"/g)) {
    console.log(\`✓ \${name}\`);
    count++;
  }
  console.log("");
}
console.log(\` \${count} pass\\n 0 fail\\nRan \${count} tests across \${files.length} files.\`);
`,
  "scripts/index.ts": `// Build the search index of the notes, one note at a time, as a long task does.
const notes = ["small ideas", "a search shortcut", "keep everything local", "a thought on names", "the release"];
console.log(\`Indexing \${notes.length} notes\`);
for (const [at, note] of notes.entries()) {
  await Bun.sleep(400);
  console.log(\`  \${at + 1}/\${notes.length}  \${note}\`);
}
console.log(\`Indexed \${notes.length} notes.\`);
`,
  "src/capture.ts": `import { save } from "./storage.ts";

/** Save a thought as a note, with the time it was taken. */
export function capture(text: string): void {
  save({ text: text.trim(), at: Date.now() });
}
`,
  "src/search.ts": `import { load, type Note } from "./storage.ts";

/** The notes that hold every word of a query, newest first. */
export function search(query: string): Note[] {
  const words = query.toLowerCase().split(/\\s+/).filter(Boolean);
  return load()
    .filter((note) => words.every((word) => note.text.toLowerCase().includes(word)))
    .sort((a, b) => b.at - a.at);
}
`,
  "src/storage.ts": `export interface Note {
  text: string;
  at: number;
}

const notes: Note[] = [];

export const save = (note: Note) => notes.push(note);
export const load = () => [...notes];
`,
  "test/capture.test.ts": `import { expect, test } from "bun:test";
import { capture } from "../src/capture.ts";
import { load } from "../src/storage.ts";

test("capture keeps the text and its time", () => {
  capture("  a thought  ");
  expect(load().at(-1)?.text).toBe("a thought");
});
`,
  "test/search.test.ts": `import { expect, test } from "bun:test";
import { capture } from "../src/capture.ts";
import { search } from "../src/search.ts";

test("search finds a captured note", () => {
  capture("small ideas");
  expect(search("ideas")).toHaveLength(1);
});

test("search puts the newest note first", () => {
  capture("old idea");
  capture("new idea");
  expect(search("idea")[0]?.text).toBe("new idea");
});
`,
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
  const thread = await engine.thread("str", {
    markdown: "Explore this project, run its checks, and suggest a useful next step.",
    on,
  });
  await engine.result(thread);
  const fork = await engine.chain({ label: "Search shortcut", source: on });
  await engine.result(await engine.rung({ word: 'shortcut = "Ctrl+K"\nquery = "small ideas"', on: fork }));
  await engine.chain({ label: "Review notes" });
  await session.refresh();
}

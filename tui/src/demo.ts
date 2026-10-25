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
  "examples/fieldwork/notes/2026/rounds/01/session/runs/immutable/cache/observations/research/field-notes/summary.md":
    "# Field notes\\n\\nCafé, 東京, and a compass 🧭.\\n",
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
  watch = false,
  record,
  cwd,
  preferences,
}: {
  seed?: boolean;
  /** Whether the operator watches the session, which streams each turn and spends nothing. */
  watch?: boolean;
  record?: string;
  cwd?: string;
  preferences?: Preferences;
} = {}): Promise<Session> {
  const { engine, host } = await openEngine({
    demo: true,
    watch,
    record,
    cwd: cwd ?? (record ? undefined : await demoDirectory()),
  });
  const session = new Session(engine, host, true, preferences);
  session.watching = watch;
  await session.refresh();
  if (seed) await seedDemo(session);
  return session;
}

/** The models that the word of the tour asks, which the roster of the demo holds. */
const asked = ["claude-cli:haiku", "claude-cli:sonnet", "claude-cli:fable"];

/** The life that the operator watches, which plays by itself and asks no model: a first message, then a word that is
 * a program and asks three models at once, then a branch from the point before that word, which goes another way. A
 * pause between two steps lets each one be read. */
export async function tour(session: Session, pause = 1500): Promise<void> {
  const { engine } = session;
  const on = engine.root;
  const wait = () => new Promise<void>((resolve) => setTimeout(resolve, pause));
  await engine.result(await engine.prompt("str", { message: "Read this project and say what it does.", on }));
  await session.refresh();
  // The point after the first answer, which the branch goes back to.
  const point = session.activity.at(-1)?.id;
  await wait();
  const word = [
    "# One word is a program: it asks three models at once, and waits for each answer.",
    `asks = {m: prompt(str, "In one line, what would you change first?", to=m) for m in ${JSON.stringify(asked).replaceAll(",", ", ")}}`,
    "answers = {m: await a for m, a in asks.items()}",
  ].join("\n");
  await engine.result(await engine.rung({ word, on }));
  await session.refresh();
  await wait();
  if (!point) throw new Error("The first answer made no act.");
  // The branch keeps the chain up to the point, and the chain after it stays as it is.
  const branch = await session.rewind(point, "Another way");
  await wait();
  await engine.result(
    await engine.prompt("str", { message: "Add the search shortcut, Ctrl+K.", on: branch }),
  );
  await session.refresh();
  await seedRichDemo(session);
}

/** A library that holds a demo session in the workspace of its directory, with the session selected, as the command
 * line holds the session that it opens. The library keeps its list beside the preferences of the demo. */
export async function demoLibrary(session: Session): Promise<Workspaces> {
  const library = new Workspaces(session.preferences, { demo: true });
  await library.select(library.adopt(session, await library.add(session.host.directory)));
  return library;
}

/**
 * Add a generated, scripted life for gallery and stress checks. Every act goes through the demo engine, so the
 * views read real chains, prompts and file changes rather than a painted frame.
 */
export async function seedRichDemo(session: Session): Promise<void> {
  const { engine } = session;
  const root = engine.root;
  const branches: string[] = [];
  for (let group = 0; group < 12; group++)
    branches.push(
      await engine.chain({ label: `Fieldwork ${String(group + 1).padStart(2, "0")}`, source: root }),
    );

  const longPath =
    "examples/fieldwork/notes/2026/rounds/01/session/runs/immutable/cache/observations/research/field-notes/summary.md";
    "examples/fieldwork/notes/2026/rounds/01/session/runs/immutable/cache/observations/research/field-notes/summary.md";
  const source = [
    "def index_notes(paths):",
    "  found = []",
    "  for path in paths:",
    "    try:",
    "      text = read(path)",
    "      if text.content:",
    "        found.append((path, len(text.lines)))",
    "    except Refused as error:",
    "      found.append((path, str(error)))",
    "  return found",
    "",
    `paths = ["README.md", "src/capture.ts", "src/search.ts", "src/storage.ts", ` +
      `${JSON.stringify(longPath)}]`,
    "rows = index_notes(paths)",
    "cursor = 0",
    "while cursor < len(rows):",
    "  path, detail = rows[cursor]",
    "  if isinstance(detail, int):",
    '    print(f"{path}: {detail} lines")',
    "  else:",
    '    print(f"{path}: {detail}")',
    "  cursor += 1",
    "",
    "for part in range(4):",
    "  try:",
    '    result = await bash("printf \'Fieldwork pass %s\\\\n\' " + str(part) + "")',
    "    if result.code != 0:",
    '      raise Refused("the scripted check failed")',
    "  except Refused as error:",
    "    print(str(error))",
    "",
    'report = Text("FIELDWORK.md", "# Fieldwork\\n\\n")',
    "for path, detail in rows:",
    '  report = report.append(f"- {path}: {detail}\\n")',
    "write(report)",
    "await wait(0.01)",
    'close("The scripted fieldwork report is ready.")',
  ];
  const longProgram = [
    ...source,
    ...Array.from(
      { length: 48 },
      (_, index) =>
        `# Field note ${index + 1}: compare the path, its line count, and the next useful action.`,
    ),
  ].join("\n");
  for (const [index, chain] of branches.entries()) {
    await engine.result(await engine.rung({ word: longProgram, on: chain }));
    await engine.result(
      await engine.rung({
        word: `write(Text("FIELDWORK-${index + 1}.md", ${JSON.stringify(
          `# Fieldwork ${index + 1}\n\nA generated report from the scripted demo.\n`,
        )}))`,
        on: chain,
      }),
    );
  }

  const parents: string[] = [];
  for (const [index, chain] of branches.entries()) {
    const parent = await engine.prompt("str", {
      message: `Review fieldwork ${index + 1}: check the report and name one follow-up.`,
      to: "operator",
      on: chain,
    });
    parents.push(parent);
  }
  const threads = parents.flatMap((parent, group) =>
    Array.from({ length: 10 }, (_, index) =>
      engine.prompt("str", {
        message: `Fieldwork ${group + 1}, question ${index + 1}: what should the next check cover?`,
        to: "operator",
        on: parent,
      }),
    ),
  );
  await Promise.all(threads);
  await session.refresh();
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

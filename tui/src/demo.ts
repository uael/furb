import { existsSync } from "node:fs";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { openEngine } from "./bridge.ts";
import type { Preferences } from "./preferences.ts";
import { Session } from "./session.ts";

export async function seedDemoFiles(directory: string): Promise<void> {
  if (existsSync(join(directory, "README.md"))) return;
  await mkdir(directory, { recursive: true });
  await writeFile(
    join(directory, "README.md"),
    "# Fieldnotes\n\nA small place to keep ideas.\n\n- Capture a thought\n- Find it when it matters\n- Keep everything local\n",
    { flag: "wx" },
  );
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
  await life.grant({ usd: 2, on: life.root });
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

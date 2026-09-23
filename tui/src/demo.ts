import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { type Turn, World } from "@furb/engine";
import { openEngine } from "./bridge.ts";
import type { Preferences } from "./preferences.ts";
import { Workspace } from "./workspace.ts";

export async function createDemoWorld(record?: string): Promise<World> {
  const directory = record
    ? dirname(record)
    : join(await mkdtemp(join(tmpdir(), "furb-demo-")), "fieldnotes");
  if (!record) {
    await mkdir(directory);
    await writeFile(
      join(directory, "README.md"),
      "# Fieldnotes\n\nA small place to keep ideas.\n\n- Capture a thought\n- Find it when it matters\n- Keep everything local\n",
    );
  }
  const world = new World({
    cwd: directory,
    record: record ?? join(directory, "demo.jsonl"),
    answer: async (_actor, _chain, turns, signal): Promise<Turn> => {
      await new Promise<void>((resolve, reject) => {
        const timer = setTimeout(resolve, JSON.stringify(turns).includes("show live progress") ? 1800 : 180);
        signal.addEventListener(
          "abort",
          () => {
            clearTimeout(timer);
            reject(new Error("cancelled"));
          },
          { once: true },
        );
      });
      const code =
        turns.filter((turn) => turn[0] === "assistant").length === 0
          ? 'notes = read("README.md")\ncheck = await bash("printf \'✓ capture\\n✓ search\\n✓ local storage\\n\'")\nclose("## A clear starting point\\nFieldnotes keeps ideas close. The project has three small parts: capture, search, and local storage.\\n\\nAll three checks passed. A useful next step is to add a **search shortcut**, then cover it with a focused test.")'
          : 'close("The next step is ready. Keep the change small, run its checks, and inspect the result here.")';
      return ["assistant", [code], [3240, 184, 2800, 0, 0.0024], null];
    },
  });
  return world;
}

export async function demoWorkspace(seed = false, preferences?: Preferences): Promise<Workspace> {
  const { life, world } = await openEngine({ demo: true });
  const workspace = new Workspace(life, world, true, preferences);
  await workspace.refresh();
  if (seed) await seedDemo(workspace);
  return workspace;
}

export async function seedDemo(workspace: Workspace): Promise<void> {
  const { life } = workspace;
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
  await workspace.refresh();
}

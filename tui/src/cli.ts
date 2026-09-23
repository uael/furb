#!/usr/bin/env bun
import { existsSync } from "node:fs";
import { mkdir, readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { parseArgs } from "node:util";
import { efforts, type WorldOptions } from "@furb/engine";
import { createCliRenderer } from "@opentui/core";
import { App } from "./app.ts";
import { openEngine } from "./bridge.ts";
import { demoWorkspace } from "./demo.ts";
import { Preferences } from "./preferences.ts";
import { sessionChoices } from "./sessions.ts";
import { palettes } from "./theme.ts";
import { Workspace } from "./workspace.ts";

const { values } = parseArgs({
  args: process.argv.slice(2).filter((value, index) => index !== 0 || value !== "--"),
  options: {
    demo: { type: "boolean" },
    help: { type: "boolean", short: "h" },
    cwd: { type: "string" },
    model: { type: "string" },
    effort: { type: "string" },
    record: { type: "string" },
    resume: { type: "string" },
    roster: { type: "string", multiple: true },
  },
});
if (values.help) {
  console.log(`furb-tui [--demo] [--cwd path] [--model provider:model] [--effort ${efforts.join("|")}]
         [--record file.jsonl | --resume file.jsonl] [--roster provider:model ...]

Enter sends a prompt. Shift+Enter adds a line. Ctrl+P opens actions. F1 shows all keys.
The default model is claude-cli:sonnet, through your Claude CLI subscription.
Other providers use pi-ai and its environment credentials.`);
  process.exit(0);
}
if (values.effort && !efforts.some((effort) => effort === values.effort)) throw new Error("Invalid effort.");
if (values.resume && !existsSync(resolve(values.resume))) throw new Error(`No session at ${values.resume}.`);
const directory = resolve(values.cwd ?? process.cwd());
const sessionsDirectory = resolve(directory, ".furb/sessions");
await mkdir(sessionsDirectory, { recursive: true });
let workspace: Workspace;
const preferences = new Preferences();
const worldOptions: WorldOptions = {
  cwd: values.cwd ? directory : undefined,
  model: values.model,
  effort: values.effort as WorldOptions["effort"],
  roster: values.roster,
};
async function open(record: string): Promise<Workspace> {
  const demo = await readFile(`${record}.ui.json`, "utf8")
    .then((text) => Boolean(JSON.parse(text).demo))
    .catch(() => false);
  const { life, world } = await openEngine({ ...worldOptions, record, demo });
  const result = new Workspace(life, world, demo, preferences);
  await result.refresh();
  return result;
}
workspace = values.demo
  ? await demoWorkspace(false, preferences)
  : await open(
      resolve(
        values.resume ??
          values.record ??
          `${sessionsDirectory}/${new Date().toISOString().replaceAll(":", "-")}.jsonl`,
      ),
    );
const renderer = await createCliRenderer({
  exitOnCtrlC: false,
  backgroundColor: palettes[workspace.theme].background,
  targetFps: 30,
  useMouse: true,
});
let app: App;
let closing = false;
const quit = async () => {
  if (closing) return;
  closing = true;
  app.dispose();
  renderer.destroy();
  await workspace.dispose();
};
const sessions = async () => {
  const location = workspace.world.records.path ? dirname(workspace.world.records.path) : sessionsDirectory;
  return sessionChoices(
    location,
    async (path) => {
      if (path === workspace.world.records.path) return;
      const next = await open(path);
      app.dispose();
      await workspace.dispose();
      workspace = next;
      app = new App(renderer, workspace, { quit, sessions, newSession });
    },
    newSession,
  );
};
const newSession = async () => {
  const next = values.demo
    ? await demoWorkspace(false, preferences)
    : await open(resolve(sessionsDirectory, `${new Date().toISOString().replaceAll(":", "-")}.jsonl`));
  app.dispose();
  await workspace.dispose();
  workspace = next;
  app = new App(renderer, workspace, { quit, sessions, newSession });
};
app = new App(renderer, workspace, { quit, sessions, newSession });
process.once("SIGTERM", () => {
  void quit();
});

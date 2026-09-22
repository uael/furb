#!/usr/bin/env bun
import { existsSync } from "node:fs";
import { mkdir, readdir, readFile, stat } from "node:fs/promises";
import { basename, dirname, resolve } from "node:path";
import { parseArgs } from "node:util";
import type { WorldOptions } from "@furb/engine";
import { createCliRenderer } from "@opentui/core";
import { App } from "./app.ts";
import { openEngine } from "./bridge.ts";
import { demoWorkspace } from "./demo.ts";
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
  console.log(`furb-tui [--demo] [--cwd path] [--model provider:model] [--effort low|medium|high|xhigh]
         [--record file.jsonl | --resume file.jsonl] [--roster provider:model ...]

Enter sends a prompt. Shift+Enter adds a line. Ctrl+P opens actions. F1 shows all keys.
The default model is claude-cli:sonnet, through your Claude CLI subscription.
Other providers use pi-ai and its environment credentials.`);
  process.exit(0);
}
if (values.effort && !["minimal", "low", "medium", "high", "xhigh"].includes(values.effort))
  throw new Error("Invalid effort.");
if (values.resume && !existsSync(resolve(values.resume))) throw new Error(`No session at ${values.resume}.`);
const directory = resolve(values.cwd ?? process.cwd());
const sessionsDirectory = resolve(directory, ".furb/sessions");
await mkdir(sessionsDirectory, { recursive: true });
let workspace: Workspace;
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
  const result = new Workspace(life, world, demo);
  await result.refresh();
  return result;
}
workspace = values.demo
  ? await demoWorkspace()
  : await open(
      resolve(
        values.resume ??
          values.record ??
          `${sessionsDirectory}/${new Date().toISOString().replaceAll(":", "-")}.jsonl`,
      ),
    );
const renderer = await createCliRenderer({
  exitOnCtrlC: false,
  backgroundColor: "#101817",
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
  const paths = (await readdir(location))
    .filter((file) => file.endsWith(".jsonl"))
    .sort()
    .reverse();
  const saved = await Promise.all(
    paths.map(async (file) => {
      const path = resolve(location, file);
      const info = await stat(path);
      const metadata = await readFile(`${path}.ui.json`, "utf8")
        .then((text) => JSON.parse(text) as { sessionName?: string; cost?: number })
        .catch(() => ({}) as { sessionName?: string; cost?: number });
      return {
        label: metadata.sessionName ?? basename(file, ".jsonl"),
        detail: `${info.mtime.toLocaleString()}  ·  $${(metadata.cost ?? 0).toFixed(4)}  ·  ${(info.size / 1024).toFixed(1)} KiB`,
        run: async () => {
          if (path === workspace.world.records.path) return;
          // Open the next record first, so a bad record leaves the current workspace available.
          const next = await open(path);
          app.dispose();
          await workspace.dispose();
          workspace = next;
          app = new App(renderer, workspace, { quit, sessions, newSession });
        },
      };
    }),
  );
  return [
    { label: "+ New session", detail: "Start a fresh life in this project", run: newSession },
    ...saved,
  ];
};
const newSession = async () => {
  const next = values.demo
    ? await demoWorkspace()
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

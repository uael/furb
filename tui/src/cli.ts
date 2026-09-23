#!/usr/bin/env bun
import { existsSync } from "node:fs";
import { mkdir, readFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";
import { efforts, furbDirectory, type WorldOptions } from "@furb/engine";
import { createCliRenderer } from "@opentui/core";
import { App } from "./app.ts";
import { demoDirectory, removeDemoDirectories } from "./demo.ts";
import { Extensions } from "./extensions.ts";
import { defaultModel, type EngineOptions } from "./models.ts";
import { Preferences } from "./preferences.ts";
import { sessionChoices } from "./sessions.ts";
import { palettes } from "./theme.ts";
import { Workspaces } from "./workspaces.ts";

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
    extension: { type: "string", multiple: true },
  },
});
if (values.help) {
  console.log(`furb-tui [--demo] [--cwd path] [--model provider:model] [--effort ${efforts.join("|")}]
         [--record file.jsonl | --resume file.jsonl] [--roster provider:model ...]

Enter sends a prompt. Ctrl+J adds a line, and so does Shift+Enter in a terminal with the kitty keyboard
protocol. Ctrl+P opens actions. F1 shows all keys that the terminal sends.
The default model is ${defaultModel}, through your Claude CLI subscription.
Other providers use pi-ai and its environment credentials.`);
  process.exit(0);
}
if (values.effort && !efforts.some((effort) => effort === values.effort)) throw new Error("Invalid effort.");
if (values.resume && !existsSync(resolve(values.resume))) throw new Error(`No session at ${values.resume}.`);
const preferences = new Preferences();
const record = values.resume ?? values.record;
const savedDirectory =
  record && !values.cwd
    ? await readFile(`${resolve(record)}.world.json`, "utf8")
        .then((text) => JSON.parse(text)?.options?.cwd as string | undefined)
        .catch(() => undefined)
    : undefined;
const directory =
  values.demo && !values.cwd && !record
    ? await demoDirectory()
    : resolve(values.cwd ?? savedDirectory ?? process.cwd());
if (values.demo) await mkdir(directory, { recursive: true });
const worldOptions: EngineOptions = {
  model: values.model,
  effort: values.effort as WorldOptions["effort"],
  roster: values.roster,
  demo: values.demo,
};
const library = new Workspaces(
  preferences,
  worldOptions,
  values.demo ? join(furbDirectory(directory), "workspaces.json") : undefined,
);
const group = await library.add(directory);
await library.refresh();
if (record) await library.import(record, group);
else await library.create(group);
const initial = library.current?.session;
if (!initial) throw new Error("The session did not open.");
const extensions = new Extensions(() => {
  const session = library.current?.session;
  if (!session) throw new Error("No session is selected.");
  return {
    life: session.life,
    chain: session.selected,
    directory: session.workingDirectory,
    notify: (message) => {
      session.notice = message;
    },
    submit: (message) => session.submit(message),
  };
});
try {
  for (const path of values.extension ?? []) await extensions.load(resolve(path));
} catch (error) {
  await library.dispose();
  throw error;
}
const renderer = await createCliRenderer({
  exitOnCtrlC: false,
  // The TUI ends on a signal by its own quit, since the view saves its draft before the renderer goes.
  exitSignals: [],
  backgroundColor: palettes[initial.theme].background,
  targetFps: 30,
  useMouse: true,
});
let app: App;
let closing = false;
/** Each step runs whatever the steps before it came to, so that the sessions close even when the terminal is gone. */
const quit = async () => {
  if (closing) return;
  closing = true;
  const failures: unknown[] = [];
  for (const step of [
    () => app.dispose(),
    () => renderer.destroy(),
    () => extensions.dispose(),
    () => library.dispose(),
    () => removeDemoDirectories(),
  ])
    try {
      await step();
    } catch (error) {
      failures.push(error);
    }
  for (const error of failures) console.error(error);
  if (failures.length) process.exitCode = 1;
};
const newSession = async () => {
  await library.create();
};
const sessions = async () => {
  await library.refresh();
  const group = library.groupOf();
  return sessionChoices(group, (entry) => library.select(entry), newSession);
};
const options = { quit, sessions, newSession, workspaces: library, extensions };
app = new App(renderer, initial, options);
library.on("select", (session) => {
  if (app.session === session) return;
  app.dispose();
  app = new App(renderer, session, options);
});
for (const signal of ["SIGINT", "SIGTERM", "SIGHUP", "SIGQUIT"] as const)
  process.once(signal, () => void quit().finally(() => process.exit()));

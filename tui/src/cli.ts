#!/usr/bin/env bun
import { existsSync } from "node:fs";
import { mkdir, readFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";
import { efforts, furbDirectory, onConsoleEnd } from "@furb/engine";
import { createCliRenderer } from "@opentui/core";
import { follow } from "./app.ts";
import { demoDirectory, removeDemoDirectories } from "./demo.ts";
import type { EngineOptions } from "./models.ts";
import { isLifeView, lifeViews } from "./overview.ts";
import { Preferences } from "./preferences.ts";
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
    view: { type: "string" },
  },
});
if (values.help) {
  console.log(`furb-tui [--demo] [--cwd path] [--model provider:model] [--effort ${efforts.join("|")}]
         [--record file.jsonl | --resume file.jsonl] [--roster provider:model ...] [--view main|loom|graph]

Enter sends a prompt. ⌃J (Control J) adds a line, and so does ⇧Enter (Shift Enter) in a terminal with the kitty
keyboard protocol. ⌃P opens actions. F1 shows all keys that the terminal sends. ⌃ is Control, ⌥ is Option or Alt,
and ⇧ is Shift.
The default model is the one that the catalog offers first: opus, through your Claude CLI subscription, when this
machine holds the claude command line, then a model of a provider of the network, then a model of the local server.
A session offers every model of the claude command line, of a provider whose credentials stand in the environment,
such as ANTHROPIC_API_KEY, and of the local server that FURB_LOCAL_URL names, each named as provider:model, and
/model lists them. With --roster, a session offers its model and the models of --roster, and a model of the list
of /model joins the session when you choose it.
--view shows the life as the board of main, the loom of its chains in time, or the graph of its programs. ⌥V
switches the view at run time, and the setting keeps the last one.`);
  process.exit(0);
}
if (values.view && !isLifeView(values.view)) throw new Error(`Choose a view of ${lifeViews.join(", ")}.`);
if (values.effort && !efforts.some((effort) => effort === values.effort)) throw new Error("Invalid effort.");
if (values.resume && !existsSync(resolve(values.resume))) throw new Error(`No session at ${values.resume}.`);
const preferences = new Preferences();
if (isLifeView(values.view)) preferences.view = values.view;
const record = values.resume ?? values.record;
const savedDirectory =
  record && !values.cwd
    ? await readFile(`${resolve(record)}.session.json`, "utf8")
        .then((text) => JSON.parse(text)?.options?.cwd as string | undefined)
        .catch(() => undefined)
    : undefined;
const directory =
  values.demo && !values.cwd && !record
    ? await demoDirectory()
    : resolve(values.cwd ?? savedDirectory ?? process.cwd());
if (values.demo) await mkdir(directory, { recursive: true });
const engineOptions: EngineOptions = {
  model: values.model,
  effort: values.effort,
  roster: values.roster,
  demo: values.demo,
};
const library = new Workspaces(
  preferences,
  engineOptions,
  values.demo ? join(furbDirectory(directory), "workspaces.json") : undefined,
);
const group = await library.add(directory);
await library.refresh();
try {
  if (record) await library.import(record, group);
  else await library.create(group);
  // The demo shows a real life beside its scripted one, recorded from a log, and opens on it.
  const recordedLife = join(import.meta.dir, "..", "fixtures", "conductor.jsonl");
  if (values.demo && !record && !values.cwd && existsSync(recordedLife))
    await library.import(recordedLife, group);
} catch (error) {
  // A life that does not open ends the TUI with what it said, and with no trace of the stack.
  console.error(`furb-tui: ${error instanceof Error ? error.message : String(error)}`);
  await library.dispose();
  process.exit(1);
}
const initial = library.current?.session;
if (!initial) throw new Error("The session did not open.");
const renderer = await createCliRenderer({
  exitOnCtrlC: false,
  // The TUI ends on a signal by its own quit, since the view saves its draft before the renderer goes.
  exitSignals: [],
  backgroundColor: palettes[initial.theme].background,
  targetFps: 30,
  useMouse: true,
});
let closing = false;
/** Each step runs whatever the steps before it came to, so that the sessions close even when the terminal is gone. */
const quit = async () => {
  if (closing) return;
  closing = true;
  const failures: unknown[] = [];
  for (const step of [
    () => app().dispose(),
    () => renderer.destroy(),
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
const app = follow(renderer, { quit, workspaces: library });
for (const signal of ["SIGINT", "SIGTERM", "SIGHUP", "SIGQUIT"] as const)
  process.once(signal, () => void quit().finally(() => process.exit()));
// The runtime gives no signal for Ctrl+Break or for the close of the console of Windows, and ends the process at once.
onConsoleEnd(() => void quit().finally(() => process.exit()));

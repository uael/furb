#!/usr/bin/env bun
import { existsSync } from "node:fs";
import { mkdir, readFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";
import { createCliRenderer, RGBA } from "@opentui/core";
import { efforts, furbDirectory, onConsoleEnd } from "furb";
import { follow } from "./app.ts";
import { demoDirectory, demoSession, removeDemoDirectories, tour } from "./demo.ts";
import type { EngineOptions } from "./models.ts";
import { opening } from "./opening.ts";
import { isLifeView, lifeViews } from "./overview.ts";
import { Preferences } from "./preferences.ts";
import type { Session } from "./session.ts";
import { colorless, fps, own, paletteOf, terminalPalette, themeOf } from "./theme.ts";
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
    "screen-reader": { type: "boolean" },
    "reduced-motion": { type: "boolean" },
  },
});
if (values.help) {
  console.log(`furb-tui [--demo] [--cwd path] [--model provider:model] [--effort ${efforts.join("|")}]
         [--record file.jsonl | --resume file.jsonl] [--roster provider:model ...] [--view main|loom|graph]
         [--screen-reader] [--reduced-motion]

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
switches the view at run time, and the setting keeps the last one.
--screen-reader shows one column of text, with no box, no spinner, and nothing that moves, and --reduced-motion
stops each spinner and each part that moves with time. /access turns each on or off, and the setting keeps it.
NO_COLOR, when it is set and not empty, turns every color off. The theme Terminal, the default, follows the terminal
when it is dark or light. A question that waits while the terminal has no focus shows a notification, or rings the
bell.`);
  process.exit(0);
}
if (values.view && !isLifeView(values.view)) throw new Error(`Choose a view of ${lifeViews.join(", ")}.`);
if (values.effort && !efforts.some((effort) => effort === values.effort)) throw new Error("Invalid effort.");
if (values.resume && !existsSync(resolve(values.resume))) throw new Error(`No session at ${values.resume}.`);
const preferences = new Preferences();
if (isLifeView(values.view)) preferences.view = values.view;
if (values["screen-reader"]) preferences.screenReader = true;
if (values["reduced-motion"]) preferences.reducedMotion = true;
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
const shown = themeOf(preferences.theme, preferences.terminalMode);
const renderer = await createCliRenderer({
  exitOnCtrlC: false,
  // The TUI ends on a signal by its own quit, since the view saves its draft before the renderer goes.
  exitSignals: [],
  backgroundColor: colorless.on ? RGBA.defaultBackground() : paletteOf(shown).background,
  targetFps: fps,
  useMouse: true,
});
// The first frame shows what opens before the record is read, which takes seconds for a long life.
const opened = await opening(renderer, {
  directory,
  record: record && resolve(record),
  name: shown,
  quit: () => {
    renderer.destroy();
    process.exit(130);
  },
});
// The theme that follows the terminal takes the colors that the terminal says, and a terminal that says none, or does
// not answer in time, keeps the theme of its mode. The terminal answers while the life opens.
const palette =
  preferences.theme === "terminal" && !colorless.on
    ? renderer.getPalette({ timeout: 300 }).catch(() => undefined)
    : undefined;
// Each life runs in a host of its own, which outlives the window, but the demo of a project that goes with the window.
const engineOptions: EngineOptions = {
  model: values.model,
  effort: values.effort,
  roster: values.roster,
  demo: values.demo,
  host: !(values.demo && !record && !values.cwd),
};
const library = new Workspaces(
  preferences,
  engineOptions,
  values.demo ? join(furbDirectory(directory), "workspaces.json") : undefined,
);
const group = await library.add(directory);
/** The life that the demo plays, which the window shows first. */
let watched: Session | undefined;
await library.refresh();
try {
  if (record) await library.import(record, group);
  else if (!values.demo || values.cwd) await library.create(group);
  else {
    // The demo holds a real life recorded from a log, which only reads, and opens on a life that plays by itself,
    // which a new user can go on with.
    const recordedLife = join(import.meta.dir, "..", "fixtures", "conductor.jsonl");
    if (existsSync(recordedLife)) await library.import(recordedLife, group);
    watched = await demoSession({ watch: true, cwd: directory, preferences });
    watched.sessionName = "Watch a life";
    await library.show(watched);
  }
} catch (error) {
  // A life that does not open ends the TUI with what it said, and with no trace of the stack.
  renderer.destroy();
  console.error(`furb-tui: ${error instanceof Error ? error.message : String(error)}`);
  await library.dispose();
  process.exit(1);
}
const initial = library.current?.session;
if (!initial) {
  renderer.destroy();
  throw new Error("The session did not open.");
}
if (palette) {
  const colors = await palette;
  own.palette = terminalPalette(colors?.defaultForeground ?? null, colors?.defaultBackground ?? null);
  if (own.palette) renderer.setBackgroundColor(own.palette.background);
}
opened();
let closing = false;
/** Each step runs whatever the steps before it came to, so that the sessions close even when the terminal is gone. The
 * window detaches from each life, and a life in a host of its own goes on, unless end is set, which ends each life. */
const quit = async (end = false) => {
  if (closing) return;
  closing = true;
  const failures: unknown[] = [];
  for (const step of [
    () => app().dispose(),
    () => renderer.destroy(),
    () => library.dispose(end),
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
if (watched) void tour(watched).catch(watched.fail);
for (const signal of ["SIGINT", "SIGTERM", "SIGHUP", "SIGQUIT"] as const)
  process.once(signal, () => void quit().finally(() => process.exit()));
// The runtime gives no signal for Ctrl+Break or for the close of the console of Windows, and ends the process at once.
onConsoleEnd(() => void quit().finally(() => process.exit()));

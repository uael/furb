#!/usr/bin/env bun
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { parseArgs } from "node:util";
import { efforts, type WorldOptions } from "@furb/engine";
import { createCliRenderer } from "@opentui/core";
import { App } from "./app.ts";
import { seedDemoFiles } from "./demo.ts";
import { Extensions } from "./extensions.ts";
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

Enter sends a prompt. Shift+Enter adds a line. Ctrl+P opens actions. F1 shows all keys.
The default model is claude-cli:sonnet, through your Claude CLI subscription.
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
    ? join(await mkdtemp(join(tmpdir(), "furb-demo-")), "fieldnotes")
    : resolve(values.cwd ?? savedDirectory ?? process.cwd());
if (values.demo) await mkdir(directory, { recursive: true });
if (values.demo && !values.cwd && !record) await seedDemoFiles(directory);
const worldOptions: WorldOptions & { demo?: boolean } = {
  model: values.model,
  effort: values.effort as WorldOptions["effort"],
  roster: values.roster,
  demo: values.demo,
};
const library = new Workspaces(
  preferences,
  worldOptions,
  values.demo ? join(directory, ".furb/workspaces.json") : undefined,
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
    directory: session.directory || session.world.directory,
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
  backgroundColor: palettes[initial.theme].background,
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
  try {
    await extensions.dispose();
  } finally {
    await library.dispose();
  }
};
const newSession = async () => {
  await library.create();
};
const sessions = async () => {
  await library.refresh();
  const group = library.groupOf();
  const entries = new Map((group?.sessions ?? []).map((entry) => [entry.path, entry]));
  return sessionChoices(
    dirname(library.current?.path ?? directory),
    async (path) => {
      const entry = entries.get(path);
      if (entry) await library.select(entry);
    },
    newSession,
    entries,
  );
};
const options = { quit, sessions, newSession, workspaces: library, extensions };
app = new App(renderer, initial, options);
library.on("select", (session) => {
  if (app.session === session) return;
  app.dispose();
  app = new App(renderer, session, options);
});
for (const signal of ["SIGTERM", "SIGHUP"] as const)
  process.once(signal, () => {
    void quit();
  });

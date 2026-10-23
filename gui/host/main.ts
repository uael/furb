#!/usr/bin/env bun
// The host of a life for the app, as a program: the core of the app starts it, and a person starts it to show the UI
// in a browser. It prints the address of its socket as its first line, then opens the record or the project that its
// words name, and ends when its stdin ends, as when the app that started it ends, or on a signal.

import { join, resolve } from "node:path";
import { parseArgs } from "node:util";
import { demoDirectory, removeDemoDirectories } from "@furb/tui/src/demo.ts";
import type { EngineOptions } from "@furb/tui/src/models.ts";
import { Preferences } from "@furb/tui/src/preferences.ts";
import { Workspaces } from "@furb/tui/src/workspaces.ts";
import { furbDirectory } from "furb";
import { Host } from "./host.ts";
import { serve } from "./serve.ts";

const { values } = parseArgs({
  options: {
    demo: { type: "boolean" },
    cwd: { type: "string" },
    record: { type: "string" },
    model: { type: "string" },
    effort: { type: "string" },
    roster: { type: "string", multiple: true },
    list: { type: "string" },
    port: { type: "string" },
    "no-extensions": { type: "boolean" },
  },
});
// The demo stands in a project of its own, which a list of its own holds, as in the TUI.
const directory =
  values.demo && !values.cwd && !values.record ? await demoDirectory() : values.cwd && resolve(values.cwd);
const options: EngineOptions = {
  model: values.model,
  effort: values.effort,
  roster: values.roster,
  demo: values.demo,
  // A check opens a record as it stands: it enables no extension that the record does not run.
  ...(values["no-extensions"] ? { extensions: false } : {}),
};
const list =
  values.list ?? (values.demo && directory ? join(furbDirectory(directory), "workspaces.json") : undefined);
const workspaces = new Workspaces(new Preferences(), options, list);
const host = new Host(workspaces);
const served = serve(host, Number(values.port ?? 0));
console.log(JSON.stringify({ url: served.url }));

let ending = false;
const end = async () => {
  if (ending) return;
  ending = true;
  try {
    await host.dispose();
  } finally {
    served.stop();
    await removeDemoDirectories();
    process.exit();
  }
};
for (const signal of ["SIGINT", "SIGTERM", "SIGHUP"] as const) process.once(signal, () => void end());
process.stdin.on("end", () => void end());
process.stdin.resume();

const report = (error: unknown) =>
  console.error(`furb-gui-host: ${error instanceof Error ? error.message : error}`);
await workspaces.refresh().catch(report);
if (values.record) await host.open(values.record).catch(report);
else if (directory) await host.create(directory).catch(report);

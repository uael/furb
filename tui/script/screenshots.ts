import { mkdir, readdir, rm, writeFile } from "node:fs/promises";
import { basename, join, resolve } from "node:path";
import { createTestRenderer } from "@opentui/core/testing";
import { until } from "../../bind/typescript/test/until.ts";
import type { App } from "../src/app.ts";
import { demoSession, removeDemoDirectories, seedConductor, seedDemoFiles } from "../src/demo.ts";
import { loadParsers } from "../src/parsers.ts";
import type { Exit, Session } from "../src/session.ts";
import { palettes } from "../src/theme.ts";
import type { Workspaces } from "../src/workspaces.ts";
import { rasterize } from "./raster.ts";
import { highlighting, home, mount } from "./stage.ts";

// The model of the demo writes slowly, so that its work stays in progress while a capture shows it. The session runs
// in a worker, which reads the environment once, as it starts.
const projects = await home("furb-gallery", { FURB_DEMO_PACE: "200" });
/** A project of the gallery in its home, new, with the files of the demo. */
async function project(name: string): Promise<string> {
  const directory = join(projects, name);
  await rm(directory, { recursive: true, force: true });
  await seedDemoFiles(directory);
  return directory;
}

// FURB_GALLERY_OUT writes the gallery to another folder, and FURB_GALLERY_ONLY draws only the shots whose names it
// matches, so that a change of design is seen in a few seconds.
const output = resolve(process.env.FURB_GALLERY_OUT ?? "docs/screenshots");
const only = process.env.FURB_GALLERY_ONLY ? new RegExp(process.env.FURB_GALLERY_ONLY) : undefined;
await mkdir(output, { recursive: true });
if (!only) for (const name of await readdir(output)) if (name.endsWith(".png")) await rm(join(output, name));
const test = await createTestRenderer({ width: 152, height: 46 });
let session: Session = await demoSession({ cwd: await project("fieldnotes") });
let app!: () => App;
let library!: Workspaces;
/** A session on the stage, under the name of a first session when it has no name of its own. */
async function show(next: Session): Promise<void> {
  session = next;
  if (session.sessionName === basename(session.host.directory)) await session.command("/name Release 0.4");
  ({ library, app } = await mount(session, test.renderer));
}
await show(session);

/** The state of each thread by its label. */
const states = () => {
  app().render();
  return Object.fromEntries(app().loom.threads.map((thread) => [thread.label, thread.state]));
};
/** A condition that the App reads with no event of its own, as its focus after a key. */
async function soon(ready: () => boolean): Promise<void> {
  for (const start = Date.now(); !ready(); await Bun.sleep(20))
    if (Date.now() - start > 30000) throw new Error(`Waited for ${ready}.`);
}
async function capture(name: string): Promise<void> {
  session.notice = "";
  do await session.refresh();
  while (session.loading);
  app().render();
  await test.flush();
  // Code is colored by a parser off the main thread: the frame is taken once every code block is highlighted.
  await Promise.all(highlighting(test.renderer.root));
  await test.flush();
  if (only && !only.test(name)) return;
  await writeFile(`${output}/${name}.png`, rasterize(test.captureSpans(), palettes[session.theme], "furb"));
  // The text of each shot goes to a folder that FURB_GALLERY_TEXT names, to read the gallery without its pictures.
  const texts = process.env.FURB_GALLERY_TEXT;
  if (texts) await writeFile(join(texts, `${name}.txt`), test.captureCharFrame());
}
/** The thread of a label selected in the loom. */
async function pick(label: string): Promise<void> {
  const at = app().loom.threads.findIndex((thread) => thread.label === label);
  app().focus = "loom";
  await app().move(at - app().selectedThread);
}

try {
  await loadParsers();
  await capture("01-welcome");
  await seedConductor(session);
  await until(session, () => {
    const now = states();
    return now["w search"] === "done" && now["w docs"] === "working" && app().loom.owed.length === 1;
  });
  await capture("02-loom");
  app().focus = "thread";
  app().selectedLine = app().lines.findIndex((line) => line.type === "rung" && line.more > 4);
  await capture("03-thread");
  await app().open();
  await soon(() => app().focus === "lens");
  await capture("04-lens");
  app().back();
  test.mockInput.pressTab();
  await soon(() => app().mode === "answer");
  await test.mockInput.typeText("high, then keep the layout");
  await capture("05-answer");
  test.mockInput.pressEnter();
  await until(session, () => {
    app().render();
    return app().loom.owed.length === 0;
  });
  app().focus = "loom";
  await pick("Main");
  test.mockInput.pressArrow("left");
  await soon(() => app().folds.size === 1);
  await capture("06-folded");
  test.mockInput.pressArrow("right");
  await soon(() => app().folds.size === 0);
  await pick("w gif");
  await capture("07-paused");
  await pick("Main");
  const command = await session.engine.bash(
    "printf 'Building the search index...\\n'; sleep 1; printf 'Indexing 3 notes...\\n'; sleep 2; printf 'done\\n'",
    { on: session.selected },
  );
  await until(session, () =>
    Boolean((session.acts.find((act) => act.id === command)?.value as Exit | undefined)?.stdout?.content),
  );
  app().focus = "thread";
  app().selectedLine = Number.MAX_SAFE_INTEGER;
  await capture("08-live-command");
  await session.engine.result(command);
  const progress = await session.engine.prompt("str", {
    message: "show live progress",
    on: session.selected,
  });
  await until(session.host, () =>
    [...session.host.streams.values()].some((stream) => stream.text.length > 70),
  );
  app().selectedLine = Number.MAX_SAFE_INTEGER;
  await capture("09-model-progress");
  await session.engine.result(progress);
  await test.mockInput.typeText(":");
  await soon(() => app().mode === "python");
  await test.mockInput.typeText('notes = read("README.md")');
  await capture("10-python");
  app().word.setText("this is invalid python !!!");
  test.mockInput.pressEnter();
  await until(session, () => session.acts.some((act) => act.run?.status === "failed"));
  app().selectedLine = Number.MAX_SAFE_INTEGER;
  await capture("11-gate-findings");
  test.mockInput.pressEscape();
  await soon(() => app().focus !== "word");
  app().focus = "thread";
  app().selectedLine = app().lines.findIndex((line) => line.type === "prompt" && line.act.by === "operator");
  test.mockInput.pressKey("e");
  await soon(() => app().mode === "door");
  await capture("12-door");
  test.mockInput.pressEscape();
  await soon(() => app().focus !== "word");
  test.mockInput.pressKey("o", { ctrl: true });
  await soon(() => app().focus === "lives");
  await capture("13-lives");
  test.mockInput.pressEscape();
  await soon(() => app().focus === "loom");
  await test.mockInput.typeText("?");
  await soon(() => app().help);
  await capture("14-help");
  await test.mockInput.typeText("?");
  await soon(() => !app().help);
  test.resize(96, 32);
  await capture("15-narrow");
  test.resize(152, 46);
  session.theme = "paper";
  await capture("16-paper");
  session.theme = "midnight";
  await capture("17-midnight");
  session.theme = "github";
  app().focus = "loom";
  await pick("Main");
  for (let step = 0; step < 6; step++) await test.mockInput.typeText("[");
  await soon(() => app().cursor !== undefined);
  await capture("18-time-cursor");
  test.mockInput.pressEscape();
  await soon(() => app().cursor === undefined);
} finally {
  app().dispose();
  test.renderer.destroy();
  await library.dispose();
  await removeDemoDirectories();
}

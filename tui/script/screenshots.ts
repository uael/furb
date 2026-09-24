import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";
import { CodeRenderable, type Renderable } from "@opentui/core";
import { createTestRenderer } from "@opentui/core/testing";
import { until } from "../../bind/typescript/test/until.ts";
import { App } from "../src/app.ts";
import { openEngine } from "../src/bridge.ts";
import { removeDemoDirectories, seedDemo, seedDemoFiles } from "../src/demo.ts";
import { Extensions } from "../src/extensions.ts";
import { loadParsers } from "../src/parsers.ts";
import { Preferences } from "../src/preferences.ts";
import { Session } from "../src/session.ts";
import { sessionChoices } from "../src/sessions.ts";
import { palettes } from "../src/theme.ts";
import { Workspaces } from "../src/workspaces.ts";
import { idle } from "../test/idle.ts";
import { rasterize } from "./raster.ts";

// The gallery runs in a home of its own, so the paths it shows read as the paths of a user do: ~/fieldnotes. The
// runtime reads the home directory once, so the script runs again as a child that starts in that home.
const home = process.env.FURB_GALLERY_HOME;
if (!home) {
  // The home has the same path at each run, so that the transcript, which shows the exact paths, reads the same.
  const gallery = join(tmpdir(), "furb-gallery");
  await rm(gallery, { recursive: true, force: true });
  await mkdir(gallery, { recursive: true });
  try {
    const child = Bun.spawn([process.execPath, import.meta.path], {
      // The model of the demo writes slowly, so that its work stays in progress while a capture shows it. The World
      // runs in a worker, which reads the environment once, as it starts.
      env: {
        ...process.env,
        HOME: gallery,
        USERPROFILE: gallery,
        FURB_GALLERY_HOME: gallery,
        FURB_DEMO_PACE: "200",
      },
      stdio: ["inherit", "inherit", "inherit"],
    });
    process.exitCode = await child.exited;
  } finally {
    await rm(gallery, { recursive: true, force: true });
  }
  process.exit();
}
/** A project of the gallery in its home, new, with the files of the demo. */
async function project(name: string): Promise<string> {
  const directory = join(home ?? "", name);
  await rm(directory, { recursive: true, force: true });
  await seedDemoFiles(directory);
  return directory;
}
/** A new demo session in the project that the conversation of the gallery is about. */
async function demoSession(): Promise<Session> {
  const { life, world } = await openEngine({ demo: true, cwd: await project("fieldnotes") });
  const opened = new Session(life, world, true);
  await opened.refresh();
  return opened;
}

// FURB_GALLERY_OUT writes the gallery to another folder, and FURB_GALLERY_ONLY draws only the shots whose names it
// matches, so that a change of design is seen in a few seconds.
const output = resolve(process.env.FURB_GALLERY_OUT ?? "docs/screenshots");
const only = process.env.FURB_GALLERY_ONLY ? new RegExp(process.env.FURB_GALLERY_ONLY) : undefined;
await mkdir(output, { recursive: true });
let session = await demoSession();
const test = await createTestRenderer({ width: 152, height: 46 });
let app!: App;
let library!: Workspaces;
const extensions = new Extensions(() => ({
  life: session.life,
  chain: session.selected,
  directory: session.directory,
  notify(message) {
    session.notice = message;
  },
  submit: (message) => session.submit(message),
}));
const options = () => ({
  quit() {},
  workspaces: library,
  extensions,
  newSession: async () => {
    await library.create();
  },
  sessions: async () => {
    await library.refresh();
    return sessionChoices(
      library.groupOf(),
      (entry) => library.select(entry),
      async () => {
        await library.create();
      },
    );
  },
});
const followSelection = () =>
  library.on("select", (next: Session) => {
    if (app.session === next) return;
    app.dispose();
    session = next;
    app = new App(test.renderer, session, options());
  });
async function mount(next: Session): Promise<void> {
  session = next;
  if (session.sessionName === basename(session.world.directory)) await session.command("/name Session 1");
  library = new Workspaces(session.preferences, { demo: true });
  const group = await library.add(session.world.directory);
  const entry = library.adopt(session, group);
  await library.select(entry);
  library.preferences.sidebar = true;
  library.preferences.save();
  app = new App(test.renderer, session, options());
  followSelection();
}
await mount(session);
function highlighting(node: Renderable): Promise<void>[] {
  return [
    ...(node instanceof CodeRenderable ? [node.highlightingDone] : []),
    ...node.getChildren().flatMap(highlighting),
  ];
}

/** The shots whose notice tells what the step did, which keep it. The others show no notice of an earlier step. */
const notices = /^(34|36|37|43|44|45|50)-/;
/** The work of the chain completes, and the queue sends what waits in it, until nothing is left to do. */
async function settle(): Promise<void> {
  await session.refresh();
  for (let attempt = 0; attempt < 10; attempt++) {
    const pending = session.activity.filter((act) => !act.done && ["prompt", "rung"].includes(act.kind));
    if (!pending.length && !session.queued.length) break;
    // A queue that waits on no act sends its next message in a moment.
    if (!pending.length) await new Promise((done) => setTimeout(done, 100));
    await Promise.all(pending.map((act) => session.life.result(act.id).catch(() => {})));
    await session.refresh();
  }
}
/** The pointer moves to the corner of the screen after a click, so that no row stays lit by the hover of a click that
 * a later capture does not show. */
async function rest(): Promise<void> {
  await test.mockMouse.moveTo(0, test.renderer.height - 1);
}
async function capture(name: string): Promise<void> {
  if (!notices.test(name)) session.notice = "";
  // A read that a change starts while the last one ends leaves the view loading, so the capture waits for none.
  if (name !== "25-loading" && name !== "27-view-error")
    do await session.refresh();
    while (session.loading);
  app.render();
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

try {
  await loadParsers();
  await capture("01-welcome");
  await seedDemo(session);
  await session.command("/name Explore project");
  await idle(session);
  await session.refresh();
  await capture("02-feed");
  session.show("transcript");
  await capture("03-transcript");
  const written = session.world.changes;
  await session.life.result(
    await session.life.rung(
      'write(read("README.md").append("\\n## Keyboard\\nPress Ctrl+K to find a note.\\n"))',
      { on: session.selected },
    ),
  );
  await until(session.world, () => session.world.changes > written);
  session.show("changes");
  await capture("04-changes");
  session.show("feed");
  app.palette();
  await capture("05-command-palette");
  app.closeOverlay();
  app.models();
  await capture("06-models");
  app.closeOverlay();
  app.effortPicker();
  await capture("07-effort");
  app.closeOverlay();
  const question = await session.life.prompt("bool", "Apply the search shortcut to the main chain?", {
    on: session.selected,
    to: "operator",
  });
  await until(session.world, () => session.world.prompts.has(question));
  await session.refresh();
  await capture("08-operator-question");
  app.question();
  await capture("09-operator-dialog");
  app.closeOverlay();
  await session.world.answer(session.operatorPrompt?.id ?? "", "yes");
  await until(session.world, () => !session.world.prompts.size);
  app.toggleMode();
  app.composer.setText('notes = read("README.md")\nprint(notes.content)');
  await capture("10-python-input");
  await app.inspect("notes");
  await capture("11-value-inspector");
  app.closeOverlay();
  session.theme = "paper";
  await capture("12-light-theme");
  session.theme = "midnight";
  await capture("13-midnight-theme");
  app.themes();
  await capture("14-theme-picker");
  app.closeOverlay();
  session.theme = "github";
  app.chains();
  await capture("15-chains");
  app.closeOverlay();
  app.help();
  await capture("16-help");
  app.closeOverlay();
  test.resize(82, 32);
  await capture("17-narrow");
  test.resize(152, 46);
  app.toggleMode();
  app.composer.setText("");
  const command = await session.life.bash(
    "printf 'Building the search index...\\n'; sleep 1; printf '3 notes indexed.\\n'",
    { on: session.selected },
  );
  await until(session.world, () =>
    session.world.facts.some((fact) => fact[0] === "out" && fact[1] === command),
  );
  await session.refresh();
  app.render();
  await test.flush();
  const commandRow = app.scroll
    .getChildren()
    .find((node) => node.id === command)
    ?.getChildren()[0];
  if (!commandRow) throw new Error("The command row is not visible.");
  await test.mockMouse.click(commandRow.x + 1, commandRow.y);
  await rest();
  app.scroll.scrollTo(app.scroll.scrollHeight);
  await capture("18-live-command");
  await session.life.result(command);
  const progress = await session.life.prompt("str", "show live progress", { on: session.selected });
  // The model has written part of its word, and writes the rest.
  await until(session.world, () =>
    [...session.world.streams.values()].some((stream) => stream.text.length > 70),
  );
  await session.refresh();
  await capture("19-model-progress");
  await session.life.result(progress);
  await session.submit("/run this is invalid python !!!").catch(session.fail);
  await capture("20-gate-findings");
  const record = session.world.records.path;
  if (!record) throw new Error("The demo session has no record.");
  await session.life.wait(60);
  app.dispose();
  await library.dispose();
  const resumed = await openEngine({ record, demo: true });
  session = new Session(resumed.life, resumed.world, true);
  await session.refresh();
  await mount(session);
  await capture("21-paused-resume");
  app.closeOverlay();
  app.openPalette("Sessions", await options().sessions());
  await capture("22-sessions");
  app.closeOverlay();
  await session.world.resume();
  await session.refresh();
  app.rewind();
  await capture("23-rewind-tree");
  app.closeTree();

  app.dispose();
  await library.dispose();
  session = await demoSession();
  await mount(session);
  app.render();
  await test.flush();
  test.mockInput.pressKey("f", { ctrl: true });
  await test.mockInput.typeText("no matching message");
  await capture("24-empty-results");
  test.mockInput.pressEscape();
  // A chain that opens while its turns are read shows that it loads.
  const opened = await session.life.chain("Notes");
  await session.refresh();
  const snapshot = session.world.snapshot.bind(session.world);
  let release = () => {};
  const held = new Promise<void>((resolve) => {
    release = resolve;
  });
  session.world.snapshot = async (...read) => {
    await held;
    return snapshot(...read);
  };
  const opening = session.select(opened);
  await capture("25-loading");
  release();
  await opening;
  session.world.snapshot = snapshot;
  await session.select(session.life.root);
  await session.submit("/read missing-file.txt");
  await until(session, () => session.acts.some((act) => act.run?.status === "failed"));
  await capture("26-error");
  await session.life.write({ path: "preview.txt", content: "A change to inspect.\n" });
  const journal = `${session.world.records.path}.changes.jsonl`;
  const savedJournal = await readFile(journal);
  await writeFile(journal, "{ damaged journal }");
  session.show("changes");
  await session.refresh().catch(session.fail);
  await capture("27-view-error");
  await writeFile(journal, savedJournal);

  app.dispose();
  await library.dispose();
  const projects = home ?? "";
  const notes = await project("fieldnotes"),
    atlas = await project("atlas");
  const preferences = new Preferences(join(projects, "config/ui.json"));
  const archived = await openEngine({
    demo: true,
    cwd: atlas,
    record: join(atlas, ".furb/sessions/archive.jsonl"),
  });
  const archive = new Session(archived.life, archived.world, true, preferences);
  await archive.command("/name Archive");
  await archive.dispose();
  library = new Workspaces(preferences, { demo: true });
  const first = await library.add(notes),
    second = await library.add(atlas);
  const main = await library.create(first, "Implementation");
  const checks = await library.create(first, "Checks");
  const review = await library.create(first, "Review");
  const changelog = await library.create(second, "Changelog");
  const research = await library.create(second, "Research");
  if (!main.session || !checks.session || !review.session || !changelog.session || !research.session)
    throw new Error("The workspace fixtures did not open.");
  await seedDemo(main.session);
  await checks.session.life.bash("printf 'Checking the project...\\n'; sleep 60");
  await review.session.life.prompt("bool", "Apply the new navigation?", { to: "operator" });
  await changelog.session.life.result(
    await changelog.session.life.rung('summary = "Release notes are ready"'),
  );
  await research.session.life.wait(60);
  await research.session.submit("/pause");
  await library.select(main);
  session = main.session;
  app = new App(test.renderer, session, options());
  followSelection();
  await until(
    library,
    () =>
      checks.status === "working" &&
      review.status === "blocked" &&
      changelog.status === "done" &&
      research.status === "paused",
  );
  await capture("28-workspace-tree");
  app.render();
  await test.flush();
  const word = app.scroll
    .getChildren()
    .find((node) => /^rung\d+$/.test(node.id) && app.scroll.viewport.y <= node.y);
  const heading = word?.getChildren()[0];
  if (heading) await test.mockMouse.click(heading.x + 1, heading.y);
  await rest();
  await capture("29-collapsed-rung");
  library.toggle(second);
  await capture("30-collapsed-workspace");
  library.toggle();
  await capture("31-hidden-sidebar");
  library.toggle();
  library.toggle(second);
  await app.workspacePicker();
  await capture("32-workspace-picker");
  app.closeOverlay();
  app.composer.setText("/tree");
  await app.submit();
  await capture("33-session-tree");
  app.closeTree();
  await session.submit("show live progress");
  session.enqueue("Check keyboard navigation after this answer.");
  await capture("34-queued-follow-up");
  app.composer.setText("/queue");
  await app.submit();
  await capture("35-queue-editor");
  app.closeOverlay();
  // The queued message is sent before the next one, so that the feed reads in the same order at each run.
  await settle();
  await session.attachImage(resolve("docs/screenshots/01-welcome.png"));
  app.composer.setText("Review the layout in this image.");
  await capture("36-image-attachment");
  await app.submit();
  await session.refresh();
  const imagePrompt = session.acts.findLast(
    (act) => act.kind === "prompt" && String(act.words[1]).startsWith("Review the layout"),
  );
  if (!imagePrompt) throw new Error("The image prompt was not submitted.");
  await session.life.result(imagePrompt.id);
  await session.refresh();
  await session.submit("/share");
  await capture("37-share-conversation");
  app.closeOverlay();
  app.composer.setText("/delete");
  await app.submit();
  await test.mockInput.typeText("Archive");
  test.mockInput.pressEnter();
  await capture("38-delete-session");
  app.closeOverlay();
  const editor = join(projects, "editor.sh");
  await writeFile(
    editor,
    '#!/bin/sh\nprintf "edited_in_editor = True\\nprint(edited_in_editor)\\n" > "$1"\n',
    { mode: 0o700 },
  );
  const priorEditor = process.env.EDITOR,
    priorVisual = process.env.VISUAL;
  try {
    process.env.EDITOR = `/bin/sh '${editor}'`;
    delete process.env.VISUAL;
    app.toggleMode();
    await app.editDraft();
    if (!app.composer.plainText.includes("edited_in_editor"))
      throw new Error("The external editor did not return its draft.");
    await capture("39-external-editor");
  } finally {
    if (priorEditor === undefined) delete process.env.EDITOR;
    else process.env.EDITOR = priorEditor;
    if (priorVisual === undefined) delete process.env.VISUAL;
    else process.env.VISUAL = priorVisual;
  }
  if (session.mode === "python") app.toggleMode();
  app.composer.setText("");
  app.composer.focus();
  await test.mockInput.typeText("@");
  await session.projectFiles();
  await capture("40-file-picker");
  app.composer.setText("");
  await test.mockInput.typeText("/e");
  await capture("41-slash-suggestions");
  app.composer.setText("");
  app.closeOverlay();
  app.composer.setText(`/extension ${resolve("tui/examples/project-summary.ts")}`);
  await app.submit();
  app.palette();
  await test.mockInput.typeText("Summarize this project");
  await capture("42-extension-command");
  app.closeOverlay();
  await settle();
  await session.submit("/undo");
  await capture("43-undo-message");
  await session.submit("/redo");
  await capture("44-redo-message");
  app.composer.setText("A draft to keep for later");
  app.stash();
  app.composer.setText("A quick question first");
  await capture("45-stash");
  // The stash comes back into the empty input and leaves, and a space after a command whose values are known lists
  // them.
  app.composer.setText("");
  test.mockInput.pressKey("s", { ctrl: true });
  app.composer.setText("");
  await test.mockInput.typeText("/effort ");
  await capture("46-value-suggestions");
  // A word of the model that the gate refused, and that its next word replaced, folds and reads as retried.
  app.composer.setText("");
  await session.submit("Give me an answer in a fence.");
  await settle();
  await capture("47-retried-word");
  // An archived session folds under Archived, and the pointer on a session row shows its remove button.
  const stale = library.groups[1]?.sessions.find((entry) => entry.name === "Archive");
  if (!stale) throw new Error("The gallery has no Archive session.");
  await library.archive(stale);
  app.render();
  await test.flush();
  /** The column and the row of the first place of the screen that shows a text, at or after a column. */
  const find = (text: string, from = 0): [number, number] => {
    const rows = test.captureCharFrame().split("\n");
    const row = rows.findIndex((line) => line.indexOf(text, from) >= 0);
    if (row < 0) throw new Error(`The screen shows no ${text}.`);
    return [(rows[row] ?? "").indexOf(text, from), row];
  };
  const [, changelogRow] = find("Changelog");
  const removeAt = (test.captureCharFrame().split("\n")[changelogRow] ?? "").lastIndexOf("×");
  await test.mockMouse.moveTo(removeAt, changelogRow);
  await capture("48-archive-session");
  // A right click on a session row opens its menu at the pointer, and Rename takes the new name in the row.
  const [researchAt, researchRow] = find("Research");
  await test.mockMouse.click(researchAt + 2, researchRow, 2);
  await capture("49-session-menu");
  const [renameAt, renameRow] = find("Rename", researchAt);
  await test.mockMouse.click(renameAt + 1, renameRow);
  await test.flush();
  await test.mockInput.typeText(" notes");
  await rest();
  await capture("50-rename-session");
} finally {
  app.dispose();
  test.renderer.destroy();
  await library.dispose();
  await extensions.dispose();
  await removeDemoDirectories();
}

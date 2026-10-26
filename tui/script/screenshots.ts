import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { cp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { basename, delimiter, join, resolve } from "node:path";
import { createTestRenderer } from "@opentui/core/testing";
import type { Exit } from "furb/life";
import { until } from "../../bind/typescript/test/until.ts";
import { type App, follow } from "../src/app.ts";
import { openEngine } from "../src/bridge.ts";
import { seedLife } from "../src/demo/life.ts";
import { seedWorkspaces } from "../src/demo/workspaces.ts";
import { demoSession, removeDemoDirectories, seedDemo, seedDemoFiles, tour } from "../src/demo.ts";
import { isLifeView } from "../src/overview.ts";
import { loadParsers } from "../src/parsers.ts";
import { Preferences } from "../src/preferences.ts";
import { Session } from "../src/session.ts";
import { colorless, paletteOf } from "../src/theme.ts";
import { Workspaces } from "../src/workspaces.ts";
import { idle } from "../test/idle.ts";
import { rasterize } from "./raster.ts";
import { click, end, find, highlighting, home, mount, watch } from "./stage.ts";

/** The recorded life of a conductor, from the fixtures of the TUI. */
const conductor = resolve(import.meta.dir, "..", "fixtures", "conductor.jsonl");
// The model of the demo writes slowly, so that its work stays in progress while a capture shows it. The session runs
// in a worker, which reads the environment once, as it starts.
// The gallery finds no model: no claude command line on its PATH, no key, and no local server, so that the first run
// of a real life shows the check of the setup as a new machine does. The demo needs no model.
const bare = {
  PATH: (process.env.PATH ?? "")
    .split(delimiter)
    .filter(
      (folder) => folder && !existsSync(join(folder, process.platform === "win32" ? "claude.exe" : "claude")),
    )
    .join(delimiter),
  FURB_CLAUDE_BIN: "",
  FURB_LOCAL_URL: "",
  ...Object.fromEntries(
    Object.keys(process.env)
      .filter((name) => /_(API_KEY|KEY|TOKEN)$/.test(name))
      .map((name) => [name, ""]),
  ),
};
const projects = await home("furb-gallery", { FURB_DEMO_PACE: "200", ...bare });
/** What the script waits for: a screen whose state never comes ends the script with an error that names the screen. */
const waits = watch();
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
/** A new session in the project that the conversation of the gallery is about. */
const gallerySession = async () => demoSession({ cwd: await project("fieldnotes") });
let session = await gallerySession();
const test = await createTestRenderer({ width: 152, height: 46 });
let app!: () => App;
let library!: Workspaces;
/** A session on the stage, under the name of a first session when it has no name of its own. */
async function show(next: Session): Promise<void> {
  session = next;
  if (session.sessionName === basename(session.host.directory)) await session.command("/name Session 1");
  ({ library, app } = await mount(session, test.renderer));
}
await show(session);

/** The shots whose notice tells what the step did, which keep it. The others show no notice of an earlier step. */
const notices = /^(34|36|37|42|43|44|45|50|55|58)-/;
/** The view of the life that each shot of main shows, which FURB_GALLERY_VIEW names, so that the gallery of main is
 * captured again in the loom and in the graph. */
const view = isLifeView(process.env.FURB_GALLERY_VIEW) ? process.env.FURB_GALLERY_VIEW : "main";
/** The work of the chain completes, and the queue sends what waits in it, until nothing is left to do. */
/** The acts that the life of the demo leaves open on purpose, which no step waits for. */
let standing = new Set<string>();
async function settle(): Promise<void> {
  await session.refresh();
  for (let attempt = 0; attempt < 10; attempt++) {
    const pending = session.activity.filter(
      (act) => !act.done && ["prompt", "rung"].includes(act.kind) && !standing.has(act.id),
    );
    if (!pending.length && !session.queued.length) break;
    // A queue that waits on no act sends its next message in a moment.
    if (!pending.length) await new Promise((done) => setTimeout(done, 100));
    await Promise.all(pending.map((act) => session.engine.result(act.id).catch(() => {})));
    await session.refresh();
  }
}
/** The pointer moves to the corner of the screen after a click, so that no row stays lit by the hover of a click that
 * a later capture does not show. */
async function rest(): Promise<void> {
  await test.mockMouse.moveTo(0, test.renderer.height - 1);
}
async function capture(name: string): Promise<void> {
  waits(`the screen ${name}`);
  if (!notices.test(name)) session.notice = "";
  if (!/^(6[6-9]|7[0-3])-/.test(name)) app().session.preferences.view = view;
  // A read that a change starts while the last one ends leaves the view loading, so the capture waits for none.
  if (name !== "25-loading" && name !== "27-view-error")
    do await session.refresh();
    while (session.loading);
  app().render();
  await test.flush();
  // Code is colored by a parser off the main thread: the frame is taken once every code block is highlighted.
  await Promise.all(highlighting(test.renderer.root));
  // The folded rungs color the first line of their word off the main thread too, and the view draws them again.
  await app().colorsDone();
  app().render();
  await test.flush();
  waits(`the screen after ${name}`);
  if (only && !only.test(name)) return;
  await writeFile(`${output}/${name}.png`, rasterize(test.captureSpans(), paletteOf(session.theme), "furb"));
  // The text of each shot goes to a folder that FURB_GALLERY_TEXT names, to read the gallery without its pictures.
  const texts = process.env.FURB_GALLERY_TEXT;
  if (texts) await writeFile(join(texts, `${name}.txt`), test.captureCharFrame());
}

let failed = false;
try {
  await loadParsers();
  await capture("01-welcome");
  standing = await seedLife(session);
  await session.command("/name Release 0.4");
  await session.refresh();
  session.show("transcript");
  await capture("03-transcript");
  // The project is a repository of git on a branch of its own, so the review compares the branch with main.
  const git = (...args: string[]) =>
    spawnSync("git", [
      "-C",
      session.host.directory,
      "-c",
      "user.name=furb",
      "-c",
      "user.email=furb@example.com",
      ...args,
    ]);
  await writeFile(join(session.host.directory, ".gitignore"), "demo.jsonl*\n");
  git("init", "-q", "-b", "main");
  git("add", ".");
  git("commit", "-q", "-m", "Field notes");
  git("checkout", "-q", "-b", "keyboard");
  const written = session.host.changes;
  await session.engine.result(
    await session.engine.rung({
      word: 'write(read("README.md").append("\\n## Keyboard\\nPress Ctrl+K to find a note.\\n"))',
      on: session.selected,
    }),
  );
  await until(session.host, () => session.host.changes > written);
  session.show("changes");
  await capture("04-changes");
  // The review in each other scope: this turn, the life, and the branch against main, then a comment on a line.
  session.review("turn");
  await capture("76-changes-turn");
  // A note that another chain writes stands in the life, and not in this chain.
  const reviewChain = session.chains.find((one) => one.words[0] === "Review notes")?.id;
  if (!reviewChain) throw new Error("The gallery has no chain Review notes.");
  await session.engine.result(
    await session.engine.rung({
      word: 'write(Text("review.md", "# Review\\n\\n- The README names the key that finds a note.\\n"))',
      on: reviewChain,
    }),
  );
  await until(session.host, () => session.host.changes > written + 1);
  session.review("life");
  await capture("77-changes-life");
  session.review("branch");
  await capture("78-changes-branch");
  session.review("chain");
  await capture("04-changes");
  await click(test, "Comment a line");
  await rest();
  for (let step = 0; step < 4; step++) test.mockInput.pressArrow("down");
  await test.flush();
  await capture("79-changes-comment");
  test.mockInput.pressEnter();
  await test.flush();
  await capture("80-changes-comment-input");
  app().composer.setText("");
  await rest();
  session.show("feed");
  app().palette();
  await capture("05-command-palette");
  app().closeOverlay();
  app().models();
  await capture("06-models");
  app().closeOverlay();
  app().providers();
  await session.host.providers();
  await capture("91-providers");
  app().closeOverlay();
  app().effortPicker();
  await capture("07-effort");
  app().closeOverlay();
  const question = await session.engine.prompt("bool", {
    message: "Apply the search shortcut to the main chain?",
    on: session.selected,
    to: "operator",
  });
  await until(session.host, () => session.host.prompts.has(question));
  await session.refresh();
  await session.point(question);
  await capture("08-operator-question");
  app().question();
  await capture("09-operator-dialog");
  app().closeOverlay();
  await session.host.answer(question, "yes");
  await until(session.host, () => !session.host.prompts.has(question));
  // A question of a choice offers its values as a list in place of the input, and Other for the words of the operator.
  const runner = await session.engine.prompt("Literal['vitepress', 'mkdocs', 'plain']", {
    message: "Which runner builds the docs of the project?",
    on: session.selected,
    to: "operator",
  });
  await until(session.host, () => session.host.prompts.has(runner));
  await session.refresh();
  await session.point(runner);
  await capture("09-choice-question");
  // Other gives the input to the words of the operator, under the values that stay listed.
  test.mockInput.pressKey("4");
  await test.mockInput.typeText("None of them: keep the plain Markdown.");
  await capture("94-other-answer");
  app().composer.setText("");
  await session.host.answer(runner, "mkdocs");
  await until(session.host, () => !session.host.prompts.has(runner));
  // A list of values is ticked with Space and sent with Enter.
  const flags = await session.engine.prompt("list[Literal['fast', 'safe', 'quiet']]", {
    message: "Which flags does the release build use?",
    on: session.selected,
    to: "operator",
  });
  await until(session.host, () => session.host.prompts.has(flags));
  await session.refresh();
  await session.point(flags);
  test.mockInput.pressKey(" ");
  test.mockInput.pressArrow("down");
  test.mockInput.pressArrow("down");
  test.mockInput.pressKey(" ");
  await capture("95-many-choice");
  await session.host.answer(flags, '["fast", "quiet"]');
  await until(session.host, () => !session.host.prompts.has(flags));
  // A shape that is more than text says what its line is.
  const files = await session.engine.prompt("list[str]", {
    message: "Which files does the change touch?",
    on: session.selected,
    to: "operator",
  });
  await until(session.host, () => session.host.prompts.has(files));
  await session.refresh();
  await session.point(files);
  await test.mockInput.typeText('["README.md", "docs/tui.md"]');
  await capture("96-list-question");
  app().composer.setText("");
  await session.host.answer(files, '["README.md", "docs/tui.md"]');
  await until(session.host, () => !session.host.prompts.has(files));
  const notice = await session.engine.prompt("None", {
    message: "The search shortcut is on the main chain.",
    on: session.selected,
    to: "operator",
  });
  await until(session.host, () => session.host.notices.has(notice));
  await session.refresh();
  await capture("22-operator-notice");
  app().question();
  await until(session.host, () => !session.host.notices.has(notice));
  const markdown = await session.engine.prompt("None", {
    message: [
      "## The search shortcut",
      "",
      "Press **Ctrl+K** to find a note, as `search.py` binds it. See [the guide](https://example.com/guide).",
      "",
      "- It reads each note",
      "  - in the folder `notes/`",
      "- It shows the *first* match",
      "",
      "> The index is built once.",
      "",
      "| Key | Action |",
      "| --- | --- |",
      "| Ctrl+K | Find |",
      "",
      "```python",
      "def search(notes: list[str], word: str) -> str | None:",
      "    for note in notes:",
      "        if word in note:  # the first match",
      "            return note",
      "    return None",
      "```",
      "",
      "```sh",
      "uv run pytest -q test/test_search.py",
      "```",
      "",
      "```",
      "notes/",
      "    shortcuts.md",
      "```",
    ].join("\n"),
    on: session.selected,
    to: "operator",
  });
  // A notice settles as a quiet line, which opens to its whole message.
  await until(session.host, () => session.acts.some((act) => act.id === markdown && act.done));
  session.folds[markdown] = false;
  await session.refresh();
  await capture("51-markdown");
  // The rung that wrote the file opens to its read and its write, each with what it did.
  session.folds[markdown] = true;
  const wrote = session.acts.find((act) => act.kind === "write")?.by;
  if (wrote) session.folds[wrote] = false;
  app().toggleMode();
  app().composer.setText('notes = read("README.md")\nprint(notes.content)');
  await capture("10-python-input");
  await app().inspect("notes");
  await capture("11-value-inspector");
  app().closeOverlay();
  session.theme = "paper";
  await capture("12-light-theme");
  session.theme = "midnight";
  await capture("13-midnight-theme");
  app().themes();
  await capture("14-theme-picker");
  app().closeOverlay();
  session.theme = "github";
  app().chains();
  await capture("15-chains");
  app().closeOverlay();
  app().openThreads(session.selected);
  await capture("81-chain-threads");
  app().closeThreads();
  app().help();
  await capture("16-help");
  app().closeOverlay();
  app().toggleMode();
  app().composer.setText("");
  // A word runs a command, whose line is lit while it runs, with what it prints under it.
  const command = await session.engine.rung({
    word: `index = await bash("printf 'Building the search index...\\n'; sleep 4; printf '3 notes indexed.\\n'")\nprint(index.stdout.content)`,
    on: session.selected,
  });
  // The command shows once it has printed its first line.
  await until(session, () =>
    session.acts.some(
      (act) =>
        act.kind === "bash" &&
        act.by === command &&
        !act.done &&
        Boolean((act.value as Exit | undefined)?.stdout?.content),
    ),
  );
  // After a second, the line says how long its command has run.
  await new Promise((done) => setTimeout(done, 1200));
  await session.refresh();
  app().scroll.scrollTo(app().scroll.scrollHeight);
  await capture("18-live-command");
  await session.engine.result(command);
  const progress = await session.engine.prompt("str", {
    message: "show live progress",
    on: session.selected,
  });
  // The model has written part of its word, and writes the rest.
  await until(session.host, () =>
    [...session.host.streams.values()].some((stream) => stream.text.length > 70),
  );
  await session.refresh();
  await capture("19-model-progress");
  await session.engine.result(progress);
  // A call of a model that failed for a cause that passes waits for its next attempt, which the footer counts down.
  // The streams hold the retry while the capture reads the life again, which sends the streams of the life anew.
  const retry = { attempt: 2, most: 10, until: Date.now() + 3600, why: "connection reset" };
  const retrying = new Map([["rung0", { chain: session.selected, text: "", thinking: "", retry }]]);
  Object.defineProperty(session.host, "streams", { configurable: true, get: () => retrying, set: () => {} });
  await capture("62-model-retry");
  Object.defineProperty(session.host, "streams", { configurable: true, writable: true, value: new Map() });
  await session.submit("/run this is invalid python !!!").catch(session.fail);
  await capture("20-gate-findings");
  const record = session.host.record;
  if (!record) throw new Error("The demo session has no record.");
  await session.engine.wait([], { seconds: 60, on: session.engine.root });
  app().dispose();
  await library.dispose();
  session = await demoSession({ record });
  await show(session);
  await capture("21-paused-resume");
  app().closeOverlay();
  await session.host.resume();
  await session.refresh();
  app().rewind();
  await capture("23-rewind-tree");
  app().closeTree();
  // A message whose answer writes the README: a rewind to it asks what comes back, the conversation, the files, or
  // both. The files then come back, so the later captures read the README as it was.
  await session.engine.result(
    await session.engine.prompt("str", { message: "Add the search shortcut, Ctrl+K.", on: session.selected }),
  );
  await session.refresh();
  const shortcut = session.activity.findLast((act) => session.isUserPrompt(act))?.id ?? "";
  app().rewind();
  app().render();
  await test.flush();
  test.mockInput.pressEnter();
  for (let tries = 0; tries < 400 && !test.captureCharFrame().includes("Restore the files"); tries++) {
    await Bun.sleep(10);
    app().render();
    await test.flush();
  }
  await capture("74-rewind-files");
  app().closeOverlay();
  await session.restoreAt(shortcut);

  app().dispose();
  await library.dispose();
  await show(await gallerySession());
  app().render();
  await test.flush();
  test.mockInput.pressKey("f", { ctrl: true });
  await test.mockInput.typeText("no matching message");
  await capture("24-empty-results");
  test.mockInput.pressEscape();
  // A chain that opens while its turns are read shows that it loads.
  const opened = await session.engine.chain({ label: "Notes" });
  await session.refresh();
  const snapshot = session.host.snapshot.bind(session.host);
  let release = () => {};
  const held = new Promise<void>((resolve) => {
    release = resolve;
  });
  session.host.snapshot = async (...read) => {
    await held;
    return snapshot(...read);
  };
  const opening = session.select(opened);
  await capture("25-loading");
  release();
  await opening;
  session.host.snapshot = snapshot;
  await session.select(session.engine.root);
  await session.submit("/read missing-file.txt");
  await until(session, () => session.acts.some((act) => act.run?.status === "failed"));
  await capture("26-error");
  await session.engine.write(
    { path: "preview.txt", content: "A change to inspect.\n" },
    { on: session.engine.root },
  );
  const journal = `${session.host.record}.changes.jsonl`;
  const savedJournal = await readFile(journal);
  await writeFile(journal, "{ damaged journal }");
  session.show("changes");
  await session.refresh().catch(session.fail);
  await capture("27-view-error");
  await writeFile(journal, savedJournal);

  app().dispose();
  await library.dispose();
  const preferences = new Preferences(join(projects, "config/ui.json"));
  library = new Workspaces(preferences, { demo: true });
  // Four projects of one developer: sessions that past windows left on disk, and sessions that run in this one, each in
  // the state that a real day leaves it, around the life of the release that the window shows.
  const { groups, entries } = await seedWorkspaces({
    library,
    preferences,
    directory: async (name) => {
      const folder = join(projects, name);
      await rm(folder, { recursive: true, force: true });
      await mkdir(folder, { recursive: true });
      return folder;
    },
  });
  const first = groups.fieldnotes;
  const second = groups.atlas;
  if (!first || !second) throw new Error("The workspace fixtures did not open.");
  const main = await library.create(first, "Release 0.4");
  if (!main.session) throw new Error("The workspace fixtures did not open.");
  standing = await seedLife(main.session);
  await library.select(main);
  session = main.session;
  app = follow(test.renderer, { quit() {}, workspaces: library });
  await until(
    library,
    () =>
      entries.Changelog?.status === "done" &&
      entries["Offline mode"]?.status === "blocked" &&
      entries["Pagination bug"]?.status === "working" &&
      entries.Research?.status === "paused",
  );
  await capture("28-workspace-tree");
  app().render();
  await test.flush();
  // A click on a line of a program that made an act opens it in place, with what the act gave.
  const rows = test.captureCharFrame().split("\n");
  const line = rows.findIndex((row) => /^ +\d+ .*\b(bash|read)\(/.test(row));
  if (line >= 0) await test.mockMouse.click((rows[line]?.search(/\d+ /) ?? 0) + 4, line);
  await rest();
  await capture("29-open-line");
  library.toggle(second);
  await capture("30-collapsed-workspace");
  library.toggle();
  await capture("31-hidden-sidebar");
  library.toggle();
  library.toggle(second);
  await app().workspacePicker();
  await capture("32-workspace-picker");
  app().closeOverlay();
  app().composer.setText("/tree");
  await app().submit();
  await capture("33-session-tree");
  app().closeTree();
  // The next shots are about the input, and an undo waits for each prompt to end, which the release does not while its
  // workers run. They use a session of the same workspace whose conversation ends.
  const desk = await library.create(first, "Search shortcut");
  if (!desk.session) throw new Error("The session Search shortcut did not open.");
  await seedDemo(desk.session);
  await library.select(desk);
  session = desk.session;
  await session.submit("show live progress");
  session.enqueue("Check keyboard navigation after this answer.");
  await capture("34-queued-follow-up");
  app().composer.setText("/queue");
  await app().submit();
  await capture("35-queue-editor");
  app().closeOverlay();
  // The queued message is sent before the next one, so that the feed reads in the same order at each run.
  await settle();
  await session.attachImage(resolve("docs/screenshots/01-welcome.png"));
  app().composer.setText("Review the layout in this image.");
  await capture("36-image-attachment");
  await app().submit();
  await session.refresh();
  const imagePrompt = session.acts.findLast(
    (act) => act.kind === "prompt" && String(act.words[1]).startsWith("Review the layout"),
  );
  if (!imagePrompt) throw new Error("The image prompt was not submitted.");
  await session.engine.result(imagePrompt.id);
  await session.refresh();
  await session.submit("/share");
  await capture("37-share-conversation");
  app().closeOverlay();
  app().composer.setText("/delete");
  await app().submit();
  await test.mockInput.typeText("Archive");
  test.mockInput.pressEnter();
  await capture("38-delete-session");
  app().closeOverlay();
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
    app().toggleMode();
    await app().editDraft();
    if (!app().composer.plainText.includes("edited_in_editor"))
      throw new Error("The external editor did not return its draft.");
    await capture("39-external-editor");
  } finally {
    if (priorEditor === undefined) delete process.env.EDITOR;
    else process.env.EDITOR = priorEditor;
    if (priorVisual === undefined) delete process.env.VISUAL;
    else process.env.VISUAL = priorVisual;
  }
  if (session.mode === "python") app().toggleMode();
  app().composer.setText("");
  app().composer.focus();
  await test.mockInput.typeText("@");
  await session.projectFiles();
  await capture("40-file-picker");
  app().composer.setText("");
  await test.mockInput.typeText("/e");
  await capture("41-slash-suggestions");
  app().composer.setText("");
  app().closeOverlay();
  app().composer.setText("/extensions");
  await app().submit();
  await capture("42-extension-command");
  await settle();
  await session.submit("/undo");
  await capture("43-undo-message");
  await session.submit("/redo");
  await capture("44-redo-message");
  app().composer.setText("A draft to keep for later");
  app().stash();
  app().composer.setText("A quick question first");
  await capture("45-stash");
  // The stash comes back into the empty input and leaves, and a space after a command whose values are known lists
  // them.
  app().composer.setText("");
  test.mockInput.pressKey("s", { ctrl: true });
  app().composer.setText("");
  await test.mockInput.typeText("/effort ");
  await capture("46-value-suggestions");
  // The grey rest of a line from the history, as fish shows it, which Right takes.
  const priorHistory = session.histories[session.draftKey];
  session.histories[session.draftKey] = [
    "Explain how the history of this project is kept",
    "Find the slowest test and say why it is slow",
  ];
  app().composer.setText("");
  await test.mockInput.typeText("Find the slow");
  await capture("46b-history-suggestion");
  app().composer.setText("");
  if (priorHistory) session.histories[session.draftKey] = priorHistory;
  else delete session.histories[session.draftKey];
  // A colon that starts a word lists the emojis whose shortcode holds what follows it.
  await test.mockInput.typeText("Ship it :grin");
  await capture("46c-emoji-suggestions");
  app().composer.setText("");
  // A word of the model that the gate refused, and that its next word replaced, folds and reads as retried.
  app().composer.setText("");
  await session.submit("Give me an answer in a fence.");
  await settle();
  await capture("47-retried-word");
  // An archived session folds under Archived, and the pointer on a session row shows its remove button.
  const stale = library.groups[1]?.sessions.find((entry) => entry.name === "Archive");
  if (!stale) throw new Error("The gallery has no Archive session.");
  await library.archive(stale);
  app().render();
  await test.flush();
  const [, changelogRow] = find(test, "Changelog");
  const removeAt = (test.captureCharFrame().split("\n")[changelogRow] ?? "").lastIndexOf("×");
  await test.mockMouse.moveTo(removeAt, changelogRow);
  await capture("48-archive-session");
  // A right click on a session row opens its menu at the pointer, and Rename takes the new name in the row.
  const [researchAt, researchRow] = find(test, "Research");
  await test.mockMouse.click(researchAt + 2, researchRow, 2);
  await capture("49-session-menu");
  await click(test, "Rename", researchAt);
  await test.flush();
  await test.mockInput.typeText(" notes");
  await rest();
  await capture("50-rename-session");
  // A life at the scale of a real one: many chains, and questions to the owner open on three of them at once, one of
  // which a conductor relays at its length. The desk over the input counts them, and says which one the input answers.
  test.mockInput.pressEscape();
  app().dispose();
  await library.dispose();
  await show(await gallerySession());
  standing = await seedLife(session);
  await session.command("/name Conduct the work");
  await until(session.host, () => session.host.prompts.size >= 4);
  await session.select(session.engine.root);
  await rest();
  await capture("52-many-questions");
  app().question();
  await capture("53-question-dialog");
  // Tab goes to the next question, and shows its chain under the dialog.
  test.mockInput.pressTab();
  await until(session, () => session.selected !== session.engine.root);
  await capture("54-next-question");
  const merge = session.aimed?.id ?? "";
  test.mockInput.pressEnter();
  await until(session.host, () => !session.host.prompts.has(merge));
  app().closeOverlay();
  await capture("55-answer-landed");

  // A record whose replay drifts on the build of its version opens to read, with a dialog that says why it drifts.
  const older = await demoSession({ cwd: await project("fieldnotes-old") });
  await seedDemo(older);
  const drifts = older.host.record ?? "";
  await older.dispose();
  const kept = await readFile(drifts, "utf8");
  await writeFile(drifts, kept.replace(/("bash","bash\d+","rung\d+","chain\d+","[^"]*"),""/, "$1,false"));
  // Keep the log with the record, since it holds the close that completed its prompt.
  await library.import(drifts, await library.add(older.host.directory));
  session = library.current?.session ?? session;
  await capture("56-record-to-read");
  // The dialog closes, and the record stays read only: the footer says so and offers a new life, the composer is off,
  // and the work that had no end when the record ended shows no spinner and no clock.
  app().closeOverlay();
  await capture("56b-record-read-only");

  app().dispose();
  await library.dispose();
  await show(await gallerySession());
  // A thread of the owner whose model hires two workers, each on a chain of its own, and a later message of his.
  await session.submit("Hire two workers.");
  await until(
    session,
    () =>
      session.acts.filter((act) => act.kind === "rung" && act.on !== session.engine.root && !act.done)
        .length === 2,
  );
  const thread = session.activity.find((act) => session.isUserPrompt(act))?.id ?? "";
  await session.submit("Think until I cancel.");
  await until(session, () => {
    const last = session.lastOwn;
    return Boolean(
      last && last.id !== thread && session.activity.some((act) => act.by === last.id && !act.done),
    );
  });
  await session.refresh();
  app().render();
  await test.flush();
  // A right click on the message offers to take it back.
  const [column, row] = find(test, "┃ Think until I cancel.");
  await test.mockMouse.click(column + 4, row, 2);
  await capture("57-message-menu");
  app().closeOverlay();
  // A cancel of the thread reaches the workers, so it asks first.
  await session.command(`/cancel ${thread}`);
  await rest();
  await capture("58-cancel-confirm");
  app().closeOverlay();
  // ⌃C takes back the last message alone, and the workers go on.
  const message = session.lastOwn?.id;
  await session.command("/cancel");
  await until(session, () => session.acts.some((act) => act.id === message && act.done));
  await rest();
  await capture("59-take-back");
  // The graph at the level of one act shows the card of the act in the feed, which says that the command runs.
  if (view === "main") {
    const guide = await session.engine.bash(
      "printf 'Building the page capture\\n'; sleep 30; printf 'The guide has one page\\n'",
      { on: session.selected },
    );
    await until(session, () =>
      Boolean((session.acts.find((act) => act.id === guide)?.value as Exit | undefined)?.stdout?.content),
    );
    app().switchView("graph");
    app().graph.level = "act";
    app().render();
    await capture("73-graph-command");
    app().graph.level = "threads";
    app().switchView("main");
    await session.command(`/cancel ${guide}`);
    await session.engine.result(guide).catch(() => {});
  }
  // The last row of a thread while its model works: a loader left of the number of the line that comes, and
  // `# thinking` while the model shows no thought, then the last line of what it thinks.
  const thinks = await session.engine.prompt("str", {
    message: "Think until I cancel.",
    on: session.selected,
  });
  await until(session, () =>
    session.acts.some((act) => act.kind === "rung" && act.by === thinks && !act.done),
  );
  await session.refresh();
  await capture("84-thinking-row");
  const step = session.acts.find((act) => act.kind === "rung" && act.by === thinks)?.id ?? "";
  const streams = session.host.streams;
  const thought = "The two workers think in parallel, so I wait for both before I read their notes.";
  // The streams of the life are held still for the capture, so no piece that the life writes changes the row.
  Object.defineProperty(session.host, "streams", {
    configurable: true,
    get: () => new Map([[step, { chain: session.selected, text: "", thinking: thought }]]),
    set: () => {},
  });
  await new Promise((done) => setTimeout(done, 200));
  await capture("85-thinking-tokens");
  Object.defineProperty(session.host, "streams", { configurable: true, writable: true, value: streams });
  await session.command(`/cancel ${thinks}`);
  await session.engine.result(thinks).catch(() => {});

  // The feed of a real life: a life of the conductor, recorded from its log, with a score of chains at work, which
  // shows and runs nothing. The library of the demo ends first, with the sessions and the workers that it holds.
  app().dispose();
  await library.dispose();
  const life = await openEngine({ record: conductor, cwd: await project("conductor") });
  session = new Session(life.engine, life.host, true);
  await session.refresh();
  ({ library, app } = await mount(session, test.renderer));
  await capture("02-feed");
  // A string too long for its line folds to an underlined ..., which a hover shows whole and a click opens in place.
  const folded = test.captureCharFrame().split("\n");
  const foldRow = folded.findIndex((row) => /\.\.\.["']/.test(row));
  if (foldRow < 0) throw new Error("No string folds in the feed.");
  const foldColumn = (folded[foldRow] ?? "").search(/\.\.\.["']/) + 1;
  await test.mockMouse.moveTo(foldColumn, foldRow);
  await new Promise((done) => setTimeout(done, 400));
  await capture("82-string-hover");
  await test.mockMouse.click(foldColumn, foldRow);
  await rest();
  await capture("83-open-string");
  // The string folds again for the shots that follow.
  session.folds = Object.fromEntries(
    Object.entries(session.folds).filter(([key]) => !key.startsWith("string:")),
  );
  // A worker of the life, whose ladders are long: the earlier rungs of a ladder fold into one line.
  await session.select(session.chains.find((chain) => session.labelOf(chain.id) === "life3/feed")?.id ?? "");
  await capture("60-long-ladder");
  // The conductor runs one long word, under which its messages pile up, and the earlier ones fold into one line.
  await session.select(session.engine.root);
  app().render();
  await test.flush();
  const fold = app()
    .scroll.getChildren()
    .find((node) => node.id.startsWith("more-"));
  app().scroll.stickyScroll = false;
  app().scroll.scrollTo(Math.max(0, (fold?.y ?? 0) - app().scroll.y + app().scroll.scrollTop - 6));
  await test.flush();
  await capture("61-folded-steps");
  test.resize(82, 32);
  await capture("17-narrow");
  test.resize(152, 46);
  // The views of the life side by side, on the same life: the loom of its chains in time over the feed, with the
  // cursor in time, then the graph of its programs at each level of its zoom.
  await session.select(session.engine.root);
  app().switchView("loom");
  await capture("66-loom");
  app().steer(true);
  await session.select(session.chains.find((chain) => session.labelOf(chain.id) === "life3/feed")?.id ?? "");
  app().travel(-40);
  await capture("67-loom-time-cursor");
  app().travel(Number.MAX_SAFE_INTEGER);
  app().steer(false);
  await session.select(session.engine.root);
  app().switchView("graph");
  await capture("68-graph");
  app().steer(true);
  await app().graph.key("return");
  await capture("69-graph-map");
  await app().graph.key("return");
  await capture("70-graph-program");
  // The word of the last rung of the conductor that made an act, then that act, whose card the feed shows whole.
  for (let step = 0; step < 4; step++) await app().graph.key("up");
  await app().graph.key("return");
  await capture("71-graph-word");
  await app().graph.key("return");
  await capture("72-graph-act");
  app().steer(false);
  app().switchView("main");

  // A first run of a real life, not a demo, on this home, which holds no model: the welcome checks the setup, with a
  // fix for each part that is missing, over the steps that the extensions played on the chain.
  app().dispose();
  await library.dispose();
  const fresh = await project("first-run");
  const machine = await openEngine({ cwd: fresh, record: join(fresh, "session.jsonl") });
  const real = new Session(machine.engine, machine.host, true);
  await real.refresh();
  await until(real, () => real.setup !== undefined && real.catalogRead && real.activity.length > 0);
  await show(real);
  await capture("63-setup-no-model");
  const base = real.setup;
  if (!base) throw new Error("the setup did not arrive");
  real.catalog = [["local:qwen3", ["low", "medium", "high"], 65536]];
  real.setup = {
    ...base,
    providers: [...base.providers, { provider: "local", env: [], offered: 1, enabled: true }],
  };
  real.emit("change");
  await capture("92-setup-configured-local");
  real.catalog = [];
  real.setup = {
    ...base,
    providers: [
      ...base.providers.filter((one) => one.provider !== "local"),
      { provider: "local", env: ["LOCAL_KEY"], offered: 0, enabled: true },
    ],
  };
  real.emit("change");
  await capture("93-setup-configured-local-missing");

  // Watch a life: the demo plays by itself, asks no model, and costs nothing.
  app().dispose();
  await library.dispose();
  const watched = await demoSession({ watch: true, cwd: await project("watch") });
  await show(watched);
  const playing = tour(watched, 300);
  await until(
    watched,
    () => watched.acts.filter((act) => act.kind === "prompt" && act.words[2] && !act.done).length === 3,
  );
  await rest();
  await capture("64-watch-a-life");
  await playing;
  await watched.select(watched.engine.root);
  // The moment goes back to the word as it began, before it asked any model.
  const asks = () => watched.at(watched.activity).some((act) => act.kind === "prompt" && act.words[2]);
  while (asks()) watched.travel(-1);
  await capture("65-watch-earlier");
  // The bar of the watch stays over the feed on the smallest terminal.
  test.resize(80, 24);
  await capture("81-watch-small");
  test.resize(152, 46);

  // A project extension gives the TUI a part, which loads live: the panel of a todo list over the input, and the hint
  // of the line of each act todo in the feed. A module that breaks shows its error in its panel.
  app().dispose();
  await library.dispose();
  const listed = await project("todo");
  const folder = join(listed, ".furb", "extensions", "todo");
  await cp(resolve(import.meta.dir, "..", "fixtures", "todo"), folder, { recursive: true });
  await writeFile(join(listed, ".furb", "config.json"), '{"extensions": {"todo": "extensions/todo"}}\n');
  await show(await demoSession({ cwd: listed }));
  const todo = (await readFile(join(folder, "todo.py"), "utf8")).replace(
    /^from furb\.engine import .*\n+/m,
    "",
  );
  await session.command(
    `/run ${todo}\ntodo("Draw the todo list in a panel", done=True)\ntodo("Load the part again when its file changes", done=True)\ntodo("Show the error of a module that breaks")`,
  );
  await settle();
  while (!app().parts.version) await Bun.sleep(10);
  // The line of each act todo says what the part draws as its row.
  app().render();
  await test.flush();
  await rest();
  await capture("74-extension-part");
  await writeFile(join(folder, "tui.ts"), "export default (furb) => { furb.panel(");
  while (!app().parts.loaded.find((one) => one.name === "todo")?.error) await Bun.sleep(10);
  await capture("75-extension-part-error");
  app().dispose();
  await library.dispose();

  // The part of the official extension compaction, which the crate carries: a rule at each compaction of a chain,
  // with its summary, between what the model no longer reads and what it reads now. The board says how many times
  // each chain compacted.
  await show(await demoSession({ cwd: await project("compaction") }));
  const compaction = (
    await readFile(resolve(import.meta.dir, "..", "..", "extensions", "compaction", "compaction.py"), "utf8")
  ).replace(/^from furb\.engine import \([^)]*\)\n+/m, "");
  await session.command(`/run ${compaction}\nchain("review")\nplan = read("README.md")`);
  await settle();
  await session.command(
    '/run compact(summary="The plan stands in README.md, and the operator chose the calm palette.\\nThe parser is done and its tests pass. The docs wait.")',
  );
  await settle();
  await session.command('/run docs = "The docs of the parser"');
  await settle();
  await session.command(
    '/run compact(summary="The docs of the parser are written.\\nThe screenshots of the parser wait.")',
  );
  await settle();
  await rest();
  await capture("75-compaction");
  app().dispose();
  await library.dispose();

  // An older compaction went on in a new chain, as Main -> compacted: the run shows as one row of the sidebar and as one
  // feed, with the rule of the compaction between the feed of each chain.
  await show(await demoSession({ cwd: await project("compacted") }));
  const root = session.engine.root;
  await session.command(
    `/run ${compaction}\ndef restore(source, on=""):\n  return None\nchain("review")\nplan = read("README.md")`,
  );
  await settle();
  await session.command(
    `/run newer = chain("compacted", "${root}")\nrung("restore('${root}')\\n<s:summary>\\nThe plan stands in README.md, and the parser is done.\\nThe docs wait.</s:summary>\\nreground()", on=newer)\nrung('docs = "The docs of the parser"', on=newer)`,
  );
  await settle();
  await rest();
  await capture("75-compaction-chains");
  app().dispose();
  await library.dispose();

  // Access: the mode of a screen reader, a terminal with no color, and the theme of a light terminal, on a life of
  // the demo that holds a conversation and a question.
  const access = await gallerySession();
  await seedDemo(access);
  await show(access);
  await access.engine.prompt("str", {
    message: "Which name should the search shortcut show, Ctrl+K or ⌘K?",
    to: "operator",
    on: access.engine.root,
  });
  await until(access, () => access.desk.asks.length > 0);
  access.preferences.screenReader = true;
  library.emit("access");
  await capture("76-screen-reader");
  access.preferences.screenReader = false;
  colorless.on = true;
  library.emit("access");
  await capture("77-no-color");
  colorless.on = false;
  library.emit("access");
  access.theme = "terminal";
  test.renderer.emit("theme_mode", "light");
  await capture("78-terminal-light");
  test.renderer.emit("theme_mode", "dark");
} catch (error) {
  console.error(error);
  failed = true;
}
waits("the end of the script");
await end(
  failed,
  () => app().dispose(),
  () => test.renderer.destroy(),
  () => library.dispose(),
  removeDemoDirectories,
);

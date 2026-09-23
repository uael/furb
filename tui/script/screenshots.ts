import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { createTestRenderer } from "@opentui/core/testing";
import { Resvg } from "@resvg/resvg-js";
import { App } from "../src/app.ts";
import { openEngine } from "../src/bridge.ts";
import { demoSession, seedDemo, seedDemoFiles } from "../src/demo.ts";
import { Extensions } from "../src/extensions.ts";
import { loadParsers } from "../src/parsers.ts";
import { Preferences } from "../src/preferences.ts";
import { Session } from "../src/session.ts";
import { sessionChoices } from "../src/sessions.ts";
import { palettes } from "../src/theme.ts";
import { Workspaces } from "../src/workspaces.ts";

const output = resolve("docs/screenshots");
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
    const entries = new Map((library.groupOf()?.sessions ?? []).map((entry) => [entry.path, entry]));
    return sessionChoices(
      dirname(session.world.records.path ?? "."),
      async (path) => {
        const entry = entries.get(path);
        if (entry) await library.select(entry);
      },
      async () => {
        await library.create();
      },
      entries,
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
  library.preferences.sidebar = false;
  library.preferences.save();
  app = new App(test.renderer, session, options());
  followSelection();
}
await mount(session);
const xml = (value: string) =>
  value.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;");
const color = (value: { toInts(): number[] }) =>
  `#${value
    .toInts()
    .slice(0, 3)
    .map((part) => part.toString(16).padStart(2, "0"))
    .join("")}`;

async function capture(name: string): Promise<void> {
  if (name !== "28-loading" && name !== "30-view-error") await session.refresh();
  app.render();
  await test.flush();
  await Bun.sleep(80);
  await test.flush();
  const frame = test.captureSpans();
  const cell = 9,
    rowHeight = 20,
    top = 0;
  const width = frame.cols * cell,
    height = frame.rows * rowHeight + top;
  const parts = [
    `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}"><rect width="100%" height="100%" fill="${palettes[session.theme].background}"/>`,
  ];
  for (const [line, content] of frame.lines.entries()) {
    let column = 0;
    for (const span of content.spans) {
      const x = column * cell,
        y = line * rowHeight + top;
      const spanWidth = span.width * cell;
      parts.push(
        `<rect x="${x}" y="${y}" width="${spanWidth}" height="${rowHeight}" fill="${color(span.bg)}"/>`,
      );
      if (span.text.trim())
        parts.push(
          `<text x="${x}" y="${y + 15}" font-family="Menlo,DejaVu Sans Mono,monospace" font-size="14" font-weight="${span.attributes & 1 ? "700" : "400"}" fill="${color(span.fg)}" textLength="${spanWidth}" lengthAdjust="spacingAndGlyphs" xml:space="preserve">${xml(span.text)}</text>`,
        );
      column += span.width;
    }
  }
  parts.push("</svg>");
  await writeFile(
    `${output}/${name}.png`,
    new Resvg(parts.join(""), { font: { loadSystemFonts: true } }).render().asPng(),
  );
  await writeFile(`/tmp/furb-${name}.txt`, test.captureCharFrame());
}

try {
  await loadParsers();
  await capture("01-welcome");
  await seedDemo(session);
  await session.command("/name Explore project");
  await Bun.sleep(300);
  await session.refresh();
  await capture("02-conversation");
  session.show("program");
  await capture("03-program");
  session.show("activity");
  app.render();
  app.scroll.scrollTo(10000);
  await capture("04-activity");
  session.show("facts");
  app.render();
  await test.flush();
  test.mockInput.pressKey("f", { ctrl: true });
  await test.mockInput.typeText("answer");
  await capture("05-facts");
  test.mockInput.pressEscape();
  session.show("transcript");
  await capture("06-transcript");
  await session.life.result(
    await session.life.rung(
      'write(read("README.md").append("\\n## Keyboard\\nPress Ctrl+K to find a note.\\n"))',
      { on: session.selected },
    ),
  );
  await Bun.sleep(60);
  session.show("changes");
  await capture("07-changes");
  app.palette();
  await capture("08-command-palette");
  app.closeOverlay();
  app.models();
  await capture("09-models");
  app.closeOverlay();
  app.effortPicker();
  await capture("26-effort");
  app.closeOverlay();
  await session.life.prompt("bool", "Apply the search shortcut to the main chain?", {
    on: session.selected,
    to: "operator",
  });
  await Bun.sleep(60);
  await session.refresh();
  session.show("conversation");
  await capture("10-operator-question");
  app.question();
  await capture("18-operator-dialog");
  app.closeOverlay();
  await session.world.answer(session.operatorPrompt?.id ?? "", "yes");
  app.toggleMode();
  app.composer.setText('notes = read("README.md")\nprint(notes.content)');
  session.show("program");
  await capture("11-python-input");
  session.emit("inspect", "notes");
  await Bun.sleep(40);
  await capture("12-value-inspector");
  app.closeOverlay();
  session.theme = "paper";
  await capture("13-light-theme");
  session.theme = "midnight";
  await capture("14-midnight-theme");
  app.chains();
  await capture("15-chains");
  app.closeOverlay();
  app.help();
  await capture("16-help");
  app.closeOverlay();
  test.resize(82, 32);
  await capture("17-narrow");
  test.resize(152, 46);
  session.theme = "github";
  const command = await session.life.bash(
    "printf 'Building the search index...\\n'; sleep 1; printf '3 notes indexed.\\n'",
    { on: session.selected },
  );
  await Bun.sleep(60);
  await session.refresh();
  session.show("activity");
  session.query = command;
  await session.refresh();
  app.render();
  await test.flush();
  const commandRow = app.scroll
    .getChildren()
    .find((node) => node.id === command)
    ?.getChildren()[0];
  if (!commandRow) throw new Error("The command row is not visible.");
  await test.mockMouse.click(commandRow.x + 1, commandRow.y);
  await capture("22-live-command");
  await session.life.result(command);
  const progress = await session.life.prompt("str", "show live progress", { on: session.selected });
  await Bun.sleep(60);
  await session.refresh();
  session.show("conversation");
  await capture("23-model-progress");
  await session.life.result(progress);
  await session.submit("/run this is invalid python !!!").catch(session.fail);
  await capture("19-gate-findings");
  const record = session.world.records.path;
  if (!record) throw new Error("The demo session has no record.");
  await session.life.wait(60);
  app.dispose();
  await library.dispose();
  const resumed = await openEngine({ record, demo: true });
  session = new Session(resumed.life, resumed.world, true);
  await session.refresh();
  await mount(session);
  await capture("20-paused-resume");
  app.closeOverlay();
  app.openPalette("Sessions", await options().sessions());
  await capture("21-sessions");
  app.closeOverlay();
  await session.world.resume();
  await session.refresh();
  app.rewind();
  await capture("24-rewind-transcript");
  app.closeOverlay();
  app.ladders();
  test.mockInput.pressEnter();
  await test.flush();
  app.composer.setText("result = len(notes.lines)\nprint(result)");
  await capture("25-prompt-repl");

  app.dispose();
  await library.dispose();
  session = await demoSession();
  await mount(session);
  session.show("facts");
  await session.refresh();
  app.render();
  await test.flush();
  test.mockInput.pressKey("f", { ctrl: true });
  await test.mockInput.typeText("no matching fact");
  await capture("27-empty-results");
  test.mockInput.pressEscape();
  const running = session.life.rung("total = 0\nfor item in range(1000000):\n  total += item");
  session.show("program");
  const reading = session.refresh();
  await capture("28-loading");
  await session.life.result(await running);
  await reading;
  await session.submit("/read missing-file.txt");
  await Bun.sleep(80);
  await capture("29-error");
  await session.life.write({ path: "preview.txt", content: "A change to inspect.\n" });
  const journal = `${session.world.records.path}.changes.jsonl`;
  const savedJournal = await readFile(journal);
  await writeFile(journal, "{ damaged journal }");
  session.show("changes");
  await session.refresh().catch(session.fail);
  await capture("30-view-error");
  await writeFile(journal, savedJournal);

  app.dispose();
  await library.dispose();
  const projects = await mkdtemp(join(tmpdir(), "furb-gallery-"));
  const notes = join(projects, "fieldnotes"),
    atlas = join(projects, "atlas");
  await mkdir(notes);
  await mkdir(atlas);
  await seedDemoFiles(notes);
  await seedDemoFiles(atlas);
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
  await Bun.sleep(100);
  if (
    checks.status !== "working" ||
    review.status !== "blocked" ||
    changelog.status !== "done" ||
    research.status !== "paused"
  )
    throw new Error("The workspace status fixtures have not settled.");
  await capture("31-workspace-tree");
  session.show("program");
  await session.refresh();
  app.render();
  await test.flush();
  const word = app.scroll.getChildren().find((node) => node.id.startsWith("rung://"));
  const heading = word?.getChildren()[0];
  if (heading) await test.mockMouse.click(heading.x + 1, heading.y);
  await capture("32-collapsed-rung");
  library.toggle(second);
  await capture("33-collapsed-workspace");
  library.toggle();
  await capture("34-hidden-workspace-sidebar");
  library.toggle();
  library.toggle(second);
  app.workspacePicker();
  await Bun.sleep(100);
  await capture("35-workspace-picker");
  app.closeOverlay();
  await session.submit("/tree");
  await capture("36-session-tree");
  app.closeOverlay();
  session.show("conversation");
  await session.submit("show live progress");
  session.enqueue("Check keyboard navigation after this answer.");
  await capture("37-queued-follow-up");
  await session.submit("/queue");
  await capture("38-queue-editor");
  app.closeOverlay();
  await session.attachImage(resolve("docs/screenshots/01-welcome.png"));
  app.composer.setText("Review the layout in this image.");
  await capture("39-image-attachment");
  test.mockInput.pressEnter();
  await Bun.sleep(100);
  await session.refresh();
  const imagePrompt = session.acts.findLast(
    (act) => act.kind === "prompt" && String(act.words[1]).startsWith("Review the layout"),
  );
  if (!imagePrompt) throw new Error("The image prompt was not submitted.");
  await session.life.result(imagePrompt.id);
  await session.refresh();
  await session.submit("/share");
  await capture("40-share-conversation");
  app.closeOverlay();
  app.composer.setText("/delete");
  test.mockInput.pressEnter();
  await Bun.sleep(80);
  await test.mockInput.typeText("Archive");
  test.mockInput.pressEnter();
  await capture("41-delete-session");
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
    test.mockInput.pressKey("e", { meta: true });
    for (let attempt = 0; attempt < 100 && !app.composer.plainText.includes("edited_in_editor"); attempt++)
      await Bun.sleep(20);
    if (!app.composer.plainText.includes("edited_in_editor"))
      throw new Error("The external editor did not return its draft.");
    await capture("42-external-editor");
  } finally {
    if (priorEditor === undefined) delete process.env.EDITOR;
    else process.env.EDITOR = priorEditor;
    if (priorVisual === undefined) delete process.env.VISUAL;
    else process.env.VISUAL = priorVisual;
  }
  app.toggleMode();
  app.composer.setText("@");
  await Bun.sleep(100);
  await capture("43-file-picker");
  app.closeOverlay();
  app.composer.setText(`/extension ${resolve("tui/examples/project-summary.ts")}`);
  test.mockInput.pressEnter();
  await Bun.sleep(100);
  app.palette();
  await test.mockInput.typeText("Summarize this project");
  await capture("44-extension-command");
  app.closeOverlay();
  await session.refresh();
  for (let attempt = 0; attempt < 10; attempt++) {
    const pending = session.activity.filter((act) => !act.done && ["prompt", "rung"].includes(act.kind));
    if (!pending.length && !session.queued.length) break;
    await Promise.all(pending.map((act) => session.life.result(act.id).catch(() => {})));
    await session.refresh();
  }
  await session.submit("/undo");
  await capture("45-undo-message");
  await session.submit("/redo");
  await capture("46-redo-message");
} finally {
  app.dispose();
  test.renderer.destroy();
  await library.dispose();
  await extensions.dispose();
}

import { mkdir, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { createTestRenderer } from "@opentui/core/testing";
import { Resvg } from "@resvg/resvg-js";
import { App } from "../src/app.ts";
import { openEngine } from "../src/bridge.ts";
import { demoWorkspace, seedDemo } from "../src/demo.ts";
import { loadParsers } from "../src/parsers.ts";
import { sessionChoices } from "../src/sessions.ts";
import { Workspace } from "../src/workspace.ts";

const output = resolve("docs/screenshots");
await mkdir(output, { recursive: true });
let workspace = await demoWorkspace();
const test = await createTestRenderer({ width: 152, height: 46 });
let app = new App(test.renderer, workspace, { quit() {} });
const xml = (value: string) =>
  value.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;");
const color = (value: { toInts(): number[] }) =>
  `#${value
    .toInts()
    .slice(0, 3)
    .map((part) => part.toString(16).padStart(2, "0"))
    .join("")}`;

async function capture(name: string): Promise<void> {
  app.render();
  await test.flush();
  await Bun.sleep(80);
  await test.flush();
  const frame = test.captureSpans();
  const cell = 9,
    rowHeight = 20,
    top = 36;
  const width = frame.cols * cell,
    height = frame.rows * rowHeight + top;
  const parts = [
    `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}"><rect width="100%" height="100%" fill="#101817"/><rect width="100%" height="${top}" fill="#243329"/><text x="${width / 2}" y="23" text-anchor="middle" font-family="Menlo,DejaVu Sans Mono,monospace" font-size="12" fill="#a3b7a8">furb · ${xml(name.replaceAll("-", " "))}</text>`,
  ];
  ["#e59b88", "#ddc086", "#a7c991"].forEach((fill, i) => {
    parts.push(`<circle cx="${18 + i * 19}" cy="18" r="5" fill="${fill}"/>`);
  });
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
  await seedDemo(workspace);
  await Bun.sleep(300);
  await workspace.refresh();
  await capture("02-conversation");
  workspace.show("program");
  await capture("03-program");
  workspace.show("activity");
  app.render();
  app.scroll.scrollTo(10000);
  await capture("04-activity");
  workspace.show("facts");
  workspace.query = "answer";
  await capture("05-facts");
  workspace.show("transcript");
  await capture("06-transcript");
  await workspace.life.result(
    await workspace.life.rung(
      'write(read("README.md").append("\\n## Keyboard\\nPress Ctrl+K to find a note.\\n"))',
      { on: workspace.selected },
    ),
  );
  await Bun.sleep(60);
  workspace.show("changes");
  await capture("07-changes");
  app.palette();
  await capture("08-command-palette");
  app.closeOverlay();
  app.models();
  await capture("09-models");
  app.closeOverlay();
  await workspace.life.prompt("bool", "Apply the search shortcut to the main chain?", {
    on: workspace.selected,
    to: "operator",
  });
  await Bun.sleep(60);
  await workspace.refresh();
  workspace.show("conversation");
  await capture("10-operator-question");
  app.question();
  await capture("18-operator-dialog");
  app.closeOverlay();
  await workspace.world.answer(workspace.operatorPrompt?.id ?? "", "yes");
  app.toggleMode();
  app.composer.setText('notes = read("README.md")\nprint(notes.content)');
  workspace.show("program");
  await capture("11-python-input");
  workspace.emit("inspect", "notes");
  await Bun.sleep(40);
  await capture("12-value-inspector");
  app.closeOverlay();
  workspace.theme = "paper";
  await capture("13-light-theme");
  workspace.theme = "midnight";
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
  workspace.theme = "forest";
  const command = await workspace.life.bash(
    "printf 'Building the search index...\\n'; sleep 1; printf '3 notes indexed.\\n'",
    { on: workspace.selected },
  );
  await Bun.sleep(60);
  await workspace.refresh();
  workspace.show("activity");
  workspace.query = command;
  await capture("22-live-command");
  await workspace.life.result(command);
  const progress = await workspace.life.prompt("str", "show live progress", { on: workspace.selected });
  await Bun.sleep(60);
  await workspace.refresh();
  workspace.show("conversation");
  await capture("23-model-progress");
  await workspace.life.result(progress);
  await workspace.submit("/run this is invalid python !!!").catch(workspace.fail);
  await capture("19-gate-findings");
  const record = workspace.world.records.path;
  if (!record) throw new Error("The demo session has no record.");
  await workspace.life.wait(60);
  app.dispose();
  await workspace.dispose();
  const resumed = await openEngine({ record, demo: true });
  workspace = new Workspace(resumed.life, resumed.world, true);
  await workspace.refresh();
  app = new App(test.renderer, workspace, { quit() {} });
  await capture("20-paused-resume");
  app.closeOverlay();
  app.openPalette(
    "Sessions",
    await sessionChoices(
      dirname(record),
      async () => {},
      async () => {},
    ),
  );
  await capture("21-sessions");
  app.closeOverlay();
  app.rewind();
  await capture("24-rewind-transcript");
  app.closeOverlay();
  app.ladders();
  test.mockInput.pressEnter();
  await test.flush();
  app.composer.setText("result = len(notes.lines)\nprint(result)");
  await capture("25-prompt-repl");
} finally {
  app.dispose();
  test.renderer.destroy();
  await workspace.dispose();
}

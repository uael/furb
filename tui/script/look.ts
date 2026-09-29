import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { createTestRenderer } from "@opentui/core/testing";
import { follow } from "../src/app.ts";
import { Preferences } from "../src/preferences.ts";
import { hexes, motion } from "../src/theme.ts";
import { Workspaces } from "../src/workspaces.ts";
import { rasterize } from "./raster.ts";
import { highlighting } from "./stage.ts";

// One real life in the TUI, drawn to a picture and a text of the screen while it works. Each argument is a thread that
// the operator sends once the one before it closes, or all at once when LOOK_PARALLEL is set. LOOK_DIR is the project,
// LOOK_OUT the folder of the frames, LOOK_MODEL the model, LOOK_EVERY the milliseconds between two frames, LOOK_HEIGHT
// the rows of the screen, and LOOK_ANSWERS what the operator answers to each question of the model, split by |.
const project = process.env.LOOK_DIR ?? "";
const out = process.env.LOOK_OUT ?? "";
if (!project || !out) throw new Error("LOOK_DIR and LOOK_OUT name the project and the folder of the frames.");
await mkdir(out, { recursive: true });
const preferences = new Preferences(join(out, "ui.json"));
preferences.sidebar = true;
const [model, effort] = (process.env.LOOK_MODEL ?? "claude-cli:opus/high").split("/");
const library = new Workspaces(preferences, { model, effort }, join(out, "workspaces.json"));
await library.create(await library.add(project));
const session = library.current?.session;
if (!session) throw new Error("The session did not open.");
const test = await createTestRenderer({ width: 152, height: Number(process.env.LOOK_HEIGHT) || 46 });
const app = follow(test.renderer, { quit() {}, workspaces: library });
let frame = 0;
async function capture(name: string): Promise<void> {
  await session?.refresh();
  app().render();
  await test.flush();
  await Promise.all(highlighting(test.renderer.root));
  // A step that the view shows first lands bright and settles to its tone, which is the tone an operator reads.
  await new Promise((done) => setTimeout(done, motion.settle));
  await test.flush();
  const file = `${String(frame++).padStart(3, "0")}-${name}`;
  await writeFile(
    join(out, `${file}.png`),
    rasterize(test.captureSpans(), hexes(session?.theme ?? "furb"), "furb"),
  );
  await writeFile(join(out, `${file}.txt`), test.captureCharFrame());
}
const every = Number(process.env.LOOK_EVERY) || 4000;
const answers = (process.env.LOOK_ANSWERS ?? "").split("|").filter(Boolean);
const texts = process.argv.slice(2);
// LOOK_PARALLEL sends every thread at once, and draws the chain while they work, until each one closes.
if (process.env.LOOK_PARALLEL) {
  const threads: string[] = [];
  for (const text of texts) {
    await session.submit(text);
    threads.push(session.thread);
    await session.select(session.selected);
  }
  const started = Date.now();
  const open = () => threads.filter((id) => !session.acts.find((act) => act.id === id)?.done);
  while (open().length) {
    await capture(`chain-${Math.round((Date.now() - started) / 1000)}s`);
    await session.open(open()[0] ?? "");
    await capture(`thread-${Math.round((Date.now() - started) / 1000)}s`);
    await session.select(session.selected);
    await new Promise((done) => setTimeout(done, every));
  }
  await new Promise((done) => setTimeout(done, every * 2));
  await capture("chain-closed");
  for (const [index, id] of threads.entries()) {
    await session.open(id);
    await capture(`t${index}-closed`);
  }
  texts.length = 0;
}
for (const [index, text] of texts.entries()) {
  await session.submit(text);
  const thread = session.thread;
  await capture(`t${index}-sent`);
  const started = Date.now();
  while (!session.acts.find((act) => act.id === thread)?.done) {
    await new Promise((done) => setTimeout(done, every));
    await capture(`t${index}-${Math.round((Date.now() - started) / 1000)}s`);
    // A question of the model waits for the operator, who gives the next of LOOK_ANSWERS, as typed in the input.
    if (session.host.threads.size && answers.length) {
      await capture(`t${index}-asked`);
      await session.submit(answers.shift() ?? "");
      await capture(`t${index}-answered`);
    }
  }
  // A late result of the thread asks its model again, so the life gets room to acknowledge it.
  await new Promise((done) => setTimeout(done, every * 2));
  await capture(`t${index}-closed`);
  // The thread again with every word open, as the operator reads what each word did.
  await session.open(thread);
  session.preferences.foldRungs = false;
  await capture(`t${index}-open`);
  session.preferences.foldRungs = true;
  await session.select(session.selected);
  await capture(`t${index}-chain`);
}
app().dispose();
test.renderer.destroy();
process.exit(0);

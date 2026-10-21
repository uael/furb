import { writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { createTestRenderer } from "@opentui/core/testing";
import { conducting, demoSession, removeDemoDirectories, seedDemoFiles } from "../src/demo.ts";
import { loadParsers } from "../src/parsers.ts";
import { palettes } from "../src/theme.ts";
import { idle } from "../test/idle.ts";
import { gif, pack, type Still } from "./gif.ts";
import { pixels } from "./raster.ts";
import { highlighting, home, mount } from "./stage.ts";

// The model of the demo writes its words as a stream in the animation, as a real model does.
process.env.FURB_DEMO_STREAM = "1";
const directory = join(await home("furb-animation"), "fieldnotes");

/** The terminal of the animation, and the scale of its pictures, which a screen of high density shows sharp. */
const columns = 138,
  rows = 40,
  zoom = 2;
const output = resolve(process.env.FURB_ANIMATION_OUT ?? "docs/furb.gif");
await seedDemoFiles(directory);
const session = await demoSession({ cwd: directory });
await session.command("/name Release 0.4");
// With the kitty keyboard protocol, an Escape is a key of its own that no key after it joins.
const test = await createTestRenderer({ width: columns, height: rows, useMouse: true, kittyKeyboard: true });
const { library, app } = await mount(session, test.renderer);
const stills: Still[] = [];
let size = { width: 0, height: 0 };
/** The screen as it is now, which stays for a time in hundredths of a second. */
async function still(delay: number): Promise<void> {
  // The input is colored by a parser off the main thread, which a moment lets finish.
  await new Promise((done) => setTimeout(done, 30));
  await session.refresh();
  app().render();
  await test.flush();
  await Promise.all(highlighting(test.renderer.root));
  await test.flush();
  const image = pixels(test.captureSpans(), palettes[session.theme], "furb", zoom);
  // The text of each picture goes to a folder that FURB_ANIMATION_TEXT names, to read the animation without it.
  const texts = process.env.FURB_ANIMATION_TEXT;
  if (texts)
    await writeFile(join(texts, `${String(stills.length).padStart(3, "0")}.txt`), test.captureCharFrame());
  size = { width: image.width, height: image.height };
  stills.push(pack(image.pixels, delay));
}
/** Text typed a few characters at a time, each step a picture. */
async function type(text: string, step = 3, delay = 5): Promise<void> {
  for (let at = 0; at < text.length; at += step) {
    await test.mockInput.typeText(text.slice(at, at + step));
    await still(delay);
  }
}
/** The screen while the life works, a picture for each moment, for a count of moments. */
async function moments(count: number, delay = 8): Promise<void> {
  for (let moment = 0; moment < count; moment++) {
    await new Promise((done) => setTimeout(done, delay * 10));
    await still(delay);
  }
}
/** A key, and the picture after it. */
async function press(name: string, delay: number, modifiers?: { ctrl?: boolean }): Promise<void> {
  if (name === "escape") test.mockInput.pressEscape();
  else if (["up", "down", "left", "right"].includes(name)) test.mockInput.pressArrow(name as "up");
  else if (name === "enter") test.mockInput.pressEnter();
  else if (name === "tab") test.mockInput.pressTab();
  else test.mockInput.pressKey(name, modifiers);
  await still(delay);
}
/** The thread of a label selected in the loom, a key at a time. */
async function pick(label: string): Promise<void> {
  for (let step = 0; step < 3 && app().focus !== "loom"; step++) await press("escape", 15);
  const at = app().loom.threads.findIndex((thread) => thread.label === label);
  const by = at - app().selectedThread;
  for (let step = 0; step < Math.abs(by); step++) await press(by > 0 ? "down" : "up", 25);
}

try {
  await loadParsers();
  await still(160);
  // The operator asks the model of the root, and the thread shows the prompt, its rung and its acts.
  await press("i", 30);
  await type("Explore this project, run its checks, and suggest a next step.", 3, 4);
  await press("enter", 10);
  await moments(30);
  await idle(session);
  await still(160);
  await press("escape", 20);
  // A word of python makes a conductor, which hires four workers, each on a chain of its own.
  await type(":", 1, 30);
  app().word.setText(conducting);
  await still(220);
  await press("enter", 10);
  await press("escape", 10);
  await moments(40);
  await still(200);
  // Tab answers the question that a worker put to the operator, wherever it stands.
  await press("tab", 120);
  await type("high", 1, 12);
  await press("enter", 10);
  await moments(12);
  // The folds of the loom hold the conductor and its workers in one thread.
  await pick("Main");
  await press("left", 160);
  await press("right", 80);
  // A worker opens as its ladder, and a rung as its word.
  await pick("w search");
  await press("enter", 140);
  await press("enter", 220);
  await press("escape", 40);
  await press("escape", 60);
  // The verbs act on the selection: space pauses a worker, and x cancels its work.
  await pick("w docs");
  await press(" ", 140);
  await press("x", 160);
  await moments(6);
  // v changes the variant: the graph shows the threads beside the map of the programs, and the messages flow.
  await press("v", 60);
  await moments(20);
  await press("down", 40);
  await press("enter", 60);
  await moments(20);
  await press("escape", 40);
  // The palettes change every surface at once.
  await press("/", 30);
  await type("theme day", 2, 6);
  await press("enter", 220);
  await press("/", 20);
  await type("theme dusk", 3, 6);
  await press("enter", 200);
  // The lights of the title bar are rare pixels, which the palette keeps all the same.
  await writeFile(output, gif(stills, size.width, size.height, [0xff5f57, 0xfebc2e, 0x28c840]));
  console.log(`${output}: ${stills.length} pictures`);
} finally {
  app().dispose();
  test.renderer.destroy();
  await library.dispose();
  await removeDemoDirectories();
}

import { writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { createTestRenderer } from "@opentui/core/testing";
import { openEngine } from "../src/bridge.ts";
import { removeDemoDirectories, seedDemoFiles } from "../src/demo.ts";
import { loadParsers } from "../src/parsers.ts";
import { Session } from "../src/session.ts";
import { palettes } from "../src/theme.ts";
import { idle } from "../test/idle.ts";
import { gif, pack, type Still } from "./gif.ts";
import { pixels } from "./raster.ts";
import { find, highlighting, home, mount } from "./stage.ts";

// The model of the demo writes its words as a stream in the animation, as a real model does.
process.env.FURB_DEMO_STREAM = "1";
const directory = join(await home("furb-animation"), "fieldnotes");

/** The terminal of the animation, and the scale of its pictures, which a screen of high density shows sharp. */
const columns = 138,
  rows = 40,
  zoom = 2;
const output = resolve(process.env.FURB_ANIMATION_OUT ?? "docs/furb.gif");
await seedDemoFiles(directory);
const { engine, host } = await openEngine({ demo: true, cwd: directory });
const session = new Session(engine, host, true);
await session.refresh();
await session.command("/name Explore project");
// With the kitty keyboard protocol, an Escape is a key of its own that no key after it joins.
const test = await createTestRenderer({ width: columns, height: rows, useMouse: true, kittyKeyboard: true });
const { library, app } = await mount(session, test.renderer);
const stills: Still[] = [];
let size = { width: 0, height: 0 };
/** The screen as it is now, which stays for a time in hundredths of a second. */
async function still(delay: number): Promise<void> {
  // The input is colored by a parser off the main thread, which a moment lets finish.
  await new Promise((done) => setTimeout(done, 30));
  app().render();
  await test.flush();
  await Promise.all(highlighting(test.renderer.root));
  await test.flush();
  const image = pixels(test.captureSpans(), palettes[session.theme], "furb", zoom);
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
/** The screen while the session works, a picture for each moment, until it rests. */
async function working(delay = 8): Promise<void> {
  for (let moment = 0; moment < 120; moment++) {
    await new Promise((done) => setTimeout(done, delay * 10));
    await still(delay);
    if (!session.activity.some((act) => !act.done && ["prompt", "rung", "bash"].includes(act.kind))) break;
  }
  await idle(session);
  await session.refresh();
}
/** A click on the first place of the screen that shows a text. */
async function click(text: string): Promise<void> {
  const [column, row] = find(test, text);
  await test.mockMouse.click(column + 1, row);
}

try {
  await loadParsers();
  await still(180);
  await type("Explore this project, run its checks, and suggest a next step.", 2, 4);
  await still(40);
  test.mockInput.pressEnter();
  await working();
  await still(260);
  // Python input: the operator writes a word with the same gate as the model.
  test.mockInput.pressKey("r", { ctrl: true });
  await still(60);
  await type('write(read("README.md").append("\\nPress Ctrl+K to search.\\n"))', 3, 3);
  await still(80);
  test.mockInput.pressEnter();
  await working();
  test.mockInput.pressKey("r", { ctrl: true });
  await still(150);
  await click("Changes");
  await still(200);
  await click("Transcript");
  await still(150);
  await click("Feed");
  await still(80);
  // The palette finds any action by its name or its slash command.
  test.mockInput.pressKey("p", { ctrl: true });
  await still(120);
  await type("rew", 1, 12);
  await still(140);
  test.mockInput.pressEscape();
  await still(30);
  // Escape twice opens the rewind tree in the feed. A picture takes longer to draw than the time between the two
  // presses, so the picture comes after both.
  test.mockInput.pressEscape();
  test.mockInput.pressEscape();
  await still(160);
  if (!test.captureCharFrame().includes("Rewind")) throw new Error("The rewind tree did not open.");
  test.mockInput.pressArrow("down");
  await still(60);
  test.mockInput.pressArrow("down");
  await still(110);
  test.mockInput.pressEscape();
  await still(120);
  // Ctrl+T changes the colors of every surface, and the choice shows at once.
  test.mockInput.pressKey("t", { ctrl: true });
  await still(80);
  await type("paper", 1, 10);
  await still(60);
  test.mockInput.pressEnter();
  await still(200);
  test.mockInput.pressKey("t", { ctrl: true });
  await still(50);
  await type("github", 1, 8);
  test.mockInput.pressEnter();
  await still(220);
  // The lights of the title bar are rare pixels, which the palette keeps all the same.
  await writeFile(output, gif(stills, size.width, size.height, [0xff5f57, 0xfebc2e, 0x28c840]));
  console.log(`${output}: ${stills.length} pictures`);
} finally {
  app().dispose();
  test.renderer.destroy();
  await library.dispose();
  await removeDemoDirectories();
}

import { expect, test } from "bun:test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createTestRenderer, type RecordedFrame, TestRecorder } from "@opentui/core/testing";
import { mount } from "../script/stage.ts";
import { demoSession, removeDemoDirectories, seedDemoFiles } from "../src/demo.ts";
import { loadParsers } from "../src/parsers.ts";

const COLUMNS = 138,
  ROWS = 40;
/** The lines of the word that the demo model streams first, in their order. */
const LINES = ['notes = read("README.md")', "check = await bash("];
/** The color that the renderer gives a cell that no color was given: the color of code drawn plain. */
const PLAIN = "255,255,255,255";

/** Each frame that the renderer drew while the model of the demo streamed its first word and ran it. */
async function streamed(): Promise<RecordedFrame[]> {
  process.env.FURB_DEMO_STREAM = "1";
  const directory = await mkdtemp(join(tmpdir(), "furb-flicker-"));
  await seedDemoFiles(directory);
  const session = await demoSession({ cwd: directory });
  const screen = await createTestRenderer({ width: COLUMNS, height: ROWS, useMouse: true, exitSignals: [] });
  const { library, app } = await mount(session, screen.renderer);
  const recorder = new TestRecorder(screen.renderer, { recordBuffers: { fg: true } });
  const busy = () =>
    session.activity.some((act) => !act.done && ["prompt", "rung", "bash"].includes(act.kind));
  const pause = (milliseconds: number) => new Promise((done) => setTimeout(done, milliseconds));
  try {
    await loadParsers();
    await screen.flush();
    screen.renderer.start();
    recorder.rec();
    await screen.mockInput.typeText("Explore this project, run its checks, and suggest a next step.");
    screen.mockInput.pressEnter();
    for (let waited = 0; waited < 20000 && !busy(); waited += 50) await pause(50);
    for (let waited = 0; waited < 20000 && busy(); waited += 50) await pause(50);
    await pause(500);
    recorder.stop();
    return recorder.recordedFrames;
  } finally {
    screen.renderer.stop();
    app().dispose();
    screen.renderer.destroy();
    await library.dispose();
    await removeDemoDirectories();
    await rm(directory, { recursive: true, force: true });
  }
}

/** The character and the color of each cell of a frame, row by row. */
function cells(frame: RecordedFrame): { char: string; fg: string }[] {
  const chars = frame.frame.split("\n").flatMap((line) => [...line.padEnd(COLUMNS)].slice(0, COLUMNS));
  return chars.map((char, at) => ({
    char,
    fg: [...(frame.buffers?.fg?.slice(at * 4, at * 4 + 4) ?? [])].join(","),
  }));
}

test("no frame draws a cell of code plain between two frames that draw it in its colors, while a model streams", async () => {
  const frames = (await streamed()).map(cells);
  expect(frames.length).toBeGreaterThan(20);
  const blinks: string[] = [];
  for (let k = 1; k + 1 < frames.length; k++)
    for (const [at, now] of (frames[k] ?? []).entries()) {
      const before = frames[k - 1]?.[at],
        after = frames[k + 1]?.[at];
      if (!before || !after || now.char === " " || now.fg !== PLAIN) continue;
      if (before.char === now.char && after.char === now.char && before.fg !== PLAIN && after.fg !== PLAIN)
        blinks.push(`frame ${k}, row ${Math.floor(at / COLUMNS)}, column ${at % COLUMNS}`);
    }
  // Code shows in the colors of its lines in the first frame that makes its card again.
  expect(blinks).toEqual([]);
  // The lines of the word were drawn in their colors at all.
  const last = frames.at(-1) ?? [];
  const first = LINES[0] ?? "";
  const row = last.findIndex(
    (_, at) =>
      last
        .slice(at, at + first.length)
        .map((one) => one.char)
        .join("") === first,
  );
  expect(row).toBeGreaterThanOrEqual(0);
  expect(last[row]?.fg).not.toBe(PLAIN);
}, 30000);

test("the row that a model writes takes the next number in the frame that shows the line that it finished", async () => {
  const frames = await streamed();
  const numbers: number[] = [];
  for (const frame of frames) {
    const lines = frame.frame.split("\n");
    const row = lines.findIndex((line) => /^\s*[\u2800-\u28ff]\s+\d+ /.test(line));
    if (row < 0) continue;
    const number = Number(/(\d+) /.exec(lines[row] ?? "")?.[1]);
    numbers.push(number);
    // Each line before the number of the row stands above it as code, in the same frame.
    const above = lines.slice(0, row).filter((line) => !/[\u2800-\u28ff]/.test(line.slice(0, 8)));
    for (const line of LINES.slice(0, number - 1)) expect(above.some((one) => one.includes(line))).toBe(true);
  }
  expect(Math.max(...numbers)).toBeGreaterThanOrEqual(3);
}, 30000);

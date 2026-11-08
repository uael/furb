// The owner types @ and a part of a file name in a folder of 50000 files, or from ../ in a folder of 50000 files beside
// it: each key shows at once, and the list of the picker fills in after, as its walk and its match end.
import { expect } from "bun:test";
import { mkdir, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { scenario, test, type Window } from "./harness.ts";

/** A tree of 500 folders of 100 files. */
async function tree(at: string): Promise<void> {
  for (let folder = 0; folder < 500; folder++) {
    await mkdir(join(at, `folder${folder}`), { recursive: true });
    await Promise.all(
      Array.from({ length: 100 }, (_, file) => writeFile(join(at, `folder${folder}`, `file${file}.ts`), "")),
    );
  }
}

/** Whether the picker still walked or matched as each key of a query showed: each key is pressed and the next frame
 * shows it. A picker that walks and ranks on the thread of the window ends its walk before the frame of the key, and
 * a picker that never waits shows the key with its walk still running. The order of the frame and of the walk is what
 * the test reads, and the load of the machine only makes the walk run longer, so no load moves the check. */
async function keys(window: Window, query: string): Promise<boolean[]> {
  const running: boolean[] = [];
  let typed = "";
  for (const key of query) {
    typed += key;
    window.screen.mockInput.pressKey(key);
    expect(await window.frame()).toContain(typed);
    running.push(window.app.filePicker?.running ?? false);
  }
  return running;
}

/** A query of the picker from its @, typed as fast as the window draws it, and then the meant file. */
async function picks(window: Window, query: string, meant: string): Promise<void> {
  await window.screen.mockInput.typeText("look at the ");
  expect(await keys(window, query), "the picker ran at the frame of a key").toContain(true);
  await window.shows((frame) => frame.includes(meant), "the meant file", 30);
}

test("a key of the file picker in a folder of 50000 files never waits for the walk or the match", () =>
  scenario([], async (window, yard) => {
    await tree(yard.work);
    await picks(window, "@folder42/file7", "folder42/file7.ts");
  }));

test("a key of the file picker in a folder of 50000 files beside the folder of the life never waits for its walk", () =>
  scenario([], async (window, yard) => {
    await tree(join(dirname(yard.work), "other"));
    await picks(window, "@../other/folder42/file7", "other/folder42/file7.ts");
  }));

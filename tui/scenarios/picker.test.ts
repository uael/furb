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

/** The time of each key of a text, typed as fast as the window draws it: from its press to the frame that shows it, so
 * it holds the suggestions that the key draws. */
async function keys(window: Window, text: string): Promise<number[]> {
  const waited: number[] = [];
  let typed = "";
  for (const key of text) {
    typed += key;
    const began = performance.now();
    window.screen.mockInput.pressKey(key);
    const frame = await window.frame();
    waited.push(performance.now() - began);
    expect(frame).toContain(typed);
  }
  return waited;
}

/** Words with no picker, then a query of the picker from its @, while its walk runs, and then the meant file. A picker
 * that walks and ranks on the thread of the window makes a key of the picker take three to ten times a key with no
 * picker in the folder, and near a second outside it, where it walked at each key. A key of a picker that never waits
 * takes about the time of a key with no picker, and up to three times it when the walk and the match on threads of
 * their own share a loaded machine with the window; the middle key holds a key that the machine slows now and then. */
async function picks(window: Window, query: string, meant: string): Promise<void> {
  const plain = await keys(window, "look at the ");
  const picked = await keys(window, query);
  const middle = (waited: number[]) =>
    [...waited].sort((one, other) => one - other)[Math.floor(waited.length / 2)] ?? 0;
  const shown = (waited: number[]) => waited.map((one) => one.toFixed(0)).join(" ");
  expect(middle(picked), `with no picker ${shown(plain)}, in the picker ${shown(picked)}`).toBeLessThan(
    3 * middle(plain) + 10,
  );
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

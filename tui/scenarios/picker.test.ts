// The owner types @ and a part of a file name in a folder of 50000 files: each key shows at once, and the list of the
// picker fills in after, as its walk and its match end.
import { expect } from "bun:test";
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { scenario, test } from "./harness.ts";

test("a key of the file picker in a folder of 50000 files never waits for the walk or the match", () =>
  scenario([], async (window, yard) => {
    for (let folder = 0; folder < 500; folder++) {
      await mkdir(join(yard.work, `folder${folder}`));
      await Promise.all(
        Array.from({ length: 100 }, (_, file) =>
          writeFile(join(yard.work, `folder${folder}`, `file${file}.ts`), ""),
        ),
      );
    }
    // The keys come as fast as the window draws them: words with no picker, then the @ that opens it and a name, while
    // its walk runs. The time of a key runs from its press to the frame that shows it, so it holds the suggestions that
    // the key draws.
    let typed = "";
    const keys = async (text: string) => {
      const waited: number[] = [];
      for (const key of text) {
        typed += key;
        const began = performance.now();
        window.screen.mockInput.pressKey(key);
        const frame = await window.frame();
        waited.push(performance.now() - began);
        expect(frame).toContain(typed);
      }
      return waited;
    };
    const middle = (waited: number[]) =>
      [...waited].sort((one, other) => one - other)[Math.floor(waited.length / 2)] ?? 0;
    const plain = await keys("look at the ");
    const picked = await keys("@folder42/file7");
    // A picker that walks the folder and ranks the whole list on the thread of the window makes a key of the picker
    // take three to seven times a key with no picker on this tree. A key of a picker that never waits takes about the
    // time of a key with no picker, which a loaded machine slows as much, and the middle key holds a key that it slows
    // now and then.
    const told = `with no picker ${plain.map((one) => one.toFixed(0)).join(" ")}, in the picker ${picked.map((one) => one.toFixed(0)).join(" ")}`;
    expect(middle(picked), told).toBeLessThan(2 * middle(plain) + 5);
    await window.shows((frame) => frame.includes("folder42/file7.ts"), "the meant file", 30);
  }));

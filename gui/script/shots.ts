// Captures of the screens of the app, in WebKit, the engine of the web view of Tauri on macOS, on the fixture that the
// build holds, or on the one that the build took from FURB_FIXTURE: run `vite build` in gui, then
// `bun run shots [directory]`. Each capture has its text beside it, in a file of the same name.

import { spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { webkit } from "playwright";

const out = process.argv[2] ?? "captures";
const screens: [name: string, query: string][] = [
  ["1-thread", "thread=prompt3"],
  ["2-thread-paper", "thread=prompt3&theme=paper"],
  ["3-picker", "picker=1"],
];

// FURB_SHOTS names other screens, as name=query pairs split by spaces, as for the fixture of a record.
const asked = process.env.FURB_SHOTS?.split(" ").map((one): [string, string] => {
  const at = one.indexOf("=");
  return [one.slice(0, at), one.slice(at + 1).replaceAll(",", "&")];
});
if (asked?.length) screens.splice(0, screens.length, ...asked);

await mkdir(out, { recursive: true });
const server = spawn("./node_modules/.bin/vite", ["preview"], { stdio: "ignore" });
await new Promise((done) => setTimeout(done, 1500));
const browser = await webkit.launch();
try {
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 2 });
  for (const [name, query] of screens) {
    await page.goto(`http://localhost:4174/?${query}`);
    await page.waitForTimeout(Number(process.env.FURB_SHOTS_WAIT ?? 1200));
    await page.screenshot({ path: join(out, `${name}.png`) });
    // The text of each capture beside it, so a reader with no eyes on the image reads what it shows.
    await writeFile(join(out, `${name}.txt`), await page.evaluate(() => document.body.innerText));
  }
} finally {
  await browser.close();
  server.kill();
}

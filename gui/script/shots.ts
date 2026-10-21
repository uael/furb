// Captures of the screens of the app, in WebKit, the engine of the web view of Tauri on macOS, on the fixture of the
// demo, or on the one that the build took from FURB_FIXTURE: run `vite build` in gui, then `bun run shots [directory]`.

import { spawn } from "node:child_process";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { webkit } from "playwright";

const out = process.argv[2] ?? "captures";
const screens: [name: string, query: string][] = [
  ["1-thread", "screen=thread&thread=prompt3"],
  ["2-programs-out", "screen=programs&zoom=out"],
  ["3-programs-in", "screen=programs&zoom=in"],
  ["4-code", "screen=code&thread=prompt3"],
  ["5-past", "screen=thread&thread=prompt1&past=0.4"],
  ["6-door", "screen=code&thread=prompt3&door=1"],
  ["7-thread-paper", "screen=thread&thread=prompt3&theme=paper"],
  ["8-programs-paper", "screen=programs&theme=paper"],
  ["9-picker", "picker=1"],
];

// FURB_SHOTS names other screens, as name=query pairs split by spaces, as for the fixture of a record.
const asked = process.env.FURB_SHOTS?.split(" ").map((one): [string, string] => {
  const at = one.indexOf("=");
  return [one.slice(0, at), one.slice(at + 1).replaceAll(",", "&")];
});
if (asked?.length) screens.splice(0, screens.length, ...asked);
screens.splice(
  0,
  screens.length,
  ...asked.map(([name, query]) => [name, query.replaceAll(",", "&")] as [string, string]),
);

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
  }
} finally {
  await browser.close();
  server.kill();
}

// Captures of the screens of the app, in WebKit, the engine of the web view of Tauri on macOS, on the view of the
// demo: run `bun run build` in gui, then `bun run shots [directory]`.

import { spawn } from "node:child_process";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { webkit } from "playwright";

const out = process.argv[2] ?? "captures";
const screens: [name: string, query: string][] = [
  ["1-thread", "screen=thread&thread=prompt3"],
  ["2-programs", "screen=programs&thread=prompt3"],
  ["3-code", "screen=code&thread=prompt3"],
  ["4-thread-root", "screen=thread&thread=prompt1"],
  ["5-thread-paper", "screen=thread&thread=prompt3&theme=paper"],
  ["6-programs-paper", "screen=programs&thread=prompt3&theme=paper"],
  ["7-code-paper", "screen=code&thread=prompt3&theme=paper"],
];

await mkdir(out, { recursive: true });
const server = spawn("./node_modules/.bin/vite", ["preview"], { stdio: "ignore" });
await new Promise((done) => setTimeout(done, 1500));
const browser = await webkit.launch();
try {
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 2 });
  for (const [name, query] of screens) {
    await page.goto(`http://localhost:4174/?${query}`);
    await page.waitForTimeout(900);
    await page.screenshot({ path: join(out, `${name}.png`) });
  }
} finally {
  await browser.close();
  server.kill();
}

// Captures of a life in the app, in WebKit, the engine of the web view of Tauri on macOS: run `vite build` in gui,
// then `bun run shots [directory]`. The script starts a host on the demo of the TUI, which asks no model, and drives
// the UI as a person does: it starts a life, sends a message, captures what the model writes while it streams and
// its answer, sends a message whose model asks a question, captures the question, answers it, and captures the
// thread once it goes on. Then it opens the menu of an act, cancels work that runs after it confirms, and rewinds to
// a message whose answer wrote a file, with the files. Each capture has its text beside it, in a file of the same name. A screen that does not
// come in a minute ends the run with an error that names it.

import { spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { type Page, webkit } from "playwright";
import { startHost } from "./started.ts";

const out = process.argv[2] ?? "captures";
await mkdir(out, { recursive: true });
// The demo streams slowly enough that a capture catches its model as it writes.
const host = await startHost(["--demo"], { ...process.env, FURB_DEMO_PACE: "120" });
const server = spawn(join(import.meta.dir, "..", "node_modules", ".bin", "vite"), ["preview"], {
  cwd: join(import.meta.dir, ".."),
  stdio: "ignore",
});
await new Promise((done) => setTimeout(done, 1500));
const browser = await webkit.launch();

/** A screen captured, with its text beside it, so a reader with no eyes on the image reads what it shows. */
async function shot(page: Page, name: string): Promise<void> {
  await page.waitForTimeout(400);
  await page.screenshot({ path: join(out, `${name}.png`) });
  await writeFile(join(out, `${name}.txt`), await page.evaluate(() => document.body.innerText));
}

/** A text that the page shows within a minute, or an error that names the screen that waits for it. */
async function shows(page: Page, screen: string, text: string | RegExp): Promise<void> {
  try {
    await page.getByText(text).first().waitFor({ timeout: 60000 });
  } catch {
    throw new Error(`The screen ${screen} did not come: the page shows no ${text}.`);
  }
}

/** A message typed in the composer and sent with Enter, as a person sends it. */
async function send(page: Page, text: string): Promise<void> {
  const box = page.locator("textarea");
  await box.fill(text);
  await box.press("Enter");
}

try {
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 2 });
  const address = (theme: string) =>
    `http://localhost:4174/?theme=${theme}&host=${encodeURIComponent(host.url)}`;
  await page.goto(address("ink"));
  await shows(page, "1-life", "Send a message to start the work.");
  await shot(page, "1-life");
  await send(page, "Read this project and show live progress.");
  await shows(page, "2-streams", "Its model writes this step");
  await page.waitForTimeout(1500);
  await shot(page, "2-streams");
  await shows(page, "3-answer", "Session 1 · Ready");
  await shot(page, "3-answer");
  await send(page, "Ask me first: which effort?");
  await shows(page, "4-asks", "Which effort should the next step take?");
  await shot(page, "4-asks");
  await send(page, "high");
  await shows(page, "5-answered", /Your answer went to/);
  await shot(page, "5-answered");
  await page.goto(address("paper"));
  await shows(page, "6-paper", "Done. The change is small, its checks pass, and the result is in the feed.");
  await shot(page, "6-paper");
  await page.goto(`${address("ink")}&picker=1`);
  await shows(page, "7-picker", "New life");
  await shot(page, "7-picker");
  await page.goto(address("ink"));
  await send(page, "Hire two workers");
  await shows(page, "8-menu", "Think until I cancel.");
  await page
    .getByRole("button", { name: /^Controls of prompt/ })
    .first()
    .click();
  await shows(page, "8-menu", /^Cancel prompt/);
  await shot(page, "8-menu");
  await page
    .getByText(/^Cancel prompt/)
    .first()
    .click();
  await shows(page, "9-confirm", "Keep it running");
  await shot(page, "9-confirm");
  await page.getByText(/^Cancel it and/).click();
  await shows(page, "9-confirm", /is cancelled/);
  await send(page, "Add the search shortcut, Ctrl+K.");
  await shows(page, "10-rewind", "Ctrl+K opens the search now, and the README says so.");
  await page
    .getByRole("button", { name: /^Controls of prompt/ })
    .first()
    .click();
  await page
    .getByText(/^Rewind to prompt/)
    .first()
    .click();
  await shows(page, "10-rewind", "Restore the files");
  await shot(page, "10-rewind");
  await page.getByText("Both", { exact: true }).click();
  await shows(page, "11-branch", "A new branch of the chain: send a message to go on from there.");
  await shot(page, "11-branch");
} finally {
  await browser.close();
  server.kill();
  host.stop();
}

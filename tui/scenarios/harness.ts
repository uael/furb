// The harness of the scenarios: a life of the owner's own path. A scripted model, the program scenario.ts of the
// programs of the tests, answers a real host, which a headless window of the TUI drives with keys and the mouse on a
// test terminal, as the owner does. Each scenario holds a yard of its own: the folder of the life, the configs, the
// record and the model.
import { test as each, setDefaultTimeout } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { mkdir, mkdtemp, realpath, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createTestRenderer } from "@opentui/core/testing";
import { executable, programs } from "../../bind/typescript/test/executable.ts";
import * as stage from "../script/stage.ts";
import { App } from "../src/app.ts";
import { type Engine, openEngine } from "../src/bridge.ts";
import { aboutOf, hostPlace } from "../src/hosting.ts";
import type { EngineOptions } from "../src/models.ts";
import { Preferences } from "../src/preferences.ts";
import { Session } from "../src/session.ts";
import { Workspaces } from "../src/workspaces.ts";

/** A rule of the script of the model: the cue that a turn reads, and the word that answers it, streamed in parts of
 * `chunk` characters `pause` milliseconds apart, and held until the file `hold` stands beside the model. */
export interface Rule {
  cue: string;
  word: string;
  chunk?: number;
  pause?: number;
  hold?: string;
}

// A scenario starts a host, a model and a window, and a loaded machine of CI runs many at once.
setDefaultTimeout(120000);

/** The test of a scenario. On Windows each life stays in its window, with no host to quit, kill or attach to, so the
 * scenarios run on macOS and Linux. */
export const test = process.platform === "win32" ? each.skip : each;

/** The model of every scenario. */
export const MODEL = "claude-cli:opus";

/** The moment a condition holds, read again every 25 ms, or a failure that names what did not come. */
export async function until(
  ready: () => unknown | Promise<unknown>,
  what: string,
  seconds = 60,
): Promise<void> {
  for (const deadline = Date.now() + seconds * 1000; !(await ready()); await Bun.sleep(25))
    if (Date.now() > deadline) throw new Error(`Waited ${seconds} seconds for ${what}.`);
}

/** Whether a process runs. */
export function alive(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

/** The yard of a scenario: its folders, its record, its model and the options of each window that it opens. */
export class Yard {
  constructor(
    readonly root: string,
    readonly work: string,
    readonly model: string,
    readonly record: string,
  ) {}
  static async make(script: Rule[], config: Record<string, unknown> = {}): Promise<Yard> {
    const root = await realpath(await mkdtemp(join(tmpdir(), "furb-scenario-")));
    const [work, home, configs] = [join(root, "work"), join(root, "model"), join(root, "config")];
    for (const one of [work, home, configs, join(work, ".furb", "sessions")])
      await mkdir(one, { recursive: true });
    // The life names no session: a name asks the model a question that no scenario scripts.
    await writeFile(
      join(configs, "config.json"),
      JSON.stringify({ extensions: { naming: false }, ...config }),
    );
    executable(join(programs, "scenario.ts"), home, "claude");
    const yard = new Yard(root, work, home, join(work, ".furb", "sessions", "life.jsonl"));
    await yard.script(script);
    // The host that a window starts reads the configs of the yard alone.
    process.env.FURB_CONFIG_DIR = configs;
    return yard;
  }
  /** Each record that a window of the yard opened, whose host the end of the yard ends. */
  readonly records = new Set<string>();
  /** The script that the model answers from, which a scenario may change while the life runs. */
  async script(rules: Rule[]): Promise<void> {
    await writeFile(join(this.model, "script.json"), JSON.stringify(rules));
  }
  /** Let the replies that wait for the file go. */
  async release(hold: string): Promise<void> {
    await writeFile(join(this.model, hold), "");
  }
  /** Each turn that the model answered: the cue that it answered, or null, and the text that it read. */
  turns(): { cue: string | null; text: string }[] {
    const file = join(this.model, "turns");
    if (!existsSync(file)) return [];
    return readFileSync(file, "utf8")
      .split("\n")
      .filter(Boolean)
      .map((line) => JSON.parse(line));
  }
  /** The options of a window on the record of the yard, or on another record in its folder of sessions. */
  options(record = this.record): EngineOptions {
    const claude = join(this.model, process.platform === "win32" ? "claude.exe" : "claude");
    return { host: true, cwd: this.work, record, claude, model: MODEL, roster: [MODEL] };
  }
  /** The process of the host of a record of the yard, while one runs. */
  host(record = this.record): number | undefined {
    const place = hostPlace(record);
    const about = place && aboutOf(place);
    return about && alive(about.pid) ? about.pid : undefined;
  }
  /** End the host that a scenario left, then remove the yard. */
  async remove(): Promise<void> {
    for (const record of this.records) {
      const pid = this.host(record);
      if (!pid) continue;
      process.kill(pid, "SIGTERM");
      await until(() => !alive(pid), "the host to end", 15).catch(() => process.kill(pid, "SIGKILL"));
    }
    await rm(this.root, { recursive: true, force: true });
  }
}

/** A headless window of the TUI on the life of a yard, in a host of its own: the session, the App on a test terminal,
 * and what a scenario does through them. */
export class Window {
  private constructor(
    readonly session: Session,
    readonly app: App,
    readonly screen: Awaited<ReturnType<typeof createTestRenderer>>,
    readonly library: Workspaces,
  ) {}
  static async open(yard: Yard, record = yard.record, size = { width: 140, height: 44 }): Promise<Window> {
    const options = yard.options(record);
    yard.records.add(record);
    const { engine, host } = await openEngine(options);
    const preferences = new Preferences(join(yard.root, "ui", "ui.json"));
    const session = new Session(engine, host, false, preferences);
    await session.refresh();
    const library = new Workspaces(preferences, options, join(yard.root, "ui", "workspaces.json"));
    await library.select(library.adopt(session, await library.add(yard.work)));
    const screen = await createTestRenderer({ exitSignals: [], ...size });
    const app = new App(screen.renderer, session, { quit() {}, workspaces: library });
    await screen.flush();
    app.composer.focus();
    return new Window(session, app, screen, library);
  }
  /** The columns of the test terminal. */
  get width(): number {
    return this.screen.renderer.width;
  }
  get engine(): Engine {
    return this.session.engine;
  }
  get root(): string {
    return this.session.engine.root;
  }
  /** The frame that the window draws now. */
  async frame(): Promise<string> {
    this.app.render();
    await this.screen.flush();
    return this.screen.captureCharFrame();
  }
  /** The moment a frame holds a condition, drawn again every 25 ms. */
  async shows(ready: (frame: string) => boolean, what: string, seconds = 60): Promise<string> {
    let shown = "";
    const drawn = async () => {
      shown = await this.frame();
      return ready(shown);
    };
    await until(drawn, what, seconds).catch((error: Error) => {
      throw new Error(`${error.message}\nThe last frame:\n${shown}`);
    });
    return shown;
  }
  /** Type a line in the input, and send it with Enter, as the operator does. */
  async type(line: string): Promise<void> {
    this.app.composer.focus();
    await this.screen.mockInput.typeText(line);
    this.screen.mockInput.pressEnter();
    await this.frame();
  }
  /** A click on the first place of the frame that shows a text. */
  async click(text: string): Promise<void> {
    await this.frame();
    await stage.click(this.screen, text);
    await this.frame();
  }
  /** The window goes, and the life goes on in its host, unless `end` ends it. */
  async close(end = false): Promise<void> {
    this.app.dispose();
    this.screen.renderer.destroy();
    if (end) await this.session.host.end().catch(() => {});
    await this.library.dispose();
  }
}

/** A scenario: a yard with its script, a window on it, and the end of each after the scenario, whatever it came to. */
export async function scenario(
  script: Rule[],
  use: (window: Window, yard: Yard) => Promise<void>,
  config?: Record<string, unknown>,
): Promise<void> {
  const yard = await Yard.make(script, config);
  let window: Window | undefined;
  try {
    window = await Window.open(yard);
    await use(window, yard);
  } catch (error) {
    // What the model read at each turn says most of why a scenario failed.
    for (const [n, turn] of yard.turns().entries())
      console.error(`--- turn ${n + 1}, answered as ${JSON.stringify(turn.cue)}:\n${turn.text.slice(-3000)}`);
    throw error;
  } finally {
    await window?.close(true).catch(() => {});
    await yard.remove();
  }
}

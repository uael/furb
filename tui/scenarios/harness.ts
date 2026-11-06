// The harness of the scenarios: a life of the owner's own path. A scripted model, the program scenario.ts of the
// programs of the tests, answers a real host, which a headless window of the TUI drives with keys and the mouse on a
// test terminal, as the owner does. Each scenario holds a yard of its own: the folder of the life, the configs, the
// record and the model.
import { test as each, setDefaultTimeout } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { chmod, mkdir, mkdtemp, realpath, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createTestRenderer } from "@opentui/core/testing";
import { executable, programs } from "../../binding/typescript/test/executable.ts";
import * as stage from "../script/stage.ts";
import { App } from "../src/app.ts";
import { type Engine, openEngine } from "../src/bridge.ts";
import { daemon, furbProgram } from "../src/client.ts";
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

/** The grace of the daemon of a yard, in seconds: a window that quits and one that opens just after hold one life, and
 * a life that a window ends is gone soon after. */
const GRACE = 2;

/** The program furb of the run, read before a yard names its own in FURB_BIN. */
const PROGRAM = furbProgram();

/** The yard of a scenario: its folders, its record, its model and the options of each window that it opens. */
export class Yard {
  constructor(
    readonly root: string,
    readonly work: string,
    readonly model: string,
    readonly record: string,
    readonly cache: string,
  ) {}
  static async make(script: Rule[], config: Record<string, unknown> = {}): Promise<Yard> {
    // The daemon of the yard is its own, in a cache of its own, so that a scenario that kills it kills no life of
    // another test. Its socket needs a short path, since a socket has a bound on it.
    const cache = await realpath(await mkdtemp(join("/tmp", "furb-scenario-")));
    const program = PROGRAM;
    const served = join(cache, "furb-host");
    // The daemon that a window starts writes its process beside its socket, which a scenario kills.
    await writeFile(
      served,
      `#!/bin/sh\necho $$ > '${join(cache, "pid")}'\nexec '${program}' "$@" --grace ${GRACE}\n`,
    );
    await chmod(served, 0o755);
    process.env.XDG_CACHE_HOME = cache;
    process.env.FURB_BIN = served;
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
    const yard = new Yard(root, work, home, join(work, ".furb", "sessions", "life.jsonl"), cache);
    await yard.script(script);
    // The host that a window starts reads the configs of the yard alone.
    process.env.FURB_CONFIG_DIR = configs;
    return yard;
  }
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
    return { cwd: this.work, record, claude, model: MODEL, roster: [MODEL] };
  }
  /** The process of the daemon of the yard, which holds its lives, while one runs. */
  host(): number | undefined {
    const file = join(this.cache, "pid");
    const pid = existsSync(file) ? Number(readFileSync(file, "utf8").trim()) : 0;
    return pid && alive(pid) ? pid : undefined;
  }
  /** The end of the life of a record of the yard, which its daemon holds no more. */
  async ended(record = this.record): Promise<void> {
    await daemon().ended(record, (GRACE + 30) * 1000);
  }
  /** The record as it stood before the end of its life wrote its checkpoint, which a life that opens on it goes on
   * from with turns of its own: with the checkpoint cut, the next life replays each word from the first entry. It
   * gives how many lines of checkpoint it cut. */
  async uncheckpoint(record = this.record): Promise<number> {
    const lines = readFileSync(record, "utf8").split("\n").filter(Boolean);
    let kept = lines.length;
    while (kept > 0 && (JSON.parse(lines[kept - 1] ?? "[]") as unknown[][])[0]?.[0] === "checkpoint") kept--;
    await writeFile(
      record,
      lines
        .slice(0, kept)
        .map((line) => `${line}\n`)
        .join(""),
    );
    return lines.length - kept;
  }
  /** End the daemon that a scenario left, then remove the yard. */
  async remove(): Promise<void> {
    // The window dials no more, so it starts no daemon again.
    daemon().close();
    const pid = this.host();
    if (pid) {
      process.kill(pid, "SIGTERM");
      await until(() => !alive(pid), "the daemon to end", 15).catch(() => process.kill(pid, "SIGKILL"));
    }
    await rm(this.root, { recursive: true, force: true });
    await rm(this.cache, { recursive: true, force: true });
  }
}

/** A headless window of the TUI on the life of a yard, in the daemon of the yard: the session, the App on a test
 * terminal, and what a scenario does through them. */
export class Window {
  private constructor(
    readonly session: Session,
    readonly app: App,
    readonly screen: Awaited<ReturnType<typeof createTestRenderer>>,
    readonly library: Workspaces,
    readonly yard: Yard,
    readonly record: string,
  ) {}
  static async open(yard: Yard, record = yard.record, size = { width: 140, height: 44 }): Promise<Window> {
    const options = yard.options(record);
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
    return new Window(session, app, screen, library, yard, record);
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
    // The composer takes no Enter while it sends the text before, and a life in the daemon answers that send later
    // than a life in the window did.
    await until(() => !this.app.submitting, "the send before this line", 30);
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
  /** The window goes, and the life goes on in the daemon for its grace, or, with `end`, the window waits for its end. */
  async close(end = false): Promise<void> {
    this.app.dispose();
    this.screen.renderer.destroy();
    await this.library.dispose();
    await this.session.host.dispose().catch(() => {});
    if (end) await this.yard.ended(this.record).catch(() => {});
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

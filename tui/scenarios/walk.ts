// The walk of the audit of the TUI: a headless window drives a life with keys, clicks and sizes, as the owner does in
// his terminal, and keeps each frame that it met, with the action that led to it, for the judges of the audit. Its
// probes read each frame and each step with no judge, and find what a person sees at once: a key that does nothing, a
// dialog that leaves a trace when it closes, an input that no longer takes the keys, a value of the code on the screen,
// an error that the window logged, a life that says it is not open, and an answer that never came. The skill
// converge-tui runs it on a life of a scripted model and on copies of the newest records of the project, which open
// read-only, so no walk asks a model.
import { mkdir, writeFile } from "node:fs/promises";
import { basename, dirname, join } from "node:path";
import { createTestRenderer } from "@opentui/core/testing";
import { cells, rasterize } from "../script/raster.ts";
import { highlighting, mount } from "../script/stage.ts";
import type { App } from "../src/app.ts";
import { openEngine } from "../src/bridge.ts";
import { bindings } from "../src/keys.ts";
import { loadParsers } from "../src/parsers.ts";
import { Session } from "../src/session.ts";
import { paletteOf } from "../src/theme.ts";
import { changed, chord, type Finding, leaks, shut } from "./probes.ts";
import { type Rule, until, Window, Yard } from "./yard.ts";

type Screen = Awaited<ReturnType<typeof createTestRenderer>>;

/** What a walk drives: the test terminal, the App on it, and the palette that draws its images. */
interface View {
  screen: Screen;
  app: () => App;
  palette: () => ReturnType<typeof paletteOf>;
}

export interface Step {
  name: string;
  action: string;
}

/** The action of the last step of each journey, so that a walk shows that it went to the end. */
export const END = "the end of the journey";

/** How long a step waits for what it expects, in seconds, before a probe finds it stuck. */
const BOUND = 60;

/** The sizes of a terminal that a walk shrinks the window to, as the owner does when he splits his screen. */
const SIZES = [
  [100, 30],
  [80, 24],
  [60, 20],
] as const;

const SIZE = { width: 140, height: 44 };

/** A mark that a walk types to see that the input takes the keys: no key of the table answers its letters. */
const MARK = "zzqq";

/** The actions of the table of keys that a walk leaves alone: they reach outside the window (the editor, the clipboard,
 * the end of the window), or change the life that the walk drives (a new chain, the take back of a message, a pause,
 * the journal of file changes), or move in a view that the walk does not open. */
const OUTSIDE = new Set([
  "editor",
  "image",
  "copy",
  "quit",
  "newChain",
  "takeBack",
  "pause",
  "repair",
  "renew",
  "scope",
]);

/** The actions that open a view that Esc closes, back to the input. */
const VIEWS = new Set([
  "palette",
  "chains",
  "models",
  "effort",
  "themes",
  "workspaces",
  "search",
  "python",
  "ladders",
  "names",
  "help",
  "rail",
  "steer",
]);

/** The actions that a second press undoes. */
const TOGGLES = new Set(["sidebar", "details", "foldHere"]);

/** A long answer in markdown, with a paragraph past the width of the feed, a block of code and a list. */
const LONG = [
  'close("""# A long answer',
  "",
  "The walk writes a paragraph that runs past the width of the feed, so that the window wraps it. "
    .repeat(4)
    .trim(),
  "",
  "```python",
  "def walk(steps: int) -> int:",
  "    return sum(range(steps))",
  "```",
  "",
  "- a first point",
  "- a second point with `code` in it",
  "",
  'The end of the long answer.""")',
].join("\n");

/** The answers of the scripted model of the walk: a short one, a long one that streams, a question of each shape, a
 * command, a fault, a turn that holds while a message queues behind it, and a second thread. */
export const SCRIPT: Rule[] = [
  { cue: "Walk: hello.", word: 'close("Hello from the walk.")' },
  { cue: "Walk: a long answer.", word: LONG, chunk: 16, pause: 40 },
  {
    cue: "Walk: ask me.",
    word: 'color = await prompt(str, "Walk: which color?", to="operator")\nclose(f"You said {color}.")',
  },
  {
    cue: "Walk: pick a way.",
    word: 'way = await prompt(Literal["north", "south"], "Walk: which way?", to="operator")\nclose(f"Going {way}.")',
  },
  {
    cue: "Walk: run a command.",
    word: 'out = await bash("echo walking")\nclose("Walked: " + out.stdout.content.strip())',
  },
  { cue: "Walk: fail.", word: "fault = 1 / 0" },
  { cue: "ZeroDivisionError", word: 'close("Recovered from the fault.")' },
  { cue: "Walk: hold.", word: 'close("Held, then done.")', hold: "go" },
  { cue: "Walk: queued.", word: 'close("Queued, then answered.")' },
  { cue: "Walk: second thread.", word: 'close("The second thread answers.")' },
];

/** The errors that the window logs while a walk runs, which the probe of the next step finds. */
const logged: string[] = [];

function said(error: unknown): string {
  return (error instanceof Error ? error.message : String(error)).split("\n")[0] ?? "";
}

/** Keep each error that the window logs and each promise that it rejected with no catch, until the walk ends. */
function listen(): () => void {
  const error = console.error;
  console.error = (...args: unknown[]) => {
    logged.push(args.map(String).join(" ").slice(0, 400));
    error(...args);
  };
  const rejected = (reason: unknown) => logged.push(`a promise rejected with no catch: ${said(reason)}`);
  process.on("unhandledRejection", rejected);
  return () => {
    console.error = error;
    process.off("unhandledRejection", rejected);
  };
}

/** A journey of a walk: its steps, each a capture, and what its probes found. */
export class Walk {
  readonly steps: Step[] = [];
  readonly findings: Finding[] = [];
  constructor(
    readonly view: View,
    readonly out: string,
    readonly journey: string,
  ) {}
  /** The frame of the window, drawn once its code has its colors. */
  async drawn(): Promise<string> {
    const { screen } = this.view;
    const app = this.view.app();
    app.render();
    await screen.flush();
    await Promise.all(highlighting(screen.renderer.root));
    await app.colorsDone();
    app.render();
    await screen.flush();
    return screen.captureCharFrame();
  }
  get last(): string {
    return this.steps.at(-1)?.name ?? this.journey;
  }
  find(step: string, probe: string, detail: string): void {
    this.findings.push({ step, probe, detail });
  }
  /** Do an action, then keep the frame that it left as a step, and probe it: a failed action is stuck. */
  async step(action: string, act: () => unknown = () => {}): Promise<string> {
    const name = `${this.journey}-${String(this.steps.length + 1).padStart(2, "0")}`;
    console.log(`${name}: ${action}`);
    try {
      await act();
    } catch (error) {
      this.find(name, "stuck", `${action}: ${said(error)}`);
    }
    const frame = await this.drawn();
    for (const row of leaks(frame)) this.find(name, "leak", row);
    if (shut(frame)) this.find(name, "shut", "the window says that its life is not open");
    for (const fault of logged.splice(0)) this.find(name, "fault", fault);
    this.steps.push({ name, action });
    if (this.out) {
      const spans = this.view.screen.captureSpans();
      const shot = join(this.out, name);
      await writeFile(`${shot}.png`, rasterize(spans, this.view.palette(), "furb"));
      await writeFile(`${shot}.json`, cells(spans));
      await writeFile(`${shot}.txt`, action);
    }
    return frame;
  }
  press(on: string): void {
    const { key, modifiers } = chord(on);
    this.view.screen.mockInput.pressKey(key, modifiers);
  }
  async shows(text: string): Promise<void> {
    await until(async () => (await this.drawn()).includes(text), JSON.stringify(text), BOUND);
  }
  /** Whether the input takes the keys: a mark that the walk types shows on the screen, and goes again. */
  async takes(why: string): Promise<void> {
    const app = this.view.app();
    await this.view.screen.mockInput.typeText(MARK);
    if (!(await this.drawn()).includes(MARK))
      this.find(this.last, "input", `the keys typed ${why} reach no input that shows them`);
    app.composer.setText("");
    await this.drawn();
  }
  /** Press each key of the table that stays in the window: a view opens and Esc closes it with no trace, back to an
   * input that takes the keys, a toggle undoes itself, and each other key shows what it does. */
  async keys(): Promise<void> {
    for (const { key, binding } of bindings) {
      const run = binding.run;
      const on = binding.on?.[0];
      if (!run || !on || OUTSIDE.has(run)) continue;
      const label = `${binding.chord ?? key.chord} (${key.action})`;
      const before = await this.drawn();
      // A frame that moves on its own, as a clock, says nothing of what a key changed.
      const still = before === (await this.drawn());
      const pressed = await this.step(`press ${label}`, () => this.press(on));
      if (still && pressed === before && (VIEWS.has(run) || TOGGLES.has(run)))
        this.find(this.last, "dead", `${label} changed nothing on the screen`);
      if (TOGGLES.has(run)) {
        const again = await this.step(`press ${label} again`, () => this.press(on));
        if (still && again !== before)
          this.find(
            this.last,
            "toggle",
            `a second ${label} did not undo the first: ${changed(before, again).join("; ")}`,
          );
        continue;
      }
      const closed = await this.step(`press Esc after ${label}`, () => this.press("escape"));
      if (still && VIEWS.has(run) && closed !== before)
        this.find(
          this.last,
          "residue",
          `Esc after ${label} left a trace: ${changed(before, closed).join("; ")}`,
        );
      await this.takes(`after Esc closed ${label}`);
    }
  }
  /** Shrink the window to each size of SIZES, where the input still takes the keys, then give it its size again. */
  async sizes(): Promise<void> {
    for (const [width, height] of SIZES) {
      await this.step(`resize the terminal to ${width}x${height}`, () =>
        this.view.screen.resize(width, height),
      );
      await this.takes(`at ${width}x${height}`);
    }
    await this.step(`resize the terminal to ${SIZE.width}x${SIZE.height}`, () =>
      this.view.screen.resize(SIZE.width, SIZE.height),
    );
  }
}

/** The journey of a life of the scripted model, from its first message: each shape of answer, a question of each
 * shape, a command, a fault, a message that queues, a second thread, a click, the scroll, the sizes and the keys. */
export async function lived(walk: Walk, window: Window, yard: Yard): Promise<void> {
  const ask = (line: string, answer: string) => async () => {
    await window.type(line);
    await walk.shows(answer);
  };
  await walk.step("open a new life");
  await walk.step("send Walk: hello. and wait for its answer", ask("Walk: hello.", "Hello from the walk."));
  await walk.step(
    "send Walk: a long answer. and look while it streams",
    ask("Walk: a long answer.", "A long answer"),
  );
  await walk.step("wait for the end of the long answer", () => walk.shows("The end of the long answer."));
  await walk.step("send Walk: ask me. and wait for the question", ask("Walk: ask me.", "Walk: which color?"));
  await walk.step("answer blue in the input", ask("blue", "You said blue."));
  await walk.step(
    "send Walk: pick a way. and wait for the choice",
    ask("Walk: pick a way.", "Walk: which way?"),
  );
  await walk.step("choose the second value with ↓ and Enter", async () => {
    walk.press("down");
    await walk.drawn();
    walk.press("return");
    await walk.shows("Going south.");
  });
  await walk.step(
    "send Walk: run a command. and wait for its output",
    ask("Walk: run a command.", "Walked: walking"),
  );
  await walk.step(
    "send Walk: fail. and wait for the life to recover",
    ask("Walk: fail.", "Recovered from the fault."),
  );
  await walk.step("send Walk: hold. and look while the model works", async () => {
    await window.type("Walk: hold.");
    await Bun.sleep(1500);
  });
  await walk.step("send Walk: queued. while the model works", () => window.type("Walk: queued."));
  await walk.step("let the model answer, then the message that queued", async () => {
    await yard.release("go");
    await walk.shows("Held, then done.");
    await walk.shows("Queued, then answered.");
  });
  await walk.step("open a new thread with /newthread", () => window.type("/newthread"));
  await walk.step(
    "send Walk: second thread. on it",
    ask("Walk: second thread.", "The second thread answers."),
  );
  await walk.step("click the first message of the life", () => window.click("Walk: hello."));
  for (const on of ["pageup", "pageup", "pagedown", "pagedown"])
    await walk.step(`press ${on === "pageup" ? "PageUp" : "PageDown"}`, () => walk.press(on));
  await walk.sizes();
  await walk.keys();
  await walk.step(END);
}

/** The journey of a record of the project that opens read-only, as a person reads a life: the sidebar and its
 * threads, the jumps between messages, a fold, the scroll, the chains, the two newest chains, the sizes and the keys. */
export async function read(walk: Walk, session: Session): Promise<void> {
  await walk.step("open the record");
  await walk.step("press ⌥T to move in the sidebar", () => walk.press("meta+t"));
  for (const n of [1, 2, 3]) await walk.step(`press ↓ in the sidebar, ${n}`, () => walk.press("down"));
  await walk.step("press Enter to open what the sidebar points at", () => walk.press("return"));
  await walk.step("press Esc back to the input", () => walk.press("escape"));
  for (const n of [1, 2]) await walk.step(`press ⌥[ to the message before, ${n}`, () => walk.press("meta+["));
  await walk.step("press ⌥C to fold the thread in view", () => walk.press("meta+c"));
  await walk.step("press ⌥C to open it again", () => walk.press("meta+c"));
  for (const n of [1, 2]) await walk.step(`press PageUp, ${n}`, () => walk.press("pageup"));
  const root = session.engine.root;
  const others = session.chains.map((one) => one.id).filter((id) => id !== root);
  for (const chain of others.slice(-2))
    await walk.step(`open the chain ${chain}`, () => session.select(chain));
  await walk.step(`open the root ${root} again`, () => session.select(root));
  await walk.sizes();
  await walk.keys();
  await walk.step(END);
}

/** Walk a life of the scripted model, then each record, read-only, and write each capture in `out`, when it names a
 * folder, with walk.json: each step and each finding. */
export async function walk(out: string, records: string[]): Promise<{ steps: Step[]; findings: Finding[] }> {
  const all = { steps: [] as Step[], findings: [] as Finding[] };
  const keep = (one: Walk) => {
    all.steps.push(...one.steps);
    all.findings.push(...one.findings);
  };
  if (out) await mkdir(out, { recursive: true });
  await loadParsers();
  const stop = listen();
  try {
    const yard = await Yard.make(SCRIPT);
    let window: Window | undefined;
    try {
      const opened = await Window.open(yard, yard.record, SIZE, true);
      window = opened;
      const view = {
        screen: opened.screen,
        app: () => opened.app,
        palette: () => paletteOf(opened.session.theme),
      };
      const journey = new Walk(view, out, "life");
      try {
        await lived(journey, opened, yard);
      } finally {
        keep(journey);
      }
    } finally {
      await window?.close(true).catch(() => {});
      await yard.remove();
    }
    for (const record of records) {
      const name = basename(record, ".jsonl");
      const screen = await createTestRenderer({ exitSignals: [], kittyKeyboard: true, ...SIZE });
      try {
        const life = await openEngine({ record, cwd: dirname(record), readOnly: true });
        const session = new Session(life.engine, life.host);
        await session.refresh();
        const { library, app } = await mount(session, screen.renderer);
        const journey = new Walk({ screen, app, palette: () => paletteOf(session.theme) }, out, name);
        try {
          await read(journey, session);
        } finally {
          keep(journey);
          app().dispose();
          await library.dispose();
        }
      } catch (error) {
        all.findings.push({ step: name, probe: "open", detail: said(error) });
      } finally {
        screen.renderer.destroy();
      }
    }
  } finally {
    stop();
  }
  if (out) await writeFile(join(out, "walk.json"), JSON.stringify(all, null, 1));
  return all;
}

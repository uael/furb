import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { type CapturedFrame, CodeRenderable, type Renderable, type RGBA } from "@opentui/core";
import { createTestRenderer } from "@opentui/core/testing";
import { isDone } from "furb/life";
import * as stage from "../script/stage.ts";
import { App } from "../src/app.ts";
import { demoLibrary, demoSession } from "../src/demo.ts";
import { Preferences } from "../src/preferences.ts";
import type { Session } from "../src/session.ts";

/** Preferences in a config directory of the test alone, which the yard of the tests removes. A session on a recorded
 * life that is given none reads the preferences of the user, and a library that holds it writes the list of the user
 * beside them. */
/** Whether an act exists and is not done: an act that is missing is not open, so a check that it stays open fails. */
export function open(session: Session, id: string): boolean {
  const act = session.acts.find((one) => one.id === id);
  return act !== undefined && !isDone(act);
}

/** The first frame that passes a test, as the renderer draws it once the session changed, or an error that names what
 * the test waits for once the renderer drew for 25 seconds with no such frame. */
export async function framed(frame: () => Promise<string>, passes: (shown: string) => boolean, what: string) {
  for (const until = Date.now() + 25_000; ; await Bun.sleep(20)) {
    const shown = await frame();
    if (passes(shown)) return shown;
    if (Date.now() > until) throw new Error(`No frame showed ${what}: ${shown.slice(0, 2000)}`);
  }
}

export function ownPreferences(): Preferences {
  return new Preferences(join(mkdtempSync(join(tmpdir(), "furb-config-")), "ui.json"));
}

/** What a test of the App holds: the session, the App on it, the test terminal, the frame that the App draws anew,
 * a click on the first place of that frame that shows a text, and how many times the App asked to quit. */
export interface Composing {
  session: Session;
  app: App;
  screen: Awaited<ReturnType<typeof createTestRenderer>>;
  frame: () => Promise<string>;
  click: (text: string) => Promise<void>;
  quits: () => number;
}

/** A demo session that a test uses, which ends after it. */
export async function withDemo(
  use: (session: Session) => Promise<void>,
  options?: Parameters<typeof demoSession>[0],
): Promise<void> {
  const session = await demoSession(options);
  try {
    await use(session);
  } finally {
    await session.dispose();
  }
}

/** A session in an App on a test terminal, which a test uses and which ends after it: a demo session, which the
 * conversation of the demo fills when `opened` is true, or the session that the test opened. A library of its own
 * holds the session, as the command line holds it. */
export async function composing(
  use: (context: Composing) => Promise<void>,
  options: Parameters<typeof createTestRenderer>[0] = { width: 132, height: 44 },
  opened: boolean | Session = false,
): Promise<void> {
  const session = typeof opened === "boolean" ? await demoSession({ seed: opened }) : opened;
  const library = await demoLibrary(session);
  // A test terminal ends on no signal, as the terminal of the TUI does, so that a pipe that a model command line
  // closes early ends no test.
  const screen = await createTestRenderer({ exitSignals: [], ...options });
  let quits = 0;
  const app = new App(screen.renderer, session, {
    quit() {
      quits++;
    },
    workspaces: library,
  });
  const frame = async () => {
    // The crate folds the ladder of the chain shown, which a frame draws once it came.
    for (let pass = 0; pass < 3; pass++) {
      app.render();
      await session.laddersRead();
    }
    app.render();
    await screen.flush();
    return screen.captureCharFrame();
  };
  try {
    await screen.flush();
    app.composer.focus();
    const click = async (text: string) => {
      await frame();
      await stage.click(screen, text);
    };
    await use({ session, app, screen, frame, click, quits: () => quits });
  } finally {
    app.dispose();
    screen.renderer.destroy();
    await library.dispose();
  }
}

/** Presses Escape and returns once the keys have heard it. A terminal holds a lone Escape for a short pause, to tell it
 * from the start of a sequence, so a frame that the test draws just after the press can come before the key. The
 * listener goes first, since a key that a binding takes stops before the listeners that come after. */
export function pressEscape(screen: Composing["screen"]): Promise<void> {
  const heard = new Promise<void>((resolve) => {
    const listener = () => {
      screen.renderer.keyInput.off("keypress", listener);
      resolve();
    };
    screen.renderer.keyInput.prependListener("keypress", listener);
  });
  screen.mockInput.pressEscape();
  return heard;
}

/** The text, the color and the ground of the cell at a column of a row of the screen. */
export function cellAt(frame: CapturedFrame, x: number, y: number): { text: string; fg?: RGBA; bg?: RGBA } {
  let at = 0;
  for (const span of frame.lines[y]?.spans ?? []) {
    if (x < at + span.width) return { text: span.text, fg: span.fg, bg: span.bg };
    at += span.width;
  }
  throw new Error(`No cell stands at column ${x} of row ${y}.`);
}
/** Draws the App until no code of it is being highlighted and a frame and the next are the same, since a card that the
 * App draws anew shows its markdown only once its highlighting ends. `still` drops from each frame what changes with
 * time alone, as a spinner. */
export async function drawn(
  frame: () => Promise<string>,
  root: Renderable,
  still: (text: string) => string = (text) => text,
): Promise<string> {
  let shown = still(await frame());
  for (let tries = 0; tries < 200; tries++) {
    await Promise.all(stage.highlighting(root));
    await Bun.sleep(5);
    const next = still(await frame());
    if (next === shown && !busy(root)) return next;
    shown = next;
  }
  return shown;
}

/** Whether a code of a node, or of a node that it holds, is still being highlighted. */
function busy(node: Renderable): boolean {
  return (node instanceof CodeRenderable && node.isHighlighting) || node.getChildren().some(busy);
}

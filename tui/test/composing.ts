import { type CapturedFrame, CodeRenderable, type Renderable, type RGBA } from "@opentui/core";
import { createTestRenderer } from "@opentui/core/testing";
import * as stage from "../script/stage.ts";
import { App } from "../src/app.ts";
import { demoLibrary, demoSession } from "../src/demo.ts";
import type { Session } from "../src/session.ts";

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

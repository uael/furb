import { createTestRenderer } from "@opentui/core/testing";
import { App } from "../src/app.ts";
import { demoLibrary, demoSession } from "../src/demo.ts";
import type { Session } from "../src/session.ts";

/** What a test of the App holds: the session, the App on it, the test terminal, the frame that the App draws anew,
 * and how many times the App asked to quit. */
export interface Composing {
  session: Session;
  app: App;
  screen: Awaited<ReturnType<typeof createTestRenderer>>;
  frame: () => Promise<string>;
  quits: () => number;
}

/** A session in an App on a test terminal, which a test uses and which ends after it: a demo session, which the
 * conversation of the demo fills when `opened` is true, or the session that the test opened. A library of its own
 * holds the session, as the command line holds it. */
export async function composing(
  use: (context: Composing) => Promise<void>,
  options: Parameters<typeof createTestRenderer>[0] = { width: 120, height: 44 },
  opened: boolean | Session = false,
): Promise<void> {
  const session = typeof opened === "boolean" ? await demoSession(opened) : opened;
  const library = await demoLibrary(session);
  const screen = await createTestRenderer(options);
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
    await use({ session, app, screen, frame, quits: () => quits });
  } finally {
    app.dispose();
    screen.renderer.destroy();
    await library.dispose();
  }
}

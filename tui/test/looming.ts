import { createTestRenderer } from "@opentui/core/testing";
import { App } from "../src/app.ts";
import { demoLibrary, demoSession, seedConductor, seedDemo } from "../src/demo.ts";
import type { Session } from "../src/session.ts";

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

/** What a test of the App holds: the session, the App on it, the test terminal, the frame that the App draws anew,
 * and how many times the App asked to quit. */
export interface Looming {
  session: Session;
  app: App;
  screen: Awaited<ReturnType<typeof createTestRenderer>>;
  frame: () => Promise<string>;
  quits: () => number;
}

/** A demo session in an App on a test terminal, which a test uses and which ends after it: an empty life, the
 * conversation of the demo, or the demo of a conductor and its workers. A library of its own holds the session, as
 * the command line holds it. */
export async function looming(
  use: (context: Looming) => Promise<void>,
  {
    seed = "none",
    width = 152,
    height = 46,
  }: { seed?: "none" | "demo" | "conductor"; width?: number; height?: number } = {},
): Promise<void> {
  const session = await demoSession();
  if (seed === "demo") await seedDemo(session);
  if (seed === "conductor") await seedConductor(session);
  const library = await demoLibrary(session);
  const screen = await createTestRenderer({ width, height });
  let quits = 0;
  const app = new App(screen.renderer, session, {
    quit() {
      quits++;
    },
    workspaces: library,
  });
  const frame = async () => {
    await session.refresh();
    app.render();
    await screen.flush();
    return screen.captureCharFrame();
  };
  try {
    await screen.flush();
    await use({ session, app, screen, frame, quits: () => quits });
  } finally {
    app.dispose();
    screen.renderer.destroy();
    await library.dispose();
  }
}

/** The moment a condition holds, read again every few milliseconds, for what changes with no event, as the focus
 * of the App after a key. The deadline comes before the thirty seconds that a test has, so the failure names what it
 * waited for. */
export async function soon(ready: () => boolean): Promise<void> {
  for (const start = Date.now(); !ready(); await Bun.sleep(10))
    if (Date.now() - start > 25000) throw new Error(`Waited 25 seconds for ${ready}.`);
}

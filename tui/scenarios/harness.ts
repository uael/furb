// The harness of the scenarios: a life of the owner's own path, in a yard of its own, which a headless window of the
// TUI drives with keys and the mouse on a test terminal, as the owner does. The yard and the window are in yard.ts,
// which the walk of the audit of the TUI shares.
import { test as each, setDefaultTimeout } from "bun:test";
import { type Rule, Window, Yard } from "./yard.ts";

export * from "./yard.ts";

// A scenario starts a host, a model and a window, and a loaded machine of CI runs many at once.
setDefaultTimeout(120000);

/** The test of a scenario. On Windows each life stays in its window, with no host to quit, kill or attach to, so the
 * scenarios run on macOS and Linux. */
export const test = process.platform === "win32" ? each.skip : each;

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

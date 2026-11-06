import type { createTestRenderer } from "@opentui/core/testing";
import type { App } from "../src/app.ts";

/** The renderer of a test, as the scripts of the TUI mount it. */
type Screen = Awaited<ReturnType<typeof createTestRenderer>>;

/**
 * The refusal that a record open to read gives to a message: the operator types a plain text, the composer takes none
 * and stays empty, and the frame says that the record is read only. It throws when the composer keeps the text or
 * when the frame does not say it.
 */
export async function refusal(screen: Screen, app: App): Promise<void> {
  await screen.mockInput.typeText("Go on with the work.");
  app.render();
  await screen.flush();
  if (app.composer.plainText)
    throw new Error(`The composer of a record open to read kept ${JSON.stringify(app.composer.plainText)}.`);
  if (!screen.captureCharFrame().includes("Read only"))
    throw new Error("The frame of a record open to read does not say it is read only.");
}

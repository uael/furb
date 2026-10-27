import { expect, test } from "bun:test";
import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { openEngine } from "../src/bridge.ts";
import { Preferences } from "../src/preferences.ts";
import { Session } from "../src/session.ts";
import { composing, drawn } from "./composing.ts";

/** The recorded life of a conductor on preferences of its own, which no other test shares. */
async function recorded(): Promise<{ session: Session; path: string }> {
  const { engine, host } = await openEngine({
    record: join(import.meta.dir, "..", "fixtures", "conductor.jsonl"),
  });
  const path = join(await mkdtemp(join(tmpdir(), "furb-variants-")), "ui.json");
  return { session: new Session(engine, host, true, new Preferences(path)), path };
}

test("the current view is the default, a switch at run time shows each variant, and the switch back shows the current view as it was", async () => {
  const { session: opened, path } = await recorded();
  expect(new Preferences(path).view).toBe("main");
  await composing(
    async ({ session, app, screen, frame }) => {
      await session.refresh();
      expect(app.lifeView).toBe("main");
      // A card shows its markdown only once its highlighting ends, and the spinner and the hint beside it change with
      // time alone.
      const still = (text: string) =>
        text
          .split("\n")
          .filter((line) => !line.includes("Working"))
          .join("\n");
      const before = await drawn(frame, screen.renderer.root, still);
      const seen = new Map<string, string>();
      for (const view of ["loom", "graph"] as const) {
        screen.mockInput.pressKey("v", { meta: true });
        expect(app.lifeView).toBe(view);
        expect(new Preferences(path).view).toBe(view);
        seen.set(view, await frame());
        expect(seen.get(view)).not.toBe(before);
      }
      expect(seen.get("loom")).not.toBe(seen.get("graph"));
      screen.mockInput.pressKey("v", { meta: true });
      expect(app.lifeView).toBe("main");
      expect(new Preferences(path).view).toBe("main");
      expect(await drawn(frame, screen.renderer.root, still)).toBe(before);
    },
    { width: 152, height: 46 },
    opened,
  );
});

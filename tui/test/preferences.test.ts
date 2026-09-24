import { expect, test } from "bun:test";
import { mkdir, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Preferences } from "../src/preferences.ts";

test("a save of the preferences that fails names its file, and the preferences stay as they were", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-preferences-"));
  try {
    const preferences = new Preferences(join(directory, "ui.json"));
    // A directory where the save writes its file makes the save fail on every system.
    await mkdir(join(directory, "ui.json.tmp"));
    expect(() => preferences.save("paper")).toThrow("ui.json.tmp");
    expect(preferences.theme).toBe("github");
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

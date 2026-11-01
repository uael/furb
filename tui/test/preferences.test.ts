import { expect, test } from "bun:test";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
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
    expect(preferences.theme).toBe("terminal");
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("the width of the sidebar that a file of an earlier build saved by itself counts for nothing, and a width written in a file of this build holds", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-preferences-"));
  try {
    const path = join(directory, "ui.json");
    await writeFile(path, JSON.stringify({ theme: "ink", sidebarWidth: 32 }));
    const earlier = new Preferences(path);
    expect(earlier.sidebarWidth).toBe(56);
    earlier.save("ink");
    const saved = JSON.parse(await readFile(path, "utf8")) as Record<string, unknown>;
    expect(saved.version).toBe(2);
    expect("sidebarWidth" in saved).toBe(false);
    await writeFile(path, JSON.stringify({ ...saved, sidebarWidth: 64 }));
    const chosen = new Preferences(path);
    expect(chosen.sidebarWidth).toBe(64);
    chosen.save("paper");
    expect(JSON.parse(await readFile(path, "utf8")).sidebarWidth).toBe(64);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

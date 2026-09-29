import { expect, mock, test } from "bun:test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { store } from "@furb/engine";
import { Preferences } from "../src/preferences.ts";
import * as records from "../src/records.ts";
import { Workspaces } from "../src/workspaces.ts";

// The replay of the saved records comes back when the test lets it, so it lands after an open of the record it read,
// as a replay that takes long does. This file holds the one test that needs it.
const replayed = records.inspectRecords;
let land = () => {};
const landing = new Promise<void>((resolve) => {
  land = resolve;
});
mock.module("../src/records.ts", () => ({
  inspectRecords: async (paths: string[], signal: AbortSignal) => {
    const states = await replayed(paths, signal);
    await landing;
    return states;
  },
}));

test("a replay that began before an open of its record leaves the row as the open told it", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-replay-"));
  const library = () => new Workspaces(new Preferences(join(directory, "config/ui.json")), { demo: true });
  const first = library();
  const held = await first.create(await first.add(directory), "Held elsewhere");
  await first.dispose();
  const second = library();
  const lease = store(held.path)[1];
  try {
    const group = await second.add(directory);
    const row = group.sessions.find((entry) => entry.path === held.path);
    if (!row) throw new Error("No row for the held record.");
    await expect(second.select(row)).rejects.toThrow("Another process owns");
    const replay = new Promise((resolve) => second.once("change", resolve));
    land();
    await replay;
    expect(row.status).toBe("error");
    expect(row.error).toContain("Another process owns");
  } finally {
    lease.dispose();
    await second.dispose();
    await rm(directory, { recursive: true, force: true });
  }
});

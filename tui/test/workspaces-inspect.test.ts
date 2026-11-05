import { expect, mock, test } from "bun:test";
import { mkdir, mkdtemp, rm, truncate, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import { Preferences } from "../src/preferences.ts";
import { Workspaces } from "../src/workspaces.ts";

// The replays that the list asks for, with the answer that each gives: a record with a name that holds "work" has
// unfinished work. Each record that changed is scanned for its name, and only those of replay are replayed.
const asked: string[][] = [];
mock.module("../src/records.ts", () => ({
  inspectRecords: async (paths: string[], _signal: AbortSignal, replay: string[] = paths) => {
    asked.push(replay);
    return Object.fromEntries(
      paths.map((path) => {
        const replayed = replay.includes(path);
        return [path, { pending: replayed && path.includes("work") ? 1 : 0, replayed }];
      }),
    );
  },
}));

const settle = async (library: Workspaces, name: string, status: string) => {
  while (library.groups[0]?.sessions.find((row) => row.name === name)?.status !== status) await Bun.sleep(5);
};

test("the list replays a record once for each change of it, and no record past the limit", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-inspect-"));
  const sessions = join(directory, ".furb/sessions");
  await mkdir(sessions, { recursive: true });
  const small = join(sessions, "work.jsonl");
  const long = join(sessions, "long.jsonl");
  await writeFile(small, "");
  await writeFile(long, "");
  await truncate(long, 17 * 2 ** 20);
  const library = () => new Workspaces(new Preferences(join(directory, "config/ui.json")), { demo: true });
  const rows = (one: Workspaces) =>
    Object.fromEntries(one.groups[0]?.sessions.map((row) => [row.name, row.status]) ?? []);
  const first = library();
  try {
    await first.add(directory);
    await settle(first, "work", "runs");
    expect(asked.map((paths) => paths.map((path) => basename(path)))).toEqual([["work.jsonl"]]);
    expect(rows(first)).toEqual({ work: "runs", long: "saved" });
    await first.refresh();
    expect(asked).toHaveLength(1);
  } finally {
    await first.dispose();
  }
  asked.length = 0;
  const second = library();
  try {
    await second.add(directory);
    // A later run takes the state of a record that did not change from the file beside the list, and replays nothing.
    expect(asked).toEqual([]);
    expect(rows(second)).toEqual({ work: "runs", long: "saved" });
    await writeFile(small, "x");
    await second.refresh();
    await Bun.sleep(100);
    expect(asked.map((paths) => paths.map((path) => basename(path)))).toEqual([["work.jsonl"]]);
  } finally {
    await second.dispose();
    await rm(directory, { recursive: true, force: true });
  }
});

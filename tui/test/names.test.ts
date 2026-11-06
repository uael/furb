import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { inspectRecord, type NativeEar, store } from "furb";
import { until } from "../../binding/typescript/test/until.ts";
import { recordName } from "../src/names.ts";
import { Preferences } from "../src/preferences.ts";
import { Workspaces } from "../src/workspaces.ts";
import { ended } from "./ended.ts";

/** A test in a temporary directory of its own, whose libraries read the list of workspaces in it, as each run of the
 * TUI does, and end with it. */
async function withLibraries(
  use: (directory: string, open: () => Workspaces) => Promise<void>,
): Promise<void> {
  const directory = await mkdtemp(join(tmpdir(), "furb-names-"));
  const libraries: Workspaces[] = [];
  const open = () => {
    const library = new Workspaces(new Preferences(join(directory, "config/ui.json")), { demo: true });
    libraries.push(library);
    return library;
  };
  try {
    await use(directory, open);
  } finally {
    for (const library of libraries) await library.dispose();
    await rm(directory, { recursive: true, force: true });
  }
}

/** The record as a build of another engine leaves it after a crash: it names that engine, holds no checkpoint at its
 * tip and no snapshot or log beside it, and its first question is one that this build does not make, so its replay
 * refuses it, as a drift does. */
async function another(record: string): Promise<void> {
  const text = (await readFile(record, "utf8"))
    .split("\n")
    .filter((line) => !line.startsWith('[["checkpoint",'))
    .join("\n");
  const fact =
    /\["made","chain1","store","[0-9a-f]{64}",(?:\d+|null)(?:,(?:"(?:[^"\\]|\\.)*"|[^\]",]+))*\]/.exec(
      text,
    )?.[0];
  if (!fact) throw new Error("The record names no engine.");
  const stand = '["stand","stand1","operator","chain1"]';
  expect(text).toContain(stand);
  await writeFile(
    record,
    text
      .replace(fact, `["made","chain1","store","${"0".repeat(64)}"]`)
      .replace(stand, '["stand","stand1","operator","chain1","other"]'),
  );
  for (const beside of [".snapshot", ".log", ".told"]) await rm(`${record}${beside}`, { force: true });
  expect(
    await inspectRecord(record).then(
      () => "replayed",
      () => "refused",
    ),
  ).toBe("refused");
}

/** A saved session of a library, made in a directory with a name or none, whose life ended. */
async function saved(directory: string, open: () => Workspaces, name?: string): Promise<string> {
  const library = open();
  const entry = await library.create(await library.add(directory));
  if (name !== undefined) {
    await library.rename(entry, "First");
    await library.rename(entry, name);
  }
  const path = entry.path;
  await library.dispose();
  // The life ends in the daemon once its window let it go, and only then does its record stand still.
  await ended(path);
  return path;
}

test("a plain scan of a record finds the last name that the operator gave, as a word of a rung of any build", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-names-"));
  try {
    const record = join(directory, "life.jsonl");
    const rung = (name: string) =>
      JSON.stringify([
        [
          "rung",
          "rung1",
          "operator",
          "chain1",
          `act("rename_session", "", lambda id: (yield "done", id, ${JSON.stringify(name)}))`,
          "",
          "",
        ],
        1,
      ]);
    await writeFile(
      record,
      [rung("pf-1208"), "not a fact", rung('say "it" \\ twice'), '[["chain"'].join("\n"),
    );
    expect(recordName(record)).toBe('say "it" \\ twice');
    expect(recordName(join(directory, "none.jsonl"))).toBeUndefined();
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("a name that one build gave shows in the list of another build, when the record drifts and another host holds it", () =>
  withLibraries(async (directory, open) => {
    const record = await saved(directory, open, "pf-1194");
    await another(record);
    let lease: NativeEar | undefined;
    try {
      lease = store(record)[1];
      const library = open();
      const row = (await library.add(directory)).sessions.find((entry) => entry.path === record);
      await until(library, () => row?.name === "pf-1194");
      expect(row?.session).toBeUndefined();
    } finally {
      lease?.dispose();
    }
  }));

test("a name that an older release kept beside the record shows in the list and in the session that opens it, and stays there until the record holds it", () =>
  withLibraries(async (directory, open) => {
    const record = await saved(directory, open);
    const path = `${record}.ui.json`;
    const held = () => JSON.parse(readFileSync(path, "utf8")) as Record<string, unknown>;
    await writeFile(path, JSON.stringify({ ...held(), sessionName: "pf-1193" }));
    const library = open();
    const group = await library.add(directory);
    const row = group.sessions.find((entry) => entry.path === record);
    await until(library, () => row?.name === "pf-1193");
    await library.import(record, group);
    const session = library.current?.session;
    if (!session) throw new Error("The record did not open.");
    await until(library, () => session.sessionName === "pf-1193");
    await library.dispose();
    // The name stays in its old home, or the life that could write the record moved it there.
    expect(held().sessionName ?? recordName(record)).toBe("pf-1193");
  }));

import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import type { Fact } from "furb";
import { openEngine } from "../src/bridge.ts";
import { Preferences } from "../src/preferences.ts";
import { Session } from "../src/session.ts";

export const longWord =
  'result = await bash("printf original-source", stdin="keep the whole payload", timeout=600, on="chain1")';
export const original = [
  "<s:brief>",
  'Keep every quoted line, including 雪😀, quotes "like this", and the final instruction. '.repeat(3),
  "Last instruction: original copy and edit.",
  "</s:brief>",
  'text = """one "quoted" line',
  'second line 雪😀"""',
  'path = r"""C:\\new',
  '\\drafts"""',
  longWord,
].join("\n");

/** An inert recording: no model, shell word, or editor in the recording ever runs. */
export async function ruffFixture(source = original) {
  const directory = await mkdtemp(join(tmpdir(), "furb-ruff-view-"));
  const record = join(directory, "record.jsonl");
  const facts: Fact[] = [
    ["chain", "chain1", "operator", "", "Ruff display", ""],
    ["stand", "stand1", "operator", "chain1"],
    [
      "done",
      "stand1",
      "",
      [
        [
          ["operator", [], 200000],
          ["codex:gpt-6.1-sol", ["high"], 475000],
        ],
        directory,
        "codex:gpt-6.1-sol/high",
      ],
    ],
    ["prompt", "prompt1", "operator", "chain1", "str", "Keep original Python", "codex:gpt-6.1-sol/high"],
    ["rung", "rung1", "prompt1", "chain1", source, "", ""],
    ["run", "run1", "chain1", "chain1", "rung1", source, ""],
    ["done", "run1", "", null],
    ["done", "rung1", "", null],
    ["rung", "rung2", "prompt1", "chain1", "next_value = 2", "", ""],
    ["run", "run2", "chain1", "chain1", "rung2", "next_value = 2", ""],
    ["done", "run2", "", null],
    ["done", "rung2", "", null],
    ["rung", "rung3", "prompt1", "chain1", "", "", ""],
  ];
  await writeFile(
    record,
    [JSON.stringify({ recorded: true }), ...facts.map((fact) => JSON.stringify(fact)), ""].join("\n"),
  );
  const { engine, host } = await openEngine({ demo: true, record, cwd: directory });
  const session = new Session(engine, host, true, new Preferences(join(directory, "ui.json")));
  await session.refresh();
  session.preferences.sidebar = false;
  session.preferences.reducedMotion = true;
  session.folds = { "thread:prompt1": false };
  session.host.streams.set("rung3", {
    chain: "chain1",
    text: "streamed = [1,",
    thinking: "Choosing the next value",
  });
  return { session, record, directory, remove: () => rm(directory, { recursive: true, force: true }) };
}

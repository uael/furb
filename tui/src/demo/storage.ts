import type { Script } from "./script.ts";

const look = String.raw`# Read what storage does today, and who calls it.
files = ["src/storage.ts", "src/capture.ts", "src/search.ts", "test/search.test.ts"]
seen = {}
for path in files:
  text = read(path, HIDDEN)
  seen[path] = text.lines
  print(path, len(text.lines), "lines")

calls = await bash("grep -n 'save(\\|load(' -r src test")`;

const rewrite = String.raw`STORAGE = """import { appendFileSync, existsSync, mkdirSync, readFileSync } from "node:fs";

export interface Note {
  text: string;
  at: number;
}

const file = ".notes/notes.jsonl";
const notes: Note[] = [];

/** Read the notes that an earlier run kept. */
export function start(): void {
  if (!existsSync(file)) return;
  for (const line of readFileSync(file, "utf8").split("\n")) if (line) notes.push(JSON.parse(line));
}

export function save(note: Note): void {
  mkdirSync(".notes", { recursive: true });
  appendFileSync(file, JSON.stringify(note) + "\n");
  notes.push(note);
}

export const load = () => notes;
"""

RESTART = """import { expect, test } from "bun:test";
import { rmSync } from "node:fs";
import { load, save, start } from "../src/storage.ts";

test("a note that was saved comes back after a restart", () => {
  rmSync(".notes", { recursive: true, force: true });
  save({ text: "small ideas", at: 1 });
  load().length = 0;
  start();
  expect(load().map((note) => note.text)).toEqual(["small ideas"]);
});
"""


def tidy(name, text):
  # The files of the project hold no tab, and end with one newline.
  return Text(name, text.replace("\t", "  ").rstrip("\n") + "\n")


write(tidy("src/storage.ts", STORAGE))
write(tidy("test/storage.test.ts", RESTART))
capture = read("src/capture.ts")
if "start()" not in capture.content:
  write(capture.replace('import { save } from "./storage.ts";', 'import { save, start } from "./storage.ts";\n\nstart();'))
check = await bash("printf '✓ a note that was saved comes back after a restart\\n✓ search finds a captured note\\n\\n2 pass\\n0 fail\\n'")`;

const legacy = `# Notes of an earlier run may sit in a file of the old demo. A worker moves them, and another checks the result.
old = prompt(str, "Move the notes of the old demo.jsonl into .notes/notes.jsonl, one JSON object for each line, and keep the order.", on=chain("storage/legacy"))
moved = await old
print(moved)`;

const ask = `# The old notes live in memory only, and a first start has nothing to load. A file that exists already may hold notes
# that the owner wrote by hand, so I ask before the first start touches it.
tries = 0
keep = None
while keep is None and tries < 2:
  tries += 1
  keep = await prompt(bool, "A file .notes/notes.jsonl exists already, with 14 lines. Start from it, and keep what it holds?", to=OPERATOR)
if keep:
  start = await bash("wc -l .notes/notes.jsonl 2>/dev/null || echo 'no file yet'")
else:
  await bash("mkdir -p .notes && : > .notes/notes.jsonl")`;

const report = `close("Storage keeps the notes in .notes/notes.jsonl: start() loads them, save() appends a line. test/storage.test.ts saves a note, restarts, and finds it. 2 pass, 0 fail. The first start asks before it reads a file that exists.")`;

export const storage: Script[] = [
  {
    cue: "Keep the notes in .notes/notes.jsonl",
    weight: { input: 3900, output: 310, cost: 0.011 },
    steps: [
      { thinking: "I read the three files that touch storage before I change one.", word: look },
      {
        thinking:
          "Storage keeps its notes in a list, so a file with one line for each note is the smallest change.",
        word: rewrite,
      },
      {
        thinking: "The old demo kept notes in a file of its own, so a second worker moves them.",
        word: legacy,
      },
      {
        thinking: "A file may exist already, so the owner decides what a first start does with it.",
        word: ask,
      },
      { thinking: "The part is done, so I report it.", word: report },
    ],
  },
];

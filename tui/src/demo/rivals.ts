import type { Script } from "./script.ts";

/** The notes that the audit reads, which the demo puts in the project before the life starts. */
export const research: Record<string, string> = {
  "research/obsidian.md": [
    "# Obsidian",
    "",
    "Search opens with Ctrl+Shift+F and looks in the text of every note of the vault.",
    "",
    "## Operators",
    "",
    "- file: matches the name of a note",
    "- path: matches the folder of a note",
    "- tag: matches a tag, as #idea",
    "- line: matches words on one line",
    "- section: matches words under one heading",
    "",
    "## Quick switcher",
    "",
    "Ctrl+O finds a note by its name only, with fuzzy matching, and is the key most people use.",
    "",
    "## Gaps",
    "",
    "- Results are not ranked by how new a note is.",
    "- A typo finds nothing in the text search.",
    "",
  ].join("\n"),
  "research/bear.md": [
    "# Bear",
    "",
    "The search bar sits over the list of notes, and it filters the list as you type.",
    "",
    "- A word matches the title and the text, and the match is lit in the note.",
    "- A tag in the sidebar narrows the list to that tag.",
    "- @todo, @today and @images list the notes by what they hold.",
    "- Text in a picture is found too, in the paid version.",
    "",
    "## Gaps",
    "",
    "- No operators for the path of a note: a note has tags, not folders.",
    "",
  ].join("\n"),
  "research/logseq.md": [
    "# Logseq",
    "",
    "Ctrl+K opens one box that finds pages and blocks.",
    "",
    "- It shows the page, the block, and the words around the match.",
    "- A block is a line of an outline, so a match points at one idea and not at a whole file.",
    "- Queries are written in a note, as {{query (and [[idea]] (task TODO))}}, and the list stays alive.",
    "",
    "## Gaps",
    "",
    "- The index builds again after a large change, and search is slow until it ends.",
    "- A query needs its own syntax.",
    "",
  ].join("\n"),
};

const audit = `APPS = ["obsidian", "bear", "logseq"]


def hire(app):
  where = chain("rivals/" + app)
  return prompt(str, f"Read research/{app}.md and say how it finds a note, in three lines, and what Fieldnotes should copy.", on=where)


asked = {app: hire(app) for app in APPS}
findings = {}
for app, act in asked.items():
  findings[app] = await act
  print(app, "->", findings[app].splitlines()[0])`;

const table = String.raw`def row(app, text):
  lines = [x.strip() for x in text.splitlines() if x.strip()]
  return "| " + app + " | " + (lines[0] if lines else "") + " | " + (lines[-1] if len(lines) > 1 else "") + " |"


rows = [row(app, text) for app, text in findings.items()]
body = "# What to copy\n\n| App | How it finds a note | Copy |\n| --- | --- | --- |\n" + "\n".join(rows) + "\n"
write(Text("research/summary.md", body))
close("Three apps read. Copy the one box of Logseq, the operators of Obsidian, and the lit match of Bear. research/summary.md holds the table.")`;

const obsidian = `def headings(text):
  found = []
  number = 0
  lines = text.lines
  while number < len(lines):
    if lines[number].startswith("## "):
      found.append((lines[number][3:], number + 1))
    number += 1
  return found


page = read("research/obsidian.md", HIDDEN)
for title, number in headings(page):
  print(f"line {number}: {title}")
ops = [x for x in page.lines if x.startswith("- ") and ":" in x.split(" ")[1]]
print(len(ops), "operators")`;

const obsidianEnd = `close("Obsidian finds text with Ctrl+Shift+F and narrows it with operators: file:, path:, tag:, line:, section:. Ctrl+O finds a note by name. Copy: the tag: operator, since Fieldnotes has no folders.")`;

const bear = `page = read("research/bear.md", HIDDEN)
bullets = sorted({x[2:].split(" ")[0] for x in page.lines if x.startswith("- ")})
gaps = [x for x in page.lines if x.startswith("- No ")]
print(bullets)
close("Bear filters the list of notes as you type, and lights the match inside the note. A tag narrows the list. Copy: the lit match, so a person sees why a note matched. " + (gaps[0][2:] if gaps else ""))`;

const logseq = `page = read("research/logseq.md", HIDDEN)
note = await prompt(str, "Read research/logseq.md and say in one line what a block is, and why a match that points at a block helps.", on=chain("logseq/blocks"))
print(note)
close("Logseq opens one box on Ctrl+K and finds pages and blocks, with the words around the match. " + note + " Copy: the one box on Ctrl+K, which Fieldnotes binds already.")`;

const blocks = `close("A block is one line of an outline. A match that points at a block shows one idea and not a whole file, so a person finds it faster.")`;

export const rivals: Script[] = [
  {
    cue: "Compare how three note apps find a note",
    weight: { input: 4800, output: 460, cost: 0.088 },
    steps: [
      {
        thinking: "Each app is its own task, so I hire one worker for each, and read what they say.",
        word: audit,
      },
      { thinking: "I put the three answers in one table that the owner can read.", word: table },
    ],
  },
  {
    cue: "Read research/obsidian.md",
    weight: { input: 2600, output: 240, cost: 0.021 },
    steps: [
      { thinking: "The operators are the part of this note that Fieldnotes can use.", word: obsidian },
      { thinking: "I say what it does and what to copy.", word: obsidianEnd },
    ],
  },
  {
    cue: "Read research/bear.md",
    weight: { input: 2300, output: 190, cost: 0.017 },
    steps: [{ thinking: "The note is short, so one word reads it and answers.", word: bear }],
  },
  {
    cue: "Read research/logseq.md and say how",
    weight: { input: 2400, output: 220, cost: 0.019 },
    steps: [
      {
        thinking: "I do not know the word block here, so a worker on its own chain explains it.",
        word: logseq,
      },
    ],
  },
  {
    cue: "what a block is",
    weight: { input: 1500, output: 80, cost: 0.006 },
    steps: [{ thinking: "One sentence answers it.", word: blocks }],
  },
];

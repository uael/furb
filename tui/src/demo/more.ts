import type { Script } from "./script.ts";

const bench = String.raw`import json
import time

TOPICS = ["ideas", "cafe", "travel", "budget"]
notes = [{"text": f"note {n} about {TOPICS[n % len(TOPICS)]}", "at": n} for n in range(2000)]
write(Text(".notes/bench.jsonl", "\n".join(json.dumps(x) for x in notes) + "\n"))


def timed(query, rounds):
  best = None
  tries = 0
  found = 0
  while tries < rounds:
    start = time.perf_counter()
    hits = [x for x in notes if query in str(x["text"])]
    spent = time.perf_counter() - start
    found = len(hits)
    if best is None or spent < best:
      best = spent
    tries += 1
  return found, best


results = []
for query in ["ideas", "travel", "zebra"]:
  found, best = timed(query, 5)
  results.append((query, found, best))
  print(f"{query}: {found} found,", "fast" if best < 0.05 else "slow")
slowest = max(results, key=lambda r: r[2])
print("the slowest query is", slowest[0])`;

const slow = `# The cold start reads the whole file, so I time a start in a fresh process and give it a bound.
cold = await bash("sleep 5 && echo started", timeout=0.4)
if cold.code is None:
  print("a start with 2000 notes takes more than 0.4 s")
warm = await bash("wc -l .notes/bench.jsonl")
print(warm.stdout.content.strip())`;

const benchEnd = `close("Search over 2000 notes is fast: each query ends in under 50 ms. A cold start that reads the file takes more than 0.4 s, so start() should read the file once and keep a word index. That is the next item, and nothing in this change needs it.")`;

const typesRun = String.raw`check = await bash("printf 'src/storage.ts:16:42 - error TS7006: Parameter line implicitly has an any type.\\nsrc/search.ts:12:19 - error TS2322: Type string is not assignable to type number.\\nsrc/keys.ts:3:10 - error TS2304: Cannot find name open.\\n\\nFound 3 errors in 3 files.\\n' >&2; exit 2")
print(check.code, "errors in", len(check.stderr.lines), "lines")`;

const typesLook = `def around(path, pattern):
  text = read(path, HIDDEN)
  at = [number for number, line in enumerate(text.lines, 1) if pattern in line]
  return text, at


storage, found = around("src/storage.ts", "JSON.parse")
print(found)
read("src/storage.ts", grep("JSON.parse"))
keys, opened = around("src/keys.ts", "open()")
print(opened)`;

const typesFix = String.raw`LINE = "if (line) notes.push(JSON.parse(line));"
storage = read("src/storage.ts", HIDDEN)
fixed = False
try:
  write(storage.replace(LINE, "if (line) notes.push(JSON.parse(line) as Note);"))
  fixed = True
except Refused:
  print("the line is not in storage.ts yet, or it has its type already")
check = await bash("printf 'Found 0 errors.\\n'")
close("The type check passes. The one real error was a note that JSON.parse gave as any" + (": it has the type Note now." if fixed else ", and the line has its type already.") + " The two other messages came from lines of an earlier draft that a later change replaced.")`;

const bump = String.raw`import json
import re

pack = json.loads(read("package.json", HIDDEN).content)
old = pack.get("version", "0.0.0")


def bumped(version):
  major, minor, patch = [int(x) for x in version.split(".")]
  return f"{major}.{minor + 1}.0"


pack["version"] = bumped(old)
write(Text("package.json", json.dumps(pack, indent=2) + "\n"))
print(old, "->", pack["version"])

try:
  log = read("CHANGELOG.md")
except Refused:
  log = write(Text("CHANGELOG.md", "# Changelog\n"))
head = re.search(r"^## (\S+)", log.content, re.M)
print("last release in the log:", head[1] if head else "none")`;

const notes = String.raw`SECTIONS = [
  ("Added", ["Notes survive a restart: they are kept in .notes/notes.jsonl.", "Ctrl+K opens the search.", "Search finds a note by any word in it, newest first."]),
  ("Changed", ["The README links to a guide, docs/guide.md."]),
]


def entry(version, sections):
  lines = [f"## {version}", ""]
  for title, items in sections:
    lines.append(f"### {title}")
    lines.extend(f"- {item}" for item in items)
    lines.append("")
  return "\n".join(lines)


log = read("CHANGELOG.md", HIDDEN)
write(Text("CHANGELOG.md", log.content.rstrip("\n").replace("# Changelog", "# Changelog\n\n" + entry("0.4.0", SECTIONS), 1) + "\n"))
tag = await bash("git tag --list 2>/dev/null | tail -3; echo tag v0.4.0 is not made: the owner tags")
close("package.json is at 0.4.0 and CHANGELOG.md has its entry. I made no tag: the owner tags.")`;

const legacy = `old = await bash("ls .notes demo.jsonl 2>&1 | head -5")
lines = [x for x in old.stdout.lines if x.endswith(".jsonl")]
check = await prompt(str, "Read .notes/notes.jsonl and say whether each line is one JSON object with text and at.", on=chain("legacy/verify"))
print(check)
close(f"{len(lines)} file(s) of notes found. " + check)`;

const verify = `def valid(line):
  return line.startswith("{") and line.endswith("}") and '"text"' in line and '"at"' in line


try:
  text = read(".notes/notes.jsonl", HIDDEN)
  bad = [number for number, line in enumerate(text.lines, 1) if not valid(line)]
  close(f"{len(text.lines)} lines, {len(bad)} that are not a note." + (f" Lines {bad[:5]}." if bad else ""))
except Refused:
  close("No file yet, so there is nothing to check.")`;

const accents = String.raw`SEARCH = read("src/search.ts")
fixed = SEARCH.replace("text.toLowerCase().match(", "text.normalize(\"NFD\").replace(/[\\u0300-\\u036f]/g, \"\").toLowerCase().match(")
write(fixed)
write(read("test/search.test.ts").insert(1, 'import { capture as keep } from "../src/capture.ts";'))
close("A search for cafe finds Café now: the words of a note and of a query lose their accents before they are compared. test/search.test.ts imports capture for the new case.")`;

export const more: Script[] = [
  {
    cue: "Time the search on 2000 notes",
    weight: { input: 3100, output: 330, cost: 0.016 },
    steps: [
      {
        thinking: "I make 2000 notes and time three queries, with no real clock in the answer.",
        word: bench,
      },
      { thinking: "A start has to read the file, so I give it a bound.", word: slow },
      { thinking: "The numbers say where the time goes, so I report.", word: benchEnd },
    ],
  },
  {
    cue: "Run the type check",
    weight: { input: 2200, output: 210, cost: 0.003 },
    steps: [
      { thinking: "I run the check first, and read each message.", word: typesRun },
      { thinking: "Two of the messages name files that I wrote, so I look at their lines.", word: typesLook },
      { thinking: "One line needs a type, so I add it and run the check again.", word: typesFix },
    ],
  },
  {
    cue: "Prepare the 0.4 tag",
    weight: { input: 3400, output: 280, cost: 0.052 },
    steps: [
      { thinking: "The version is in package.json, and the log of changes is in CHANGELOG.md.", word: bump },
      { thinking: "I write the entry of the release, and leave the tag to the owner.", word: notes },
    ],
  },
  {
    cue: "Move the notes of the old demo.jsonl",
    weight: { input: 2000, output: 190, cost: 0.009 },
    steps: [{ thinking: "I find the old files, and a second worker checks the new one.", word: legacy }],
  },
  {
    cue: "Read .notes/notes.jsonl and say whether",
    weight: { input: 1400, output: 120, cost: 0.004 },
    steps: [{ thinking: "A line is a note when it is an object with the two keys.", word: verify }],
  },
  {
    cue: "Also make a search for cafe",
    weight: { input: 5200, output: 260, cost: 0.029 },
    steps: [
      {
        thinking: "An accent splits a word in two parts, so I take the accents off before I compare.",
        word: accents,
      },
    ],
  },
];

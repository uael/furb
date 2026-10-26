import type { Script } from "./script.ts";

/** Two more messages of the owner on the root chain, once the first workers run. */
export const audit = "Audit the keyboard: which keys does the app answer, and do two of them clash?";
export const checklist = "Run the release checklist and tell me what is left.";

const keys = `# Each file is read by a worker of its own, and I compare what they find.
PARTS = {
  "keys/capture": "src/capture.ts",
  "keys/search": "src/search.ts",
  "keys/shortcut": "src/keys.ts",
}


def ask(label, path):
  where = chain(label)
  return prompt(str, f"List the keys that {path} answers, one for each line as 'key: what it does'. Say 'none' if it answers no key.", on=where)


asked = {label: ask(label, path) for label, path in PARTS.items()}
found = {}
for label, act in asked.items():
  found[label] = (await act).strip().splitlines()
  print(label, "->", len(found[label]), "line(s)")

owners = {}
for label, lines in found.items():
  for line in lines:
    key = line.split(":")[0].strip().lower()
    if key and key != "none":
      owners.setdefault(key, []).append(label)
twice = sorted(key for key, who in owners.items() if len(who) > 1)
if twice:
  close("These keys clash: " + ", ".join(twice))
else:
  close(f"{len(owners)} key(s) in all, and no two files answer the same one: " + ", ".join(sorted(owners)))`;

const list = (path: string, answer: string) =>
  `text = read("${path}", HIDDEN)
if "key" in text.content.lower():
  close(${JSON.stringify(answer)})
else:
  close("none")`;

const checks = `CHECKLIST = [
  ("A test stands for each part", "ls test"),
  ("The guide exists", "ls docs/guide.md"),
  ("The README names the shortcut", "grep -c 'Ctrl+K' README.md"),
  ("The version is 0.4.0", "grep '\\"version\\": \\"0.4.0\\"' package.json"),
  ("Notes survive a restart", "ls .notes/notes.jsonl"),
]


def state(got):
  if got.code != 0:
    return "open"
  return "done"


rows = []
left = 0
for item, command in CHECKLIST:
  got = await bash(command, show=HIDDEN)
  mark = state(got)
  if mark == "open":
    left += 1
  rows.append(f"- [{'x' if mark == 'done' else ' '}] {item}")
close(f"{left} item(s) left of {len(CHECKLIST)}.\\n\\n" + "\\n".join(rows))`;

export const keyboard: Script[] = [
  {
    cue: "Audit the keyboard",
    weight: { input: 6900, output: 380, cost: 0.041 },
    steps: [{ thinking: "One worker for each file, and a table of the keys that they find.", word: keys }],
  },
  {
    cue: "List the keys that src/capture.ts answers",
    weight: { input: 1700, output: 90, cost: 0.004 },
    steps: [
      { thinking: "The file has no handler of keys.", word: list("src/capture.ts", "Enter: saves the note") },
    ],
  },
  {
    cue: "List the keys that src/search.ts answers",
    weight: { input: 1700, output: 90, cost: 0.004 },
    steps: [
      { thinking: "Search takes a text and no key.", word: list("src/search.ts", "Enter: runs the search") },
    ],
  },
  {
    cue: "List the keys that src/keys.ts answers",
    weight: { input: 1800, output: 110, cost: 0.005 },
    steps: [
      {
        thinking: "The file holds the one key of the search.",
        word: list("src/keys.ts", "ctrl+k: opens the search\nenter: runs the search"),
      },
    ],
  },
  {
    cue: "Run the release checklist",
    weight: { input: 7400, output: 330, cost: 0.036 },
    steps: [
      { thinking: "Each item is one command, and an item is done when its command succeeds.", word: checks },
    ],
  },
];

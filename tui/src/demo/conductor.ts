import type { Script } from "./script.ts";

/** The message of the owner that starts the life of the demo, on the root chain. */
export const release =
  "Conduct the 0.4 release of Fieldnotes: the notes survive a restart, search finds a note by any word in it, Ctrl+K opens the search, and the guide says so. Hire a worker for each part, keep each change small, and ask me only what you cannot decide.";

/** The owner writes again while the workers run, in the same thread. */
export const watching = "Tell me as soon as search lands, with the tests that prove it.";

const plan = `# Release 0.4 of Fieldnotes. Each part has a worker on a chain of its own, and I read each report as it ends.
PLAN = [
  ("w/storage", "claude-cli:sonnet/medium", "Keep the notes in .notes/notes.jsonl, so they survive a restart: load the file at start, and append a line at each save. Cover a restart in a test."),
  ("w/search", "claude-cli:opus/medium", "Make search find a note by any word in it, newest first. Cover an empty query and a query that matches nothing."),
  ("w/shortcut", "claude-cli:sonnet/high", "Bind Ctrl+K to the search, and say so in the README. A test presses the key."),
  ("w/guide", "claude-cli:haiku", "Write docs/guide.md for a person who has never used the app: capture, search, restart."),
  ("w/rivals", "claude-cli:fable", "Compare how three note apps find a note, from research/*.md, and say what Fieldnotes should copy."),
  ("w/perf", "claude-cli:sonnet/medium", "Time the search on 2000 notes, and say where the time goes. Change no code."),
  ("w/types", "claude-cli:haiku", "Run the type check of the project, and fix each error that our change made."),
  ("w/release", "claude-cli:opus/high", "Prepare the 0.4 tag: bump the version, and write the entry of the release in CHANGELOG.md. Make no tag."),
  ("w/ci", "claude-cli:sonnet/medium", "Run the checks of the project on each change with sh ci.sh, and tell me when one fails."),
]
# The cheap model gets a ceiling, and the audit gets room to read.
CEILING = {"w/guide": 0.004, "w/rivals": 6.0}


def hire(label, model, brief):
  where = chain(label)
  if label in CEILING:
    grant(CEILING[label], on=where)
  return prompt(str, brief, to=model, on=where)


pending = {label: hire(label, model, brief) for label, model, brief in PLAN}
reports = {}
while pending:
  await wait(300, *pending.values())
  for label in [name for name in pending if over(pending[name])]:
    got = peek(pending.pop(label))
    reports[label] = got if isinstance(got, str) else repr(got)
    print(f"{label} ended: {reports[label][:90]}")
print(f"{len(reports)} of {len(PLAN)} reports are in")`;

const landed = String.raw`done = [f"- {label}: {text.splitlines()[0]}" for label, text in reports.items()]
close("Fieldnotes 0.4 is ready to tag.\n\n" + "\n".join(done))`;

const watchWord = `# The search worker closes with its report, so I read it before I answer.
report = peek(pending["w/search"]) if "w/search" in pending and over(pending["w/search"]) else None
close("I will tell you when w/search closes its report. Its tests are in test/search.test.ts.")`;

export const conductor: Script[] = [
  {
    cue: "Conduct the 0.4 release",
    weight: { input: 5200, output: 420, cost: 0.031 },
    steps: [
      {
        thinking:
          "Nine parts are independent of each other, so each gets a worker, and I wait for their reports.",
        word: plan,
      },
      { thinking: "Each report is in, so I say what ships.", word: landed },
    ],
  },
  {
    cue: "Tell me as soon as search lands",
    weight: { input: 6100, output: 90, cost: 0.012 },
    steps: [{ thinking: "The owner wants one notice, so I say when it comes.", word: watchWord }],
  },
];

/** The words that the model of the demo answers a prompt with, turn by turn, by how the message of the prompt starts.
 * A word that is empty thinks until a cancel. The demo life of many threads runs on them: a release that a conductor
 * hires four workers for, a worker that hires two writers, a review that hangs on a change, two questions for the
 * operator, a command that streams, a worker that a grant pauses, and a second thread that reaches parity with Pi. */
export const script: [cue: string, words: string[]][] = [
  [
    "Ship the release 0.4",
    [
      String.raw`# Hire a worker for each part of the release
lanes = {}
briefs = [
  ("search", "Make the search ignore case, with a test, and open its PR."),
  ("docs", "Write the docs of the release: a guide and a changelog."),
  ("ui", "Ask me first, then tidy the layout of the notes."),
  ("gif", "Record the animation of the release."),
]
for key, brief in briefs:
  lane = chain("w " + key)
  if key == "gif":
    grant(0.004, on=lane)
  lanes[key] = prompt(str, brief, on=lane)
first = await wait(600, *lanes.values())`,
      `# Wait for every lane, then report
for lane in lanes.values():
  await lane
close("The release 0.4 is ready.")`,
    ],
  ],
  [
    "Make the search ignore case",
    [
      String.raw`# Make the search ignore case
code = read("src/search.ts")
write(code.replace("note.text.includes(query)", "note.text.toLowerCase().includes(query.toLowerCase())"))
# Run the tests, then open the pull request
run = bash("printf '✓ search finds a captured note\\n✓ search ignores case\\n'")
check = await run
pr = await bash("printf 'https://github.com/uael/fieldnotes/pull/12\\n'")
# Ask a reviewer to read the change
review = prompt(str, f"Review the change of src/search.ts:4 that {run} tested, in https://github.com/uael/fieldnotes/pull/12.", on=chain("review search"))
close(f"Opened https://github.com/uael/fieldnotes/pull/12, and {review} reviews it.")`,
    ],
  ],
  [
    "Review the change of src/search.ts",
    [
      String.raw`# Read the change, then ask what the search must match
diff = await bash("printf '@@ -4 +4 @@\\n-  note.text.includes(query)\\n+  note.text.toLowerCase().includes(query.toLowerCase())\\n'")
accents = await prompt(str, "Must the search match accents too, so that cafe finds café?", to=OPERATOR)`,
      `close("The change reads well, and the search matches what you asked.")`,
    ],
  ],
  [
    "Write the docs of the release",
    [
      String.raw`# Hire a writer for each document
guide = prompt(str, "Write the guide of the search, and build its pages.", on=chain("docs/guide"))
log = prompt(str, "Write the changelog of the release 0.4, and open its PR.", on=chain("docs/changelog"))
first = await wait(600, guide, log)`,
      `# Wait for the guide, then report
await guide
close("The docs of 0.4 are written.")`,
    ],
  ],
  [
    "Write the guide of the search",
    [
      String.raw`# Build the guide, one page at a time
build = await bash("for page in capture search storage keys themes notes tags export; do printf 'Building the page %s\\n' $page; sleep 20; done; printf 'The guide has eight pages\\n'", timeout=600)`,
    ],
  ],
  [
    "Write the changelog of the release",
    [
      String.raw`# Write the changelog, then open its pull request
write(Text("CHANGELOG.md", "# 0.4\n\n- The search ignores case.\n- Ctrl+K opens the search.\n"))
pr = await bash("printf 'https://github.com/uael/fieldnotes/pull/13\\n'")
close("Opened https://github.com/uael/fieldnotes/pull/13 with the changelog of 0.4.")`,
    ],
  ],
  [
    "Record the animation of the release",
    [
      String.raw`# Record the frames of the animation
frames = await bash("printf 'Recorded 40 frames of the search\\n'")`,
      String.raw`# Encode the frames as a gif
gif = await bash("printf 'Encoded furb.gif, 1.2 MB\\n'")`,
    ],
  ],
  [
    "Reach parity with Pi",
    [
      String.raw`# Hire a worker for each gap with Pi
gaps = {key: prompt(str, f"Close the gap of {key} with Pi.", on=chain("parity/" + key)) for key in ["sessions", "rpc", "themes"]}
first = await wait(600, *gaps.values())`,
      `# Wait for every gap, then report
for gap in gaps.values():
  await gap
close("furb reaches parity with Pi.")`,
    ],
  ],
  [
    "Close the gap of sessions",
    [
      String.raw`# Resume a session as Pi does, then open the pull request
pr = await bash("printf 'https://github.com/uael/furb/pull/14\\n'")
close("Opened https://github.com/uael/furb/pull/14: a session resumes as in Pi.")`,
    ],
  ],
  ["Close the gap of rpc", [""]],
  ["Close the gap of themes", ['close("The themes match Pi: five palettes, with the same roles.")']],
];

/** The word that the model of the demo answers with: the prompt that the last turn advances, its message, and how
 * many turns it had, from the turns as the model reads them. Nothing when the script holds no word for it. */
export function scripted(turns: string): string | undefined {
  const asked = [...turns.matchAll(/advance on (prompt\d+)/g)].at(-1)?.[1];
  if (!asked) return undefined;
  const count = turns.split(`advance on ${asked}`).length - 1;
  for (const [cue, words] of script) {
    const text = JSON.stringify(cue).slice(1, -1);
    // A message stands in single quotes, in double quotes when it holds a single quote, and as a quote when it holds
    // more than one line.
    const heads = [
      `${asked}_message = '${text}`,
      `${asked}_message = \\"${text}`,
      `<s:${asked}_message>\\n${text}`,
    ];
    if (!heads.some((head) => turns.includes(head))) continue;
    return words[count - 1];
  }
  return undefined;
}

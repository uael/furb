import type { Script } from "./script.ts";

const draft = `GUIDE = """# Fieldnotes

Fieldnotes keeps your ideas close. You write a note, and you find it later.

## Write a note

Type what you think and press Enter. The note is saved at once, with the time.

## Find a note

Press Ctrl+K and type one word that the note holds. The newest notes come first.

## Stop and start again

Close the app when you want. The next start reads your notes again.
"""
write(Text("docs/guide.md", GUIDE))
print(len(GUIDE.splitlines()), "lines written")`;

const more = String.raw`readme = read("README.md")
if "docs/guide.md" not in readme.content:
  write(readme.append("\nRead the guide in docs/guide.md.\n"))
close("docs/guide.md explains how to write a note, find it with Ctrl+K, and start again. The README links to it.")`;

export const guide: Script[] = [
  {
    cue: "Write docs/guide.md",
    weight: { input: 2100, output: 260, cost: 0.0024 },
    steps: [
      { thinking: "A person who has never used the app needs three short parts.", word: draft },
      { thinking: "The guide stands, so I link it from the README.", word: more },
    ],
  },
];

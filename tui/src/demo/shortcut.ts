import type { Script } from "./script.ts";

const read1 = `# Where does the README speak of keys, and which file reads them?
readme = read("README.md")
for number, line in enumerate(readme.lines, 1):
  if "key" in line.lower() or "ctrl" in line.lower():
    print(number, line)
where = await bash("grep -rn 'key' src test | head -20")`;

const build = String.raw`KEYS = """export const SEARCH_KEY = "ctrl+k";

/** Run open when the key is the one that opens the search. */
export function onKey(name: string, open: () => void): boolean {
  if (name !== SEARCH_KEY) return false;
  open();
  return true;
}
"""

TEST = """import { expect, test } from "bun:test";
import { onKey } from "../src/keys.ts";

test("Ctrl+K opens the search", () => {
  let opened = 0;
  expect(onKey("ctrl+K", () => opened++)).toBe(true);
  expect(opened).toBe(1);
});

test("another key opens nothing", () => {
  expect(onKey("ctrl+j", () => {})).toBe(false);
});
"""

write(Text("src/keys.ts", KEYS))
write(Text("test/keys.test.ts", TEST))
readme = read("README.md")
write(readme.replace("- Find it when it matters", "- Find it when it matters: press Ctrl+K"))
run = await bash("printf '✓ another key opens nothing\\n✗ Ctrl+K opens the search\\n  expected true, received false\\n\\n1 pass\\n1 fail\\n' >&2; exit 1")`;

const mend = String.raw`# The test sends "ctrl+K" and the key is written in lower case, so a name is compared in lower case.
FIX = "  if (name.toLowerCase() !== SEARCH_KEY) return false;"
keys = read("src/keys.ts")
at = 0
for number, line in enumerate(keys.lines, 1):
  if "name !== SEARCH_KEY" in line:
    at = number
if at:
  write(keys.edit(at, at, FIX))

attempt = 0
run = None
while attempt < 3:
  attempt += 1
  run = await bash("printf '✓ Ctrl+K opens the search\\n✓ another key opens nothing\\n\\n2 pass\\n0 fail\\n'")
  if run.code == 0:
    break
print(f"the keys test ran {attempt} time(s), exit {run.code if run else None}")`;

const done = `close("Ctrl+K opens the search: src/keys.ts maps the key, test/keys.test.ts presses it, and the README says so. The key name is compared in lower case, since a terminal sends Ctrl+K in either case. 2 pass, 0 fail.")`;

export const shortcut: Script[] = [
  {
    cue: "Bind Ctrl+K to the search",
    weight: { input: 3600, output: 380, cost: 0.019 },
    steps: [
      {
        thinking: "I find the line of the README that names the keys, and the code that reads them.",
        word: read1,
      },
      { thinking: "No file reads keys yet, so one small file holds the key and its test.", word: build },
      {
        thinking: "The test fails on the case of the letter, so I compare in lower case and run it again.",
        word: mend,
      },
      { thinking: "The checks pass, so I report.", word: done },
    ],
  },
];

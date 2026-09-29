import { expect, test } from "bun:test";
import { stepsOf } from "../src/conversation.ts";

test("a command stands under the step above the line that writes it, or that builds it in a loop or an f-string", () => {
  const word = [
    "# List the files",
    'bash("ls")',
    "# Read each one",
    "for name in names:",
    '  bash(f"cat {name}")',
    "# Count the lines",
    "bash(\"printf 'a\\\\nb\\\\n' | wc -l\")",
  ].join("\n");
  expect(stepsOf(word, ["ls", "cat a", "cat b", "printf 'a\\nb\\n' | wc -l"])).toEqual([0, 1, 1, 2]);
});

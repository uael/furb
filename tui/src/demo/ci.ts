import type { Script } from "./script.ts";

const loop = String.raw`CI = """printf 'Checking the project...\n'
for check in capture search storage keys; do
  printf '✓ %s\n' "$check"
  sleep 1
done
printf 'Watching for a change\n'
sleep 600
"""
write(Text("ci.sh", CI))
run = await bash("sh ci.sh", timeout=900)
print(run.code)`;

export const ci: Script[] = [
  {
    cue: "Run the checks of the project on each change",
    weight: { input: 1800, output: 120, cost: 0.008 },
    steps: [{ thinking: "The checks are one script, and it runs until a change ends it.", word: loop }],
  },
];

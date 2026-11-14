"""The one home of the kind of a file of the diff. It reads `git diff --numstat` on stdin and prints one line for each kind."""
import re
import sys

RULES = [
    ("made by a tool or data", r"\.jsonl$|schema\.json$|protocol\.ts$|index\.d\.cts$|/fixtures/|corpus|\.lock$|catalog\.json$"),
    ("tests", r"(^|/)tests?/|\.test\.|/scenarios/|test_.*\.py$"),
    ("docs", r"\.md$"),
]
sums = {}
for line in sys.stdin:
    added, deleted, path = line.rstrip("\n").split("\t", 2)
    if added == "-":
        continue
    kind = next((k for k, rule in RULES if re.search(rule, path)), "code")
    one = sums.setdefault(kind, [0, 0])
    one[0] += int(added)
    one[1] += int(deleted)
for kind, (added, deleted) in sorted(sums.items()):
    print(f"against main, {kind}: +{added} -{deleted} net {added - deleted:+}")

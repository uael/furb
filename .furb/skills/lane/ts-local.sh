#!/bin/sh
# The oracle of the tests of the TypeScript side on this machine. usage: ts-local.sh <tree>
# It builds the tree, runs `bun run test`, and exits 0 when each test that failed is a line of ts-held.txt.
kit=$(cd "$(dirname "$0")" && pwd)
cd "$1" || exit 2
log="${TMPDIR:-/tmp}/ts-local.log"
bun run build > "$log" 2>&1 || { echo "the build failed:"; tail -15 "$log"; exit 1; }
bun run test > "$log" 2>&1; code=$?
sed -E 's/\x1b\[[0-9;]*m//g' "$log" | grep -E '^\(fail\) ' | sed -E 's/^\(fail\) //; s/ \[[0-9.]+ms\]$//' | sort -u > "$log.fails"
grep -E '[0-9]+ pass, [0-9]+ fail' "$log" | tail -1
new=$(grep -v -x -F -f "$kit/ts-held.txt" "$log.fails")
if [ -n "$new" ]; then echo "tests that fail and that no ruling holds:"; echo "$new"; echo "the whole output: $log"; exit 1; fi
if [ "$code" != 0 ] && [ ! -s "$log.fails" ]; then echo "bun run test exits $code with no failed test:"; tail -15 "$log"; exit 1; fi
echo "each test passes, but the held ones: $(wc -l < "$log.fails" | tr -d ' ') held"

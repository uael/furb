#!/bin/sh
# The oracle of the tests of the TypeScript side on CI. usage: ts-ci.sh <tree> <base>
# It waits for the three checks "the bindings on <system>" of the head of the tree, which must be the head
# of origin/<base>, and exits 0 when each test that failed in them is a line of ts-held.txt. 3: the head is not
# pushed. 4: CI_WAIT seconds (3600) passed.
kit=$(cd "$(dirname "$0")" && pwd)
cd "$1" || exit 2
sha=$(git rev-parse HEAD); git fetch -q origin "$2" || exit 2
[ "$(git rev-parse FETCH_HEAD)" = "$sha" ] || { echo "HEAD $sha is not the head of origin/$2: push it first"; exit 3; }
end=$(( $(date +%s) + ${CI_WAIT:-3600} ))
while :; do
  runs=$(gh api "repos/uael/furb/commits/$sha/check-runs?per_page=100" --paginate --jq '.check_runs[]|select(.name|startswith("the bindings on "))|[.name,.status,(.conclusion//""),(.id|tostring)]|@tsv')
  n=$(printf '%s\n' "$runs" | grep -c .); ended=$(printf '%s\n' "$runs" | awk -F'\t' '$2=="completed"' | grep -c .)
  if [ "$n" -ge 3 ] && [ "$ended" = "$n" ]; then break; fi
  if [ "$(date +%s)" -ge "$end" ]; then echo "$sha: the wait ended with $ended of $n jobs done"; exit 4; fi
  echo "$(date +%T) $sha waits: $ended of $n jobs done"; sleep 60
done
out="${TMPDIR:-/tmp}/ts-ci-$sha.txt"; : > "$out"
printf '%s\n' "$runs" | while IFS="$(printf '\t')" read -r name state result id; do
  [ "$result" = success ] && continue
  log="${TMPDIR:-/tmp}/ts-ci-$id.log"
  gh api "repos/uael/furb/actions/jobs/$id/logs" 2>/dev/null | tr -d '\r' | sed -E 's/\x1b\[[0-9;]*m//g; s/^[0-9T:.-]+Z //' > "$log"
  grep -E '^\(fail\) ' "$log" | sed -E 's/^\(fail\) //; s/ \[[0-9.]+ms\]$//' | sort -u > "$log.fails"
  new=$(grep -v -x -F -f "$kit/ts-held.txt" "$log.fails")
  if [ -n "$new" ]; then echo "$name ($result), tests that fail and that no ruling holds:" >> "$out"; echo "$new" | sed 's/^/  /' >> "$out"; echo "  the log: $log" >> "$out"
  elif [ ! -s "$log.fails" ]; then echo "$name ($result) with no failed test:" >> "$out"; grep -E 'error|##\[error\]|timed out' "$log" | tail -6 | cut -c1-220 | sed 's/^/  /' >> "$out"; echo "  the log: $log" >> "$out"; fi
done
if [ -s "$out" ]; then echo "$sha red:"; cat "$out"; exit 1; fi
echo "$sha: each test of the three jobs passes, but the held ones"

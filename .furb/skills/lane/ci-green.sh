#!/bin/sh
# The oracle of the jobs of CI. usage: ci-green.sh <tree> <base> <name of a check>...
# Exit 0 when each named check of the head of the tree ended with success, 1 when one did not, 3 when the head of the
# tree is not the head of origin/<base>, 4 when CI_WAIT seconds (3000) passed with a check that still runs.
tree=$1; base=$2; shift 2
cd "$tree" || exit 2
sha=$(git rev-parse HEAD)
git fetch -q origin "$base" || exit 2
there=$(git rev-parse FETCH_HEAD)
if [ "$there" != "$sha" ]; then echo "HEAD $sha is not the head of origin/$base, $there: push it first"; exit 3; fi
end=$(( $(date +%s) + ${CI_WAIT:-3000} ))
while :; do
  runs=$(gh api "repos/uael/furb/commits/$sha/check-runs?per_page=100" --paginate --jq '.check_runs[]|[.name,.status,(.conclusion//""),.details_url]|@tsv') || runs=""
  pending=""; bad=""
  for job in "$@"; do
    verdict=$(printf '%s\n' "$runs" | awk -F'\t' -v j="$job" '$1==j{n++; if($2!="completed")p=1; else if($3!="success"){b=1; u=$3" "$4}} END{if(!n||p)print "pending"; else if(b)print "bad "u; else print "ok"}')
    case "$verdict" in
      pending) pending="$pending [$job]";;
      bad*) bad="$bad
  $job: ${verdict#bad }";;
    esac
  done
  if [ -z "$pending" ]; then
    if [ -n "$bad" ]; then echo "$sha red:$bad"; exit 1; fi
    echo "$sha green: $*"; exit 0
  fi
  if [ "$(date +%s)" -ge "$end" ]; then echo "$sha: the wait ended. red so far:$bad"; echo "still running:$pending"; exit 4; fi
  echo "$(date +%T) $sha waits:$pending"
  sleep 60
done

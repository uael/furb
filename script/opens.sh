#!/bin/sh
# Does each record open on a build of furb, with no model and no network?
# usage: sh script/opens.sh <furb binary> <out.tsv> [before.tsv]
# It opens a copy-on-write copy of each record, never the record, with `furb turns`, which makes the life again from
# the record and only inspects it, one record at a time: the records that the file OPENS_LIST names, one path on each
# line, or else the records of tests/corpus and tests/replay. One line for each record goes to out.tsv: its path, the
# verdict (opens when the command exits 0, refused when it exits otherwise, bound when OPENS_BOUND seconds (1800)
# passed, memory when it passed 8 GB), the seconds, the most MB, and the first line of furb that it wrote. Given the
# table of a run before, it exits 1 when a record that opened then does not open now.
furb=$1; out=$2; before=${3:-}; bound=${OPENS_BOUND:-1800}
root=$(cd "$(dirname "$0")/.." && pwd)
work=${TMPDIR:-/tmp}/opens.$$
: > "$out"
{
  if [ -n "${OPENS_LIST:-}" ]; then cat "$OPENS_LIST"; else
    ls "$root"/tests/corpus/*/*.jsonl "$root"/tests/replay/*.jsonl "$root"/tests/replay/*/*.jsonl 2>/dev/null
  fi
} | while read -r R; do
  rm -rf "$work"; mkdir -p "$work/cwd"
  cp -c "$R" "$work/record.jsonl" 2>/dev/null || cp "$R" "$work/record.jsonl"
  log="$work.log"
  start=$(date +%s)
  ( cd "$work/cwd" && HTTP_PROXY=http://127.0.0.1:9 HTTPS_PROXY=http://127.0.0.1:9 ALL_PROXY=http://127.0.0.1:9 \
    exec "$furb" turns --record "$work/record.jsonl" --cwd "$work/cwd" ) > "$log" 2>&1 < /dev/null &
  pid=$!; peak=0; verdict=""
  while kill -0 $pid 2>/dev/null; do
    sleep 2
    rss=$(ps -o rss= -p $pid 2>/dev/null | awk '{printf "%.0f", $1/1024}')
    [ "${rss:-0}" -gt "$peak" ] && peak=$rss
    if [ "${rss:-0}" -gt 8000 ]; then verdict=memory; kill $pid 2>/dev/null; break; fi
    if [ "$(( $(date +%s) - start ))" -ge "$bound" ]; then verdict=bound; kill $pid 2>/dev/null; break; fi
  done
  wait $pid 2>/dev/null; code=$?
  [ -z "$verdict" ] && { [ "$code" = 0 ] && verdict=opens || verdict="refused:$code"; }
  last=$(grep -m1 '^furb:' "$log" | tr -d '\r\t' | cut -c1-240); [ -z "$last" ] && last=$(tail -n 1 "$log" | tr -d '\r\t' | cut -c1-240)
  printf '%s\t%s\t%s\t%s\t%s\n' "$R" "$verdict" "$(( $(date +%s) - start ))" "$peak" "$last" >> "$out"
  printf '%s %s %ss %sMB\n' "$(basename "$R")" "$verdict" "$(( $(date +%s) - start ))" "$peak"
done
rm -rf "$work" "$work.log"
[ -z "$before" ] && exit 0
awk -F'\t' 'NR==FNR { was[$1]=$2; next } { now[$1]=$2 }
END {
  bad = 0
  for (r in was) if (was[r] == "opens" && now[r] != "opens") { print "opened before and not now: " r " (" now[r] ")"; bad = 1 }
  if (!bad) print "each record that opened before opens now"
  exit bad
}' "$before" "$out"

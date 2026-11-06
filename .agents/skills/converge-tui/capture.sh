#!/bin/sh
# Capture the TUI of origin/main in the tree that this script runs in: the gallery of the demo and the newest records of
# the project, each a PNG with the cells of its frame, in <out>/head, after a build.
# Then walk it as a person does, with keys, clicks and sizes, on a life of a scripted model and on those records, which
# needs the program furb: each step of the walk is a capture of <out>/head/walk, and walk.json holds what its probes found.
# Usage: sh capture.sh <main checkout> <out> [<records.json of the last capture>]
# That records.json pins the records and the size of each, and the slice of a long life and its moment, so that a
# capture changes only with the code. The slice is a recorded life that tui/script/fixture.ts cuts from the log, on
# which converge-capture.ts plays a script of the actions of an operator at 222x58 and smaller.
repo=$1 out=$2 pins=${3:-} here=$(dirname "$0")
at="$out/head"
fail() { printf '%s\t%s\t%s\n' "$1" "$2" "$3" >>"$out/fails.txt"; }
rm -rf "$at" "$out/records" "$out/slice" "$out/fails.txt"
mkdir -p "$out/records" "$out/slice" "$at"
{ git fetch -q origin main && git checkout -q --detach origin/main && git clean -fdq; } >"$at/checkout.log" 2>&1 || { fail head checkout "$at/checkout.log"; exit 0; }
git rev-parse HEAD >"$out/sha"
[ -z "$pins" ] || [ ! -f "$pins" ] || cp "$pins" "$out/pins.json"
python3 -I "$here/records.py" "$repo/.furb/sessions" "$out" 3 160000000 >"$out/records.log" 2>&1 || fail head records "$out/records.log"
{ bun install --frozen-lockfile && bun run build; } >"$at/build.log" 2>&1 || { fail head build "$at/build.log"; exit 0; }
export FURB_GALLERY_ROOT="$out/gallery" FURB_STILL_CLOCK=1767225600000
FURB_GALLERY_OUT="$at/demo" FURB_GALLERY_CELLS="$at/demo" bun run screenshots >"$at/demo.log" 2>&1 || fail head demo "$at/demo.log"
FURB_CONVERGE_OUT="$at/life" FURB_CONVERGE_RECORDS="$(ls "$out"/records/*.jsonl 2>/dev/null)" bun run tui/script/converge-capture.ts >"$at/life.log" 2>&1 || fail head life "$at/life.log"
slice=$(python3 -I -c 'import json,sys; s=json.load(open(sys.argv[1])); print(s and "%s\t%s" % (s["log"], s["moment"]) or "")' "$out/slice.json" 2>/dev/null)
if [ -n "$slice" ]; then
  log=$(printf '%s' "$slice" | cut -f1) moment=$(printf '%s' "$slice" | cut -f2) life="$out/slice/$(basename "$log" .jsonl.log).jsonl"
  { bun run tui/script/fixture.ts "$log" "$life" "$moment" && python3 -I "$here/records.py" pin "$out" "$life"; } >"$at/slice.log" 2>&1 || fail head slice "$at/slice.log"
  FURB_CONVERGE_OUT="$at/life" FURB_CONVERGE_SLICE="$life" bun run tui/script/converge-capture.ts >>"$at/slice.log" 2>&1 || fail head slice "$at/slice.log"
fi
{ cargo build -p furb-cli; } >"$at/furb.log" 2>&1 || { fail head furb "$at/furb.log"; exit 0; }
FURB_WALK_OUT="$at/walk" FURB_CONVERGE_RECORDS="$(ls "$out"/records/*.jsonl 2>/dev/null)" bun run tui/script/converge-walk.ts >"$at/walk.log" 2>&1 || fail head walk "$at/walk.log"

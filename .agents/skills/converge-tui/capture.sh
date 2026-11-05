#!/bin/sh
# Capture the TUI of origin/main in the tree that this script runs in: the gallery of the demo and the newest records of
# the project, each a PNG with the cells of its frame, in <out>/head, after a build that takes a slot of the machine.
# Usage: sh capture.sh <main checkout> <out> [<records.json of the last capture>]
# That records.json pins the records and the size of each, so that a capture changes only with the code.
repo=$1 out=$2 pins=${3:-} here=$(dirname "$0")
at="$out/head"
fail() { printf '%s\t%s\t%s\n' "$1" "$2" "$3" >>"$out/fails.txt"; }
rm -rf "$at" "$out/records" "$out/fails.txt"
mkdir -p "$out/records" "$at"
{ git fetch -q origin main && git checkout -q --detach origin/main && git clean -fdq; } >"$at/checkout.log" 2>&1 || { fail head checkout "$at/checkout.log"; exit 0; }
git rev-parse HEAD >"$out/sha"
[ -z "$pins" ] || [ ! -f "$pins" ] || cp "$pins" "$out/pins.json"
python3 -I "$here/records.py" "$repo/.furb/sessions" "$out" 3 160000000 >"$out/records.log" 2>&1 || fail head records "$out/records.log"
{ bun install --frozen-lockfile && sh script/build-slot.sh bun run build; } >"$at/build.log" 2>&1 || { fail head build "$at/build.log"; exit 0; }
export FURB_GALLERY_ROOT="$out/gallery" FURB_STILL_CLOCK=1767225600000
FURB_GALLERY_OUT="$at/demo" FURB_GALLERY_CELLS="$at/demo" bun run screenshots >"$at/demo.log" 2>&1 || fail head demo "$at/demo.log"
FURB_CONVERGE_OUT="$at/life" FURB_CONVERGE_RECORDS="$(ls "$out"/records/*.jsonl 2>/dev/null)" bun run tui/script/converge-capture.ts >"$at/life.log" 2>&1 || fail head life "$at/life.log"

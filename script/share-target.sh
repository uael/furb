#!/bin/sh
# Link the `target` of a worktree to the one that each worktree of the clone shares, <main checkout>-target, so that a
# dependency builds once. A tree that holds a target of its own drops it: the target is a cache. The script leaves a
# tree as it is when a build runs in it (cargo holds .cargo-lock open for a build), and when its .cargo/config.toml has
# no FURB_TREE: with no stamp, two trees reuse the crates of each other in a shared target and test the code of the
# other. The hooks post-merge and post-commit run it, so a tree links itself when it merges the branch that holds it.
# Usage: script/share-target.sh [worktree, the current folder by default]
set -eu

tree=${1:-$PWD}
[ ! -L "$tree/target" ] || exit 0
grep -q FURB_TREE "$tree/.cargo/config.toml" 2>/dev/null || exit 0
common=$(git -C "$tree" rev-parse --path-format=absolute --git-common-dir)
if [ "$(git --git-dir="$common" rev-parse --is-bare-repository)" = true ]; then
  main=$common
else
  main=$(dirname "$common")
fi
shared=$main-target

# A test binary holds no lock, but its command line names the target.
# shellcheck disable=SC2009
if [ -n "$(lsof -t "$tree"/target/*/.cargo-lock 2>/dev/null)" ] ||
  ps -A -ww -o args= | grep -F "$tree/target" | grep -qv 'grep -F'; then
  exit 0
fi
# The target leaves at once by a rename, and a build that starts after it finds the link. The rest of the delete runs
# apart, since a target of 100 GB takes minutes to delete.
drop=
if [ -d "$tree/target" ]; then
  drop=$shared-drop.$$
  mv "$tree/target" "$drop"
fi
mkdir -p "$shared"
ln -sfn "$shared" "$tree/target"
if [ -n "$drop" ]; then rm -rf "$drop" >/dev/null 2>&1 & fi

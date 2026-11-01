#!/bin/sh
# Make the build of a version of furb in the store of builds of this machine: script/builds.sh <version>.
# It builds the command line at the last commit of origin/main that holds that version, in a worktree that it
# removes, and runs it once, which puts it in the store, as src/world/builds.rs says.
set -eu
version=${1:?usage: script/builds.sh <version>}
root=$(cd "$(dirname "$0")/.." && pwd)
git -C "$root" fetch -q origin main
# The last commit of a version is the parent of the commit that made the next version, or the tip of main.
next=$(git -C "$root" log --format=%H -G "^pub const VERSION: i64 = $((version + 1));" origin/main -- src/world/store.rs | tail -n 1)
commit=$(git -C "$root" rev-parse "${next:-origin/main}${next:+^}")
if ! git -C "$root" show "$commit:src/world/store.rs" | grep -q "^pub const VERSION: i64 = $version;\$"; then
  echo "builds.sh: no commit of origin/main holds the version $version" >&2
  exit 1
fi
branch="furb-build-$version-$$"
at="${TMPDIR:-/tmp}/$branch"
"$root/script/worktree.sh" "$at" "$branch" "$commit"
trap 'git -C "$root" worktree remove --force "$at"; git -C "$root" branch -D "$branch" >/dev/null' EXIT
(cd "$at" && cargo build -p furb-cli)
"$at/target/debug/furb" --version
echo "builds.sh: the build of version $version from $commit stands in the store of builds"

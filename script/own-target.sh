#!/bin/sh
# Give a tree a cargo target of its own in place of a link to a target that trees share. A target that trees share
# rebuilds the crates of the workspace at each build of another tree, since cargo gives a crate of the workspace the
# same name in each tree. The new target is a copy on write of the one that the link names, so the tree starts warm,
# and a second run leaves the tree as it is. A file system with no copy on write gives the tree an empty target.
# Usage: script/own-target.sh [tree, the current folder by default]
set -eu

tree=${1:-$PWD}
[ -L "$tree/target" ] || exit 0
shared=$(cd "$tree/target" && pwd -P)
case $(uname) in
  Darwin) clone() { cp -cRp "$1" "$2"; } ;;
  *) clone() { cp -a --reflink=always "$1" "$2"; } ;;
esac
next=$tree/target.own.$$
if ! clone "$shared" "$next" 2>/dev/null; then
  rm -rf "$next"
  mkdir "$next"
  echo "This file system has no copy on write: the target of $tree starts empty."
fi
rm "$tree/target"
mv "$next" "$tree/target"

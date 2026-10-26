#!/bin/sh
# Make a git worktree whose build starts warm: its build directories are a copy on write of those of the main
# checkout, so cargo, maturin and bun build only what changed.
# Usage: script/worktree.sh <path> <branch> [base, origin/main by default]
# A branch that stands on origin, or in this clone, is checked out and not made again: a local branch that only lags
# behind origin moves up to it, and a local branch with work that origin lacks stays as it is. The base is the start
# of a new branch alone.
set -eu

if [ $# -lt 2 ] || [ $# -gt 3 ]; then
  echo "usage: $0 <path> <branch> [base]" >&2
  exit 2
fi
path=$1
branch=$2
base=${3:-origin/main}
case $path in
  /*) ;;
  *) path=$PWD/$path ;;
esac

common=$(git -C "$(dirname "$0")" rev-parse --path-format=absolute --git-common-dir)
if [ "$(git --git-dir="$common" rev-parse --is-bare-repository)" = true ]; then
  main=$common
else
  main=$(dirname "$common")
fi

git -C "$main" fetch -q origin

# Where the branch stands decides how it opens. Git refuses a branch that another worktree holds, and says where.
if git -C "$main" show-ref --verify -q "refs/heads/$branch"; then
  git -C "$main" worktree add -q "$path" "$branch"
  opened=local
  if [ $# -eq 3 ]; then
    echo "The branch $branch stands already, so the base $base is not used."
  fi
elif git -C "$main" show-ref --verify -q "refs/remotes/origin/$branch"; then
  git -C "$main" worktree add -q -b "$branch" "$path" "origin/$branch"
  opened=remote
  if [ $# -eq 3 ]; then
    echo "The branch $branch stands on origin, so the base $base is not used."
  fi
else
  git -C "$main" worktree add -q -b "$branch" "$path" "$base"
  opened=new
fi

if git -C "$path" show-ref --verify -q "refs/remotes/origin/$branch"; then
  git -C "$path" branch -q --set-upstream-to="origin/$branch" "$branch"
  if [ $opened = local ]; then
    if git -C "$path" merge-base --is-ancestor HEAD "origin/$branch"; then
      git -C "$path" merge -q --ff-only "origin/$branch"
    elif git -C "$path" merge-base --is-ancestor "origin/$branch" HEAD; then
      echo "The local branch $branch holds commits that origin lacks. It stays as it is."
    else
      echo "The local branch $branch and origin/$branch each hold commits that the other lacks. Look at both before you push."
    fi
  fi
fi
if [ $opened != new ]; then
  behind=$(git -C "$path" rev-list --count "HEAD..origin/main" 2>/dev/null || echo 0)
  if [ "$behind" -gt 0 ]; then
    echo "$branch is $behind commits behind origin/main. Rebase it there, or merge origin/main into it, before you build."
  fi
fi

# What the python side needs, said at each end of the script.
python_note() {
  echo "$path has no .venv. uv sync there makes it, builds the crate again and takes minutes, and uv run --no-sync before it"
  echo "falls back to the system python, which lacks the packages. Run: (cd $path && uv sync)"
  if [ -x "$main/.venv/bin/python" ]; then
    echo "Until then, run a probe with $main/.venv/bin/python."
  fi
}

# A copy on write: clonefile on APFS, a reflink on Btrfs or XFS. Each keeps the timestamps, which cargo compares.
case $(uname) in
  Darwin) clone() { cp -cRp "$1" "$2"; } ;;
  *) clone() { cp -a --reflink=always "$1" "$2"; } ;;
esac
if [ ! -f "$main/Cargo.toml" ]; then
  echo "$main holds no checkout to copy: $path is ready on $branch, and builds from nothing."
  python_note
  exit 0
fi
if ! clone "$main/Cargo.toml" "$path/.worktree-probe" 2>/dev/null; then
  echo "This file system has no copy on write: $path builds from nothing."
  python_note
  exit 0
fi
rm -f "$path/.worktree-probe"

# The output of cargo, which maturin and napi share, the packages of bun, the build of the TypeScript package, and
# the cache of the tests of the TUI. The environment of python is not copied: its scripts name the path of the main
# checkout, and uv sync makes it again on the warm target.
for dir in target node_modules bind/typescript/node_modules tui/node_modules bind/typescript/dist .furb/tests; do
  if [ -e "$main/$dir" ] && [ ! -e "$path/$dir" ]; then
    mkdir -p "$(dirname "$path/$dir")"
    clone "$main/$dir" "$path/$dir"
  fi
done
for node in "$main"/bind/typescript/*.node "$main"/bind/typescript/index.cjs; do
  if [ -e "$node" ]; then
    clone "$node" "$path/bind/typescript/"
  fi
done
# Each member of the workspace builds again in a new path, so its incremental state is of no use.
if [ -d "$path/target" ]; then
  find "$path/target" -type d -name incremental -prune -exec rm -rf {} +
fi

# The copied dist and native module are those of the main checkout, whose commit may differ from the base. Link the
# workspace from the copied cache, then build the native module and the TypeScript package of this tree on the warm
# target, so that bun run check and bun run test run at once.
if [ -d "$path/node_modules" ] && command -v bun >/dev/null 2>&1; then
  if ! (cd "$path" && bun install --frozen-lockfile --offline >/dev/null 2>&1 && bun run build >/dev/null 2>&1); then
    echo "$path is ready on $branch, but the TypeScript side did not build. Run bun install and bun run build there."
    python_note
    exit 0
  fi
fi

echo "$path is ready on $branch."
python_note

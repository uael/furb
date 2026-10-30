#!/bin/sh
# Prove the package furb of npm as a user installs it, from the packed packages alone: npm installs the packed furb and
# the packed engine of this system in a new folder, with no registry for either, and node runs the example chains of
# docs/examples/typescript there, on a claude command line that answers from a script. The first argument is the folder
# of the packs, furb-<version>.tgz and uael-furb-engine-<system>-<version>.tgz, as npm pack names them. The fake
# claude is bind/typescript/test/programs/claude.ts, which bun compiles to a program, since Windows runs no script by
# its shebang.
set -eu
packs=$(cd "$1" && pwd)
root=$(cd "$(dirname "$0")/.." && pwd)
yard=$(mktemp -d)
mkdir "$yard/app" "$yard/fake"
(cd "$yard/fake" && bun build --compile "$root/bind/typescript/test/programs/claude.ts" --outfile claude >/dev/null)
claude=$yard/fake/claude
[ -f "$claude.exe" ] && claude=$claude.exe
printf '%s' '"close(\"Login is twice as fast.\")"' > "$yard/fake/word.1"
printf '%s' '"close(\"The notes read well.\")"' > "$yard/fake/word.2"
system=$(node -p '({"linux-x64": "linux-x64-gnu", "linux-arm64": "linux-arm64-gnu", "darwin-arm64": "darwin-arm64",
  "darwin-x64": "darwin-x64", "win32-x64": "win32-x64-msvc"})[`${process.platform}-${process.arch}`]')
cd "$yard/app"
printf '{ "name": "proof", "private": true, "type": "module" }\n' > package.json
# The engine of this system first, so that the optional dependency of furb on it finds it here and asks no registry.
npm install --no-audit --no-fund --no-save "$packs/uael-furb-engine-$system-"[0-9]*.tgz "$packs/furb-"[0-9]*.tgz
cp "$root/docs/examples/typescript/chains.ts" chains.ts
said=$(env -u FURB_LOCAL_URL -u FURB_LOCAL_KEY -u FURB_TUI FURB_CLAUDE_BIN="$claude" \
  FURB_CONFIG_DIR="$yard/config" XDG_CACHE_HOME="$yard/cache" node chains.ts)
want=$(printf 'The notes read well.\nreviewer under root')
if [ "$said" != "$want" ]; then
  printf 'chains said:\n%s\n' "$said" >&2
  exit 1
fi
echo "furb of npm runs the example chains on $system"

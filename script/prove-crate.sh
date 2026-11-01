#!/bin/sh
# Prove the crates of furb as a user will take them from crates.io, from the packed crates alone. crates.io takes no
# crate that stands on git, and furb stands on the fork of monty by git, so the proof stands in for the day when monty
# stands on crates.io with what furb needs: it copies this checkout, and there it gives each dependency on monty the
# version of the fork at the revision that Cargo.toml names. cargo packs the crates of the fork that furb runs at that
# revision, and furb and furb-cli on them. Each packed crate stands in for crates.io as a patch of crates.io, and cargo
# takes every other crate from crates.io and resolves each version afresh, with no lock of furb, as a user gets them.
# A new crate takes furb as cargo add writes it, runs the example chains of docs/examples/rust there, on a claude
# command line that answers from a script, and cargo install puts furb-cli on it and runs furb --help. The first
# argument is the folder of the proof, a new folder by default, which keeps the packed crates in crates/.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
work=${1:-$(mktemp -d)}
mkdir -p "$work"
work=$(cd "$work" && pwd)
version=$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"$/\1/p' "$root/Cargo.toml")
rev=$(sed -n 's/^monty = { git = "https:\/\/github.com\/uael\/monty", rev = "\([0-9a-f]*\)".*/\1/p' "$root/Cargo.toml")
export CARGO_TARGET_DIR="$work/target"
rm -rf "$work/monty" "$work/furb" "$work/unpacked" "$work/crates" "$work/demo" "$work/yard" "$work/bin"
mkdir "$work/crates" "$work/unpacked" "$work/furb"
git clone -q https://github.com/uael/monty "$work/monty"
git -C "$work/monty" checkout -q "$rev"
fork=$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"$/\1/p' "$work/monty/Cargo.toml")
monty="monty-macros monty-types monty-alloc monty monty-typeshed monty-type-checking"
(cd "$work/monty" && cargo package --quiet --no-verify $(printf -- '-p %s ' $monty))
# The copy of the checkout: each file that git tracks, as it stands now, and each dependency on monty by its version.
(cd "$root" && git ls-files -z | tar --null -T - -cf -) | tar -xf - -C "$work/furb"
sed -i.git 's/^\(monty[a-z-]* = { git = "https:\/\/github.com\/uael\/monty", rev = "[0-9a-f]*"\) }$/\1, version = "'"$fork"'" }/' \
  "$work/furb/Cargo.toml"
test "$(grep -c "version = \"$fork\" }" "$work/furb/Cargo.toml")" = 4
rm "$work/furb/Cargo.lock"
printf '[patch.crates-io]\n' > "$work/config.toml"
unpack() {
  tar -xzf "$1" -C "$work/unpacked"
  cp "$1" "$work/crates/"
  name=$(basename "$1" .crate)
  printf '%s = { path = "%s" }\n' "${name%-*}" "$work/unpacked/$name" >> "$work/config.toml"
}
for crate in $monty; do
  unpack "$CARGO_TARGET_DIR/package/$crate-$fork.crate"
done
(cd "$work/furb" && cargo --config "$work/config.toml" package --quiet --no-verify -p furb)
unpack "$CARGO_TARGET_DIR/package/furb-$version.crate"
cargo new --quiet --vcs none "$work/demo"
cd "$work/demo"
# cargo add reads the index of crates.io and no patch, so the line that it would write for furb is written here.
cargo add --quiet futures
sed -i.new "/^\[dependencies\]$/a\\
furb = \"$version\"
" Cargo.toml
cp "$root/docs/examples/rust/chains.rs" src/main.rs
cargo --config "$work/config.toml" build --quiet
mkdir -p "$work/yard/work" "$work/yard/fake"
cp "$root/src/world/provider/claude/fake-claude.sh" "$work/yard/fake/claude"
chmod +x "$work/yard/fake/claude"
printf '%s' '"close(\"Login is twice as fast.\")"' > "$work/yard/fake/word.1"
printf '%s' '"close(\"The notes read well.\")"' > "$work/yard/fake/word.2"
said=$(cd "$work/yard/work" && env -u FURB_LOCAL_URL -u FURB_LOCAL_KEY -u FURB_TUI \
  FURB_CLAUDE_BIN="$work/yard/fake/claude" FURB_CONFIG_DIR="$work/yard/config" XDG_CACHE_HOME="$work/yard/cache" \
  "$CARGO_TARGET_DIR/debug/demo" </dev/null)
want=$(printf 'The notes read well.\nreviewer under root')
if [ "$said" != "$want" ]; then
  printf 'chains said:\n%s\n' "$said" >&2
  exit 1
fi
(cd "$work/furb" && cargo --config "$work/config.toml" package --quiet --no-verify -p furb-cli)
unpack "$CARGO_TARGET_DIR/package/furb-cli-$version.crate"
cargo --config "$work/config.toml" install --quiet --debug --root "$work/bin" --path "$work/unpacked/furb-cli-$version"
"$work/bin/bin/furb" --help >/dev/null
echo "furb $version packs on monty $fork, adds to a new crate that resolves its dependencies afresh, runs the example chains, and cargo install gives furb"

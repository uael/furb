# The crate

`src/` serves both things: `src/furb/engine.py` is the engine, and the `.rs` files beside it are the crate that
runs that same file in the sandbox of monty and gives one life of it to a host that is not python. The crate
embeds the file, so the engine a host runs and the engine a model reads are one thing.

## What it is

- `src/sand.rs` is one session of monty, which holds one life. The one name the sandbox reaches out by is `host`,
  and every call of it carries the name of an ear and one plain value, and comes back with one plain value.
- `src/preamble.py` is the boundary. It runs in the sandbox in a module of its own, before the engine. It stands
  in for every ear of the host, it holds the Kernel, which runs the word of a rung in the module of its chain and
  asks the gate of the crate whether the word may run, and it makes every value plain on its way out and again on
  its way in. Nothing of it is bound in the engine.
- `src/gate.rs` is the gate: ty, as a library, reading the word of a rung against `engine.pyi` and the rungs of
  its chain before it. It is the Kernel's, and no host gives one.
- `src/fact.rs` is the plain form: what a value is on the boundary, and what a fact is made of.
- `src/ear.rs` is what a host writes: an ear, which hears every fact and answers with one saying, with a word to
  read in the names of the engine, or with nothing. `Host` is the one call beneath it.
- `src/life.rs` is one life: it boots on a record and on named ears, and it runs words of the operator.

Monty comes from the fork by a pinned revision, since the interpreter the engine needs is not released. A revision
and not a branch, so a build of today and a build of next month read the same interpreter. The crate cannot be
published while that holds, since crates.io takes no git dependency, and the revision becomes a version the day
the fork lands.

## What proves it

The suite in `test/` is the proof of the contract, and it runs on this engine as it runs on the engine of this
interpreter: `uv run pytest -q` runs every test on both, through `furb_monty.engine`, which is every name of
`engine.pyi` over one life of the crate. A test that binds the Kernel double of the harness is skipped on monty
by name, with the reason, since the crate holds its Kernel and hands no host one.

`tests/life.rs` drives one life of the real engine from end to end on an ear of this machine, written small. It
holds the Rust surface and is no proof of the contract.

`cargo test`, `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` hold the crate itself, and the
commit hook runs them. `uv run python script/needs.py` says what the engine and the preamble need of the
interpreter that runs them, read off the two files, which is what the fork of monty must offer and no more.

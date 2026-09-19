# furb

[![gates](https://github.com/uael/furb/actions/workflows/gates.yml/badge.svg)](https://github.com/uael/furb/actions/workflows/gates.yml)

The core of an AI harness, in one python file, `src/furb/engine.py`. A model does everything with python that it
writes: the engine, minified, is its whole system prompt, and each word it answers with runs on a chain of the
engine. The contract is `src/furb/engine.pyi`, whose docstrings hold every law, one sentence per line, and the suite
in `test/` holds one test per sentence.

`pip install furb` installs the engine and the `furb` command. It needs python 3.14.

From a clone of this repository:

- `uv sync` installs the environment, and `uv run pytest -q` runs the suite.
- `uv run python script/smoke.py` runs one real life on opus/low through the claude command line.
- `uv run python script/deepswe.py run <task>` runs one DeepSWE task and grades it.

Every pull request, and every push to `main`, runs the gates in `.github/workflows/gates.yml`: the hooks, the type
check, and the suite on linux and on macos.

## The crate

The crate beside the package runs that same engine in a sandbox and gives its surface to a host that is not
python. `src/` serves both, and the crate carries the same file, so the engine a host runs and the engine a model
reads are one thing.

What a host writes is a World: what a life may touch is the host's to decide. What the crate takes care of is the
Kernel, which gates the word of a rung and runs it. The word of a model runs where the engine runs, in the globals
of its chain, inside the sandbox, so nothing of it crosses to the host and nothing calls back into a life that
stands waiting. What crosses is a fact, plain, and the gate.

```rust
let (voice, ears) = furb::Ears::made();
let world = furb::Live::new("/w", "opus/low", roster, talks, voice);
let mut life = furb::Life::boot(session, world, gate, ears, &[])?;
let root = life.root().to_owned();
let act = life.calls(furb::verb::Prompt { shape: "int", message: "count the lines", on: &root, ..Default::default() })?;
while life.came(&act)?.is_none() {}
```

- `cargo test` runs the tests of the crate, `tests/life.rs` among them, which drives one life of the real engine
  from end to end.
- `uv run python script/needs.py` says what the engine needs of the interpreter that runs it.

Copyright (C) 2026 Abel Lucas. furb is free software under the GNU Affero General Public License, version 3, which
`LICENSE` holds: you may use, study, change and share it, and anyone who ships it or runs a changed furb as a
service must offer the source under the same terms. The engine carries no notice of its own, since its text is the
system prompt of a model and every word of it counts.

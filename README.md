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

The crate beside the package runs that same engine in monty, a python interpreter written in rust for untrusted
code, and gives one life of it to a host that is not python. The crate embeds the file, so the engine a host runs
and the engine a model reads are one thing.

What a host writes is an ear: the World, which hears every fact and says what it will. What the crate takes care
of is the Kernel, which gates the word of a rung against the contract and runs it where the engine runs, in the
module of its chain, so nothing of the word crosses to the host. What crosses is a fact, plain.

```rust
let ears = furb::Ears::new().with("world", yard);
let mut life = furb::Life::boot(furb::Sand::default(), ears, &["world".to_owned()], &furb::Value::List(vec![]))?;
let act = life.word("prompt(int, \"count the lines\", on=\"chain://operator.1\")")?;
let got = life.word(&format!("peek({:?})", act.as_str().unwrap()))?;
```

`furb-monty`, under `bind/python`, is the same life for python: every name of `engine.pyi`, one to one, over the
engine in the sandbox. `FURB_ENGINE=monty` makes `from furb import engine` give it, and the suite runs on both
engines.

- `cargo test` runs the tests of the crate, `tests/life.rs` among them, which drives one life of the real engine
  from end to end.
- `uv run python script/needs.py` says what the engine needs of the interpreter that runs it.

Copyright (C) 2026 Abel Lucas. furb is free software under the GNU Affero General Public License, version 3, which
`LICENSE` holds: you may use, study, change and share it, and anyone who ships it or runs a changed furb as a
service must offer the source under the same terms. The engine carries no notice of its own, since its text is the
system prompt of a model and every word of it counts.

# furb

[![gates](https://github.com/uael/furb/actions/workflows/gates.yml/badge.svg)](https://github.com/uael/furb/actions/workflows/gates.yml)

The core of an AI harness, in one python file, `src/furb/engine.py`. A model does everything with the python that
it writes: the engine, minified, is its whole system prompt, and each word it answers with runs on a chain of the
engine. The contract is `src/furb/engine.pyi`. Its docstrings hold every law, one sentence per line, and the suite
in `test/` holds one test per sentence.

The crate at the root runs the same engine in monty, a python interpreter written in rust, behind an async API: a
host writes one `World` trait, and a `Life` gives the verbs of the contract. Built with its `python` feature, the
crate is the package `furb-monty`, and `FURB_ENGINE=monty` makes `from furb import engine` give the engine in the
sandbox.

`pip install furb` installs the engine, the `furb` command and `furb-monty`, whose type checker gates every word
that a model writes. It needs python 3.14 or later.

From a clone of this repository:

- `uv sync` installs the environment, and `uv run pytest -q` runs the suite.
- `uv run python script/smoke.py` runs one real life on opus/low through the claude command line.
- `uv run python script/deepswe.py run <task>` runs one DeepSWE task and grades it.

The [TypeScript package](bind/typescript/README.md) reaches the same crate through N-API. Its queries and
controls are synchronous, its acts can be awaited, and it includes a World with pi-ai models, files, commands,
and records. `uv sync`, `bun install`, and `bun run build` build it from a clone.

`bun run tui` opens the [OpenTUI application](tui/README.md). `bun run demo` opens a local scripted life that
asks no model. The TUI has one view per chain, Python programs, act output, an exact transcript, file diffs,
live value inspection, themes, and saved sessions. See [the screenshot gallery](docs/tui.md).

Every pull request, and every push to `main`, runs the gates in `.github/workflows/gates.yml`: the hooks, which
include the gates of the crate, and the type check. On linux and on macos, they run the suite. On linux, on macos
and on windows, they run the gates of the TypeScript bindings and of the TUI.

Copyright (C) 2026 Abel Lucas. furb is free software under the GNU Affero General Public License, version 3, which
`LICENSE` holds: you may use, study, change and share it, and anyone who ships it or runs a changed furb as a
service must offer the source under the same terms. The engine carries no notice of its own, since its text is the
system prompt of a model and every word of it counts.

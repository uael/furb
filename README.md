<div align="center">

# furb

**The model answers in Python. You read and steer each word it runs.**

[![gates](https://github.com/uael/furb/actions/workflows/gates.yml/badge.svg)](https://github.com/uael/furb/actions/workflows/gates.yml)

<img src="docs/furb.gif" alt="The furb TUI: a message, the Python that the model writes, a Python word of the operator, the three views, the command palette, and the rewind tree" width="880">

</div>

furb is the core of an AI harness, in one Python file: `src/furb/engine.py`. The model does everything with the
Python that it writes. The engine, minified, is its whole system prompt, and each word that the model answers
with runs on a chain of the engine.

- **One file, one contract.** The contract is `src/furb/engine.pyi`. Its docstrings hold every law, one sentence
  per line, and the suite in `test/` holds one test per sentence.
- **Every word is gated.** A type checker reads each word before it runs. A word that it refuses never runs, and
  the model reads why.
- **Nothing is hidden.** The transcript that the model reads is the text that you read, and each act, value, and
  file change stays in a record that a later life replays.

## Install

```sh
pip install furb
```

This installs the engine, the `furb` command, and `furb-monty`, whose type checker gates every word that a model
writes. It needs Python 3.14 or later.

## The TUI

```sh
bun run demo    # A local, scripted life that asks no model.
bun run tui     # A real life, through the claude command line.
```

The TUI is an [OpenTUI application](tui/README.md) with three views of each chain:

- **Feed**: your messages, the Python that each model wrote, the acts that it made, and the answers.
- **Transcript**: the exact text that the model reads.
- **Changes**: the diff of each file that the life wrote.

Press **Escape twice** to open the rewind tree in the feed, and branch from any earlier message or act. **Ctrl+P**
finds every action by its name, its slash command, or its keys. **Ctrl+R** writes Python with the same gate as the
model. The sidebar holds the chains, the usage, and your workspaces, and every control answers the mouse. See
[the gallery](docs/tui.md) for each screen.

## How it is built

| Part | What it is |
| --- | --- |
| `src/furb/engine.py` | The engine, one file. It depends only on the Python interpreter. |
| `src/furb/engine.pyi` | The contract, which the suite in `test/` proves. |
| The crate at the root | The same engine in monty, a Python interpreter written in Rust, behind an async API. A host writes one `World` trait, and a `Life` gives the verbs of the contract. Built with its `python` feature, the crate is the package `furb-monty`, and `FURB_ENGINE=monty` makes `from furb import engine` give the engine in the sandbox. |
| [`bind/typescript`](bind/typescript/README.md) | The same crate through N-API. Its queries and controls are synchronous, its acts can be awaited, and it includes a World with pi-ai models, files, commands, and records. |
| [`tui`](tui/README.md) | The OpenTUI application on the TypeScript package. |

## From a clone

```sh
uv sync                                        # The environment.
uv run pytest -q                               # The suite.
uv run python script/smoke.py                  # One real life on opus/low through the claude command line.
uv run python script/deepswe.py run <task>     # One DeepSWE task, graded.
bun install && bun run build                   # The TypeScript package.
bun run screenshots && bun run animation       # The gallery and the animation above.
```

Every pull request, and every push to `main`, runs the gates in `.github/workflows/gates.yml`: the hooks, which
include the gates of the crate, and the type check. On Linux and on macOS, they run the suite. On Linux, on macOS,
and on Windows, they run the gates of the TypeScript bindings and of the TUI.

## License

Copyright (C) 2026 Abel Lucas. furb is free software under the GNU Affero General Public License, version 3, which
`LICENSE` holds: you may use, study, change, and share it, and anyone who ships it or runs a changed furb as a
service must offer the source under the same terms. The engine carries no notice of its own, since its text is the
system prompt of a model and every word of it counts.

<div align="center">

# furb

**An AI agent that answers in Python, on a record that you can read, replay and audit.**

[![gates](https://github.com/uael/furb/actions/workflows/gates.yml/badge.svg)](https://github.com/uael/furb/actions/workflows/gates.yml)

<img src="https://raw.githubusercontent.com/uael/furb/main/docs/furb.gif" alt="The furb TUI: a message, the Python that the model writes, a Python word of the operator, the three views, the command palette, the rewind tree, and a change of theme" width="880">

</div>

furb runs a model as a programmer, not as a caller of tools. You give the model a task. It answers with a short
Python program, which furb calls a word. furb checks the word, runs it, and shows the model what happened. The model
writes its next word, and so on, until it gives you its answer.

Each step goes into a record: each word, each answer of a model, and each result of a command. Stop furb at any
time, open the record again, and the work goes on from where it stopped.

## Why it matters

- **One answer does a lot of work.** Most agents call one tool for each turn of the model. In furb, one word reads
  files, runs commands, asks other models, waits for them, and joins the results with loops and conditions.
- **Nothing is hidden from you.** The whole system prompt is one Python file,
  [`src/furb/engine.py`](https://github.com/uael/furb/blob/main/src/furb/engine.py), smaller than 6500 tokens. The transcript is the exact text that the
  model reads.
- **A word is checked before it runs.** A type checker reads each word first. A word that it refuses never runs, and
  the model reads why.
- **The record is the truth.** A life that a crash stopped goes on from its record. A later run replays it exactly,
  and [an audit](https://github.com/uael/furb/blob/main/docs/audit.md) checks it with no model and no network.
- **Work runs side by side.** A word can start many commands and questions, then wait for all of them. Chains,
  which are separate conversations, work in parallel, each with its own state.
- **You stay in control.** The model can ask you a question and wait for your answer. You can pause any work, set a
  dollar ceiling, rewind a chain to an earlier point, or change the plan of a model while it works.
- **Extensions are Python too.** An extension is a word that each chain runs, so the model reads it as it reads its
  own work. [Five come with furb](https://github.com/uael/furb/blob/main/docs/extensions.md): the memory of a project, its skills, compaction, the web, and
  a guide to extend furb.

## Install

| Where | Command | What you get |
| --- | --- | --- |
| PyPI | `pip install furb` | The Python package and `furb`, the command line. It needs Python 3.14 or later. |
| npm | `npm install furb` | The engine for Node.js 22 or later and Bun 1.4.2 or later, with no Python. |
| Rust | `cargo add furb --git https://github.com/uael/furb` | The engine for a program of Rust, with no Python. Copy the [`Cargo.lock`](https://github.com/uael/furb/blob/main/Cargo.lock) of furb into a new crate before its first build, since a newer version of a dependency breaks the build today. |

> [!NOTE]
> Version 0.2.0 waits for its release. Until then, PyPI holds 0.1.0, an earlier furb with no TUI and no examples,
> npm holds nothing, and Rust takes furb from git. crates.io will hold furb when the interpreter that furb runs on is
> there too. [`CHANGELOG.md`](https://github.com/uael/furb/blob/main/CHANGELOG.md) says when a release lands.

furb asks the models of the [claude command line](https://docs.claude.com/en/docs/claude-code) when it finds it,
then the models of each provider whose key stands in the environment, such as `ANTHROPIC_API_KEY`, then the models of
a local server. [The page of models](https://github.com/uael/furb/blob/main/docs/models.md) says how to choose one.

## A first life

The TUI runs from a clone of this repository. It needs [uv](https://docs.astral.sh/uv/), the Rust toolchain and
[Bun](https://bun.sh). The first build takes some minutes:

```sh
git clone https://github.com/uael/furb && cd furb
uv sync                              # The Python environment, which builds the engine and the command line.
bun install && bun run build         # The TypeScript package that the TUI runs on.
bun run demo                         # Watch a life that plays by itself. It asks no model and costs nothing.
bun run tui                          # Start a real life on your models.
```

In a program, a life is a few lines. This one asks for a plan, and keeps its record in `release.jsonl`:

```python
import asyncio

import furb


async def main() -> None:
  life = await furb.open("release.jsonl")
  print(await life.prompt(str, "Plan the release of version 2."))


asyncio.run(main())
```

Stop it while the model works, and run it again: it finds its prompt in the record and goes on, and no answer of
the model is asked for twice. [The examples](https://github.com/uael/furb/blob/main/docs/examples/README.md) show the same program in TypeScript and in
Rust, and four more.

## The words of furb

- A **life** is one run of furb, and its **record** is the file that holds it.
- A **chain** is a conversation with its own state. A life starts with one chain, and a word can start more.
- A **prompt** asks a model, or you, for an answer of a type, such as `str` or `int`.
- A **word** is the Python that a model writes for one step, and a **rung** runs it.
- An **act** is each thing that a word does: a read, a command, a prompt, a wait.
- The **operator** is you.

## Documentation

| To | Read |
| --- | --- |
| Use the TUI | [The TUI](https://github.com/uael/furb/blob/main/tui/README.md), and [a picture of each screen](https://github.com/uael/furb/blob/main/docs/tui.md) |
| Run a life in a window | [The desktop app](https://github.com/uael/furb/blob/main/gui/README.md) |
| Choose a model, or use a local one | [Models](https://github.com/uael/furb/blob/main/docs/models.md) |
| Use furb from a program | [The examples](https://github.com/uael/furb/blob/main/docs/examples/README.md), [Python](https://github.com/uael/furb/blob/main/bind/python/README.md), [TypeScript](https://github.com/uael/furb/blob/main/bind/typescript/README.md) |
| Use the command line | [The command line](https://github.com/uael/furb/blob/main/cli/README.md), and [its JSON-RPC](https://github.com/uael/furb/blob/main/docs/rpc.md) |
| Extend furb | [Extensions](https://github.com/uael/furb/blob/main/docs/extensions.md) |
| Check a record that someone gave you | [The audit](https://github.com/uael/furb/blob/main/docs/audit.md) |
| Change furb | [Contributing](https://github.com/uael/furb/blob/main/CONTRIBUTING.md), and [the contract](https://github.com/uael/furb/blob/main/src/furb/engine.pyi), every law of the engine |

## License

> Copyright (C) 2026 Abel Lucas. furb is free software under the GNU Affero General Public License, version 3,
> which `LICENSE` holds. You may use, study, change, and share it. Anyone who ships it, or who runs a changed furb
> as a service, must offer the source under the same terms. The engine carries no notice of its own, because its
> text is the system prompt of a model and every word of it counts.

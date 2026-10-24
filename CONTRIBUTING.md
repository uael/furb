# Contributing to furb

Thank you for your interest in furb. This guide is for each person who wants to change the repository: to fix a
defect, to add a feature, or to make a document clearer. It tells you what to install, how to set up a clone, how a
word runs, where each part lives, which gates to run, and how a pull request lands.

[`AGENTS.md`](AGENTS.md) holds the full rules of the repository: the map of its parts, the contract, the meanings of
the words, the suite, the hygiene laws, every command, and the rules of prose. This guide gives you the start. When
the two do not agree, `AGENTS.md` is right, and this guide must change.

To report a defect or to ask for a feature, open an issue with its form. A small fix needs no issue first. For a
large change, open an issue first, so we can agree on its shape before you do the work.

## What you need

| Tool | Why |
| --- | --- |
| Python 3.14 or later | The engine and the suite. |
| [uv](https://docs.astral.sh/uv/) | The Python environment, the hooks, and every Python command. |
| Rust, from `rust-toolchain.toml` | The crate at the root, which `uv sync` builds into the packages `furb-monty` and `furb-cli`. rustup reads the version from the file. |
| [bun](https://bun.sh) 1.4.2 or later | The TypeScript bindings, the TUI and the desktop preview. On Windows, bun 1.3 crashes the TUI. |
| [Git for Windows](https://gitforwindows.org), on Windows | Its sh runs the commands of a life, as /bin/sh does on Linux and on macOS. |
| The `claude` command line, signed in, the credential of a provider, or a local server that `FURB_LOCAL_URL` names | Only for a real life, such as `bun run tui`, `furb prompt` or `script/smoke.py`. The demo and the suite need no model. |

## Set up a clone

```sh
git clone https://github.com/uael/furb && cd furb
uv sync                              # The Python environment, which builds the crate and the command line.
uv run pre-commit install            # The hooks, which run the gates at each commit.
bun install && bun run build         # The TypeScript workspace, and the N-API package that the TUI uses.
bun run demo                         # The TUI on a scripted life. It asks no model.
```

The first `uv sync` builds the crate, and it takes some minutes. After that, `uv run` builds again only what a change
touches.

To work on a second branch beside this clone, make its worktree with `script/worktree.sh <path> <branch>`. It copies
the builds of the clone, so the first build of the worktree is warm: about 6 minutes, in place of about 35.

## How a word runs

1. A prompt asks a model for a response of a shape, such as `str`.
2. The model answers with Python. That answer is a word, and a rung runs it.
3. The gate reads the word first: a type checker reads it, and a sandbox runs each of its imports. A word that it
   refuses does not run, and the model reads the reason in its next turn.
4. The Kernel runs the word in the module of its chain. Each call such as `read`, `bash`, `prompt`, or `wait`
   makes an act, which goes to the ears in turn until one owns it: the ear of the act, or an ear of the World,
   which reaches the disk, the machine, the models, the operator, or the record. The owner answers the act: now, or
   later.
5. What an act does and what it comes to are facts about it. The chain folds its facts into turns, which are the
   Python that the model reads next.

## The files of each part

### The engine

Read [`src/furb/engine.pyi`](src/furb/engine.pyi), the contract, before you change the engine. A change of the
contract needs a request of the owner.

| File | What it holds |
| --- | --- |
| `src/furb/engine.pyi` | The contract: the typed surface of the engine, and the laws of each definition. Only the owner changes it. |
| `src/furb/engine.py` | The engine, one file, which is the system prompt of a model. |
| `src/furb/AGENTS.md` | The technical names of the engine, and the laws that no test can hold. |
| `DOCTRINE.md` | The doctrine of furb, which each model reads once on each chain. |
| `src/furb/__init__.py` | The switch: `FURB_ENGINE` picks the engine that `from furb import engine` gives. |
| `src/engine.rs`, `src/sand.rs` | One life of `engine.py` in the sandbox of monty, how each value crosses, and the verbs that a host says. |
| `src/snapshot.rs` | The snapshot of a life beside its record: the state that a later life on the same record, build and ears opens from with no replay, which it checks against the record. |
| `src/ear.rs`, `src/fact.rs` | The ear, the one thing a life hears by, and a fact as the engine says it. |
| `src/value.rs`, `src/wire.rs` | A value that crosses between the engine and a host, and that value as JSON. |

### The Kernel and the gate

| File | What it holds |
| --- | --- |
| `src/furb/kernel.py` | The Kernel, which runs the word of a rung in the module of its chain. Both engines load it. |
| `src/furb/sheet.py` | The sheet that the gate reads a word on, and the findings that it reads off it. |
| `src/gate.rs` | The gate: the type checker of monty, which reads a sheet. |

### The World

| File | What it holds |
| --- | --- |
| `src/world.rs` | The ears of the World that the crate writes, one for each concern. |
| `src/world/files.rs` | The files: a read and a write of a path, and of a path of `scratch://`. |
| `src/world/scratch.rs` | The scratch directory of a life: its place in the cache of furb for each record, the paths of `scratch://`, and the files that a record makes again in it. |
| `src/world/bash.rs` | The commands: each runs in a shell of the machine, in the working directory of its chain, with `TMPDIR` set to the scratch directory. |
| `src/world/time.rs` | The clock, a chance, and a wait. |
| `src/world/store.rs` | The store: the record on the disk, one fact on each line with the time the store kept it after the fact, which one process at a time owns, and the engine and the version of the build that made it, which the record names. |
| `src/world/log.rs` | The log of a life beside its record: every fact that the life said, which `logged` reads while it agrees with the record. |
| `src/checkpoint.rs` | The checkpoint of a life, at the tip of its record, from which a life of any later build goes on, with the python that makes it in the names of the engine. |
| `src/world/builds.rs` | The store of builds: the program of the command line of each version that ran on the machine, in the cache of furb, which a command on a record of that version runs. |
| `src/world/reader.rs` | The reader of a record with no life, which `retold` gives, so a record of any engine opens to read. |
| `src/world/operator.rs` | The rules by which every console reads what the operator answers. |
| `src/world/provider.rs`, `src/world/provider/` | The provider of models, which [the next table](#the-provider) gives file by file. |
| `src/life.rs` | The opening of a life, which every host shares: the record, the ears of the crate, and the extensions. |
| `src/furb/world.py` | The World of Python: the ears of the crate, and the operator at its terminal. The scripts open a life on it. |

### The provider

The provider of models is an ear of the crate, which every host stands on. Its tests stand beside each module and ask
no network.

| File | What it holds |
| --- | --- |
| `src/world/provider.rs` | The ear: the standing, each reply, the messages of a turn, what a model writes as it writes it, and the usage of a turn. |
| `src/world/provider/catalog.rs` | The catalog: the snapshot and its cache, which models it offers, the credential and the address of each, the efforts of each kind of request, the default actor, and the roster. |
| `src/world/provider/local.rs` | The local server: its address, its key, the models it lists, and the window of each. |
| `src/world/provider/catalog.json` | The snapshot of models.dev and of the data of pi-ai, one model on each line, which `script/catalog.py` makes again. |
| `src/world/provider/clients.rs` | The client of rig that asks each provider, with its credential and its address. |
| `src/world/provider/pi.rs` | The protocol of pi-ai itself, which the gateway Radius speaks. |
| `src/world/provider/claude.rs` | The claude command line, as a model of rig. |
| `src/world/provider/images.rs` | The images that a message names, each kept under the digest of its bytes. |

### The doors

| File | What it holds |
| --- | --- |
| `src/binding.rs` | The host API that both doors give, each function once, and the doors, each behind the feature that names it. |
| `src/binding/py.rs` | The door to Python, which `bind/python` makes into the package `furb-monty`. |
| `bind/python/furb_monty/engine.py` | Every name of the contract over one life of the engine in the sandbox. |
| `src/binding/ts.rs` | The door to TypeScript through N-API, which also makes the declarations of `bind/typescript`. |
| `src/binding/ts/host.rs` | The door on the thread of JavaScript: a value as the engine takes it, and a generator heard as an ear. |
| `bind/typescript/src/life/` | The model of a life, which every host reads, with no native code. |
| `src/furb/life.py`, `bind/typescript/src/framework.ts`, `src/life/host.rs` | A life as a program holds it, with the same names in each door. Change the three together. |
| `docs/examples/` | Five examples for each door, which a test of that door runs. Change an example with its door. |

### The extensions

An official extension is a word, a contract and a suite in `extensions/<name>/`, and an ear of the World in
`src/extension/<name>.rs`, which answers the questions of its word from the disk. [The guide of the
extensions](docs/extensions.md) says which hygiene laws hold it and how a life runs it. The crate carries the manifest
and the word, so a change of either needs `uv sync --reinstall-package furb-monty --reinstall-package furb-cli`.

### The command line

`furb` is the program of the package `furb-cli` in `cli/`.

| File | What it holds |
| --- | --- |
| `cli/src/main.rs` | The words of the command line, on clap, and the commands `prompt`, `turns`, `run`, `extensions` and `facts`. |
| `cli/src/life.rs` | One life on the opening of the crate: the prompt of the record it takes up, and how the operator awaits an act. |
| `cli/src/console.rs` | The console of the terminal, which shows each prompt to the operator and reads the line back by the rules of the crate. |
| `cli/src/rpc.rs` | The JSON-RPC of `--mode rpc`, which [`docs/rpc.md`](docs/rpc.md) says. Change the two together. |
| `cli/src/tui.rs` | How furb finds the TUI and hands the terminal to it. |
| `cli/tests/furb.rs` | The tests, which run `furb` as a process, on a claude command line that answers from a script. |

### The TUI

| File | What it holds |
| --- | --- |
| `tui/src/app.ts` | The screen: the top line, the feed, the sidebar, the composer, the footer, and the dialogs. |
| `tui/src/session.ts` | The state of one session: its chains, its views, its drafts, its queue, and its record of the UI. |
| `tui/src/worker.ts` | The session, in a worker thread, and the demo session with its scripted answers. |
| `tui/src/theme.ts` | The palettes, the marks, and the spacing. One mark says one thing everywhere. |
| `tui/src/keys.ts`, `tui/src/commands.ts` | The keys and the slash commands, which the keymap, the footer, the help and the docs read. |
| `tui/script/` | The gallery, the animation, and the rasterizer that turns a frame into a PNG. |

A screen must read clearly at first sight. After a change that a screen shows, run `bun run screenshots` and read each
capture. With `FURB_GALLERY_ONLY` set to a pattern, such as `^0[1-5]-`, it writes only the captures that match.

### The desktop preview

`gui/` is the desktop preview on Tauri 2, which reads records and runs no life. [`gui/DESIGN.md`](gui/DESIGN.md)
says its design. Its core in `gui/src-tauri/` links the crate `furb`, and its window is in `gui/src/`.

### The suite

`AGENTS.md` gives the rules of the suite: one test file for each definition of the contract, and one test for each
sentence.

| File | What it holds |
| --- | --- |
| `test/conftest.py` | The harness: the World in memory, the Kernel of this interpreter, and each helper that several files of `test/` need. |
| `test/test_<name>.py` | The tests of one definition of the contract, one test for each sentence. |
| `test/test_hygiene.py` | The hygiene laws. |
| `test/outside/` | The tests of the World, the Kernel, the door, and the switch, which stand outside the hygiene laws. |
| `extensions/conftest.py` | The hooks of the harness for the suite of each extension, and the helpers that only those suites need. |

## The gates

Run the gates that your change touches before you push. The hooks run most of them at each commit.

| Command | What it checks |
| --- | --- |
| `uv run pre-commit run --all-files` | Every hook: the format and the lint, the type check, the suite, and the gates of the crate. |
| `uv run pytest -q` | The suite on both engines, with a coverage that must be whole. |
| `uv run pytest -q test/test_hygiene.py` | The hygiene laws alone, after a change of the engine, a contract or a suite. |
| `uv run ruff format src test script extensions` and `uv run ruff check src test script extensions` | The format and the lint of Python: two spaces of indentation, 120 columns. |
| `uv run ty check --error-on-warning` | The type check. |
| `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` | The gates of the crate and of the command line. |
| `bun run check`, `bun run lint` and `bun run test` | The type check, the lint and the tests of the TypeScript side. |
| `bun run bench` | The speed of the TUI against its budget, after a change of the TUI. |

Each pull request runs the gates of `.github/workflows/gates.yml` on Linux, on macOS and on Windows.

## The prose

Write all prose in Simplified Technical English, per [ASD-STE100](https://www.asd-ste100.org/): short sentences, one
meaning for each word, and no em dash or en dash. This holds for a document, a docstring, a comment, a commit and a
pull request.

## How a pull request lands

1. Make a branch from `main`, do the one change, and run the gates that it touches.
2. Open the pull request. Its template, [`.github/pull_request_template.md`](.github/pull_request_template.md), holds
   every rule of a pull request: its scope, its title and its body. Read it whole before you write. The workers that
   the conductor of this project hires write each body by this template too.
3. The gates and the check of the title run on the pull request. A review blocks it only for a defect that a command
   shows, or for a sentence of a contract that is false.
4. When the checks are green and the review agrees, the pull request merges as one squash commit on `main`, with its
   title as the message.
5. release-please keeps a pull request of the next release open on `main`, and writes [`CHANGELOG.md`](CHANGELOG.md)
   from the titles. When that pull request merges, it tags the release, and `.github/workflows/publish.yml` publishes
   each package at that version: on npm and on PyPI, and on crates.io once no crate of furb stands on git, which is
   when monty stands there.

## Where to read next

- [`AGENTS.md`](AGENTS.md): every rule and every command of the repository.
- [`src/furb/AGENTS.md`](src/furb/AGENTS.md): the technical names of the engine.
- [`tui/README.md`](tui/README.md) and [`docs/tui.md`](docs/tui.md): what the TUI does, and a picture of each screen.
- [`bind/typescript/README.md`](bind/typescript/README.md): the TypeScript API.
- [`docs/extensions.md`](docs/extensions.md): how an extension works.

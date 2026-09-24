# Developer guide

This guide is for a person who wants to change furb. It starts with what you need on your machine, then gives a
map of the repository, then the tasks that you do each day. `CLAUDE.md` holds the same rules in the short form
that an agent reads. When the two differ, `CLAUDE.md` is right, and this guide must change.

## The idea in two minutes

furb gives a model a Python module. Each answer of the model is a short program, which furb calls a word. The
engine runs the word in the module of a chain. The effects of the word are acts: a read, a command, a question to
a model, or a wait. The engine tells the model what each act did, as Python that the model reads in its next turn.

The whole engine is one file, `src/furb/engine.py`. Minified, it is the system prompt of the model, so each word of
it has a cost. The contract, `src/furb/engine.pyi`, says what the engine must do, one sentence per line, and the
suite in `test/` has one test for each sentence. You change the contract only with the approval of the owner.

## What you need

| Tool | Why |
| --- | --- |
| Python 3.14 or later | The engine and the suite. |
| [uv](https://docs.astral.sh/uv/) | The Python environment, the hooks, and every Python command below. |
| Rust, from `rust-toolchain.toml` | The crate at the root, which `uv sync` builds into the package `furb-monty`. rustup reads the version from the file. |
| [bun](https://bun.sh) 1.4.2 or later | The TypeScript bindings and the TUI. On Windows, bun 1.3 crashes the TUI. |
| git, and Node.js 22 with npm | The tests of the crate fetch an extension from a local git remote and pack one with npm. |
| The `claude` command line, signed in | Only for a real life, such as `bun run tui` or `script/smoke.py`. The demo and the suite need no model. |

Then, from the root of a clone:

```sh
uv sync                          # The Python environment, with the crate built into furb-monty.
uv run pre-commit install        # The hooks, which run at each commit.
bun install && bun run build     # The TypeScript workspace and the N-API package.
```

## A map of the repository

| Path | What it is |
| --- | --- |
| `src/furb/engine.py` | The engine, one file. It depends only on the Python interpreter. |
| `src/furb/engine.pyi` | The contract. The docstring of each definition holds its laws. |
| `src/furb/CLAUDE.md` | The technical names of the engine, and the laws that no test can hold. |
| `src/furb/builtin/` | The builtin extensions `files`, `bash` and `grant`: each a module, its contract, and its names in `CLAUDE.md`. |
| `extensions/` | The official extensions, such as `skills`, each with its manifest, its contract, its parts and its suite. |
| `src/extension.rs` | The extension API of the crate: the configs, the cache, the fetch, the manifests, the order and the words. |
| `test/` | The suite: one file for each definition, and one test for each sentence; `test/<name>/` for each builtin. |
| `test/outside/` | The tests of the World, the Kernel, the command line, and the provider. |
| `src/*.rs` | The crate `furb`, which runs the same engine in monty, a Python interpreter written in Rust. |
| `bind/python` | The package `furb-monty`, the crate for Python. |
| `bind/typescript` | The crate for TypeScript through N-API, with a World for models, files, commands, and records. |
| `tui/` | The terminal application, built on OpenTUI. |
| `script/` | The smoke run, the play run, and the DeepSWE rig. `script/CLAUDE.md` says how to run the rig. |
| `docs/` | The gallery of the TUI, its animation, the guide of the extensions, and this guide. |

## How a word runs

1. A prompt asks a model for a response of a shape, such as `str`.
2. The model answers with Python. That answer is a word, and a rung runs it.
3. The gate, a type checker, reads the word first. A word that it refuses does not run, and the model reads the
   reason in its next turn.
4. The Kernel runs the word in the module of its chain. Each call such as `prompt` or `wait` makes an act, which
   the World serves: the models, a wait, and the record. `read` and `bash` come from the builtin extensions, whose
   words each chain plays first, and whose parts in the World serve the disk and the machine.
5. Each act says what it did as a fact. The chain folds its facts into turns, which are the Python that the model
   reads next.

The TUI shows the same life in three views. The feed shows the facts as cards. The transcript is the exact text of
the turns. The changes are the files that the life wrote.

## Everyday tasks

### The engine and the crate

```sh
uv run pytest -q                                     # The suite on both engines, with coverage.
uv run pytest -q test/test_hygiene.py                # The hygiene laws alone.
uv run pytest -q test/bash/test_bash.py -k "timeout" # One file, or some tests of it.
uv run ruff format src test script extensions && uv run ruff check src test script extensions
uv run ty check --error-on-warning                   # The type check, against the contracts.
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
uv sync --reinstall-package furb-monty               # Build the crate again after a change of it.
```

### The TypeScript side and the TUI

```sh
bun run check && bun run lint                        # The type check and the lint.
bun test bind/typescript/test tui/test extensions    # The tests.
bun run demo                                         # The TUI on a sample project, with no model.
bun run tui                                          # The TUI on a real life.
bun run docs                                         # The tables of keys and commands of the READMEs.
bun run screenshots && bun run animation             # docs/screenshots/ and docs/furb.gif, from the real renderer.
```

If you run the tests as root, the test "the project files leave out a folder that cannot be read" fails, because
root can read every folder. It passes as a normal user.

### Before you push

```sh
uv run pre-commit run --all-files
```

This runs every gate of the commit hook. CI runs the same hooks, the type check, the suite on Linux and on macOS,
and the TypeScript gates on Linux, on macOS, and on Windows.

## Changing the engine

1. Read `src/furb/engine.pyi` whole, and `src/furb/CLAUDE.md`.
2. If the change needs a new law, or a change of a law, ask the owner first. The contract changes only with the
   owner's approval.
3. Write the test first. Its docstring is exactly one sentence of the contract, and it lives in the file of that
   definition, `test/test_<name>.py`.
4. Change `engine.py`, and keep it small. Minified, it must cost fewer than 6000 tokens, and no name may be bound
   again under a scope that already binds it. `test/test_hygiene.py` checks both.
5. Run the suite. It runs on CPython and on monty, and both must pass.

## Changing or writing an extension

An extension adds verbs to the life, and parts to the World and to the TUI. [The guide of the
extensions](extensions.md) says how to write one, how a config names it, and how to publish it.

1. Read the contract of the extension, `src/furb/builtin/<name>.pyi` or `extensions/<name>/<name>.pyi`, and its
   names in `src/furb/builtin/CLAUDE.md`.
2. Write the test first, in `test/<name>/` for a builtin or in `extensions/<name>/test/`. Its docstring is exactly
   one sentence of the contract, and the harness runs it on both engines.
3. Change the python part. It is a module that imports from `furb.engine` and from the extensions it requires; the
   crate cuts those imports out when it makes the word. The hygiene laws hold its word as they hold the engine.
4. Change the parts for a World and for the TUI, and their bun tests. A part imports only types from
   `@furb/engine`.

## Changing the TUI

| File | What it holds |
| --- | --- |
| `tui/src/app.ts` | The screen: the top line, the feed, the sidebar, the composer, the footer, and the dialogs. |
| `tui/src/session.ts` | The state of one session: its chains, its views, its drafts, its queue, and its record of the UI. |
| `tui/src/worker.ts` | The engine and its World, in a worker thread, and the demo World with its scripted answers. |
| `tui/src/theme.ts` | The palettes, the marks, and the spacing. One mark says one thing everywhere. |
| `tui/src/keys.ts`, `tui/src/commands.ts` | The keys and the slash commands, which the help and the docs read. |
| `tui/script/` | The gallery, the animation, and the rasterizer that turns a frame into a PNG. |

After a change that a screen shows, run `bun run screenshots` and `bun run animation`, then look at each image
that changed. A screen must read clearly at first sight. `bun run screenshots` with `FURB_GALLERY_ONLY` set to a
pattern, such as `^0[1-5]-`, writes only the captures that match.

## Writing prose

Every text in the repository uses Simplified Technical English (ASD-STE100): short sentences, one meaning for each
word, and the active voice. This covers comments, docstrings, commit messages, and documents. Do not use the em
dash or the en dash.

## Where to read next

- `CLAUDE.md`: the rules in short form, the meanings of the words, and the hygiene laws.
- `src/furb/CLAUDE.md`: the technical names of the engine.
- `tui/README.md` and `docs/tui.md`: what the TUI does, and a picture of each screen.
- `bind/typescript/README.md`: the TypeScript API.
- `docs/extensions.md`: the extensions, their configs, their parts, and how to write one.

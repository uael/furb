# Developer guide

This guide is for a person who wants to change furb. [`CLAUDE.md`](../CLAUDE.md) holds the rules of the repository:
the map of its parts, the contract, the meanings of the words, the suite, the hygiene laws, every command, and the
rules of prose. This guide adds what a person needs besides: the tools to install, how a word runs, and the files of
the TUI. When the two differ, `CLAUDE.md` is right, and this guide must change.

## What you need

| Tool | Why |
| --- | --- |
| Python 3.14 or later | The engine and the suite. |
| [uv](https://docs.astral.sh/uv/) | The Python environment, the hooks, and every Python command. |
| Rust, from `rust-toolchain.toml` | The crate at the root, which `uv sync` builds into the package `furb-monty`. rustup reads the version from the file. |
| [bun](https://bun.sh) 1.4.2 or later | The TypeScript bindings and the TUI. On Windows, bun 1.3 crashes the TUI. |
| The `claude` command line, signed in | Only for a real life, such as `bun run tui` or `script/smoke.py`. The demo and the suite need no model. |

Then set up a clone as the [README](../README.md#try-it) says, and install the hooks, which run at each commit:

```sh
uv run pre-commit install
```

## How a word runs

1. A prompt asks a model for a response of a shape, such as `str`.
2. The model answers with Python. That answer is a word, and a rung runs it.
3. The gate, a type checker, reads the word first. A word that it refuses does not run, and the model reads the
   reason in its next turn.
4. The Kernel runs the word in the module of its chain. Each call such as `read`, `bash`, `prompt`, or `wait`
   makes an act, which goes to the ears in turn until one owns it: the ear of the act, or an ear of the World,
   which reaches the disk, the machine, the models, the operator, or the record. The owner answers the act: now, or
   later.
5. What an act does and what it comes to are facts about it. The chain folds its facts into turns, which are the
   Python that the model reads next.

## Changing the TUI

| File | What it holds |
| --- | --- |
| `tui/src/app.ts` | The screen: the top line, the feed, the sidebar, the composer, the footer, and the dialogs. |
| `tui/src/session.ts` | The state of one session: its chains, its views, its drafts, its queue, and its record of the UI. |
| `tui/src/worker.ts` | The session, in a worker thread, and the demo session with its scripted answers. |
| `tui/src/theme.ts` | The palettes, the marks, and the spacing. One mark says one thing everywhere. |
| `tui/src/keys.ts`, `tui/src/commands.ts` | The keys and the slash commands, which the help and the docs read. |
| `tui/script/` | The gallery, the animation, and the rasterizer that turns a frame into a PNG. |

A screen must read clearly at first sight. `bun run screenshots` with `FURB_GALLERY_ONLY` set to a pattern, such as
`^0[1-5]-`, writes only the captures that match.

## Where to read next

- `src/furb/CLAUDE.md`: the technical names of the engine.
- `tui/README.md` and `docs/tui.md`: what the TUI does, and a picture of each screen.
- `bind/typescript/README.md`: the TypeScript API.

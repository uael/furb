---
name: furb-develop
description: "Work on furb from a session of furb: build the window and the daemon, reload them and the skills of the session on the build, and check the window. Use it when a chain changes the code of furb itself."
---

# Work on furb from furb

A chain that changes furb works in a worktree of the clone, never in the checkout that the window of the owner
runs. The `AGENTS.md` of the tree says each command and each gate, and the skill `rust` holds before a change of a
Rust file.

## Build

Build the programs that the window and its daemon run, at the lowest priority and with few jobs, since the window
of the owner runs on the same machine:

```sh
nice -n 19 cargo build -j 4 -p furb-cli -p furb-window
```

A change of the engine, of a kit or of the crate reaches the Python suite at the next `uv run`, which builds the
package again.

Never run `cargo clean -p furb`, `cargo clean -p furb-cli` or `cargo clean` in a tree whose window the owner uses:
it removes `target/debug/furb`, and the window can then neither reload nor start again. When a clean is needed,
build both programs again in the same command, as `cargo clean -p furb && nice -n 19 cargo build -j 4 -p furb-cli
-p furb-window`.

## Reload

The window and its daemon follow the build of their programs. To take a build at once, and the skills and the MCP
servers of a session again, press F5, or ⌘R where the terminal speaks the keys of Kitty, or send `/reload`. A model
does the same with one word on its chain:

```python
said = await reload()
```

`reload()` loads again each skill with code that the chain runs, so a word that changed on the disk plays at once
on each chain, awaits `mcp_servers()` when the chain runs the kit mcp, and then asks the host to move furb to its
newest build. Its answer says whether a new build takes the place of the daemon.

The daemon follows its own program: once a build replaced it, it starts that build in its place, and a window
starts none. The handover waits only for the commands that a life ran at the ask, and for none of the life that
asked the build; a command that starts later ends as interrupted, and its model runs it again. A program that asks
while one waits takes its place, so one handover runs at a time and the newest build alone moves in. Each life opens again at its tip, then crosses to the engine of the new build before
any word runs, as `AGENTS.md` says: each chain reads the lines of the engine that differ. The window keeps its
thread, its place and its draft, and a notice names the build that it runs.

## Check the window

Run the tests of the change alone, never the whole suite:

```sh
nice -n 19 cargo nextest run -j 4 -p furb-window --test suite <name>
```

A change of what a screen shows changes its capture in `crates/window/tests/screens`. Run the test again with
`INSTA_UPDATE=always`, read the diff of each `.snap`, and delete the line `assertion_line` that insta adds. To see
the real window in a terminal at each size, run `furb-frames capture` and read the frames in `target/frames`.

## See what the owner sees

At each message that he sends, his window writes what he saw beside its cards, in the cache of furb:
`~/Library/Caches/furb/window/seen-<pid>.json` on macOS, `~/.cache/furb/window/` on Linux, one file for each
window. It holds the size of the window and where it stood: the thread, the agent of it that he read, the place of
the feed, the draft and the moment of the time that he held. When he says that something does not work, draw it
again as he saw it, on the build of this tree:

```sh
target/debug/furb-frames render --seen ~/Library/Caches/furb/window/seen-<pid>.json --out "$TMPDIR/seen"
```

Read `frame.txt` of the folder that it prints, then `frame.json`, the cells with their colors, and `frame.png`
when the look matters. It opens a copy of his record that only inspects it, so nothing of his session moves.

## Render a session at a moment on a build

```sh
target/debug/furb-frames render <record> --at <act|millisecond|count of facts> [--thread <prompt>] [--size 200x60]
```

It copies the record up to the fact of that moment, opens the copy in a life that only inspects it, with no model
and no network, and draws the window of this build headless on the thread of that act, or on `--thread`. A record
of a long life replays for minutes, and the command says each ten seconds that it waits. To draw it on another
version, make a tree at that commit with `cargo xtask tree new <path> <branch> --base <commit>`, build its
`furb-frames`, and run that one.

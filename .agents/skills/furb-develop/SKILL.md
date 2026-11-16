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

When a new build takes the place, the daemon waits until no life runs a command, writes the image of each life and
gives it to the new daemon, so no command is cut. A program that asks while one waits takes its place, so the
newest build alone moves in. Each life opens again at its tip, then crosses to the engine of the new build before
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

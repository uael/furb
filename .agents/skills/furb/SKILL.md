---
name: furb
description: What furb is and how to drive it from its command line, with a short introduction to kits. Use it when a chain works with furb itself; furb-kit says how to make a kit, and furb-develop how to work on the code of furb.
---

# furb

furb is a harness where a model writes python: each reply is a word that runs in a sandbox, where a call of `prompt`
is the one step that is not code. A session is a life on a record, which keeps each act, so a later life opens it at its
tip and runs each word again.

## The command line

`furb` with no command opens the window on the session of the folder. Each command opens one life on one record:

- `furb prompt <message> [--record <file>] [--to <actor>] [--cwd <folder>]` prompts an actor on the root chain and
  prints what it gave. When the daemon of the machine holds the life of the record, the message goes through it, as a
  window says it.
- `furb run <word> [--record <file>]` runs a word on the root chain and prints what it gave.
- `furb turns <record>` prints the turns of the root, `furb facts <record>` each fact of a record, and
  `furb skills` each skill that a life on the record runs.
- `furb lives` lists each life that the daemon holds, `furb end` ends one, and `furb host` serves the daemon.
- `furb audit <record> <sha256>` replays a record with no model and no network, and checks each act against it.

`furb <command> --help` says each option.

## Kits

A skill tells a model what to do. A kit is a skill with code: its word defines functions that each chain runs, so a
habit becomes a function with control flow and `prompt` inside it, as a review, a loop that converges, or a list of tasks
with a view in the window. A kit can live in the record of one session, in the project, or in furb, and it can have a
contract, a suite, a process for what the sandbox cannot do, and views that each window draws.

- `skill("furb-kit")` loads `extend()` and says how to make a kit.
- `skill("furb-develop")` says how to build, reload and check furb from a session of furb.

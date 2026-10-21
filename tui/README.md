# furb TUI

The TUI shows a life of furb in two variants, which `v` switches and `--variant loom` or `--variant graph` chooses
at the start. The loom shows each chain as a row, in the order of the tree of chains, with time from left to right.
The graph shows the life as programs that run: the tree of its threads beside a map of its chains, with the messages
that flow between them, and levels of zoom down to the source of one program and to one act, as a debugger does.
You see at once which chains work, which wait for you, which stopped at a ceiling, and which failed. [The design](DESIGN.md) says why the TUI is made so, and [the gallery](../docs/tui.md)
shows each screen.

![The furb TUI in action](../docs/furb.gif)

## What the screen shows

- **The loom**, at the top. The first row is you, the operator: its cells show the prompts that you owe. Then each
  chain, under the chain whose word made it, so a conductor stands with its workers under it. A row gives the state
  of the chain, its label, its cells, the act that it waits on, its dollars against the ceiling of its grant, and the
  share of the window that its last answer read.
- **The cells** of a row show what the chain did at each moment, where a moment is the start or the end of an act.
  The lowest cell, `▁`, is work that runs: a command, a wait, or a model that writes its answer. A taller cell holds
  more acts that began or ended. A cell is yellow where the chain asks you, and red where an act failed. The newest
  moments take a column each, and older moments share a column, so a long life fits in the terminal. `older ┊` marks
  where the folded past starts, and `now` is the right edge.
- **The thread**, in the middle: the chain that you select, as its ladders. Each prompt is a line, each rung of its
  ladder is a line under it with the first line of its word, and the acts that the rung made follow by their names.
  A command that runs shows the end of what it prints, and a model shows what it writes.
- **The lens**: Enter on a line shows its act whole. A rung shows its word as python, a command its command and all
  that it printed, a prompt its message and its value, and a read or a write its text.
- **The time cursor**: `[` and `]` move back and forward in time by one moment, and `{` and `}` by ten. The loom and
  the thread show the life as it stood at the cursor, and the right edge of the loom says `then`. In the past the
  verbs that change the life do nothing, and `b` branches the selected chain there. Esc returns to now.
- **The graph**, which `v` shows. A thread is a prompt, and a prompt that a rung of a thread makes is its
  sub-thread. The threads stand in a sidebar, each with its state, its progress and what waits for you in it, and the
  map of the selected thread stands beside them: each chain as its mark and its name, the lines of the tree of hires,
  and a dot that walks along a line while a message flows. Enter zooms in: from a thread to its map, from a program
  to its source, where each line holds what its acts came to and the line that runs is marked, from a line to its
  word whole, and from an act to its lens. Esc zooms out. `n` steps a chain: it runs one more rung, then pauses.
- **The word line**, at the bottom. `i` asks the model of the selected chain, `:` writes a word of python that runs
  as a rung there, and `/` types a command. Tab answers the oldest prompt that you owe, on any chain.

The keys are the verbs of the engine, and each key acts on what you select: a chain in the loom, or an act in the
thread and the lens. The same space pauses one command and a conductor with its five workers.

<!-- keys:start -->

| Key | Action |
| --- | --- |
| ↑ ↓ or k j | Select a thread of the loom, a line of the thread, or scroll the lens |
| ← → or h l | Fold the workers of a thread into it, or show them again |
| Enter | Open the selection: a thread, a lens of an act, or the answer of a prompt that you owe |
| Esc | Go back one level |
| Tab | Answer the oldest prompt that you owe, on any chain |
| i | Ask the model of the selected chain |
| : | Write a word of python, which runs as a rung on the selected chain |
| / | Type a command, as /model, /theme or /fork |
| space | Pause the selection, or wake it |
| x | Cancel the selection, or the work of the selected chain |
| c | Close the selected act with a value |
| g | Put a ceiling of dollars on the selected chain |
| e | Edit the door of the prompt of the selection, which replays its ladder |
| b | Branch the chain after the selected act, or at the moment of the time cursor |
| [ and ] | Move the time cursor back and forward by a moment, or by ten with { and }. Esc returns to now |
| n | Step: the selected chain runs one more rung, then a pause holds it |
| v | Change the variant: the loom of the chains in time, or the graph of the programs that run |
| w | Wake the work that an earlier life left |
| ⌃O | Open the list of lives, by workspace |
| ? | Show or hide these keys |
| ⌃Q, or ⌃D twice | Save and quit |

<!-- keys:end -->

<!-- commands:start -->

| Command | Action |
| --- | --- |
| `/new` | Start a fresh life |
| `/model [model]` | Choose the model that answers on this chain |
| `/effort [level]` | Set the reasoning effort of the selected model |
| `/shape [type]` | Choose the result type of the next prompt |
| `/theme [name]` | Change the palette of every surface |
| `/chain <label>` | A conversation with its own state |
| `/fork <label>` | Copy this chain's current transcript and program |
| `/undo` | Remove the last user message from a new transcript branch |
| `/redo` | Return to the branch before undo |
| `/queue [text]` | Send after current work, or manage queued messages |
| `/run <python>` | Write a rung through the gate |
| `/bash <command>` | Stream a shell command |
| `/read <path>` | Show a file to this chain |
| `/cd <path>` | Change this chain's working directory |
| `/edit [prompt id]` | Change a prompt's program and replay it |
| `/grant <dollars>` | Pause at a dollar ceiling |
| `/context <fraction>` | Pause at a share of the model window |
| `/pause [act or chain]` | Hold delivery while work completes |
| `/wake [act or chain]` | Deliver pending work |
| `/cancel [id]` | End an act or the selected chain's work |
| `/feed <id> [text]` | Send a line of input to a command; no text closes its input |
| `/close <id> <json>` | Close an act with a JSON value |
| `/name <name>` | Set a name in the session picker |
| `/share [path]` | Export a standalone HTML conversation |
| `/export <path>` | Write the transcript and program to a new JSON file |
| `/extensions` | List the extensions that this life runs |
| `/exit` | Save every session and quit |

<!-- commands:end -->

Run `bun run docs` after a change of `tui/src/keys.ts` or `tui/src/commands.ts`, which writes these tables again.

## Install and run

The TUI is a program of bun on `@furb/engine`, the crate of furb through N-API. You build the crate once from a
checkout. The TUI needs no python and no uv: only `furb` itself and the suite of the engine need them.

### macOS

1. Install the command line tools of Xcode, which give the linker: `xcode-select --install`.
2. Install rust with rustup: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`. The file
   `rust-toolchain.toml` of the checkout chooses the version, and rustup installs it at the first build.
3. Install bun 1.4.2 or later: `curl -fsSL https://bun.sh/install | bash`.

### Linux

1. Install a C compiler and git, as `sudo apt install build-essential git` on Debian and Ubuntu, or
   `sudo dnf install gcc git` on Fedora.
2. Install rust with rustup: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`.
3. Install bun 1.4.2 or later: `curl -fsSL https://bun.sh/install | bash`.

### Windows

1. Install the Build Tools for Visual Studio with the part "Desktop development with C++", which give the linker
   of rust: `winget install Microsoft.VisualStudio.2022.BuildTools`, then add that part in the installer.
2. Install rust with rustup: `winget install Rustlang.Rustup`.
3. Install git, which gives the `sh` that furb runs commands in: `winget install Git.Git`.
4. Install bun 1.4.2 or later: `powershell -c "irm bun.sh/install.ps1 | iex"`. Bun 1.3 crashes the TUI on Windows
   when it calls into OpenTUI.

### Build and run from a checkout

The same commands on each system, from the root of the checkout:

```sh
git clone https://github.com/uael/furb.git
cd furb
bun install
bun run build                # The crate through N-API, in the dev profile. The first build takes minutes.
bun run demo                 # A scripted life of many threads at once. It asks no model.
bun run tui                  # A real life, on the models of the catalog of the crate.
bun run tui -- --resume .furb/sessions/example.jsonl
bun run tui -- --help        # The options: --cwd, --model, --effort, --record, --resume, --roster, --variant.
```

`bun run tui` asks the claude command line when it finds one, else a model of a provider whose credential stands in
the environment, such as `ANTHROPIC_API_KEY`, else a model of the local server that `FURB_LOCAL_URL` names. A real
life can cost dollars. The demo costs nothing.

### Run it from `pip install furb`

`pip install furb` gives the command `furb`, which needs python 3.14 or later. With no command, `furb` hands the
terminal to the TUI. It runs the program that `FURB_TUI` names, else the `furb-tui` on PATH, else the TUI of the
checkout that it was built from. The wheel holds no TUI, so put the TUI of a built checkout on PATH as `furb-tui`:

- On macOS and on Linux, from the root of the checkout:

  ```sh
  mkdir -p ~/.local/bin
  printf '#!/bin/sh\nexec bun "%s/tui/src/cli.ts" "$@"\n' "$PWD" > ~/.local/bin/furb-tui
  chmod +x ~/.local/bin/furb-tui
  furb --demo
  ```

- On Windows, in PowerShell, from the root of the checkout, with a folder on PATH such as `$HOME\bin`:

  ```powershell
  Set-Content "$HOME\bin\furb-tui.cmd" "@bun `"$PWD\tui\src\cli.ts`" %*"
  furb --demo
  ```

`furb` gives the TUI the options that it takes itself: `--demo`, `--record`, `--cwd`, `--model` and each `--roster`,
and the words after `--`.

### When it does not start

- `bun --version` must say 1.4.2 or later. An older bun fails to load OpenTUI, and on Windows it crashes.
- `Cannot find module '@furb/engine'` or a missing `furb.node`: run `bun run build` again. Each change of the crate
  needs a new build, and `git pull` can bring one.
- A build that fails at the link: install the linker of the system, as the steps above say, and open a new terminal
  so that PATH holds rust.
- `no TUI is found` from `furb`: set `FURB_TUI`, or put `furb-tui` on PATH, as above.
- No model answers in `bun run tui`: sign in to the claude command line, set the credential of a provider, or name a
  local server in `FURB_LOCAL_URL`. `bun run demo` needs none of them.
- Keys with Shift, as Shift+Enter, reach the TUI only from a terminal with the kitty keyboard protocol. Ctrl+J adds
  a line in every terminal.
- A screen that draws badly: use a terminal with true color and a font with the block characters, such as Ghostty,
  kitty, WezTerm, iTerm2 or Windows Terminal.

The gates of CI run `bun install`, `bun run build`, `bun run check`, `bun run lint`, `bun run test` and
`bun run tui -- --help` on Linux, macOS and Windows. No gate runs `pip install furb` with the TUI, or the TUI in a
real terminal, since a runner has no terminal and the wheel is published only from a tag.

## Lives and files

A life is a session on its record. The TUI keeps each life under `.furb/sessions` of its directory, and the `.furb`
that it makes holds a `.gitignore` that keeps it out of version control. Keep the `.jsonl`, `.session.json`,
`.changes.jsonl` and `.ui.json` files of a life together, and its `.images` directory when it has one. A life that
holds work from before opens with that work held, and `w` wakes it. ⌃O lists the lives of each workspace with their
state and their cost, and `n` there starts a new life.

The theme is a preference of the user, in `ui.json` of the config directory: `$XDG_CONFIG_HOME/furb`, `~/.config/furb`,
or `%APPDATA%\furb` on Windows, and `FURB_CONFIG_DIR` names another one. `/theme dusk`, `day`, `github`, `paper`,
`midnight` or `forest` changes it. Dusk is the default, and Day is its light theme. The tests and the gallery use
preferences of their own.

A life runs the extensions that the configs turn on, as [the guide of the extensions](../docs/extensions.md) says.
The rungs of their words stand in the thread of each chain, as every rung does. `/extensions` lists what the life runs.

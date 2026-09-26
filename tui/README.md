# furb TUI

An OpenTUI application built on `@furb/engine`. The TUI owns a Bun worker for each open session: its life and its
ears.
Rendering, input, and syntax coloring stay responsive while the sandbox gates and runs Python. The TUI needs Bun
1.4.2 or later. On Windows, Bun 1.3 crashes the TUI when it calls into OpenTUI.

```sh
uv sync
bun install
bun run build
bun run demo                 # A local, scripted life. No model request.
bun run tui                  # Claude CLI through pi-ai.
bun run tui -- --resume .furb/sessions/example.jsonl
```

Use `--cwd`, `--model provider:model`, `--effort`, `--record`, and repeated `--roster` options to configure
a new life. Sessions are saved under `.furb/sessions` in the selected directory. The `.furb` that the TUI makes holds
a `.gitignore` that keeps it out of version control. Keep the `.jsonl`, `.session.json`, `.changes.jsonl`, and
`.ui.json` files together. The last file keeps the view of the session as you left it. Keep its `.images` directory
too when the session has image attachments. An unfinished session opens paused and offers a resume choice. The
native engine replays completed work from the record.
⌃W lists the workspaces and their sessions, each with its state, save time, cost, and record size. It reads
unfinished work through native replay without locking or changing the saved record. The record shows what remains
unfinished. The list shows at once, and each saved session shows its state when its replay ends.

![The furb TUI in action](../docs/furb.gif)

[The gallery](../docs/tui.md) shows each screen of the TUI and what it does.

`/workspace path` adds a folder, and `/new` starts a session in the current workspace. `/delete` moves a chosen
session and its companions to `.furb/trash`. The trash holds a `restore.json` with the original paths, and its
record can be opened with `--resume`. Quit saves and closes all open sessions, and SIGINT, SIGTERM, SIGHUP, and
SIGQUIT quit the same way. On Windows, a console that closes and Ctrl+Break quit the same way. `sidebarWidth` in the
preferences file sets the width of the sidebar, from 26 to 48 columns. On a terminal narrower than 100 columns the
sidebar is hidden, and `/workspace` keeps all sessions available from the keyboard.

Python words have offline Tree-sitter colors. GitHub Dark is the default. Sessions share the theme, and the TUI saves
it in `$XDG_CONFIG_HOME/furb/ui.json` (or `~/.config/furb/ui.json`, and `%APPDATA%\furb\ui.json` on Windows).
`FURB_CONFIG_DIR` selects another configuration directory. Tests and screenshot generation use isolated
preferences; the CLI shares the user's choice across sessions, including demos.

The mouse reaches every control: a click is a press and a release on one control, and a drag selects text instead.
The text that a drag selects goes to the clipboard through OSC 52 when the drag ends, where the terminal supports
it, and ⌃Y copies it again. The wheel scrolls the feed, the sidebar, and every list.

A chord before "or" reaches the TUI only from a terminal with the kitty keyboard protocol. The chord after "or"
reaches it from every terminal. F1 lists the chords that the terminal in use sends.

<!-- keys:start -->

| Key | Action |
| --- | --- |
| Enter | Send a message, or run Python input |
| ⇧Enter or ⌃J | Insert a new line |
| ⌥Enter | Queue this message after current work |
| ↑ / ↓ | Previous / next sent input, from the first or the last line |
| ↑ in an empty input | Take the last queued message back to edit it |
| ⌃S | Put the input aside, or bring it back |
| Esc twice | Rewind: branch from an earlier point of the chain |
| ⌃1-3 or ⌥1-3 | Feed / transcript / changes view |
| ⌃Tab / ⇧⌃Tab | Next / previous chain |
| ⌃P | Search all actions |
| ⌃B / ⌃N | Switch / create a chain |
| ⌃M or ⌥M | Choose a model |
| ⇧Tab / ⌃T | Choose an effort / a theme |
| ⌃W / ⌃\\ | Workspaces and sessions / show or hide the sidebar |
| ⌥D / ⌥E | Fold or expand details / external editor |
| ⌃V | Paste a clipboard image |
| @ or ! | Find a project file, or run a shell command |
| ↑, ↓, Tab, Enter after / or @ | Choose a slash command, its value, or a project file as it is typed |
| ⌃F / PageUp / PageDown | Filter the current view / scroll |
| ⌃R / ⌃Space | Python input / complete a name |
| ⌃L / ⌃A | Edit a prompt program / answer an operator question |
| ⌃G / ⌃click a name | Inspect a value and follow its definition |
| Drag / ⌃Y | Select text and copy it / copy the selected text again |
| Click / right-click an act | Fold or expand it / inspect its value or prompt program |
| ⌃⌥← | Return from a definition jump |
| ⌥[ / ⌥] or ⌥P / ⌥N | Previous / next message of the feed |
| ⌃PageUp / ⌃PageDown | Previous / next page of file changes |
| ⌃C | Clear the input, close a dialog, or cancel current work |
| Esc | Close a dialog or pause current model work |
| F1 / ⌃Q / ⌃D twice | Help / save and quit / save and quit from an empty input |

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
| `/tree` | Browse the chains and their turns, and open or branch from one |
| `/rewind` | Branch from an earlier message or act of this chain |
| `/undo` | Remove the last user message from a new transcript branch |
| `/redo` | Return to the branch before undo |
| `/queue [text]` | Send after current work, or manage queued messages |
| `/files` | Search project files for an @ reference |
| `/image [path]` | Attach an image file, or paste one from the clipboard |
| `/editor` | Edit this draft with VISUAL or EDITOR |
| `/run <python>` | Write a rung through the gate |
| `/bash <command>` | Stream a shell command |
| `/read <path>` | Show a file to this chain |
| `/cd <path>` | Change this chain's working directory |
| `/edit [prompt id]` | Change a prompt's program and replay it |
| `/inspect <name>` | Read a value from this chain's module |
| `/details` | Expand or collapse an act in the current view |
| `/grant <dollars>` | Pause at a dollar ceiling |
| `/context <fraction>` | Pause at a share of the model window |
| `/pause [id]` | Hold delivery while work completes |
| `/wake [id]` | Deliver pending work |
| `/cancel [id]` | End an act or the selected chain's work |
| `/feed <id> [text]` | Send a line of input to a command; no text closes its input |
| `/close <id> <json>` | Close an act with a JSON value |
| `/name <name>` | Set a name in the session picker |
| `/share [path]` | Export a standalone HTML conversation |
| `/export <path>` | Write the transcript and program to a new JSON file |
| `/workspace [path]` | Choose a workspace or add a project folder |
| `/sidebar` | Show or hide the chains, the usage, and the workspaces |
| `/delete` | Move a session and its files to the workspace trash |
| `/autocollapse` | Toggle collapse of completed rungs |
| `/extension <path>` | Load commands from a TypeScript or JavaScript extension |
| `/exit` | Save every session and quit |

<!-- commands:end -->

The command table comes from the same source as completion, the palette, and help. Run `bun run docs` after
changing that source. Enter sends a message straight to the engine. The engine gives the next
ask to the rung that has waited longest. ⌥Enter or `/queue text` holds a follow-up until the chain's
current prompt and rungs complete. `/queue` edits or removes waiting messages. Saved queues require a resume
choice; a dispatch recorded before a crash is not sent twice. Editing a prompt's program writes its door and uses the engine's
replay. A slash command typed while a program is under edit runs as a command, since no Python program starts with a
slash. Python input uses the same gate as a model's word and marks a refused line in the word and editor.
The editor matches brackets and indents a new Python line.
`@` opens project file search. A reference that names a file, such as `@README.md` or `@"file name.txt"`, is read
before the message is sent; any other @word, such as `@dataclass`, stays text. A leading `!` runs a shell
command. ⌥E or `/editor` edits the current draft with `VISUAL`, then `EDITOR`, then `vi`, through the shell
that runs commands; configure the editor to wait until the file is saved and closed.
`/image path` attaches a PNG, JPEG, GIF, or WebP file. ⌃V or bare `/image` pastes an image through macOS
AppKit, Windows PowerShell, Wayland `wl-paste`, or X11 `xclip`. Attachments have a limit of 20 MiB each and are
copied beside the record. The provider of the session sends them as pi-ai image blocks, with their references kept
in the prompt.
Click the attachment row to open or remove an image from the draft.
`/model` and `/effort` open separate pickers. `/model name` takes `provider:model` or the model's id alone, by
the rule of the provider. Each model offers the efforts in its pi-ai metadata, saved in
the chain's roster. A model change keeps the current effort if the new model offers it.

The engine contract takes a chain as a fork source. A fork is not a filesystem rollback or an arbitrary
historical checkpoint. A branch that the rewind tree makes uses the engine's `take` filter and runs as an operator
rung, so the filter is made again after a reopen. Its module and files keep their current state. `/undo` uses the
same durable filter to remove the last user message from a new branch; `/redo` returns to its origin. These commands
change the model's conversation. The module and filesystem remain current. The TUI does not add a separate
permission or tool protocol to the engine.

`/share` writes a standalone HTML conversation, including its images and exact transcript. `/share`, `/export`,
`/image`, and `/extension` read a leading `~` as the home directory, and a relative path from the directory of the
selected chain. The share dialog
can open the file, copy its path, or upload the selected conversation to an unlisted GitHub gist through
`gh`. Upload happens only when chosen. Anyone with its link can read the shared conversation. `/export`
keeps the structured JSON export. `/context` sets the context ceiling.

Load a local command extension with `--extension <path>` or `/extension <path>`. An extension exports a setup
function that receives `ExtensionAPI` and registers commands with a label, description, and `run` function.
Its context gives the current chain, directory, engine, message submission, and notices. Extensions are loaded
only when named by the user. See [the example](examples/project-summary.ts); loaded commands join the palette
and slash completion.

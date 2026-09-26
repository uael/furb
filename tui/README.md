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
a `.gitignore` that keeps it out of version control. Keep the `.jsonl`,
`.session.json`, `.changes.jsonl`, and `.ui.json` files together. The last file saves the selected chain, view, prompt
shape, drafts, rung folds, queued follow-ups, and scroll positions. Keep its `.images` directory too when
the session has image attachments. An unfinished session opens paused and offers a resume
choice. The native engine replays completed work from the record.
The picker shows state, save time, cost, and record size. It reads unfinished work through native replay
without locking or changing the saved record. The record decides what remains unfinished. The list shows at once,
and each saved session shows its state when its replay ends.

![The furb TUI in action](../docs/furb.gif)

The top line names the session and the chain at its left, with the directory of the chain. At its right, a toggle
switches the three views of the chain: **Feed**, **Transcript**, and **Changes**. Click a view, or press ⌃1-3 (⌥1-3
in every terminal). ⌃Tab rolls to the next chain, and ⇧⌃Tab to the one before it. The feed shows your messages, the
Python that each model wrote, the acts that it made, and the answers. An act that no turn tells yet, such as a
command that you started, stands at the end of the feed until a turn tells it. The transcript shows the exact text
that the model reads, and the changes show the diff of each file that the life wrote. Each chain has its own feed
and module.

The sidebar at the right holds the chains of the session, the context and the cost that its model used, and the
workspaces. Each workspace is a project folder with many sessions, and its list scrolls on its own. Click a
workspace to fold its sessions, and click a session to open it. Open sessions keep running when you switch. A dot
shows the state of each session and workspace: blue for work, yellow for input needed or paused work, green for an
unread completion, and red for an error. An open grey dot means ready or saved. Hover the dot for its state. The
pointer on a row shows `✎` and `×` at its end. `✎` renames the session or the workspace in its row. `×` archives a
session or moves it to the trash, and it takes a workspace off the list. A right click on a row opens the same
actions in a menu. The chains list holds the root chain, the chain on screen, and each chain that is not at rest.
The other chains fold under Finished. Selecting a completed session clears its unread marker. `/workspace path`
adds a folder, and `/new` starts a session in the current workspace. `/delete` moves a chosen session and its
companions to `.furb/trash`. The trash holds a `restore.json` with the original paths, and its record can be opened
with `--resume`. Quit saves and closes all open sessions, and SIGINT, SIGTERM, SIGHUP, and SIGQUIT quit the same
way. On Windows, a console that closes and Ctrl+Break quit the same way. `⌃\` or `/sidebar` shows or hides the
sidebar, and `sidebarWidth` in the preferences file sets its width, from 26 to 48 columns. On a terminal narrower
than 100 columns the sidebar is hidden, and `/workspace` keeps all sessions available from the keyboard.

Python words have offline Tree-sitter colors and line numbers. Hover over a name for its current value; ⌃click
or ⌃G opens its fields and definition. GitHub Dark is the default. Sessions share the theme, and the TUI saves
it in `$XDG_CONFIG_HOME/furb/ui.json` (or `~/.config/furb/ui.json`, and `%APPDATA%\furb\ui.json` on Windows).
`FURB_CONFIG_DIR` selects another configuration directory. Tests and screenshot generation use isolated
preferences; the CLI shares the user's choice across sessions, including demos.

Your message stands in a panel with a bar at its left, as you typed it. Each rung that the model wrote shows as
numbered Python, and the acts that it made stand under it with a preview of their output. The answer of a prompt
comes under the name of the model that gave it. A question that a model asks you stands in a panel with a yellow
bar, and your answer follows it. One mark means one state in every view, dialog, and sidebar: a spinner for work
that runs, `◉` for a chain or a session at work, `◆` for a question that waits for you, `◌` for paused work, `✓`
for a done act, `✗` for a failure, `●` for a chain you started or a session that finished while you were away, and
`○` for rest. F1 lists them. A rung opens while it runs, and folds to its mark, its name, and its first line when it
completes. Click its heading or use ⌥D to open or fold it. Its fold stays the same across views and after
reopening. `/autocollapse` turns the automatic fold off, and on again. Right-click an act to inspect it, edit its
program, or branch after it.

The composer grows as you type, and its bar and the line under it name the mode: Prompt, Python, Answer, or Edit
program. Each part of that line is a button: the mode, the chain, the model, the effort, and the type of the
answer. Up and Down at the first and the last line of the input walk the history of what it sent. Up in an empty
input takes back the last message queued on the chain, to edit it. ⌃S puts the input aside, and ⌃S again
brings it back; the line under the input shows what waits. The footer says what the session does, and each key
that it offers is a button. A dialog dims the screen behind it, and a click outside it closes it. ⌃P lists
every action with its slash command and its keys. ⌃T opens the themes, each with a swatch of its colors.

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
| ⌃O | Saved sessions |
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
historical checkpoint. Escape twice, or `/rewind`, opens the rewind tree in the feed. The tree shows each chain,
the acts of the chain, each act under the act that made it, and each branch under the point it starts from. Up and
Down move the pointer, Left and Right fold a branch, and a click points at a row. Enter on a message makes a new
branch that has not read the message, and gives the message back to the input to send again. Enter on another act
makes a new branch that reads the chain through that act. The branch uses the engine's `take` filter and runs as an
operator rung, so the filter is made again after a reopen. Its module and files keep their current state. `/undo`
uses the same durable filter to remove the last user message from a new branch; `/redo` returns to its origin.
These commands change the model's conversation. The module and filesystem remain current. `/tree` opens the same
tree with the pointer on the chain shown, and Enter on a chain opens it. The TUI does not add a separate
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

`bun run screenshots` captures the real rendered views through OpenTUI's test renderer. The screenshots use
a scripted provider, real native engine, temporary files, and real local commands. See [the gallery](../docs/tui.md).
`bun run animation` records the animation at the top of this file the same way: it types, clicks, and waits in a
real session, and writes each picture that changed into `docs/furb.gif`.

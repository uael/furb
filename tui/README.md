# furb TUI

An OpenTUI application built on `@furb/engine`. The TUI owns a Bun worker for each open life and its World.
Rendering, input, and syntax coloring stay responsive while the sandbox gates and runs Python.

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
`.world.json`, `.changes.jsonl`, and `.ui.json` files together. The last file saves the selected chain, view, prompt
shape, drafts, rung folds, queued follow-ups, and scroll positions. Keep its `.images` directory too when
the session has image attachments. An unfinished session opens paused and offers a resume
choice. The native engine replays completed work from the record.
The picker shows state, save time, cost, and record size. It reads unfinished work through native replay
without locking or changing the saved record. The record decides what remains unfinished. The list shows at once,
and each saved session shows its state when its replay ends.

Each workspace is a project folder with many sessions. The left sidebar is one tree: click a workspace to
fold its sessions, and click a session to open it. Open sessions keep running when you switch. The workspace
dot reflects its sessions: blue for work, yellow for input needed or paused work, green for an unread
completion, and red for an error. An open grey dot means ready or saved. Hover a session for its state.
Selecting a completed session clears its unread marker. `/workspace path` adds a folder; `/new` starts a
session in the current workspace. `/delete` moves a chosen session and its companions to `.furb/trash`.
The trash holds a `restore.json` with the original paths, and its record can be opened with `--resume`. Quit saves
and closes all open sessions, and SIGINT, SIGTERM, SIGHUP, and SIGQUIT quit the same way. On Windows, a console
that closes and Ctrl+Break quit the same way.

The six views show the conversation, accepted Python program, acts, facts, exact model transcript, and file
diffs. Each chain has its own conversation and module. Python words have offline Tree-sitter colors and
line numbers. Hover over a name for its current value; Ctrl+click or Ctrl+G opens its fields and definition.
GitHub Dark is the default. Theme preferences are shared by sessions and saved in `$XDG_CONFIG_HOME/furb/ui.json`
(or `~/.config/furb/ui.json`, and `%APPDATA%\furb\ui.json` on Windows). `FURB_CONFIG_DIR` selects another
configuration directory. Tests and screenshot generation use isolated preferences; the CLI shares the user's choice
across sessions, including demos.
The left sidebar's visibility and width are preferences too. `Ctrl+\` or `/sidebar` toggles it. On narrow
terminals it is hidden, and `/workspace` keeps all sessions available from the keyboard.

Space separates turns, programs, and act results. A rung opens while it runs. Click its heading or use
Alt+D to fold it to its name and status: `running`, `failed`, or `done`. Its fold stays the same across views
and after reopening. `/autocollapse` opts into automatic collapse when a rung completes. Commands show short
output previews and expand to named fields. The input panel grows as you type.
Drag a separator to resize its sidebar. Select text and press Ctrl+Y
to copy it through OSC 52, where the terminal supports it.

A chord before "or" reaches the TUI only from a terminal with the kitty keyboard protocol. The chord after "or"
reaches it from every terminal. F1 lists the chords that the terminal in use sends.

<!-- keys:start -->

| Key | Action |
| --- | --- |
| Enter | Send a message, or run Python input |
| Shift+Enter or Ctrl+J | Insert a new line |
| Ctrl+1 through Ctrl+6 or Alt+1 through Alt+6 | Conversation / program / activity / facts / transcript / changes view |
| Ctrl+P | Search all actions |
| Ctrl+B / Ctrl+N | Switch / create a chain |
| Ctrl+M or Alt+M | Choose a model |
| Shift+Tab / Ctrl+T | Choose an effort / a theme |
| Ctrl+O | Saved sessions |
| Ctrl+W / Ctrl+\\ | Workspaces and sessions / toggle the left sidebar |
| Alt+D / Alt+E | Fold or expand details / external editor |
| Alt+Enter | Queue this message after current work |
| Ctrl+V | Paste a clipboard image |
| @ / ! | Find a project file / run a shell command |
| Up, Down, Tab, Enter after / or @ | Choose a slash command or a project file as it is typed |
| Ctrl+F / PageUp / PageDown | Filter the current view / scroll |
| Ctrl+R / Ctrl+Space | Python input / complete a name |
| Ctrl+L / Ctrl+A | Prompt programs / answer an operator question |
| Ctrl+G / Ctrl+click a name | Inspect a value and follow its definition |
| Ctrl+Y | Copy the selected text |
| Click / right-click an act | Fold or expand it / inspect its value or prompt program |
| Ctrl+Alt+Left | Return from a definition jump |
| Alt+[ / Alt+] or Alt+P / Alt+N | Previous / next prompt REPL |
| Alt+Up / Alt+Down | Previous / next submitted input |
| Ctrl+PageUp / Ctrl+PageDown | Previous / next page of file changes |
| Ctrl+C | Clear the input, close a dialog, or cancel current work |
| Escape | Close a dialog or pause current model work |
| F1 / Ctrl+Q / Ctrl+D twice | Help / save and quit / save and quit from an empty input |

<!-- keys:end -->

<!-- commands:start -->

| Command | Action |
| --- | --- |
| `/new` | Start a fresh life |
| `/exit` | Save every session and quit |
| `/workspace [path]` | Choose a workspace or add a project folder |
| `/sidebar` | Show or hide the workspace and session tree |
| `/delete` | Move a session and its files to the workspace trash |
| `/tree` | Browse this session's chains by their parent source |
| `/undo` | Remove the last user message from a new transcript branch |
| `/redo` | Return to the branch before undo |
| `/editor` | Edit this draft with VISUAL or EDITOR |
| `/files` | Search project files for an @ reference |
| `/queue [text]` | Send after current work, or manage queued messages |
| `/share [path]` | Export a standalone HTML conversation |
| `/image [path]` | Attach an image file, or paste one from the clipboard |
| `/autocollapse` | Toggle collapse of completed rungs |
| `/extension path` | Load commands from a TypeScript or JavaScript extension |
| `/name name` | Set a name in the session picker |
| `/chain label` | A conversation with its own state |
| `/fork label` | Copy this chain's current transcript and program |
| `/details` | Expand or collapse an act in the current view |
| `/rewind` | Choose the last act the new chain should read |
| `/pause [id]` | Hold delivery while work completes |
| `/wake [id]` | Deliver pending work |
| `/cancel [id]` | End an act or the selected chain's work |
| `/grant dollars` | Pause at a dollar ceiling |
| `/context fraction` | Pause at a share of the model window |
| `/run python` | Write a rung through the gate |
| `/bash command` | Stream a shell command |
| `/read path` | Show a file to this chain |
| `/cd path` | Change this chain's working directory |
| `/edit [prompt id]` | Change a prompt's program and replay it |
| `/feed id text` | Send a line of input to a command; no text closes its input |
| `/close id JSON` | Close an act with a JSON value |
| `/export path` | Write the transcript and program to a new JSON file |
| `/model [model]` | Use a model from this chain's roster |
| `/effort [level]` | Set the reasoning effort of the selected model |
| `/shape type` | Choose the result type of the next prompt |
| `/inspect name` | Read a value from this chain's module |
| `/theme name` | Change the palette of every surface |

<!-- commands:end -->

The command table comes from the same source as completion, the palette, and help. Run `bun run docs` after
changing that source. Enter sends a message straight to the engine. The engine gives the next
ask to the rung that has waited longest. Alt+Enter or `/queue text` holds a follow-up until the chain's
current prompt and rungs complete. `/queue` edits or removes waiting messages. Saved queues require a resume
choice; a dispatch recorded before a crash is not sent twice. Editing a prompt's program writes its door and uses the engine's
replay. Python input uses the same gate as a model's word and marks a refused line in the word and editor.
The editor keeps submitted input history, matches brackets, and indents a new Python line.
`@` opens project file search. A reference that names a file, such as `@README.md` or `@"file name.txt"`, is read
before the message is sent; any other @word, such as `@dataclass`, stays text. A leading `!` runs a shell
command. Alt+E or `/editor` edits the current draft with `VISUAL`, then `EDITOR`, then `vi`, through the shell
that runs commands; configure the editor to wait until the file is saved and closed.
`/image path` attaches a PNG, JPEG, GIF, or WebP file. Ctrl+V or bare `/image` pastes an image through macOS
AppKit, Windows PowerShell, Wayland `wl-paste`, or X11 `xclip`. Attachments have a limit of 20 MiB each and are
copied beside the record. The built-in World sends them as pi-ai image blocks, with their references kept in the
prompt.
Click the attachment row to open or remove an image from the draft.
`/model` and `/effort` open separate pickers. `/model name` takes `provider:model` or the model's id alone, by
the rule of the World. Each model offers the efforts in its pi-ai metadata, saved in
the chain's roster. A model change keeps the current effort if the new model offers it.

The engine contract takes a chain as a fork source. A fork is not a filesystem rollback or an arbitrary
historical checkpoint. `/rewind` offers an act picker and uses the engine's `take` filter to choose which
acts the new chain reads. The choice runs as an operator rung, so the filter is recreated after a reopen.
Its module and files keep their current state. `/undo` uses the same durable filter to remove the last user
message from a new branch; `/redo` returns to its origin. These commands change the model's conversation.
The module and filesystem remain current. `/tree` shows the session's chains and source parents; Left/Right
folds a branch and Enter opens it. The TUI does not add a separate
permission or tool protocol to the engine.

`/share` writes a standalone HTML conversation, including its images and exact transcript. `/share`, `/export`,
`/image`, and `/extension` read a leading `~` as the home directory, and a relative path from the directory of the
selected chain. The share dialog
can open the file, copy its path, or upload the selected conversation to an unlisted GitHub gist through
`gh`. Upload happens only when chosen. Anyone with its link can read the shared conversation. `/export`
keeps the structured JSON export. `/context` sets the context ceiling.

Load a local command extension with `--extension path` or `/extension path`. An extension exports a setup
function that receives `ExtensionAPI` and registers commands with a label, description, and `run` function.
Its context gives the current chain, directory, life, message submission, and notices. Extensions are loaded
only when named by the user. See [the example](examples/project-summary.ts); loaded commands join the palette
and slash completion.

`bun run screenshots` captures the real rendered views through OpenTUI's test renderer. The screenshots use
a scripted World, real native engine, temporary files, and real local commands. See [the gallery](../docs/tui.md).

# furb TUI

An OpenTUI application built on `@furb/engine`. The TUI owns a Bun worker for the life and its World.
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
a new life. Sessions are saved under `.furb/sessions` in the selected directory. Keep the `.jsonl`,
`.world.json`, `.changes.jsonl`, and `.ui.json` files together. The last file saves the selected chain, view, prompt
shape, drafts, scroll positions, and sidebar width. An unfinished session opens paused and offers a resume
choice. The native engine replays completed work from the record.
The picker shows the save time, cost, and record size. Opening a session reads its unfinished work from the
record, so an old metadata file cannot hold work that has already completed.

The six views show the conversation, accepted Python program, acts, facts, exact model transcript, and file
diffs. Each chain has its own conversation and module. Python words have offline Tree-sitter colors and
line numbers. Hover over a name for its current value; Ctrl+click or Ctrl+G opens its fields and definition.
GitHub Dark is the default. Theme preferences are shared by sessions and saved in `$XDG_CONFIG_HOME/furb/ui.json`
(or `~/.config/furb/ui.json`). `FURB_CONFIG_DIR` selects another configuration directory. Tests and screenshot
generation use isolated preferences; the CLI shares the user's choice across sessions, including demos.

Space separates turns, programs, and act results. Code and output start with short previews; click a heading
or use `/details` to expand the complete content and named fields. The input panel grows as you type.
Drag the right separator to resize the sidebar. Select text and press Ctrl+Y
to copy it through OSC 52, where the terminal supports it.

| Key | Action |
| --- | --- |
| Enter / Shift+Enter | Send / new line |
| Ctrl+1 through Ctrl+6 | Switch view |
| Ctrl+P | Search commands |
| Ctrl+B / Ctrl+N | Switch / create a chain |
| Ctrl+M / Shift+Tab / Ctrl+T | Model / effort / theme |
| Ctrl+O | Saved sessions |
| Ctrl+F / PageUp / PageDown | Search / scroll |
| Ctrl+R / Ctrl+Space | Python input / complete a name |
| Ctrl+L | Prompt programs and editing |
| Ctrl+A | Answer an operator question |
| Ctrl+G / Ctrl+Y | Inspect a name / copy selection |
| Ctrl+Alt+Left | Return from a definition jump |
| Alt+[ / Alt+] | Previous / next prompt REPL |
| Alt+Up / Alt+Down | Previous / next submitted input |
| Ctrl+PageUp / Ctrl+PageDown | Previous / next file-change page |
| Ctrl+C | Clear input, close a dialog, or cancel current work |
| Tab after `/` | Complete a slash command |
| Escape | Close a dialog or pause current model work |
| F1 / Ctrl+Q | Help / save and quit |

<!-- commands:start -->

| Command | Action |
| --- | --- |
| `/new` | Start a fresh life |
| `/name name` | Set a name in the session picker |
| `/chain label` | A conversation with its own state |
| `/fork label` | Copy this chain's current transcript and program |
| `/details` | Expand or collapse an act in the current view |
| `/rewind` | Choose the last act the new chain should read |
| `/pause [id]` | Hold delivery while work completes |
| `/wake [id]` | Deliver pending work |
| `/cancel [id]` | End an act or the selected chain's work |
| `/grant dollars` | Pause at a dollar ceiling |
| `/share fraction` | Pause at a share of the model window |
| `/run python` | Write a rung through the gate |
| `/bash command` | Stream a shell command |
| `/read path` | Show a file to this chain |
| `/cd path` | Change this chain's working directory |
| `/edit [prompt id]` | Change a prompt's program and replay it |
| `/feed id text` | Send input to a command; empty text closes input |
| `/close id JSON` | Close an act with a JSON value |
| `/export path` | Write the transcript and program to a new JSON file |
| `/model [model]` | Use a model from this chain's roster |
| `/effort [level]` | Set the reasoning effort of the selected model |
| `/shape type` | Choose the result type of the next prompt |
| `/inspect name` | Read a value from this chain's module |
| `/theme name` | Change the palette of every surface |

<!-- commands:end -->

The command table comes from the same source as completion, the palette, and help. Run `bun run docs` after
changing that source. A message sent while work runs enters the engine's queue. The engine gives the next
ask to the rung that has waited longest. Editing a prompt's program writes its door and uses the engine's
replay. Python input uses the same gate as a model's word and marks a refused line in the word and editor.
The editor keeps submitted input history, matches brackets, and indents a new Python line.
`/model` and `/effort` open separate pickers. Each model offers the efforts in its pi-ai metadata, saved in
the chain's roster. A model change keeps the current effort if the new model offers it.

The engine contract takes a chain as a fork source. A fork is not a filesystem rollback or an arbitrary
historical checkpoint. `/rewind` offers an act picker and uses the engine's `take` filter to choose which
acts the new chain reads. The choice runs as an operator rung, so the filter is recreated after a reopen.
Its module and files keep their current state. The TUI does not add a separate
permission or tool protocol to the engine.

`bun run screenshots` captures the real rendered views through OpenTUI's test renderer. The screenshots use
a scripted World, real native engine, temporary files, and real local commands. See [the gallery](../docs/tui.md).

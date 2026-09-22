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
`.world.json`, and `.ui.json` files together. The last file saves the selected chain, view, theme, prompt
shape, drafts, scroll positions, and pane widths. An unfinished session opens paused and offers a resume
choice. The native engine replays completed work from the record.

The six views show the conversation, accepted Python program, acts, facts, exact model transcript, and file
diffs. Each chain has its own conversation and module. Python words have offline Tree-sitter colors and
line numbers. Hover over a name for its current value; Ctrl+click or Ctrl+G opens its fields and definition.
Click a block heading to collapse it. Drag either separator to resize a pane. Select text and press Ctrl+Y
to copy it through OSC 52, where the terminal supports it.

| Key | Action |
| --- | --- |
| Enter / Shift+Enter | Send / new line |
| Ctrl+1 through Ctrl+6 | Switch view |
| Ctrl+P | Search commands |
| Ctrl+B / Ctrl+N | Switch / create a chain |
| Ctrl+M / Ctrl+T | Model and effort / theme |
| Ctrl+O | Saved sessions |
| Ctrl+F / PageUp / PageDown | Search / scroll |
| Ctrl+R / Ctrl+Space | Python input / complete a name |
| Ctrl+L | Prompt programs and editing |
| Ctrl+A | Answer an operator question |
| Ctrl+G / Ctrl+Y | Inspect a name / copy selection |
| Tab after `/` | Complete a slash command |
| Escape | Close a dialog or pause current model work |
| F1 / Ctrl+Q | Help / save and quit |

Slash commands expose `pause`, `wake`, `cancel`, `chain`, `fork`, `grant`, `share`, `model`, `shape`, `run`,
`bash`, `read`, `cd`, `edit`, `feed`, `close`, `export`, `inspect`, `theme`, `name`, and `new`. The command palette shows the
main actions. A message sent while work runs pauses delivery, adds the new prompt, then wakes the chain.
Editing a prompt's program writes its door and uses the engine's replay. Python input uses the same gate
as a model's word and shows its findings before an accepted word runs.

The engine contract takes a chain as a fork source. A fork is not a filesystem rollback or an arbitrary
historical checkpoint. The TUI does not add a separate permission or tool protocol to the engine.

`bun run screenshots` captures the real rendered views through OpenTUI's test renderer. The screenshots use
a scripted World, real native engine, temporary files, and real local commands. See [the gallery](../docs/tui.md).

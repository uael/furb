# furb TUI

An OpenTUI application built on `@furb/engine`. The TUI owns a Bun worker for each open session: its life and its
ears.
Rendering, input, and syntax coloring stay responsive while the sandbox gates and runs Python. The TUI needs Bun
1.4.2 or later. On Windows, Bun 1.3 crashes the TUI when it calls into OpenTUI.

```sh
uv sync
bun install
bun run build
bun run demo                 # A local, scripted life, and a real life recorded from its log. No model request.
bun run tui                  # A real life, on the models of the catalog of the crate.
bun run tui -- --resume .furb/sessions/example.jsonl
```

The welcome of an empty chain offers "Watch a life" first: a short life on a small project that plays by itself
and asks no model, so it costs nothing. Several models answer in it, a word of Python runs as a program, and a branch
starts from an earlier point. While it plays and the input is empty, `[` and `]` move the feed back and on in time,
`b` starts a branch at the moment shown, and Esc comes back to now. `/watch` plays it too. On the first run, and on
each run while no model answers, the welcome checks the setup: the claude command line, the keys in the
environment, the local server that `FURB_LOCAL_URL` names, and the models that the catalog offers, with a fix for each
part that is missing.

`furb`, the command line, opens this TUI when it has no command, with `--demo`, `--record`, `--cwd`, the model and the
effort of its `--model` as `--model` and `--effort`, each `--roster`, and the words after `--`.

A recorded life is a log of a life whose first line is a header that says `"recorded": true`. The TUI reads it as it
reads a record that another engine made, from its acts, and runs nothing: what you send to it fails with the
reason. The demo opens
`fixtures/conductor.jsonl`, a life of a conductor of furb with 23 chains, beside its scripted life, when it has no
`--cwd` and no record. `bun run tui/script/fixture.ts <record>.jsonl.log [out]` makes such a fixture from the log of
a real life: it keeps the acts that the model of a life reads, cuts each word, message and output to its first lines,
replaces each key, address of mail and home directory, and ends at the moment with the most chains at work.

Use `--cwd`, `--model provider:model`, `--effort`, `--record`, and repeated `--roster` options to configure
a new life. A session offers every model that the catalog offers, with its model first: the default model when you
name none. With `--roster`, it offers its model and the models of `--roster`, and no more.
The TUI holds three views of the whole life side by side: the board of main, the loom of the chains in time, and the
graph of the programs that run. `--view main|loom|graph` chooses one at the start, ⌥V switches it at run time,
`/view` names one or lists the three, and the preferences keep the last one. Each view stands over the feed of the
selected chain, and every other screen stays the same in each one. ⌥G gives the keys to the loom or the graph, and
Esc gives them back. `docs/tui.md` shows each view, and `tui/DESIGN.md` says how they came.
Sessions are saved under `.furb/sessions` in the selected directory. The `.furb` that the TUI makes holds
a `.gitignore` that keeps it out of version control. Keep the `.jsonl`, `.session.json`, `.changes.jsonl`, and
`.ui.json` files together. The last file keeps the view of the session as you left it. Keep its `.images` directory
too when the session has image attachments. An unfinished session opens paused and offers a resume choice. The
native engine replays completed work from the record.
A record whose replay drifts, as a record that another engine made, opens to read: the TUI shows what the record
kept, runs no word again, asks no model, and refuses each command that would change the life. A dialog names the
engine that made the record and the engine of this build, and offers to read it or to start a new life in its
workspace.
⌃W lists the workspaces and their sessions, each with its state, save time, cost, and record size. It reads
unfinished work through native replay without locking or changing the saved record. The record shows what remains
unfinished. The list shows at once, and each saved session shows its state when its replay ends.

![The furb TUI in action](../docs/furb.gif)

[The gallery](../docs/tui.md) shows each screen of the TUI and what it does.

`/workspace path` adds a folder, and `/new` starts a session in the current workspace. `/delete` archives a chosen
session, or moves it and its companions to `.furb/trash` and removes its scratch directory from the cache of furb. The
trash holds a `restore.json` with the original paths, and its record can be opened with `--resume`, which makes again
the scratch files that its words wrote. Quit saves and closes all open sessions, and SIGINT, SIGTERM, SIGHUP,
and SIGQUIT quit the same way. On Windows, a console that closes and Ctrl+Break quit the same way. `sidebarWidth` in
the preferences file sets the width of the sidebar, from 26 to 48 columns. On a terminal narrower than 100 columns the
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
| Esc twice | Rewind: go back to an earlier point of the chain, with its files or not |
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
| ↑, ↓, Tab, Enter after /, @ or : | Choose a slash command, its value, a project file, or an emoji as it is typed |
| ⌃F / PageUp / PageDown | Filter the current view / scroll |
| ⌃R / ⌃Space | Python input / complete a name |
| ⌃L / ⌃A | Edit a prompt program / answer the questions that wait |
| Tab / ⇧Tab in an answer | Next / previous question that waits |
| ⌃G / ⌃click a name | Inspect a value and follow its definition |
| Drag / ⌃Y | Select text and copy it / copy the selected text again |
| Click / right-click an act | Fold or expand it / inspect its value or prompt program |
| ⌃⌥← | Return from a definition jump |
| ⌥[ / ⌥] or ⌥P / ⌥N | Previous / next message of the feed |
| ⌥V / ⌥G | Switch the view of the life: main, the loom, or the graph / move in the loom or the graph |
| ↑↓←→ / Enter / Esc after ⌥G | Move, fold or zoom / zoom in / zoom out or go back |
| [ ] / { } after ⌥G | Move the cursor in time by one / ten moments |
| Space / x / n / b after ⌥G | Pause or wake / cancel / step / branch at the cursor the chain |
| ⌃PageUp / ⌃PageDown | Previous / next page of file changes |
| ⌃C | Clear the input, close a dialog, or cancel what you sent last (a message goes back to the input) |
| Esc | Close a dialog or pause current model work |
| F1 / ⌃Q / ⌃D twice | Help / save and quit / save and quit from an empty input |

<!-- keys:end -->

<!-- commands:start -->

| Command | Action |
| --- | --- |
| `/new` | Start a fresh life |
| `/watch` | Play a short life that asks no model and costs nothing |
| `/model [model]` | Choose the model that answers on this chain |
| `/effort [level]` | Set the reasoning effort of the selected model |
| `/shape [type]` | Choose the result type of the next prompt |
| `/theme [name]` | Change the palette of every surface |
| `/chain <label>` | A conversation with its own state |
| `/fork <label>` | Copy this chain's current transcript and program |
| `/tree` | Browse the chains and their turns, and open or branch from one |
| `/rewind` | Go back to an earlier message or act of this chain, with its files or not |
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
| `/pause [act or chain]` | Hold delivery while work completes |
| `/wake [act or chain]` | Deliver pending work |
| `/cancel [id]` | End an act, or what you sent last on this chain |
| `/feed <id> [text]` | Send a line of input to a command; no text closes its input |
| `/close <id> <json>` | Close an act with a JSON value |
| `/name <name>` | Set a name in the session picker |
| `/share [path]` | Export a standalone HTML conversation |
| `/export <path>` | Write the transcript and program to a new JSON file |
| `/workspace [path]` | Choose a workspace or add a project folder |
| `/view [main|loom|graph]` | Show the life as the board of main, the loom, or the graph |
| `/sidebar` | Show or hide the chains, the usage, and the workspaces |
| `/delete` | Archive a session, or move it and its files to the workspace trash |
| `/autocollapse` | Toggle collapse of completed rungs |
| `/extensions` | List the extensions that this life runs |
| `/exit` | Save every session and quit |

<!-- commands:end -->

The command table comes from the same source as completion, the palette, and help. Run `bun run docs` after
changing that source. Enter sends a message straight to the engine. The engine gives the next
ask to the rung that has waited longest. ⌥Enter or `/queue text` holds a follow-up until the chain's
current prompt and rungs complete. `/queue` edits or removes waiting messages. Saved queues require a resume
choice; a dispatch recorded before a crash is not sent twice. Editing a prompt's program writes its door, and the engine
makes the program again only when the edit drops a word that ran; else it keeps the module and runs only the new words.
A slash command typed while a program is under edit runs as a command, since no Python program starts with a
slash. Python input uses the same gate as a model's word and marks a refused line in the word and editor.
The editor matches brackets and indents a new Python line.
`@` opens project file search. A reference that names a file, such as `@README.md` or `@"file name.txt"`, is read
before the message is sent; any other @word, such as `@dataclass`, stays text. A leading `!` runs a shell
command. ⌥E or `/editor` edits the current draft with `VISUAL`, then `EDITOR`, then `vi`, through the shell
that runs commands; configure the editor to wait until the file is saved and closed.
`/image path` attaches a PNG, JPEG, GIF, or WebP file. ⌃V or bare `/image` pastes an image through macOS
AppKit, Windows PowerShell, Wayland `wl-paste`, or X11 `xclip`. Attachments have a limit of 20 MiB each and are
copied beside the record. The provider of the session sends them to a model that takes images, with their
references kept in the prompt.
Click the attachment row to open or remove an image from the draft.
`/model` and `/effort` open separate pickers. The model picker lists the models of the session, then every
model that the catalog of the crate offers. `/model name` takes `provider:model` or the model's id alone, by the
rule of the catalog. A model of the catalog that the session does not offer joins the session: the session saves
it, opens again on its record, starts again the work that was running, but for a command, which ends as
interrupted, and sends the next prompt of the chain to that model. Each model offers the efforts that the catalog gives it, saved in the chain's roster. A model change
keeps the current effort if the new model offers it. A choice of a model or an effort sets the actor of the chain
by a rung of the operator. A prompt names no actor, so the next rung of a prompt that runs, and each follow-up that
the queue sends, asks the actor that the chain has then.

The engine contract takes a chain as a fork source. A fork is not a filesystem rollback or an arbitrary
historical checkpoint. A branch that the rewind tree makes uses the engine's `take` filter and runs as an operator
rung, so the filter is made again after a reopen. Its module keeps its current state. When a write changed a file
after the point, the rewind tree offers to restore the conversation, the files, or both. It reads the text that each
write replaced in the journal of file changes beside the record, and writes it back by an operator rung, so the
record holds the restore. A change that a command made is not restored. `/undo` uses the
same durable filter to remove the last user message from a new branch; `/redo` returns to its origin. These commands
change the model's conversation. The module and filesystem remain current. The TUI does not add a separate
permission or tool protocol to the engine.

`/share` writes a standalone HTML conversation, including its images and exact transcript. `/share`, `/export`
and `/image` read a leading `~` as the home directory, and a relative path from the directory of the selected chain.
The share dialog can open the file, copy its path, or upload the selected conversation to an unlisted GitHub gist
through `gh`. Upload happens only when chosen. Anyone with its link can read the shared conversation. `/export` keeps
the structured JSON export. `/context` sets the context ceiling.

A life runs the extensions that the configs turn on, as [the guide of the extensions](../docs/extensions.md) says,
and each chain shows the rungs of their words in its feed, as it shows any rung. An extension can also give the TUI a part, a module of TypeScript beside its word, which adds
panels over the input, the rows of the acts of a kind, keys and commands, and which the TUI loads again live, as
[the guide of the extensions](../docs/extensions.md#a-part-for-the-tui) says. `/extensions` lists what the life
runs, and `/run` runs a word of one on the chain on screen, as `/run skill("pdf")`. A cancel ends one act and the work under it, and
not what an extension started on the chain, such as the watcher of the memory.

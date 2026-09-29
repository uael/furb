# furb terminal workspace

Keys use the marks of macOS: ⌃ is Control, ⌥ is Option or Alt, and ⇧ is Shift. These captures come from the real
OpenTUI renderer and native engine, with a scripted provider of the answers of the model, temporary files, and real
local commands. Run `bun run screenshots` from the repository root to capture them again, and `bun run animation` to
record the animation below: it types, clicks, and waits in a real session, and keeps each picture that changed. The
generators are in `tui/script/`.

![The furb TUI in action](furb.gif)

## Welcome

An empty feed shows the logo of furb, what it does, the project and the model, three ways to start, and the keys
to know. A click on a way to start puts its prompt in the input. The top line names the session, the chain, and the
directory of the chain, and the toggle at its right switches the three views of the chain. The footer says what the
session does and offers the keys that act now, each a button that does what its key does. Furb is the default palette.

![Welcome](screenshots/01-welcome.png)

## Feed

Each message that you send starts a thread, and the feed shows that thread. Your message stands in a panel. Under the
name of the model, each word that the model wrote shows as its steps: the comments of the word, `#` and one line of
markdown each, which say what the lines under them do. The feed shows no name of an act. A word folds to its steps,
which `▸` marks, and under them each command that it ran, with the last line that the command printed. A click on a
step opens its Python and what each act that it made came to, in the order that it made them, which `▾` marks. The
fold stays the same across views and after a reopen, and `/autocollapse` turns the automatic fold off, and on again.
Under the steps of a word stands the diff of each file that its writes changed, with `⋯` where the diff leaves out
lines between two changes. The answer of the thread comes last.
A right click on a word inspects it, edits its program, or branches after it.

![Feed](screenshots/02-feed.png)

![A thread that changed a file](screenshots/22-thread.png)

The name of a chain in the top line, or in the sidebar, opens the feed of the chain: a card for each thread, with its
first line, its state, and the last step of its model. A click on a card opens its thread. An act that no thread
holds, such as a command that you started, stands among the cards in the order it came. The sidebar lists the threads
of each chain under it, then the context and the cost, and the workspaces.

## Transcript

The exact text that the provider sends to the model, turn by turn.

![Transcript](screenshots/03-transcript.png)

## Changes

The lines that each write through the World added or removed. The toggle counts the changes.

![Changes](screenshots/04-changes.png)

## Command palette

⌃P lists every action with its slash command, its keys, and what it does. Type to filter by any of them. A
dialog dims the screen behind it, and a click outside it closes it.

![Command palette](screenshots/05-command-palette.png)

## Models and effort

The model picker names the current model and the window of each. ⇧Tab chooses the effort of the model.

![Models](screenshots/06-models.png)

![Effort](screenshots/07-effort.png)

## A question for you

A question that a model asks you stands in the feed of its chain, and under its chain in the sidebar, with `◆` and
"asks you". While it waits, Enter answers it: the line under the input says Answer and the type of the answer, and
the bar of the input turns warm. ⌃A opens a dialog for it.

![Operator question](screenshots/08-operator-question.png)

![Answer dialog](screenshots/09-operator-dialog.png)

## The input

The input grows as you type, and the line under it says what Enter does. On a chain, or on a closed thread, Enter
starts a New thread, with the type of its answer. On a thread that a model works, Enter notifies that thread: the
model reads your note at its next reply. On a question for you, Enter answers it. Each part of that line is a button:
the mode, the model, the effort, and the type of the answer. ⌃R switches the two modes of the input, markdown and
python. In python, Enter runs your Python through the same gate as a word of the model, and the bar of the input
takes the color of a model.

![Python input](screenshots/10-python-input.png)

## Value inspector

The pointer on a name shows its current value. ⌃G, or a ⌃click on a name, opens its value, its fields, and its
definition.

![Value inspector](screenshots/11-value-inspector.png)

## Themes

Furb is the default. GitHub Dark, Forest, and Midnight are dark, and Paper is light. ⌃T opens the themes, each with a
swatch of its colors. Each palette gives the same roles: the ground, two surfaces, three tones of text, and four
colors, for you, for a model, for what needs a look, and for what is done.

![Paper theme](screenshots/12-light-theme.png)

![Midnight theme](screenshots/13-midnight-theme.png)

![Color themes](screenshots/14-theme-picker.png)

## Chains

⌃B lists the chains, each with its state and the chain it branched from. Each chain has its own feed and module. The
sidebar lists the root chain, the chain on screen, and each chain that is not at rest, and the other chains fold
under Finished.

![Chains](screenshots/15-chains.png)

## Keys, marks, and commands

One mark means one state in every view, dialog, and sidebar: a spinner for work that runs, `◉` for a chain or a
session at work, `◆` for a question that waits for you, `◌` for paused work, `✓` for a done act, `✗` for a failure,
`⊘` for a cancel, `●` for a chain you started or a session that finished while you were away, and `○` for rest. A
thread that closes shows `✓` for a moment, then `·`. F1 lists the chords that the terminal in use sends, what each
mark means, and every slash command. Only work moves: `/motion` stops every spinner, the bar of the footer, and the
blink of the caret, and a still `◉` marks work in place of a spinner.

![Help](screenshots/16-help.png)

## Narrow terminal

On a narrow terminal the sidebar is hidden, and the top line keeps the session, the chain, and the views.

![Narrow terminal](screenshots/17-narrow.png)

## Work in progress

A command streams its output while it runs. While a model writes, a spinner turns before its name with the seconds
it has taken, each step shows once its line is whole, and a bar walks in the footer. A word that the gate refuses
shows each line that it refused and the reason.

![Command streaming](screenshots/18-live-command.png)

![Model progress](screenshots/19-model-progress.png)

![Gate findings](screenshots/20-gate-findings.png)

## Paused work

A session that opens with unfinished work stays paused until you resume it.

![Paused resume](screenshots/21-paused-resume.png)

## Rewind

Escape twice opens the rewind tree in the feed. Each act stands under the act that made it, and each branch under
the point it starts from. Up and Down move the pointer, Left and Right fold a branch, and a click points at a row.
Enter on a message gives it back to the input on a new branch, and Enter on another act makes a new branch that reads
the chain through that act. The module and the files keep their state.

![Rewind tree](screenshots/23-rewind-tree.png)

## Empty, loading, and failed views

A view with nothing to show says why in its middle. An act that fails keeps its failure in the feed, and a view
that cannot load offers to read it again.

![Empty results](screenshots/24-empty-results.png)

![Loading](screenshots/25-loading.png)

![Act failure](screenshots/26-error.png)

![View error](screenshots/27-view-error.png)

## Workspaces

The sidebar groups sessions under project folders, and its list scrolls on its own. A click on a workspace folds its
sessions, and a click on a session opens it. Open sessions keep running while another is selected. A mark shows the
state of each session and workspace, from live work and saved records, as the marks above say. A click on a completed
session clears its unread mark. A click on a step opens its word and a second click folds it, a workspace folds to
its name, and ⌃\ hides the sidebar. ⌃W lists the workspaces and their
sessions, each with its state, the time since its last save, its cost, and its size.

![Workspace tree](screenshots/28-workspace-tree.png)

![An open word](screenshots/29-open-word.png)

![Collapsed workspace](screenshots/30-collapsed-workspace.png)

![Hidden sidebar](screenshots/31-hidden-sidebar.png)

![Workspace picker](screenshots/32-workspace-picker.png)

## Session tree

`/tree` opens the same tree as rewind, with the pointer on the chain shown. Enter on a chain opens it.

![Session tree](screenshots/33-session-tree.png)

## Queued follow-ups

⌥Enter queues a message until the current work completes. Up in an empty input takes the last one back to edit
it, and `/queue` edits or removes the others.

![Queued follow-up](screenshots/34-queued-follow-up.png)

![Queue controls](screenshots/35-queue-editor.png)

## Images, sharing, and sessions

`/image` or ⌃V attaches an image. `/share` writes a standalone HTML conversation. `/delete` archives a session, or
moves it to the trash of its workspace.

![Image attachment](screenshots/36-image-attachment.png)

![Share conversation](screenshots/37-share-conversation.png)

![Delete session](screenshots/38-delete-session.png)

## External editor

⌥E edits the draft in `VISUAL` or `EDITOR`.

![External editor](screenshots/39-external-editor.png)

## Suggestions

`@` suggests project files, and `/` suggests slash commands, as you type.

![File suggestions](screenshots/40-file-picker.png)

![Command suggestions](screenshots/41-slash-suggestions.png)

A space after a command whose values are known lists them: the models, the efforts, the shapes, the themes, the
paths of the project, the chains, and the acts. Enter on a value runs the command.

![Value suggestions](screenshots/46-value-suggestions.png)

## Retried words

A word of a model can fail: the gate refuses it, or it raises. When the next word of the same thread takes its
place, the failed word keeps its steps and says in one quiet line that the next word took its place. A failure that
nothing replaced shows what refused it, or what it raised, in the warm color.

![Retried word](screenshots/47-retried-word.png)

## Archived sessions

The pointer on a session row shows two buttons at its end. `✎` renames the session, and `×` asks whether to archive
the session or move it to the trash. An archived session keeps its record, and folds under an Archived row of its
workspace. A click on an archived session opens it and puts it back in the list. The state of a session shows in a
tip only while the pointer is on its dot.

![Archived session](screenshots/48-archive-session.png)

## The menu of a row

A right click on a session row or a workspace row opens its menu at the pointer. The row stays lit while its menu
is open. A session offers Open, Rename, Archive or Restore, and Move to trash. A workspace offers a new session, a
new name, a fold, and removal from the list, which keeps its folder.

![Session menu](screenshots/49-session-menu.png)

## Rename in place

Rename puts an input in the row, in place of the name. Enter keeps the new name, and Escape keeps the old one. A
session keeps its name in its saved view, and a workspace in the list of workspaces.

![Rename a session](screenshots/50-rename-session.png)

## Extensions

`/extensions` lists the extensions that the life runs, and `/run` runs a word of one. The demo runs none.

![Extension command](screenshots/42-extension-command.png)

## Undo and redo

`/undo` removes the last message from a new branch, and `/redo` returns to the branch before it.

![Undo message](screenshots/43-undo-message.png)

![Redo message](screenshots/44-redo-message.png)

## Stash

⌃S puts the input aside, and the line under the input shows what waits. ⌃S again brings it back.

![Stash](screenshots/45-stash.png)

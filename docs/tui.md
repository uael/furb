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
session does and offers the keys that act now, each a button that does what its key does. GitHub Dark is the default.

![Welcome](screenshots/01-welcome.png)

## Feed

The board over the feed shows each chain of the life on one row, in the order of its tree: its mark, its name,
what it does now, and the pulse of its acts over the life. A chain that asks you a question shows the question in
yellow. A chain at work shows the act that it waits on, a chain that failed shows why, and a chain that finished
shows the first line of its answer. A click on a row shows that chain. The board shows when the life has more than
one chain, or when a question waits for you. In a long life, it keeps the rows that need a look and counts the
chains at rest.

Under the board, the feed shows the ladder of the chain: your message in a panel, each rung of the ladder on one
row, and the answer under the name of the model that gave it. The row of a rung shows its mark, its name, the first
line of its Python in the colors of its grammar, the count of the lines after it, and the acts that it made, each in the color of its state. A
rung opens while it runs, and shows the last lines that its model writes or that its command prints. It folds to
its row when it completes. A click on its heading, or ⌥D, opens it to show its numbered Python, and under it each
act that it made with a preview of its output. The fold stays the same across views and after a reopen.
`/autocollapse` turns the automatic fold off, and on again. A right click on an act inspects it, edits its program,
or branches after it. The board and the feed come from the model of a life that `@furb/engine/life` gives, so each
host shows the same life. The sidebar shows the chains, the context and the cost, and the workspaces.

![Feed](screenshots/02-feed.png)

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

## Questions and notices for you

A model asks you a question with a prompt to the operator, and a prompt of None is a notice, which tells you
something and asks nothing. Many of them can wait at once, on many chains. The desk over the input shows what waits:
a mark for each, `◆` for a question and `◇` for a notice, in the order that they came, and the one that you act on
in the color of a question, with its place, its chain, its id, and what it wants. Under it stand the first line of
its message, the count of its other lines, and its choices or where the input sends its answer. A click on a mark
goes to its ask and shows its chain, and a click on the message opens it whole.

On the chain shown, the input answers the question that you point at, or else the oldest question of the chain. The
line under the input says so, as `answers prompt2 returns str`, and the card of that question in the feed says that
the input answers it. A click on the card of another question points the input at it.

![Many questions](screenshots/52-many-questions.png)

Each question takes an answer of its shape. A str, an int, a float, a list, or a dict takes a line of the input,
which the console reads in that shape. A choice offers its values: a bool offers yes and no, and a Literal offers
each of its values, once the console answers a Literal. A click on a choice of the desk sends it. A notice takes no
answer: ⌃A or a click dismisses it.

![Operator question](screenshots/08-operator-question.png)

![Operator notice](screenshots/22-operator-notice.png)

⌃A opens the dialog of the question that you act on, with its whole message. Tab and ⇧Tab go to the next and the
previous ask, and show its chain under the dialog. An answer goes on to the next ask that waits, and the dialog
closes when none waits.

![Answer dialog](screenshots/53-question-dialog.png)

![The next question](screenshots/54-next-question.png)

Each answer lands where you can see it: the card of the question shows your answer, the desk drops its mark, and the
footer says where the answer went and how many asks still wait.

![An answer that landed](screenshots/55-answer-landed.png)

![Answer dialog of one question](screenshots/09-operator-dialog.png)

A message, a question, a notice, and an answer show as markdown, with no marks left: headings, emphasis, links,
lists, quotes, and tables. A code block stands on a surface of its own, with its language as a label above it. The
TUI colors the code of Python, TypeScript, JavaScript, and JSON. The code of another language keeps its lines and
its indentation, and a line too long for the block does not wrap.

![Markdown](screenshots/51-markdown.png)

## Python input

The input grows as you type, and its bar and the line under it name the mode: Prompt, Python, Answer, or Edit
program. Each part of that line is a button: the mode, the chain, the model, the effort, and the type of the answer.
⌃R writes Python with the same gate as the model. The bar and the line under the input take the color of
Python.

![Python input](screenshots/10-python-input.png)

## Value inspector

The pointer on a name shows its current value. ⌃G, or a ⌃click on a name, opens its value, its fields, and its
definition.

![Value inspector](screenshots/11-value-inspector.png)

## Themes

Paper is light, and Midnight and Forest are dark. ⌃T opens the themes, each with a swatch of its colors.

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
session at work, `◆` for a question that waits for you, `◇` for a notice that you did not dismiss, `◌` for paused
work, `✓` for a done act, `✗` for a failure, `●` for a chain you started or a session that finished while you were
away, and `○` for rest. F1 lists the chords that the terminal in use sends, what each mark means, and every slash
command.

![Help](screenshots/16-help.png)

## Narrow terminal

On a narrow terminal the sidebar is hidden, and the top line keeps the session, the chain, and the views.

![Narrow terminal](screenshots/17-narrow.png)

## Work in progress

A command streams its output while it runs, and a model shows the last lines that it writes. A word that the gate
refuses shows the line and the reason.

![Command streaming](screenshots/18-live-command.png)

![Model progress](screenshots/19-model-progress.png)

![Gate findings](screenshots/20-gate-findings.png)

## Paused work

A session that opens with unfinished work stays paused until you resume it.

![Paused resume](screenshots/21-paused-resume.png)

## A record to read

A record whose replay drifts, as a record that an earlier engine made, opens to read. The TUI shows what the record
kept: each chain, each rung with its word, and each act. No word runs again, no model is asked, and a command that
would change the life is refused. A dialog names the engine that made the record and the engine of this build, and
offers to read the record or to start a new life in its workspace.

![A record to read](screenshots/56-record-to-read.png)

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
sessions, and a click on a session opens it. Open sessions keep running while another is selected. A dot shows the
state of each session and workspace, from live work and saved records: blue for work, yellow for input needed or
paused work, green for an unread completion, and red for an error. An open grey dot means ready or saved, and a click
on a completed session clears its unread mark. A rung folds its program to one line and
keeps its acts, a workspace folds to its name, and ⌃\ hides the sidebar. ⌃W lists the workspaces and their
sessions, each with its state, the time since its last save, its cost, and its size.

![Workspace tree](screenshots/28-workspace-tree.png)

![Collapsed rung](screenshots/29-collapsed-rung.png)

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
moves it to the trash of its workspace and removes its scratch directory.

![Image attachment](screenshots/36-image-attachment.png)

![Share conversation](screenshots/37-share-conversation.png)

![Delete session](screenshots/38-delete-session.png)

## External editor

⌥E edits the draft in `VISUAL` or `EDITOR`.

![External editor](screenshots/39-external-editor.png)

## Suggestions

`@` suggests project files, `/` suggests slash commands, and `:` at the start of a word suggests emojis by their
shortcodes of GitHub, as you type.

![File suggestions](screenshots/40-file-picker.png)

![Command suggestions](screenshots/41-slash-suggestions.png)

Tab or Enter puts the emoji in place of `:` and its shortcode, and a whole `:shortcode:` becomes its emoji. Escape, a
space, or a shortcode that matches no emoji closes the list and keeps the text as it is.

A space after a command whose values are known lists them: the models, the efforts, the shapes, the themes, the
paths of the project, the chains, and the acts. Enter on a value runs the command.

![Value suggestions](screenshots/46-value-suggestions.png)

## Retried words

A word of a model can fail: the gate refuses it, or it raises. When the next word of the same prompt takes its
place, the failed word folds and reads as retried, in a quiet color. A failure that nothing replaced stays open, in
red.

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

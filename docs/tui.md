# furb terminal workspace

Keys use the marks of macOS: ⌃ is Control, ⌥ is Option or Alt, and ⇧ is Shift. These captures come from the real
OpenTUI renderer and native engine. The demo World supplies scripted model
answers, temporary files, and real local commands. Run `bun run screenshots` from the repository root to capture
them again, and `bun run animation` to record the animation below. The generators are in `tui/script/`.

![The furb TUI in action](furb.gif)

## Welcome

An empty feed shows the logo of furb, what it does, the project and the model, three ways to start, and the keys
to know. A click on a way to start puts its prompt in the input. The top line names the session and the chain, and
the toggle at its right switches the views. GitHub Dark is the default.

![Welcome](screenshots/01-welcome.png)

## Feed

Your message stands in a panel. Each rung shows the numbered Python that the model wrote, and the acts that it
made stand under it with a preview of their output. The answer comes under the name of the model that gave it.
The sidebar shows the chains, the context and the cost, and the workspaces.

![Feed](screenshots/02-feed.png)

## Transcript

The exact text that the built-in World sends to the model, turn by turn.

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

A question that a model asks you stands in the feed with a yellow bar. The input takes its answer, or ⌃A opens
a dialog for it.

![Operator question](screenshots/08-operator-question.png)

![Answer dialog](screenshots/09-operator-dialog.png)

## Python input

⌃R writes Python with the same gate as the model. The bar and the line under the input take the color of
Python.

![Python input](screenshots/10-python-input.png)

## Value inspector

⌃G, or a ⌃click on a name, opens its value, its fields, and its definition.

![Value inspector](screenshots/11-value-inspector.png)

## Themes

Paper is light, and Midnight and Forest are dark. ⌃T opens the themes, each with a swatch of its colors.

![Paper theme](screenshots/12-light-theme.png)

![Midnight theme](screenshots/13-midnight-theme.png)

![Color themes](screenshots/14-theme-picker.png)

## Chains

⌃B lists the chains, each with its state and the chain it branched from.

![Chains](screenshots/15-chains.png)

## Keys, marks, and commands

F1 lists the chords that the terminal in use sends, what each mark means, and every slash command.

![Help](screenshots/16-help.png)

## Narrow terminal

On a narrow terminal the sidebar is hidden, and the top line keeps the session, the chain, and the views.

![Narrow terminal](screenshots/17-narrow.png)

## Work in progress

A command streams its output while it runs, and a model shows each word as it writes it. A word that the gate
refuses shows the line and the reason.

![Command streaming](screenshots/18-live-command.png)

![Model progress](screenshots/19-model-progress.png)

![Gate findings](screenshots/20-gate-findings.png)

## Paused work and saved sessions

A session that opens with unfinished work stays paused until you resume it. ⌃O lists the saved sessions of the
workspace, with their state, the time since their last save, their cost, and their size.

![Paused resume](screenshots/21-paused-resume.png)

![Saved sessions](screenshots/22-sessions.png)

## Rewind

Escape twice opens the rewind tree in the feed. Each act stands under the act that made it, and each branch under
the point it starts from. Enter on a message gives it back to the input on a new branch.

![Rewind tree](screenshots/23-rewind-tree.png)

## Empty, loading, and failed views

A view with nothing to show says why in its middle. An act that fails keeps its failure in the feed, and a view
that cannot load offers to read it again.

![Empty results](screenshots/24-empty-results.png)

![Loading](screenshots/25-loading.png)

![Act failure](screenshots/26-error.png)

![View error](screenshots/27-view-error.png)

## Workspaces

The sidebar groups sessions under project folders, and its list scrolls on its own. Status dots come from live work
and saved records. Open sessions keep running while another is selected. A rung folds its program to one line and
keeps its acts, a workspace folds to its name, and ⌃\ hides the sidebar. ⌃W opens the workspaces and sessions
picker.

![Workspace tree](screenshots/28-workspace-tree.png)

![Collapsed rung](screenshots/29-collapsed-rung.png)

![Collapsed workspace](screenshots/30-collapsed-workspace.png)

![Hidden sidebar](screenshots/31-hidden-sidebar.png)

![Workspace picker](screenshots/32-workspace-picker.png)

## Session tree

`/tree` opens the same tree as rewind, with the pointer on the chain shown.

![Session tree](screenshots/33-session-tree.png)

## Queued follow-ups

⌥Enter queues a message until the current work completes. Up in an empty input takes the last one back to edit
it, and `/queue` edits or removes the others.

![Queued follow-up](screenshots/34-queued-follow-up.png)

![Queue controls](screenshots/35-queue-editor.png)

## Images, sharing, and sessions

`/image` or ⌃V attaches an image. `/share` writes a standalone HTML conversation. `/delete` moves a session to
the trash of its workspace.

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

An extension adds commands to the palette and to the suggestions. To play an extension, name it in `config.json` of
the config directory, or in `.furb/config.json` of the project, which a project shares in version control:

```json
{ "extensions": { "project-summary": "../tui/examples/project-summary", "grant": false } }
```

A path, a git remote and an npm package each name an extension, and `false` turns one off, a builtin among them.
The next session plays the change. `/extensions` lists the extensions of the session, and `/extensions update`
fetches each one again. [The guide of the extensions](extensions.md) says more.

![Extension command](screenshots/42-extension-command.png)

## Undo and redo

`/undo` removes the last message from a new branch, and `/redo` returns to the branch before it.

![Undo message](screenshots/43-undo-message.png)

![Redo message](screenshots/44-redo-message.png)

## Stash

⌃S puts the input aside, and the line under the input shows what waits. ⌃S again brings it back.

![Stash](screenshots/45-stash.png)

# furb terminal workspace

These captures come from the real OpenTUI renderer and native engine. The demo World supplies scripted model
answers, temporary files, and real local commands. Run `bun run screenshots` from the repository root to
capture them again. The generator is `tui/script/screenshots.ts`.

## Workspaces and sessions

One collapsible tree groups sessions under project folders. Status dots come from live work and saved
records. Open sessions keep running while another is selected. The group dot shows the state that needs
attention first.

![Workspace tree](screenshots/31-workspace-tree.png)

The other view captures below keep the left sidebar hidden. The workspace controls remain in the header.

## Welcome

Start from a feed with space between turns and a growing input panel, with one right sidebar.
GitHub Dark is the default.

![Welcome](screenshots/01-welcome.png)

## Conversation

Markdown results, short code and output previews, expandable act details, and live usage.

![Conversation](screenshots/02-conversation.png)

## Python program

Accepted words with offline syntax colors, line numbers, and name inspection.

![Python program](screenshots/03-program.png)

## Act activity

Prompts, commands, results, and controls for each act.

![Act activity](screenshots/04-activity.png)

## Facts

Search the facts heard by the life.

![Facts](screenshots/05-facts.png)

## Exact transcript

The text the built-in World sends to the model.

![Exact transcript](screenshots/06-transcript.png)

## File changes

Before and after lines from real writes through the World.

![File changes](screenshots/07-changes.png)

## Command palette

Search actions without leaving the keyboard.

![Command palette](screenshots/08-command-palette.png)

## Models

Choose a model from the chain's roster. Effort has its own picker.

![Models](screenshots/09-models.png)

## Effort

Choose one of the efforts that the selected model offers.

![Effort](screenshots/26-effort.png)

## Operator question

A question belongs to its chain and keeps its requested shape.

![Operator question](screenshots/10-operator-question.png)

## Python input

Write and color Python, then run it through the same gate as a model.

![Python input](screenshots/11-python-input.png)

## Value inspector

Read a live value and expand its fields.

![Value inspector](screenshots/12-value-inspector.png)

## Paper theme

A light theme for all views and dialogs.

![Paper theme](screenshots/13-light-theme.png)

## Midnight theme

A second dark theme with the same syntax and state colors.

![Midnight theme](screenshots/14-midnight-theme.png)

## Chain navigation

Switch between the root and its branches.

![Chain navigation](screenshots/15-chains.png)

## Keyboard and commands

Find the keys and slash commands.

![Keyboard and commands](screenshots/16-help.png)

## Narrow terminal

The right sidebar makes room for the main view.

![Narrow terminal](screenshots/17-narrow.png)

## Typed answer dialog

Answer a question without losing the current draft.

![Typed answer dialog](screenshots/18-operator-dialog.png)

## Gate findings

Read the findings for a refused word; it does not run.

![Gate findings](screenshots/19-gate-findings.png)

## Paused resume

Saved work stays paused until the operator chooses to resume.

![Paused resume](screenshots/20-paused-resume.png)

## Saved sessions

Choose a saved life and inspect its pending work.

![Saved sessions](screenshots/21-sessions.png)

## Command streaming

Read stdout while the command is still running.

![Command streaming](screenshots/22-live-command.png)

## Model progress

Keep using the workspace while a model request is in flight.

![Model progress](screenshots/23-model-progress.png)

## Rewind transcript

Choose an act by lineage. An operator rung records the engine's filter, so the selected transcript comes
back after a reopen. The module and files keep their current state.

![Rewind transcript](screenshots/24-rewind-transcript.png)

## Prompt REPL

Open a prompt's program with its own input draft. Run operator Python or edit the prompt's door.

![Prompt REPL](screenshots/25-prompt-repl.png)

## Empty results

A search with no matching facts has an empty state in the feed.

![Empty results](screenshots/27-empty-results.png)

## Loading

The view shows its pending read while the worker runs a Python word. The interface remains available.

![Loading](screenshots/28-loading.png)

## Act failure

A failed read shows its error with the act that failed.

![Act failure](screenshots/29-error.png)

## View error

A damaged diff journal fails to load. Repair it and refresh the view.

![View error](screenshots/30-view-error.png)

## Collapsed rung

A folded rung shows its name and status. Its state is shared across views and saved with the session.

![Collapsed rung](screenshots/32-collapsed-rung.png)

## Collapsed workspace

A workspace keeps its status visible while its session rows are folded.

![Collapsed workspace](screenshots/33-collapsed-workspace.png)

## Hidden workspace sidebar

The feed takes the left sidebar's columns when it is hidden.

![Hidden workspace sidebar](screenshots/34-hidden-workspace-sidebar.png)

## Workspace picker

Find a project or session from the keyboard, with the same live status as the tree.

![Workspace picker](screenshots/35-workspace-picker.png)

## Session tree

Follow each chain's source parent. Left and Right fold branches; Enter opens a chain.

![Session tree](screenshots/36-session-tree.png)

## Queued follow-up

A message waits until the chain's current prompt and rungs finish.

![Queued follow-up](screenshots/37-queued-follow-up.png)

## Queue controls

Edit or remove a message before it is sent. Saved queues wait for a resume choice.

![Queue controls](screenshots/38-queue-editor.png)

## Image attachment

An image is copied beside the record. Its reference enters the prompt, and the World sends the image bytes
through pi-ai. Click the attachment row to preview or remove a draft image.

![Image attachment](screenshots/39-image-attachment.png)

## Share conversation

Export a standalone HTML file with images and the exact transcript. The dialog can open it, copy its path,
or upload it and a Markdown copy to an unlisted GitHub gist when the user chooses that action.

![Share conversation](screenshots/40-share-conversation.png)

## Delete session

Choose a session, then move its record and companion files to the workspace trash. Other sessions keep running.

![Delete session](screenshots/41-delete-session.png)

## External editor

The configured editor writes the draft file and returns it to the TUI. This capture uses a local editor
fixture that writes Python through the same external-editor path.

![External editor](screenshots/42-external-editor.png)

## File picker

Type `@` to search project files and insert a reference. The selected file is read with the next message.

![File picker](screenshots/43-file-picker.png)

## Extension command

A loaded TypeScript extension adds a command to the same palette and completion list.

![Extension command](screenshots/44-extension-command.png)

## Undo message

Undo opens a durable filtered branch and restores the message draft, including image attachments. The
module and files keep their current state.

![Undo message](screenshots/45-undo-message.png)

## Redo message

Redo returns to the origin of that undo.

![Redo message](screenshots/46-redo-message.png)

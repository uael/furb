# furb terminal workspace

These captures come from the real OpenTUI renderer and native engine. The demo World supplies scripted model
answers, temporary files, and real local commands. Run `bun run screenshots` from the repository root to
capture them again. The generator is `tui/script/screenshots.ts`.

## Welcome

Start a conversation, choose an action, or create a chain.

![Welcome](screenshots/01-welcome.png)

## Conversation

Markdown results, structured acts, code, and live usage.

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

## Models and effort

Choose a model and effort from the life’s roster.

![Models and effort](screenshots/09-models.png)

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

The side panels make room for the main view.

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

Choose an act by lineage. The engine's filter selects what the new chain reads; its module and files keep
their current state.

![Rewind transcript](screenshots/24-rewind-transcript.png)

## Prompt REPL

Open a prompt's program with its own input draft. Run operator Python or edit the prompt's door.

![Prompt REPL](screenshots/25-prompt-repl.png)

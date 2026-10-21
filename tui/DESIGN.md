# The TUI of furb: a design

This document says what the TUI of furb shows and why. It gives the principles, three directions that differ at
the root, and the direction that we build.

## What is different in furb

A harness of tool calls has one conversation. The model asks for a tool, the harness runs it, and the model reads
the result. A chat feed is the correct picture of that loop.

furb is not that loop:

- A life is a tree of chains. Each chain is one conversation, and its transcript is python. A conductor on the root
  can hire five workers, each on a chain of its own, and the workers ask back on the chain that hired them.
- Every thing that happens is an act with a name: bash3, read7, prompt12, chain4. An act starts, and later it is
  done with a typed outcome. Many acts run at the same time, and a word waits on many.
- A prompt is the one channel of prose. The operator is an actor that answers prompts, as a model is.
- Each act takes the same controls: pause, wake, cancel and close. A grant puts a ceiling on a chain.
- The name of a prompt is the door of its ladder: a program that you can edit. A write of the door makes the ladder
  again from the first word that changed.
- The record replays a life exactly. Time is a fact of the life, and you can go back along it.
- The work of AI here is a set of convergence loops. Each loop has a measure that goes down and a fixed stop.

A chat feed shows one chain at a time, as a long column of text. It hides the tree, the time, the names and the
controls. The old TUI is a good chat feed. The new TUI must show the life.

## Principles

1. Show the life, not a conversation. The unit of the screen is the act. A message is one kind of act.
2. Show each act with its name, its place in time, and its state. The name is the handle: you can type it, find
   it and control it.
3. The operator is an actor. The prompts that wait for you are an inbox of debts, not a chat box. The screen
   always shows how many you owe.
4. Give attention only to what needs you. A calm screen tells you that all is well. Color is for state, and the
   strong colors are for work that waits for you or that failed.
5. One verb, one key. The keys are the verbs of the engine, and each key works on the thing that you select: a
   chain, a prompt, a rung or an act. The same key pauses a command and a worker of five chains.
6. Time is an axis that you can move along. The record replays the life, so the screen can show the life at any
   point, and you can branch from that point.
7. The program is the conversation. The words of a ladder are text that you can read, edit and replay.
8. Show measures, not prose. The cost against its ceiling, the share of the window, the count of retries of a
   ladder: these tell you if a loop converges.
9. The keyboard reaches all, and the mouse reaches all. A key and a click do the same thing.

## Direction A: the Desk

The Desk is the screen of a person who manages work. It shows prompts and nothing else. The prompts are on three
columns: the prompts that you owe, the prompts that models work on, and the prompts that are settled.

```
 furb  fieldnotes                                        3 owed   5 at work   $1.84 of $5.00
 Owed to you                        │ At work                           │ Settled
 ◆ prompt41  w ui asks          2m  │ ⠦ prompt34  w tui     opus/high   │ ✓ prompt12  w docs
   Which effort should the next     │   rung242  bun run screenshots    │   github.com/uael/furb/pull/238
   step take?               str     │ ⠦ prompt52  w lint    opus/medium │ ✓ prompt19  w crate
 ◆ prompt44  conductor asks     5m  │   rung17  pytest -q        01:12  │   Merged #231
   Merge #240 now?          bool    │ ◌ prompt55  w gif     paused      │ ✗ prompt23  w ts
 ○ prompt47  w docs tells           │   ceiling $0.50 reached           │   Refused: no actor
   The gallery is captured.  None   │                                   │
 ───────────────────────────────────┴───────────────────────────────────┴──────────────────────
 ◆ prompt41  Which effort should the next step take?                          answer as str
 ▏ high
```

How you work in it:

- You triage. Up and Down move in a column, Left and Right move between the columns. Enter on a prompt that you owe
  puts the cursor in the answer line, and the answer closes the prompt with a value of its shape.
- n makes a new prompt: four fields, the actor, the chain, the shape and the message.
- The keys of the controls work on a card: space pauses or wakes it, x cancels it, g puts a grant on its chain.
- An automated life is easy to read: the conductor is one card at work, its workers are cards under it, and only
  their questions come to the first column.

What it costs: the Desk hides how the work is done. You see no word, no command and no tree. To steer a worker you
must leave the Desk.

## Direction B: the Loom

The Loom shows the life as threads on a loom. Each chain is a thread, a row of the screen, in the order of the
tree. Time goes from left to right, and each column is a moment of the record. The cells of a row show what the
chain did at that moment. The operator has a thread too, at the top: the inbox.

```
 furb  fieldnotes                                          ◆ 2 owed   ◉ 4 at work   $1.84 of $5.00
 ◆ you        operator     ◆┊────◆────────────────────────                    2 owed
 ▾ ◉ conduct  opus/xhigh   ·┊p·p·p·p·p··w━━━━━━━━━━━━━━━━━━⠦  wait3     $0.92 of $5.00  ▕██▋  ▏
 ├ ◉ w tui    opus/high    ·┊r·b$$b·r·rw·b━━━━━━━━━━━━━━━━⠦  bash180   $0.31 of $1.00  ▕█▋   ▏
 ├ ✓ w docs   opus/medium   ┊ ·r·b··b·rw·b$$·c✓                        $0.22           ▕█    ▏
 ├ ◆ w ui     opus/medium   ┊   ·r·b·rr·p◆━━━━━━━━━━━━━━━━━━  asks you  $0.18           ▕▊    ▏
 └ ◌ w gif    opus/medium   ┊     ·r·b·b·rr·b━━━◌              ceiling   $0.50 of $0.50  ▕█▉   ▏
 ─────────────────────────────────────────────────────────────────────────────────────────────────
 w tui  chain12  prompt34  str  "You are the worker of the lane tui on the repository …"
   ✗ rung224  ```python                                       refused  line 1: unexpected `
   ✓ rung231  r = await bash("git fetch -q origin main && …") bash111 ✓  cd6 ✓  read111-116 ✓
   ⠦ rung242  b = await build                                 bash180 ⠦ 01:12
              ▏ 18-live-command.txt
              ▏ 19-model-progress.txt
 ─────────────────────────────────────────────────────────────────────────────────────────────────
 › w tui ▏ ask a model, or : for python                                             opus/high  str
```

The screen has three parts:

- The loom, at the top: the operator, then each chain of the tree, under the chain whose word made it. A row gives
  the state of the chain, its label, its actor, and the thread of its acts. A cell is the strongest event of the
  chain at that moment: a dot for an act that ended well, a heavy line for work that runs, ◆ for a question to you,
  ✗ for a failure, ◌ for a pause. The thread of a running chain ends at the present, with the name of the act that it
  waits on.
- The measure of each thread stands at its right edge: its dollars against the ceiling of its grant, and the share
  of the window that its last answer read, as a bar. A loop that converges keeps these low and steady.
- The loom folds time. A moment is a start or an end of an act. The newest moments take a column each, and older
  moments share a column, more of them the older they are, so that a long life fits a terminal and the present
  has the most room. A ┊ marks where the folded past begins.
- A conductor loop is one thread with its workers under it. Left folds the workers into the thread of the conductor,
  which then counts them by state, as 3 ◉ 1 ◆ 1 ◌, and Right shows them again.
- The thread, in the middle: the chain that you select, as its ladders. Each prompt on the chain is a line, each
  rung of its ladder is a line under it with the first line of its word, and the acts of the rung follow on the same
  line by their names. A running command shows the tail of its stream.
- The word line, at the bottom: prose makes a prompt on the selected chain, and a line after : is a word of python
  that runs as a rung there.

How you work in it:

- Up and Down move between the threads, and the thread below follows. Tab goes to the oldest prompt that you owe,
  on any chain, and Enter answers it.
- Enter opens what you select: a chain opens its thread, a rung opens its word as python with all that it told, and
  an act opens its lens: the whole stream of a command, the text of a read, the change of a write, the message and
  the value of a prompt.
- The keys of the verbs work on the selection: space pauses or wakes, x cancels, c closes with a value, g grants,
  e edits the door of a prompt, b branches the chain at the point you select.
- [ and ] move a cursor back and forward in time on the loom. The thread shows the chain as it was at that moment:
  what had not started is not there, and what ran is shown as running. b at that point makes a branch that keeps the
  transcript up to it.
- An automated life is the case that the Loom is made for. The conductor and its five workers are six threads that
  run at the same time. You see which ones run, which ones wait for you, which one stopped at its ceiling, and which
  one fails again and again. You go into one of them only when it needs you.

What it costs: the Loom is dense, and a person who knows only chat must learn to read it. The cells of a thread are
small, so the thread below must give the detail.

## Direction C: the Program

The Program shows the life as source code. The ladder of a prompt is a file that you edit, and the outcome of each
act stands in the margin, at the line that made it. The chains of the life are the files of a tree at the left.

```
 chains            │ chain12 › prompt34  str                      door prompt34   ⌃S writes and replays
 ▾ conduct  ◉      │   1  r = await bash("git fetch -q origin main && git worktree …")  │ bash111 ✓ 0.8s
   ├ w tui   ⠦     │   2  if r.code == 0:                                               │
   ├ w docs  ✓     │   3    cd("/Users/uael/src/github.com/uael/furb-w-tui")            │ cd6 ✓
   ├ w ui    ◆     │   4    for path in ["src/furb/CLAUDE.md", "src/furb/engine.pyi"]:  │
   └ w gif   ◌     │   5      read(path, span(1, -1))                                   │ read111-112 ✓
                   │  ┄┄ rung242 ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
 watch             │   6  build = bash("nice -n 20 bun run build", timeout=3600)       │ bash123 ⠦ 8m54s
 ◆ prompt41 w ui   │      ▏ Compiling monty v0.0.23
 ◌ grant4 w gif    │   7  b = await build                                              │ waits
```

How you work in it:

- You read and edit the program of a ladder as text, with the colors and the gate of python. A save writes the door.
  The engine keeps each rung whose word did not change, with its acts and their outcomes, and runs the rest again.
  The margin shows which outcomes stay and which run again.
- The words of the model and your words are one program. A line that you add runs as a rung of the ladder.
- The watch list at the left holds the prompts that you owe and the acts that you pin.
- An automated life is a tree of files. You open one worker at a time, and the tree tells you which worker needs you.

What it costs: the Program shows one ladder at a time. It is the strongest tool to steer one worker, and the
weakest view of many workers at once.

## The choice: the Loom

We build the Loom, and it takes the best part of the two other directions:

- The Desk becomes the thread of the operator. The operator is an actor, so its inbox is a thread of the loom, as
  the thread of a model is. Tab goes from debt to debt, as the Desk does.
- The Program becomes the deepest level of the Loom. A rung opens as its word, and e opens the door of its prompt
  as the program of the ladder, which a save writes and replays.

Why the Loom:

- It is the only direction whose first screen shows a tree of many chains that run at the same time. Automation is
  the case that the owner names, and the Desk and the Program each show one item at a time.
- It is the only direction that gives time an axis. The record replays a life exactly, and a cursor in time is the
  natural way to show that, and to branch from a point.
- It shows every act by its name. The name is what a word, a door and a control use, so the screen and the engine
  use one vocabulary.
- It is calm when the work is calm, and it tells you at once where the work needs you: a ◆ on a thread, a ✗ that
  comes again, a ◌ at a ceiling.
- No harness of tool calls can show it, because no harness of tool calls has a tree of conversations that run at the
  same time.

### The screens

1. The loom and its thread: the first screen, as the sketch of direction B shows. With no chain at work, the loom
   shows the operator and the root, and the thread offers three ways to start.
2. The lens: an act whole, in place of the thread. A command shows its streams, live while it runs, and its stdin
   when it is open. A read shows its text, a write its change, a prompt its message and its value, a rung its word and
   all that it told.
3. The door: the program of a ladder in the editor, with the gate of python. A save writes the door.
4. The answer: the prompt that you owe, with its message whole and its shape, in the word line.
5. The lives: the workspaces and their saved lives, each with its state, its cost and the time of its last save,
   as a list that you open with ⌃O.
6. The help: every key and every command.

### The keys

| Key | What it does |
| --- | --- |
| Up, Down | Select the thread above or below, or the line above or below in the thread |
| Left, Right | Fold the workers of a thread into it, or show them again |
| Tab | Go to the oldest prompt that you owe |
| Enter | Open the selection: a chain, a rung, an act, or the answer of a prompt |
| Esc | Go back one level |
| space | Pause or wake the selection |
| x | Cancel the selection |
| c | Close the selection with a value |
| g | Put a grant on the chain of the selection |
| e | Edit the door of the prompt of the selection |
| b | Branch the chain at the selection, or at the time cursor |
| [ and ] | Move the time cursor back and forward. Esc puts it back at the present |
| : | Write a word of python on the selected chain |
| / | Type a command, as the old TUI takes them |
| ⌃O | Open the list of lives |
| F1 | Help |

### What the Session must give

The TUI reads the life through the Session of `bind/typescript` alone. The Loom needs one view that the Session
does not give: the place of each act in time. Each `LiveAct` gets `began` and `ended`, the count of facts of the life
when the act was made and when it was done. This is a logical clock: it is the order of the record, so a later life
gives the same values, and the loom of a replayed life is the same loom.

Everything else is in the Session now: the acts of every chain, the prompts that the operator owes, the streams of
the models, the controls, the doors and the pending work.

### What goes, and what stays

- The feed of cards goes. The thread and the lens take its place.
- The transcript view and the changes view become lenses: the transcript of a chain, and the change of a write.
- The sidebar goes. The loom is the tree of chains, and the list of lives takes the workspaces.
- The slash commands stay, behind /, since they act through the Session and not through a screen.
- The themes stay, with their palettes.

### The plan of the work

1. This document, in its own pull request.
2. The view of time in the Session, and the Loom on the demo session: the loom, the thread, the word line, the
   answer, the keys of the verbs. New tests replace the tests of the old screens, and the captures are made again.
3. The lens, the door, and the time cursor with its branch.
4. The lives, the help, and the narrow terminal. The Loom on a real life.

## As built

The first version of the Loom follows this document, with these changes that the work taught:

- A cell of a thread is a height, not a glyph. The lowest cell, `▁`, is work that runs, and a taller cell holds more
  acts that began or ended at that moment. A cell is yellow where the chain asks the operator, and red where an act
  failed. The cells of the past fade, so the present is the brightest part of the loom. One kind of mark keeps the
  loom quiet, and the colors carry the states.
- A short life stretches each moment over as many as three columns, so its threads fill the loom. A long life folds
  its past: the newest moments take a column each, and older moments share a column, twice as many at each step back.
- The word line asks a model with `i`, takes python with `:`, a command with `/`, and a command of the shell with `!`.
  The verbs are single keys only while the word line is not focused, and Esc leaves it.
- The list of lives, ⌃O, takes the place of the sidebar of workspaces.
- The Session of `bind/typescript` gives two new views: the place of each act in time, as `began` and `ended` on each
  act, and the answers of the models on each chain, as `Activity.answers`, from which the TUI reads the dollars and
  the share of the window of each thread. It also follows reads, writes and changes of directory as acts, so a thread
  names them.

- The time cursor moves by one moment with `[` and `]`, and by ten with `{` and `}`. It stands at a moment of the
  record: an act that began later is not shown, and an act that ended later runs, with no value yet. The verbs that
  change the life do nothing in the past, and `b` branches the selected chain at the cursor: the branch keeps the
  acts of the chain up to that moment, and the module and the files keep the state that they have now.

What is left for later work: the feed, which the next part of this document designs; the commands of the old TUI that acted on its screens alone, such as the share of a
conversation as HTML, images in the word line, the external editor, and the queue of follow-ups in the screen; a
measure of a convergence loop beyond the dollars and the window, such as the count of retries of a ladder; and the
replay of a record that an older engine made, which drifts, so an old record does not open.

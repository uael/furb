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

## The variants

The owner judged the Loom, and it did not give the effect that he expected. He gave three hints: code, debugger and
graphs. They are one direction: a life of furb is a program that runs, so the TUI is the debugger of a living
program. The TUI now holds variants. Each variant is a whole direction, not a skin, over one model of a life.
`--variant loom` or `--variant graph` chooses one when the TUI starts, and `v` changes it in a running TUI, so that
the owner can try each one on the same life and choose.

### One model of a life

Each variant reads the same views, which the Session gives and which make nothing:

- The acts, each with its place in time, `began` and `ended`, and its state.
- The loom, in `loom.ts`: the chains in the order of the tree, with the cells of their moments.
- The ladder, in `ladder.ts`: the prompts of a chain, the rungs of each, and the acts of each rung.
- The threads, in `threads.ts`: the tree of the prompts of the life, as the next part says.
- The map, in `map.ts`: the place of each chain on a plane, and the messages between the chains.
- The source, in `source.ts`: the words of the ladders of a chain as one program, with each act on the line that
  made it.

The time cursor is one for every variant: a variant shows the acts as they stood at the cursor.

### Threads

A thread is a prompt, as the owner rules. One party asks another for a result of a shape. A prompt of None on an
open prompt is a new message of that thread, until the thread closes. A prompt that a rung of a thread makes is a
sub-thread of that thread. So the tree of threads is the tree of the prompts of the life: the Session holds it now,
and each prompt holds its messages in order, its rungs and its notes, and its value at the end.

- The parent of a prompt is the prompt of the ladder of the rung that made it. A prompt that the operator made is at
  the top of the tree.
- A thread can hang on an act. A review is a prompt about a write, a word, a command or a PR. The anchor of a prompt
  is the act that it stands on, when its `on` names an act and not a chain. Until the engine takes that, the anchor
  is the first act of the life that its message names, as `write7`, a place in a file, as `src/search.ts:3-5`, or a
  pull request, as a URL of GitHub.
- The progress of a thread is how many of its sub-threads closed, of all of them.
- A thread shows what waits for the operator under it: each open prompt to the operator in its tree.
- A thread shows its pull requests: each URL of a pull request in a value of its tree, or in the output of a command
  of its rungs.

The first life designed threads too, on branches that it closed as stale. This design takes three things from them:
the comment of a word is a step that a person reads, the change of a write stands under the word that made it, and
the taste matters as much as the layout: the accent colors, the motion and the small typographic rules.

### The graph: the debugger of a living program

The graph shows a life as programs that run, and the messages between them. Zoom is semantic. Each level shows less
of the life and more of one thing, and Enter goes one level in, and Esc one level out:

1. The threads. The tree of the prompts of the life, as the outline of the life: each thread with its state, its
   first line, its progress as the count of its sub-threads that closed, and what waits for the operator in it. On a
   wide terminal the threads stand in a sidebar, and the map of the selected thread stands beside it. On a narrow
   one the threads take the whole stage, with the anchor of each where it hangs, its chain and its pull requests.
2. The map. Each chain is a program: its mark and its name, and under them, faint, the act that it waits on, its
   model and its dollars. No box holds a program: the space between the programs and the lines of the tree of hires
   give the structure. The programs that the selected thread holds stand bright, and the rest fade. A prompt from one
   chain to another flows along the lines as a dot while it is open, and its answer flows back for a moment when it
   closes. A question to the operator flows up to you, in the color of what waits for you.

   ```
    Threads                                Ship the release 0.4: the search, the docs and the animation.   8 of 14

    ✓ Explore this project, run its ch…     ◆ you
    ⠏ Ship the release 0.4: the…  ◆2 2/9    │ 2 owed
      ✓ Make the search ignore …  ◆1 0/2    │
        ⠏ Review the change of …  ◆1 0/1    ├─ ⠏ Main ────••──•─•─────────┬─ ✓ w search ────•──────•────── ◆ review search
          ◆ Must the search match acce…     │    prompt7  sonnet  $0.0096  │    sonnet  $0.0024               asks you
      ⠏ Write the docs of the rele…  1/2    │                              │
        ⠏ Write the guide of the searc…     ├─ ○ Search shortcut           ├• ⠏ w docs ───────•─•────────┬─ ⠏ docs/guide
        ✓ Write the changelog of the r…     │                              │    prompt11  sonnet  $0.0048 │    bash6  sonnet
      ⠏ Ask me first, then tidy…  ◆1 0/1    │                              │                              │
        ◆ Which effort should the next…     ├─ ○ Review notes              ├─ ◆ w ui                      └─ ✓ docs/changelog
      ◌ Record the animation of the re…     │                              │    asks you  sonnet  $0.0024
    ⠏ Reach parity with Pi: sessio…  2/3    │                              │
      ✓ Close the gap of sessions with…     └─ ⠏ Pi parity ──────────•─•──•─ ◌ w gif
   ```

3. The program. One chain as its source: the words of its ladders, one under the other, each prompt as a line that
   heads its rungs. The line that runs is marked, and the margin holds what each act came to, beside the line that
   made it. At the side stand the stack and the watch: the chain of awaits from the operator to the act that the
   chain waits on now, and the acts of the selected line with their values.

   ```
    w search  chain3  prompt2  Add the search shortcut, with a test
     1  # Read the code of search                          │ stack
     2  search = read("src/search.ts")         read2 ✓ 5 l │   prompt1  Main
     3  t = search.replace("includes", "has")             │   rung5    conduct
     4  write(t)                               write4 ✓ +1−1│   prompt2  w search
     5  r = await bash("bun test")             bash7 ✓ 0  │ ▶ bash9    gh pr create
     6▶ pr = await bash("gh pr create ...")    bash9 ⠦ 4s │ watch
                                                          │   bash9  Creating the PR
   ```

4. The word. One rung, whole: each line of its word, with what each act came to in full, and what the rung told.
5. The act. One act, whole: a command and its streams, live while it runs, a read and its text, a write and its
   change, line by line, a prompt and its messages.

The controls of a debugger act on what the level selects:

| Key | What it does |
| --- | --- |
| space | Pause the selection, as a breakpoint, or wake it, as continue |
| n | Step: wake the selected chain for one rung, then pause it again |
| x | Cancel the selection |
| c | Close the selected act with a value |
| g | Put a ceiling of dollars on the selected chain, a breakpoint on cost |
| e | Edit the door of the prompt of the selection, which replays its ladder |
| i | Ask the model of the selected chain |
| [ and ] | Move the time cursor. The graph shows the life as it stood then |
| b | Branch the selected chain at the selection, or at the time cursor |
| arrows | Move to the next thing in that direction: a node of the map by its place, a line of the source |

The layout of the map is stable. A chain takes its place when it is born and keeps it: its column is its depth in
the tree of hires, and its row is the first free row of that column at or under the row of the chain that hired it.
A new chain never moves another one. The place follows from the order of the record alone, so a later life draws
the same map.

The keyboard reaches all of it. A click on a thread or on a line of the source selects it, and a second click zooms
in. A click on a program of the map is left for later work.

### The finish

The owner asked for the finish of the best apps of its kind: their care of type, space, color and motion, on a
design that is furb's own. In the terminal the same rules hold within its means:

- A small palette with one accent. Dusk is the dark theme and the default, warm grays with a clay accent, and Day is
  the light one, on paper white. Color says the state alone: the accent is work that runs, amber is what waits for
  you, rose is a failure, and a quiet sage is what ended well. The rest is text in three tones.
- Space in place of boxes. No border holds a program, a thread or a panel: the sidebar of the threads stands on a
  surface of its own, and the rest stands on the ground, with space around it.
- State by shape and color, not by labels. The header says ◆ 2, ◉ 5 and the dollars, and shows a count only where it
  is not zero. The composer names its mode only when it does more than ask a model.
- Controls stay out of sight until they are needed. The footer offers the three keys that act most where the focus
  stands, and ? for the rest.
- Motion is short and quiet. A spinner turns where work runs, and a dot walks along a line where a message flows.
  Nothing else moves.

The skills of design that this work read: frontend-design, which this repository holds, for the restraint of one
bold element and the plan of a palette, a type and a layout before the build; ui-ux-pro-max, which #252 vendors, for
its rules of contrast, focus, motion and of the light and the dark themes, and its search of palettes, which gave a
first draft that the brief changed; and opentui, which this repository holds, for the renderables and the keys.

### As built

- The variants are `loom` and `graph`, behind `--variant` and the key `v`. The loom keeps its screens. The graph is
  `tui/src/graph.ts` over `threads.ts`, `map.ts` and `source.ts`, in the App of the loom, which holds the header, the
  composer, the lens, the lives and the help for both.
- `n` steps a chain: it wakes the chain when a pause holds it, and pauses it again once one more of its rungs ends.
- The demo is a life of many threads at once: a release that a conductor hires four workers for, a worker that hires
  two writers, a review that hangs on a change and asks the operator, a worker that asks the operator first, a grant
  that pauses a worker at its ceiling, a command that streams, and a second thread that reaches parity with Pi. The
  words of its model are in `tui/src/script.ts`.
- With a hundred chains, each of which asks the operator a question so that a hundred messages flow, a frame of the
  loom takes about 21 ms, a frame of the threads and the map about 20 ms, a frame of the map alone about 52 ms, and
  a key of the map about 8 ms, as `bun tui/script/measure.ts` measures on a laptop that runs other work. A place of
  the map never moves, so the graph finds the line of each edge and the way of each message once.
- Left for later work: a breakpoint on a kind of act, which the engine does not take; the anchor of a prompt by its
  `on`, which the engine does not take either, so the anchor comes from the message; a click on a program of the map;
  and a third variant.

### The feed

The owner asked for a design of the feed of its own: where a person reads what a chain said and did, from its start
to its end. Three directions differ at the root:

1. The transcript: the turns as the model of the chain reads them, as python. It is the one truth of the engine, and
   it shows what the model saw when an answer is wrong. But it is the input of a model, not a page for a person.
2. The page: the prose of the chain as a conversation, each message and each answer as markdown, and the work
   between them folded. It reads well, but it is a chat feed again, which hides the names, the time and the
   controls, and which the old TUI was.
3. The source: the words of the ladders as one program, with the outcome of each act beside the line that made it,
   the line that runs, and the change of each write under the word that made it.

We choose the source, which is the level "program" of the graph. The program is the conversation: the words are what
the model said, the margin is what the life answered, and the controls of the debugger act on each line. The page
lives in the level "threads", where each thread shows its first line and its answer. The transcript stays a lens for
later work, to read when a model went wrong.

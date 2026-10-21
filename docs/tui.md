# The furb TUI, screen by screen

These captures come from the real OpenTUI renderer and the native engine, with a scripted model, temporary files,
and real local commands. `bun run screenshots` captures them again, and `bun run animation` records the animation
below. The scripts are in `tui/script/`. [The design](../tui/DESIGN.md) says why the screens are made so.

![The furb TUI in action](furb.gif)

## An empty life

A new life shows the loom with two threads, you and the root, and the thread of the root offers three ways to start.
Enter on one puts its prompt in the word line.

![Welcome](screenshots/01-welcome.png)

## The loom

The demo life: the operator asked the root to explore the project, then a word of python made a conductor, which hired
four workers, each on a chain of its own. The loom shows them under the root, each in its state. `w search` did its
item. `w docs` works, and its spinner stands at the present. `w ui` asks you a question, which the thread of the
operator, at the top, counts. A pause holds `w gif` before it starts. The low cells are work that runs, and the taller
cells are moments where acts began or ended. At the right of each thread stand its dollars against the ceiling of its
grant, and the share of the window that its last answer read.

![The loom](screenshots/02-loom.png)

## The thread and the lens

The thread shows the ladders of the selected chain: each prompt, each rung of its ladder with the first line of its
word, and the acts that each rung made, by their names. Enter on a line opens its lens: here, the word of the
conductor, whole, as python.

![The thread](screenshots/03-thread.png)

![The lens](screenshots/04-lens.png)

## A prompt that you owe

Tab goes to the oldest prompt that you owe, on any chain, and the word line takes the answer. Enter closes the prompt
with it, and the worker goes on.

![Answer](screenshots/05-answer.png)

## A conductor, folded

Left folds the workers of a thread into it, and the thread counts them by state. Right shows them again.

![Folded](screenshots/06-folded.png)

## A paused worker

A pause holds the worker `w gif`. Space pauses the selection, or wakes it, and x cancels its work.

![Paused](screenshots/07-paused.png)

## Work in progress

A command streams what it prints under its line, and a model shows what it writes under its rung.

![Command streaming](screenshots/08-live-command.png)

![Model progress](screenshots/09-model-progress.png)

## Python, the gate, and the door

`:` writes a word of python, which runs as a rung on the selected chain. A word that the gate refuses shows its
finding on its line. `e` opens the door of a prompt: the program of its ladder, which a send writes and replays.

![Python](screenshots/10-python.png)

![Gate findings](screenshots/11-gate-findings.png)

![Door](screenshots/12-door.png)

## Lives and keys

⌃O lists the lives of each workspace, with their state and their cost. `?` lists every key, every mark, and what the
cells show.

![Lives](screenshots/13-lives.png)

![Help](screenshots/14-help.png)

## A narrow terminal, and the themes

On a narrow terminal the loom leaves out the measures and keeps the cells. `/theme` changes every surface: Dusk is
the default, Day is its light theme, and GitHub Dark, Paper, Midnight and Forest stay.

![Narrow terminal](screenshots/15-narrow.png)

![Day theme](screenshots/16-day.png)

![Midnight theme](screenshots/17-midnight.png)

## Time

`[` and `]` move a cursor in time on the loom by one moment, and `{` and `}` by ten. The loom and the thread show the
life as it stood at that moment: an act that began later is not there, and an act that ended later still runs. The
header counts the moment, and the right edge of the loom says `then`. In the past the verbs that change the life do
nothing, but `b` branches the selected chain at the moment. Esc returns to now.

![Time cursor](screenshots/18-time-cursor.png)

## The graph

`v` shows the other variant, the graph: the life as programs that run, on a life of many threads at once. A thread
is a prompt, and a prompt that a rung of a thread makes is its sub-thread. The sidebar holds the tree of the
threads, each with its state, its progress and what waits for you in it. Beside it stands the map of the selected
thread: each chain as its mark and its name, the lines of the tree of hires, and a dot that walks along a line while
a message flows, as a question that a worker puts to you, or the brief that a conductor gives a worker.

![The threads and the map](screenshots/19-graph.png)

Enter zooms in, and Esc zooms out. The map selects a program, and the arrows move between the programs by their
places.

![The map](screenshots/20-graph-map.png)

A program opens as its source: the words of its ladders, each line with what its acts came to, and at the side the
stack of awaits and the watch of the selected line.

![The source of a program](screenshots/21-graph-program.png)

A line opens its word whole, with what each act came to under the line that made it, and an act opens its lens: a
write shows its change, and a command its streams, live while it runs.

![A word](screenshots/22-graph-word.png)

![The change of a write](screenshots/23-graph-change.png)

![A command that streams](screenshots/24-graph-command.png)

Day is the light theme of every surface.

![The graph in the theme Day](screenshots/25-graph-day.png)

# A tour of the TUI

This page shows each screen of the furb TUI, with what it does. [The TUI](../tui/README.md) says how to start it,
and lists every key and command.

Keys use the marks of macOS: ⌃ is Control, ⌥ is Option or Alt, and ⇧ is Shift. The pictures come from the real
renderer and the real engine, with a scripted provider of the answers of the model, temporary files, and real local
commands. `bun run screenshots` takes them again, and `bun run animation` records the animation below: it types,
clicks, and waits in a real session, and keeps each picture that changed. The scripts are in `tui/script/`.

![The furb TUI in action](furb.gif)

## Welcome

An empty feed shows the logo of furb, what it does, the project and the model, the ways to start, and the keys to
know. The first way plays a life to watch, which asks no model. A click on each other way puts its prompt in the
input. The top line names the session, the chain, and the
directory of the chain, and the toggle at its right switches the three views of the chain. The footer says what the
session does and offers the keys that act now, each a button that does what its key does. Ink is the default.

![Welcome](screenshots/01-welcome.png)

## First run

On the first run, and on each run while no model answers, the welcome checks the setup: the claude command line and
whether it is signed in, the keys in the environment, the local server, and the models that answer. Each part that is
missing shows its fix. When no model answers, the welcome says so first, then offers only the life to watch, which
needs no model, and the input says so too. While the input is empty, `r` checks the setup again, as after a sign in.
The welcome shows on each chain that holds no act of you or of a model, so the steps that the extensions run as a
chain starts do not hide it. The capture is a real life on a home that holds no model.

![Setup with no model](screenshots/63-setup-no-model.png)

## Watch a life

"Watch a life", or `/watch`, plays a short life on a small project by itself. It asks no model, so the bar at the
top counts 0 model calls and $0.00, as the board does. A short Python program asks three models at once, and a branch
then goes another way from an earlier moment, with the files as they were. The watch says each step in common words:
who wrote it, and what it did, as "read a file". `bun run demo` opens on it.

![Watch a life](screenshots/64-watch-a-life.png)

While the input is empty, `[` and `]` move the feed back and on in time: an act that began later is not there yet,
and an act that ended later is not done yet. `b` starts a branch at the moment shown, with the files as they were
then, and Esc comes back to now.

![An earlier moment](screenshots/65-watch-earlier.png)

The bar stays over the feed as it scrolls, on the smallest terminal too.

![The watch at 80 by 24](screenshots/81-watch-small.png)

## Feed

The captures of the feed show a real life: a conductor and the workers that it hired, recorded from the log of a
life of furb, with 23 chains, 21 of them at work, and ladders of up to 76 rungs. `tui/fixtures/conductor.jsonl`
holds it, with no key and no path of a home in it, and `bun run demo` opens it beside the scripted life.

The sidebar says the whole life at a glance, in one place. On top, Needs you holds each question that waits for you,
the oldest first, with its thread, its message, and a button for each value of a choice, Other among them, which a
click answers. Under it, Chains holds the tree of the chains: each chain on one row, with a bar in a tone of its
own, its mark, the last part of its name, and what it does now in a few words, never a raw command, as "checks of
#364", "runs cargo test" or "waits for feed". A chain that works for a minute or more says since when, in yellow
past fifteen minutes. A chain that failed says why, and one that finished the first line of its answer. A chain
stands in the tree while it or a chain under it needs a look, and the others fold under Finished.

A click on a chain, or Enter after ⌥T, shows that chain and lists its threads in place of the tree, under a row
"← Chains" that gives the tree back, as ← and Esc do. A thread is a prompt on the chain, with the messages that came to
it. Its row has the color of its thread, its mark, as asks you, runs or resolved, and the first line of its message,
and the list scrolls. A click on a thread scrolls the feed to it, and the input then sends each message to it, as
`prompt(None, message, on=thread)`, which its model reads at its next step. A message to a resolved thread opens it
again. Under the thread chosen, Collapse folds it in the feed to its box on one line, Resolve closes it, as
`close(None, thread)` does, and Reply puts the keys in the input at the end of the thread. Space, r and o do the same
after ⌥T, and the header of the thread in the feed holds Collapse and Resolve. At the foot of the sidebar, Context says how full the
window of the chain shown is and where a grant pauses it, its tokens, and what it spent.

The feed shows the threads of the chain, each as one box. A thread is a prompt: its message heads the box as markdown,
with a line at its left that runs down the whole box: it takes the color of the thread at each of your messages, and a
quiet color elsewhere. At the right of the message stands who sent it and to which model when you did not, the type of
its answer, its state and its controls. Each message that came to the thread later stands in the box the same way, and
the answer of the model stands as markdown with no line of color. The words of its steps stand in a block of code, as
one Python program, line by line, with the number of each line in the gutter, in a quiet color. `/gutter right` puts the
numbers right of the block, and `/gutter left` inside it again, while the TUI runs. At the end of an open thread an
input takes your reply, and a reply to a thread that is over opens it again. The numbers go on from one step to the
next, so a thread counts the lines of its program from its first step to its last, also past the steps that a fold keeps
out. No step and no act takes a row of its own. Each line ends with a quiet hint of what its acts came to: a read the
count of its lines, a write the lines that it added and removed, a command its exit and the count of the lines that it
printed, a question its answer or what it waits for, and a loop its count, as x6. The first line of each step says how
long the step ran and what the answers of its model cost. The time is known for the work that the TUI saw run, since the
record holds no time. The line that runs is lit, with a mark in the gutter and, after a second, its time, and the last
lines that its command prints stream under it. A line that the gate refused has a red cross at its end, and the reason
stands under it as a comment. The line that runs has a mark at its end too, and the line that it awaits has an arrow. A
message that you send to the thread, and each notice of the chain, stands as a comment among the steps. While the model
of the thread works on a step, each line of its word that it finished stands above as Python in its colors, and the last
row of the thread is the line that comes: a loader after it, and the line that the model still writes, else the last
line that it thinks, as a comment, or `# thinking` while it shows no thought, whose letters a light crosses. The answer
of the thread stands in a panel of the color of the thread, with the model that gave it and its effort at its right. A
word that you run stands as your messages stand, as its program in a panel of its own color, and so does an act that no
word made, as a command that `/bash` starts: one line of Python that binds the act to its name, as `bash3 = bash("make
docs")`.

![The last row of a thread while its model works](screenshots/84-thinking-row.png)

![The last row of a thread while its model thinks](screenshots/85-thinking-tokens.png)

A click on a line opens it in place: its whole text, cut into rows at the column of the code, and under it what each
of its acts gave, as the output of a command or the text of a read. A second click folds it. A line too long for the
column of the code folds its longest strings first, each to its start and an underlined `...`, until it fits. The
pointer on a `...` shows the whole string, and a click opens it in place, with the rest of its line. A click on the
string folds it again. The record and the word that ran stay as they were sent: the fold is the view alone. The
colors of a line come from its whole statement, so a line that is cut or folded keeps the colors of the code that ran.

![A string under the pointer](screenshots/82-string-hover.png)

![An open string](screenshots/83-open-string.png)

A line of a program shows what it holds when the pointer rests on it, in a card right of the program: the text that a
read gave, what a command printed, the value of each other act of the line, and a message of a thread whole. A line
that the gate refused, or that raised, has a red ✗, and its card gives the error whole. A press on the card opens the
act.

![A read under the pointer](screenshots/97-read-hover.png)

![A refused line under the pointer](screenshots/98-gate-hover.png)

While a word awaits an act that an earlier line made, as `built = await build` after `build = bash(...)`, the line
that awaits runs: it is lit, it names the act and the line that made it, and it shows its time after a second. The
line that made the act is lit too, with ↓, and what its command prints stands under it.

![A line that awaits the act of an earlier line](screenshots/99-awaited-line.png)

The steps that the extensions play on a chain stand on one quiet row. A file of Markdown that memory told shows no row
for each of its headings, and a question of memory that found nothing new shows no row. A prompt of None, a notice,
blocks no word: the word that awaits it goes on at once, and no row of the feed keeps running for it.

![The quiet row of the extensions](screenshots/100-quiet-feed.png)

Collapse on the header
of a thread, or ⌥D, which lists the threads, folds the thread to its box on one line, which says its state, the
first line of its message and the count of its steps, and Expand, or a click on that line, opens it again. A dot
before a folded thread says that work came to it since you folded it, and the mark of a question says that it asks
you. A resolved thread that is
not the last of its chain starts folded, and `/autocollapse` turns that fold off, and on again. A right click on a
name inspects it.

A long thread stays short. The block of code of a thread is a window of seven rows. Its first row is the number 1,
or a ... once the window rolled, the line that runs stands second to last, and the last row is a ... while more lines
come, or the row where the model writes. A click on a ... opens the lines in a block that scrolls, up to four
hundred, and Roll up closes it. The feed shows one line in place of its earlier threads, as "28 earlier messages". A
click on the line shows them. A long message shows its first rows, with a line that shows the rest. A jump to an act
opens each fold over it. The sidebar and the feed come from the model of a life that `furb/life`
gives, so each host shows the same life. Under the chains, the sidebar shows the workspaces, each with the session
shown, each session that works, asks or failed, and its newest sessions up to three, and the older ones fold under
Earlier.

![Feed](screenshots/02-feed.png)

![A long thread](screenshots/60-long-ladder.png)

![Folded steps](screenshots/61-folded-steps.png)

## Transcript

The exact text that the provider sends to the model, turn by turn.

![Transcript](screenshots/03-transcript.png)

## Changes

The review of the files that changed, in one of four scopes: this turn, which holds the writes of the chain shown
since your last message, this chain, the life, and the branch, which holds each file that differs from the base of
the branch against main in git, a file that git does not track yet among them. Click a scope, press ⌃PageUp or
⌃PageDown, or type `/changes turn`, `chain`, `life` or `branch`. The toggle counts the writes of the life.

The list of the files comes first, each with the lines that it added and removed, and a click on a file goes to its
diff. Each file shows the net change of the scope: from its text before the first write to its text after the last.
At 140 columns or more, the two sides of a diff stand side by side, and below that one stands under the other.

![This chain](screenshots/04-changes.png)

![This turn](screenshots/76-changes-turn.png)

![The life](screenshots/77-changes-life.png)

![The branch](screenshots/78-changes-branch.png)

Each hunk has two actions. **Revert** writes the old lines of the hunk back through the World, as a word of yours on
the chain, so the record keeps the write. The word refuses when the file no longer holds the lines of the hunk where
the review shows them. **Comment a line** lists the lines of the hunk. Choose one, and the input holds the start of a
message that names the file, the line and its text: write your comment after it, and Enter sends it to the chain as a
prompt.

![Comment a line](screenshots/79-changes-comment.png)

![The comment in the input](screenshots/80-changes-comment-input.png)

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
something and asks nothing. A notice waits for nothing: it settles in the feed as a panel on one line, as a folded thread
does, with the first line of its message and the count of its other lines, and a click opens it to its whole message. No key
dismisses it. Many questions can wait at once, on many chains. The desk over the input shows what waits: a mark `◆`
for each question, in the order that they came, and the one that you act on in the color of a question, with its place, its chain, its id, and what it wants. Under it stand the first line of
its message, the count of its other lines, and its choices or where the input sends its answer. A click on a mark
goes to its ask and shows its chain, and a click on the message opens it whole.

On the chain shown, the input answers the question that you point at, or else the oldest question of the chain. The
line under the input says so, as `answers prompt2 returns str`, and the card of that question in the feed says that
the input answers it. A click on the card of another question points the input at it.

![Many questions](screenshots/52-many-questions.png)

The words that you write for a question stay with it. When another question becomes the one that the input answers, the
input shows the words of that question, and the first words come back when you go back to it. When a question closes
while you write for it, as when another window answers it, your words go to the history of the input, ↑ brings them
back, and no other question takes them.

![Words kept for a closed question](screenshots/98-words-kept.png)

Each question takes an answer of its shape. A str, an int, a float, a list, a dict, a tuple or a union takes a line of
the input, which the console reads in that shape. A shape that is more than text says what its line is: the input and
the desk name it, with an example, as `a JSON list of text, as ["a", "b"]`. A line of JSON has an item of the shape of
its place, and a line that has not that shape is refused with its reason, and stays in the input. A union that holds
None, as `int | None`, takes an empty line as None.

![A question of a list](screenshots/96-list-question.png)

A choice offers its values as a list in place of the input: a bool offers yes and no, and a Literal offers each of its
values. ↑ and ↓ move the pointer, Enter or a click sends the value, and a number sends the value at that place. The
last row, Other, gives the input to your own words when no value fits, under the values that stay listed. Enter closes
the question with them as Refused, so the await of the word that asked raises them and its model reads them. An empty
field sends nothing and says so. Esc, and ↑ from an empty field, go back to the values, and the words stay for the next
time.

![Other, with the words of the operator](screenshots/94-other-answer.png)

A list of a Literal, as `list[Literal['fast', 'safe', 'quiet']]`, lists its values with a box before each. Space or a
number ticks the value at the pointer or at that place, and Enter sends the ticked values as a list, in the order of the
list. Other gives the input to your own words here too.

![A question of many values](screenshots/95-many-choice.png)

![Operator question](screenshots/08-operator-question.png)

![A question of a choice](screenshots/09-choice-question.png)

![Operator notice](screenshots/22-operator-notice.png)

⌃A opens the dialog of the question that you act on, with its whole message. Tab and ⇧Tab go to the next and the
previous question, and show its chain under the dialog. An answer goes on to the next question that waits, and the dialog
closes when none waits.

![Answer dialog](screenshots/53-question-dialog.png)

![The next question](screenshots/54-next-question.png)

Each answer lands where you can see it: the card of the question shows your answer, the desk drops its mark, and the
footer says where the answer went and how many questions still wait.

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

Terminal, the default, takes the colors of the terminal when the terminal says them: its background, and its text a
step softer than its foreground. Else it follows the mode of the terminal: Ink on a dark terminal, and Day on a light
one. Each color keeps one meaning in every theme: the accent marks the operator and the selection, amber what waits
for the operator, red a failure, and a quiet green what is done. Paper and Day are light, and Ink, GitHub Dark,
Midnight, Forest and Dusk are dark. ⌃T opens the themes, each with a swatch of its
colors.

In Ink, the text is a soft gray and never white. Python has one color for each part, in the feed and in the graph:
purple for a keyword, green for a string, orange for a number, `None`, `True`, `False` and a backslash sequence, teal
for a name that the code calls or defines, gold for a type, and gray italic for a comment. An operator keeps the color of
the text, so a keyword is the one purple on a line. None of these colors is pastel, and each one reads on every surface
of the feed.

![The theme of a light terminal](screenshots/78-terminal-light.png)

![Paper theme](screenshots/12-light-theme.png)

![Midnight theme](screenshots/13-midnight-theme.png)

![Color themes](screenshots/14-theme-picker.png)

## Access

`--screen-reader` or `/access reader` shows one column of text, with no sidebar, no box, no spinner, and nothing that
moves, for a screen reader. `--reduced-motion` or `/access motion` stops each spinner. With `NO_COLOR` set, each
surface takes the default colors of the terminal.

![Screen reader mode](screenshots/76-screen-reader.png)

![NO_COLOR](screenshots/77-no-color.png)

## Chains

⌃B lists the chains, each with its state and the chain it branched from. Each chain has its own feed and module. The
sidebar lists in its tree the root chain, the chain on screen, and each chain that is not at rest, and the other chains
fold under Finished. A click on a chain lists its threads, each in the color of its thread.

![Chains](screenshots/15-chains.png)

![The threads of a chain](screenshots/81-chain-threads.png)

## Views of the life

Main, the loom and the graph are three views of the whole life, side by side. ⌥V switches the view at
run time, `/view` names one or lists the three, `--view main|loom|graph` chooses one at the start, and the setting
keeps the last one. The loom and the graph stand over the feed of the selected chain, and each
screen of main stays where it is in each view: the transcript, the changes, the palette, the dialogs, the desk, and
the composer. Each row of a chain shows its state, its actor, its cost, and the question that it waits on.

Main shows the feed of the selected chain under nothing, and the chains and their threads in the sidebar.

![Main](screenshots/02-feed.png)

The loom shows each chain as a thread in time, under you. Time goes from left to right, and the right edge is now. A
cell is taller where more acts began or ended, yellow where the chain asks you, and red where an act failed, and the
past fades and folds. At the right of each thread stand what it waits on or the question that it asks, its actor, its
dollars against the ceiling of its grant, and the share of the window that its last answer read.

![The loom](screenshots/66-loom.png)

⌥G gives the keys to the loom, and Esc gives them back to the input. The arrows move between the chains and fold
the workers of a chain into its row. `[` and `]` move a cursor in time by one moment, and `{` and `}` by ten: the
loom and the feed show the life as it stood then, and the right edge of the loom says `then`. The verbs that change the life do nothing in
the past, `b` branches the selected chain at the cursor, and Esc returns to now. Space pauses or wakes the selected
chain, `x` cancels its work, `n` lets it run one more rung, and Tab answers the oldest question.

![The cursor in time](screenshots/67-loom-time-cursor.png)

The graph shows the life as programs that run: the tree of its threads, one for each prompt, beside the map of its
programs, one for each chain at the place of its birth, with the lines of the tree of hires and a dot where a
message flows. Under the name of each program stand what it waits on, its actor, its dollars, and its question.

![The graph](screenshots/68-graph.png)

After ⌥G, Enter zooms in and Esc zooms out: from the threads to the map, to the source of one program, with what
each act came to beside the line that made it, to one word, and to one act, whose card the feed shows whole.

![The map](screenshots/69-graph-map.png)

![A program](screenshots/70-graph-program.png)

![A word](screenshots/71-graph-word.png)

![An act](screenshots/72-graph-act.png)

A command that runs shows as it runs at the level of one act, since its card in the feed streams what it prints.

![A command at the level of one act](screenshots/73-graph-command.png)

## Keys, marks, and commands

One mark means one state in every view, dialog, and sidebar: a spinner for work that runs, `◉` for a chain or a
session at work, `◆` for a question that waits for you, `◇` for a notice, `◌` for paused
work, `✓` for a done act, `✗` for a failure, `●` for a chain you started or a session that finished while you were
away, and `○` for rest. F1 lists the chords that the terminal in use sends, what each mark means, and every slash
command.

![Help](screenshots/16-help.png)

## Narrow terminal

On a narrow terminal the sidebar is hidden, and the top line keeps the session, the chain, and the views.

![Narrow terminal](screenshots/17-narrow.png)

## Work in progress

A command streams its output while it runs, and a model shows the last lines that it writes. A word that the gate
refuses shows the line and the reason, and one more line that says what to do: press ↑ to bring your word back to the
input, fix it and send it again, or wait while the chain asks the model again with the findings.

When a call of a model fails for a cause that passes, as a lost connection, a provider that is overloaded or a limit
of the rate, the provider asks the model again after a wait that doubles each time, up to ten attempts. The footer
counts the wait down, as `Retrying in 4 s, attempt 2 of 10, connection reset`.

![Model retry](screenshots/62-model-retry.png)

![Command streaming](screenshots/18-live-command.png)

![Model progress](screenshots/19-model-progress.png)

![Gate findings](screenshots/20-gate-findings.png)

## Paused work

A session that opens with unfinished work stays paused until you resume it.

![Paused resume](screenshots/21-paused-resume.png)

## A record to read

A record whose replay drifts, as a record that a build of another version made, opens to read. The TUI shows what the record
kept: each chain, each rung with its word, and each act. No word runs again, no model is asked, and a command that
would change the life is refused. A read leaves the record and its log as they were. It can refresh the derived cache, `<record>.told`.

A dialog gives the build that made the record and this build, each on a line of its own. A record of an earlier engine
that names no engine says "an earlier engine, not named". The dialog says what a new life from the record carries, and
offers to read the record or to start a new life in its workspace.

![A record to read](screenshots/56-record-to-read.png)

While you read, the footer keeps the sentence "Read only" with its reason, and a button, "Start a new life". The
composer is off, and its placeholder says the same sentence. The command list has an entry, "Read only", that opens the
dialog again. A chain that the record does not keep takes the first line of its first prompt as its label, or its own
name, and stands under the chain of the act that made it. Work that had no end when the record ended says "not
finished when the record ends", with no spinner and no clock. The sidebar and ⌃W show "Read only" for such a record,
and not "Error".

![A record open to read](screenshots/56b-record-read-only.png)

A new life from the record has the turns of the root chain of the record, up to a limit of size, and the list of the files that
the record wrote, in its first word, as `earlier` and `earlier_files`.

## Cancel and take back

⌃C on an empty input cancels the last message, word or command that you sent on the chain shown and that still
runs, and that act alone with the work under it. It never cancels the thread or the chain that the act stands on,
and it never cancels a prompt that a model made, such as the prompt of a worker. When that act is a message, ⌃C takes
it back: the message is cancelled and its text goes back to the input, so you change it and send it again, and the
other work of the life goes on. A right click on a message opens its menu, which offers Take back for your last
message that runs, and Cancel for another act that runs. `/cancel <id>` cancels the act that you name.

While the last message that you sent runs and the input is empty, the footer offers ⌃C to take it back.

![Take back in the footer](screenshots/97-take-back-hint.png)

![Message menu](screenshots/57-message-menu.png)

A cancel that reaches running work asks first: the acts under the act that are not over, as the workers that a
thread hired on their chains. The dialog says how many acts the cancel ends and on which chains. Keep ends nothing.
A cancel of an act with no running work under it, as a message that its model still answers, asks nothing.

![Cancel confirmation](screenshots/58-cancel-confirm.png)

![Take back a message](screenshots/59-take-back.png)

## Rewind

Escape twice opens the rewind tree in the feed. Each act stands under the act that made it, and each branch under
the point it starts from. Up and Down move the pointer, Left and Right fold a branch, and a click points at a row.
Enter on a message gives it back to the input on a new branch, and Enter on another act makes a new branch that reads
the chain through that act. The module keeps its state.

When a write changed a file after the point, on any chain, Enter asks first what comes back. Restore the
conversation makes the branch, and the files stay as they are. Restore the files puts back each file as it was at
the point, and the conversation goes on as it stands. Both does the two. For a message, the point is before the
message. The files come back by a rung of the operator on the chain, so the record holds each write and the model
reads it. A write puts back the text of a file, and a command removes a file that a write made. A change that a
command made is not restored, and the dialog says how many commands ran after the point. The journal of file
changes beside the record holds each write that the rewind reads.

![Rewind tree](screenshots/23-rewind-tree.png)

![Rewind with the files](screenshots/74-rewind-files.png)

## Empty, loading, and failed views

A view with nothing to show says why in its middle. An act that fails keeps its failure in the feed. A view that
cannot load says what broke in plain words, and never the words of the parser of JSON, and offers to read it again.
When the journal of file changes beside the record is damaged, the session still opens, with `--resume` and from the
list, and the view of changes names the change that it cannot read. It offers to repair the journal, which reads the
file again and keeps each change that it can read, or to start a new journal, which keeps the damaged one beside it.
Each has a key, ⌥R and ⌥J, and a command, `/repair` and `/renew`, that act once the view found the journal damaged. A
repair says how many changes it kept and how many lines it removed, as "2 changes kept, 1 line removed". The rewind
reads each change that the journal holds and skips each line that is no change.

![Empty results](screenshots/24-empty-results.png)

![Loading](screenshots/25-loading.png)

![Act failure](screenshots/26-error.png)

![View error](screenshots/27-view-error.png)

## Workspaces

The sidebar groups sessions under project folders, and its list scrolls on its own. A click on a workspace folds its
sessions, and a click on a session opens it. Open sessions keep running while another is selected. A dot shows the
state of each session and workspace, from live work and saved records: blue for work, yellow for input needed or
paused work, green for an unread completion, and red for an error. An open grey dot means ready or saved, and a click
on a completed session clears its unread mark. A line of a program opens in place with what its acts gave, a
workspace folds to its name, and ⌃\ hides the sidebar. ⌃W lists the workspaces and their
sessions, each with its state, the time since its last save, its cost, and its size.

On macOS and Linux, each session is a life in a host of its own, which outlives the window. A quit detaches the
window, and the commands and the models of each life go on. A later window, or a second one at the same time, opens
the same record and attaches to the life as it stands. A window of a new build takes the life over: the old host
ends it, and a host of the new build opens it again on the same record. `/end` ends the life of each open session,
then quits.
`/serve` gives two addresses of the life on the network, one to read and drive it and one to read it only, and
`furb-tui --attach <address>` opens a window of another operator on it. `tui/README.md` says where the host listens,
how other operators reach it, and when a life stays in the window.

![Workspace tree](screenshots/28-workspace-tree.png)

![An open line](screenshots/29-open-line.png)

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

## Suggestions from the history

As you type, the input shows in grey the rest of the latest line from the history that starts with the text, as fish
does. The history is what the input sent in this mode, on this chain and on the others, and it stays with the record.
It persists for each workspace too: the input also reads what the other sessions of the workspace sent, in this
life and in earlier ones, and `.furb/history.json` in the workspace keeps it. A worktree is part of the workspace of
its repository, so it shares that history. The lines of the session now open come first, in a walk with Up and Down
and in a suggestion, and the lines of the other sessions come after them. Another workspace shares nothing.
Right or `⌃F` at the end of the line takes the whole rest, and `⌥→` or `⌥F` takes its next word. Tab keeps
completing commands, paths, and emojis. The grey rest shows only while the cursor ends the text, and not while the
list of suggestions or a walk of the history with Up and Down is open.

![History suggestion](screenshots/46b-history-suggestion.png)

## Emoji

A colon that starts a word of the prompt lists the emojis whose shortcode holds what follows it, as GitHub names them. Up and Down choose, Tab or Enter puts the emoji in the prompt in place of the word, and Escape, a space or a text that no shortcode holds closes the list and keeps the text. A whole `:shortcode:` in a prompt becomes its emoji.

![Emoji suggestions](screenshots/46c-emoji-suggestions.png)

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

`/extensions` opens a dialog of the extensions that the life runs, each with what its part for the TUI gives, or the error that stopped its load. `/run` runs a word of one. The demo runs none.

![Extension command](screenshots/42-extension-command.png)

The part of a project extension adds a panel over the input, here a todo list, and the row of each act of its kind in
the feed. The TUI loads it again when its file changes. A module that breaks shows its error in its panel, and the
TUI goes on.

![The part of an extension](screenshots/74-extension-part.png)

![The error of a part](screenshots/75-extension-part-error.png)

A model can add a part to a life that runs. The operator asks for a component, here a panel for a todo list. The model
reads the guide of the skill `extending`, makes the acts of its kind with a def, and writes an extension with
`extend`. The panel stands over the input at once, and the feed shows a line for each act. Nothing restarts.

![A model adds a panel](screenshots/74b-extend-live.png)

A change that the model gets wrong shows its error in the panel, in plain words, and the rest of the TUI goes on. The
operator tells the model, and the panel draws again from the file that the model mends.

![The error of a panel](screenshots/75b-extend-broken.png)

![The panel mended](screenshots/75c-extend-fixed.png)

`/extensions` says what each part gives.

![The extensions of a life](screenshots/75d-extensions.png)

The official extension `compaction` has a part too, which comes with furb. At each compaction of a chain, the feed
shows a rule with the summary of the compaction under it: the model of the chain no longer reads what stands above
the last rule. The row of a chain in the sidebar says how many times it compacted.

![The compactions of a chain](screenshots/75-compaction.png)

An older compaction went on in a new chain with the work that its source still needed, as `Main -> compacted`. Such
a run of chains shows as one chain: one row named after its first chain and its last one, and one feed, which shows
the feed of each chain of the run in its order, with the rule of each compaction between them.

![A run of compactions into new chains](screenshots/75-compaction-chains.png)

## Undo and redo

`/undo` removes the last message from a new branch, and `/redo` returns to the branch before it.

![Undo message](screenshots/43-undo-message.png)

![Redo message](screenshots/44-redo-message.png)

## Stash

⌃S puts the input aside, and the line under the input shows what waits. ⌃S again brings it back.

![Stash](screenshots/45-stash.png)

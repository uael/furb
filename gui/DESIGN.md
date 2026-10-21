# The desktop app of furb: a design

The desktop app shows a life of furb as a debugger of a living program. A user reads the threads of the life,
watches the programs that run and the messages between them, opens one program as code with the controls of a
debugger, and moves the whole view in time. The TUI reads the same life with the same model.

## The stack

The owner chose the stack: Tauri 2, with a UI of web components.

- The core is Rust. It links the crate `furb` directly, with no N-API, and runs one life on a thread of its own. It
  reads the acts of the life from the facts that the life says, and sends them to the window.
- The UI is React and TypeScript, styled with Tailwind on the primitives of Radix. The graph of programs is React
  Flow (xyflow), and the code is CodeMirror 6.
- The UI derives each view from the acts with the model of a life, `@furb/engine/life` in `bind/typescript/src/life`,
  which the TUI reads too: the tree of the threads, the source of a chain with each act on its line, the places of
  the chains, the messages, the marks of time, the watch, the call stack, and a message as a person reads it. The
  model holds no native code, so the UI takes it from its source, and a view of the app and of the TUI never
  disagree. When the app needs a view that the model does not give, the view goes into the model.
- The web view is the one of the system: WKWebView on macOS, WebView2 on Windows, and WebKitGTK on Linux. So the
  app is small, and macOS, Windows and Linux run the same UI.
- The app is a Cargo workspace of its own, in `gui/src-tauri`, since the gates of the crate do not install the
  libraries of a web view on Linux. It builds in the target of the repository, so it shares the dependencies that the
  crate built.

## How the app gets a life

- `furb-gui` opens on the picker: the projects that the TUI lists in its `workspaces.json` and the projects that the
  app opened, each with the records of its `.furb/sessions`, the newest first. A project opens a new life on the
  models of the catalog, in a new record that the app names as the TUI does. A record goes on where its life
  stopped, or opens to read. Open a project and Read a record pick one with the dialog of the system.
- A record to read opens with no life: the core reads the facts that the record kept, so no word runs again, no
  model is asked, no command runs, and a record of any engine opens. The record keeps only what the World, the
  models and the operator said, so the core makes what it lacks: each rung that a kept act names as its maker, on
  the open prompt of its chain, with the word of the reply that it asked for, and done when the next rung starts; a
  refusal of the gate on the rung whose word it read; and each chain that a kept act stands on. A thread that a word
  closed, and a raise of a word, are not in the record: the thread shows a note where its answer would stand, and
  the graph folds the chains that hold no thread into one note. A snapshot of the transcripts, when a life keeps
  one, will give the whole view.
- `furb-gui --live DIRECTORY`, `--continue RECORD` and `--record RECORD` open the same from the command line, and
  `--demo` opens the demo: a small project, and models that answer from a script, so no model is asked.
- The core sends the window the acts that changed since its last part, as the event `life`, at most once each 80
  milliseconds, with the words of their rungs, the prompts that the operator owes, the measure of each chain, and
  what each model writes now. A window that misses a part asks the core with `life` for all that changed since what
  it holds. `watch` gives what a chain told, `send` carries each command of the operator, `open` and `close` open a
  life and go back to the picker, and `places` lists the projects.
- `furb-gui --check` ends the app once its window drew a life, and fails after 45 seconds, so a bundle is proved
  before anyone opens it.
- `furb-gui --view PATH` runs the demo with no window until it rests, and writes the life as the window takes it,
  with the watch of each chain. `gui/src/data/demo.json` is such a fixture: a browser shows the UI on it, and
  `bun run shots` takes the captures. With `--record RECORD`, it writes the fixture of that record, with the places
  of the picker; `FURB_FIXTURE` gives it to the build of the UI.

## Time

The scrubber under the header holds the whole life: a mark for each step, each question to the operator and each
end of a thread. A drag moves the cursor, the keys `[` and `]` go to the mark before and after it, and Escape or the
pill goes back to now. The pill says the mark at the cursor, or the moment. In the past, the view shows the acts as
they stood at that moment, and no control and no composer shows, since the past does not change.

## The look

The desktop apps of Claude Code and of Codex set the bar of finish. furb takes their care of type, space and motion,
and not their layout or their look.

- Two themes: Ink, a blue slate that is not black, and Paper, its light twin.
- One family of type in two forms: Atkinson Hyperlegible Next for the interface and Atkinson Hyperlegible Mono for
  the code. The Braille Institute made it to be legible, and a debugger is dense: a 0 and an O, a 1 and an l, must
  never be read as each other.
- One accent, a violet, marks the selection and the work that runs. Amber marks only what waits for the operator.
- Each state has a shape and a color, so a state reads without its color: a dot that breathes for work that runs, a
  diamond for a question to the operator, a cross for a failure, a check for an end, two bars for a pause, and a ring
  for a wait.
- Each kind of edge of the graph has one style, the same everywhere, which a legend says: a dotted line for a chain
  that a step hired, a solid line with an arrow for a prompt to a chain, and a dashed amber line for a question to
  the operator.
- An error reads as a person reads it, once: no name of an exception around it, and no path of the directory of
  the life.
- Panes differ by tone, with a hairline between two of them. Motion is short and answers an action of the user;
  reduced motion stops the breath of the work that runs.
- Each empty part of the header and of the top of the rail moves the window.

## The screens

- Picker: the projects and their records, and the two buttons that open one with the dialog of the system.
- Thread: one thread read top to bottom: what was asked, each step that its model took, folded to what it did and
  open to its code, the text that its model writes now, the threads that a step asked, and the answer. A question
  to the operator takes its answer in place.
- Programs: the graph of the chains of the life. Each chain keeps the place where it was born, which the model
  gives, so a new chain moves no other. Zoomed out, a chain shows its title, its state and its cost in large type;
  zoomed in, it holds its threads, the code of the step that runs with the line that runs, and its controls.
- Code: one thread opened as a program, as a debugger opens it: the whole ladder as code with the value of each act
  beside its line, the controls (continue, pause, step, cancel, a breakpoint at each step, the door), the call
  stack, the watch, with the names of each act under it, and the breakpoints. The door opens the program of the
  ladder in an editor, and its write runs the new program from the first line that differs.
- The rail at the left holds the life, what it spent, the questions that wait for the operator, and the tree of
  threads, each with its whole title and what its steps came to in a tip. The composer at the foot answers the
  question that waits, asks a chain in prose with a shape, or runs a word of python on it.

## The state of the work

Step 2 makes the app a real host. Each control sends a real command, and a control that cannot work, as in a record
to read or in the past, does not show.

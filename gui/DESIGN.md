# The desktop app of furb: a design

The desktop app shows a life of furb as a debugger of a living program. A user reads the threads of the life,
watches the programs that run and the messages between them, and opens one program as code with the controls of a
debugger. The TUI is a simpler form of the same app.

## The stack

The owner chose the stack: Tauri 2, with a UI of web components.

- The core is Rust. It links the crate `furb` directly, with no N-API, and runs one life on a thread of its own. It
  makes the model of the life from the facts that the life says, and from the model a view: the threads, the
  programs, the messages, the ladder of each thread as code, and the stack and the watch of each chain.
- The UI is React and TypeScript, styled with Tailwind on the primitives of Radix. The graph of programs is React
  Flow (xyflow), and the code is CodeMirror 6.
- The web view is the one of the system: WKWebView on macOS, WebView2 on Windows, and WebKitGTK on Linux. So the
  app is small, and macOS, Windows and Linux run the same UI.
- The app is a Cargo workspace of its own, in `gui/src-tauri`, since the gates of the crate do not install the
  libraries of a web view on Linux. It builds in the target of the repository, so it shares the dependencies that the
  crate built.

## How the app gets a life

- `furb-gui` opens the demo life: a small project, and models that answer from a script, so no model is asked. The
  engine runs each word, the gate reads it, and the World runs each command on this machine.
- `furb-gui --record PATH` opens a record to read. The life keeps nothing and runs nothing.
- `furb-gui --live DIRECTORY` opens a life on this machine, on the default actor of the catalog.
- The core sends the whole view to the UI at each change, as the event `view`. The UI sends the commands of the
  operator with `send`, and moves the cursor in time with `travel`, which makes the view of the life as it was at a
  fact of the record.
- `furb-gui --view PATH` runs the demo with no window until its question waits, and writes that view as JSON.
  `gui/src/data/demo.json` is such a view: a browser shows the UI on it, and `bun run shots` takes the captures.

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
- Panes differ by tone, with a hairline between two of them. Motion is short and answers an action of the user;
  reduced motion stops the breath and the flow of the edges.

## The screens

- Thread: one thread read top to bottom: what was asked, each step that its model took, folded to what it did and
  open to its code, the threads that a step asked, and the answer.
- Programs: the graph of the programs of the life. Each chain is a node that holds its state, its cost, its threads
  and the code of the step that runs, with the line that runs. The messages are the edges: a prompt from one program
  to another, or to the operator.
- Code: one thread opened as a program, as a debugger opens it: the whole ladder as code with the value of each act
  beside its line, the controls (continue, pause, step, cancel, a breakpoint, the door), the call stack and the watch.
- The rail at the left holds the life, what it spent, the questions that wait for the operator, and the tree of
  threads. The composer at the foot asks a chain in prose or in python, or answers the question that waits.

## The state of the work

Step 1 is a static mock of the three screens on the view of the demo, which the owner judges before the rest. The
window of Tauri opens on the same UI and hears a live life, but its controls send nothing yet.

# The desktop app of furb: a design

The desktop app of furb is a preview: it reads the records of furb. A user picks a record of a project and reads its
threads, each top to bottom with the steps that its model took. The app runs no life: no word runs again, no model
is asked, and no command runs. The title of its window and each of its screens say that it is a preview.

The TUI reads the same records with the same model of a life.

## The stack

The owner chose the stack: Tauri 2, with a UI of web components.

- The core is Rust. It links the crate `furb` directly, with no N-API. It reads a record with `furb::world::retold`,
  the one reader of a record that both doors expose as `retold` and the TUI reads too, on a thread of its own. It
  reads the acts from the facts that the reader gives, and sends them to the window.
- The UI is React and TypeScript, styled with Tailwind on the primitives of Radix. The code of a step is CodeMirror 6.
- The UI derives each view from the acts with the model of a life, `@furb/engine/life` in `bind/typescript/src/life`,
  which the TUI reads too: the tree of the threads, the source of a chain with each act on its line, and a message as
  a person reads it. The model holds no native code, so the UI takes it from its source, and a view of the app and of
  the TUI never disagree. When the app needs a view that the model does not give, the view goes into the model.
- The web view is the one of the system: WKWebView on macOS, WebView2 on Windows, and WebKitGTK on Linux. So the
  app is small, and macOS, Windows and Linux run the same UI.
- The app is a Cargo workspace of its own, in `gui/src-tauri`, since the gates of the crate do not install the
  libraries of a web view on Linux. It builds in the target of the repository, so it shares the dependencies that the
  crate built.

## How the app reads a record

- `furb-gui` opens on the picker: the projects that the TUI lists in its `workspaces.json`, each with the records of
  its `.furb/sessions`, the newest first. Read a record picks one with the dialog of the system.
- The reader of the crate reads a record with no life, so a record of any engine opens. While the log beside the
  record agrees with it, it gives every fact of the log, so the record shows whole. Else it gives the facts that the
  record kept, and the facts that the engine said and the record does not keep, as far as the kept facts show them.
  A thread that a word closed has no end in such a record: the thread shows a note where its answer would stand.
- `furb-gui --record RECORD` opens a record from the command line.
- The core sends the window the acts that changed since its last part, as the event `life`, at most once each 80
  milliseconds, with the words of their rungs and the measure of each chain, and says in each part whether it read
  the whole record. A window that misses a part asks the core with `life` for all that changed since what it holds.
  `open` and `close` open a record and go back to the picker, and `places` lists the projects.
- `furb-gui --check` ends the app once its window drew the whole record, and prints the count of its chains, its
  threads and its acts. It fails after 600 seconds, so a bundle is proved before anyone opens it.
- `furb-gui --record RECORD --view PATH` reads the record with no window and writes it as the window takes it, with
  the places of the picker. `gui/src/data/demo.json` is such a fixture, of the demo of the TUI: a browser shows the UI
  on it, and `bun run shots` takes the captures. `FURB_FIXTURE` gives another fixture to the build of the UI.

## The look

The desktop apps of Claude Code and of Codex set the bar of finish. furb takes their care of type, space and motion,
and not their layout or their look.

- Two themes: Ink, a blue slate that is not black, and Paper, its light twin.
- One family of type in two forms: Atkinson Hyperlegible Next for the interface and Atkinson Hyperlegible Mono for
  the code. The Braille Institute made it to be legible: a 0 and an O, a 1 and an l, must never be read as each
  other.
- One accent, a violet, marks the selection and the mark of the preview. Amber marks only a question to the
  operator.
- Each state has a shape and a color, so a state reads without its color: a dot for work that was open when the
  record ends, a diamond for a question to the operator, a cross for a failure, a check for an end, two bars for a
  pause, and a ring for a wait.
- An error reads as a person reads it, once: no name of an exception around it, and no path of the directory of
  the life.
- Panes differ by tone, with a hairline between two of them. Motion is short and answers an action of the user.
- Each empty part of the header and of the top of the rail moves the window.

## The screens

- Picker: the projects and their records, each of which opens to read, and the button that picks a record with the
  dialog of the system.
- Thread: one thread read top to bottom: what was asked, each step that its model took, folded to what it did and
  open to its code, the threads that a step asked, and the answer.
- The rail at the left holds the record and what its life spent, the questions to the operator that were open when
  the record ends, and the tree of threads, each with its whole title and what its steps came to in a tip.
- The header says the thread, while the core reads the record, and the mark Preview: read only.

# The desktop app of furb: a design

The desktop app of furb runs a life of furb in a window. A user starts a new life in a project or goes on with a
record, sends messages to its chains, sees each model and each command at work, and answers each question of a model.
The TUI does the same in a terminal, with the same host of a life and the same model of a life.

## The stack

The owner chose the stack: Tauri 2, with a UI of web components.

- The core is Rust, in `gui/src-tauri`. It holds the window and the dialogs of the system, and it starts the host of a
  life as a process of its own. It links no engine: a life runs in the host alone.
- The host is a program of bun, `gui/host`. It opens the projects and the lives with the `Workspaces` and the
  `Session` of the TUI, in `tui/src`, which drive the `Session` of `bind/typescript` in the worker of the TUI. So the
  app and the TUI have one host of a life, and a life that opens, a message that goes, an answer that the console
  reads, and a record of another engine that opens to read, each goes by the same code in both.
- The UI is React and TypeScript, styled with Tailwind on the primitives of Radix. The code of a step is CodeMirror 6.
- The UI derives each view from the acts with the model of a life, `furb/life` in `bind/typescript/src/life`,
  which the TUI reads too: the tree of the threads, the source of a chain with each act on its line, the desk of the
  questions, and a message as a person reads it. The model holds no native code, so the UI takes it from its source.
  When the app needs a view that the model does not give, the view goes into the model: the word of each rung, which
  the app draws on every chain, is `LiveAct.word`.
- The web view is the one of the system: WKWebView on macOS, WebView2 on Windows, and WebKitGTK on Linux. So the
  app is small, and macOS, Windows and Linux run the same UI.
- The app is a Cargo workspace of its own, in `gui/src-tauri`, since the gates of the crate do not install the
  libraries of a web view on Linux. It builds in the target of the repository.

## The host and the window

- The core starts `bun gui/host/main.ts` with the words of the command line, and holds its stdin open: the host ends
  when its stdin ends, and saves each life first. The first line of the host is the address of its socket, which the
  window asks the core for with the command `host`.
- The socket is one WebSocket on 127.0.0.1, at a free port. A page of any origin can open a socket to 127.0.0.1, so
  only a window that holds the token of the address gets in.
- The host sends each window parts (`gui/src/wire.ts`). The first part of a window holds the whole life. Each later
  part holds what changed: the acts that the session put in place of others, the new text of each stream after the
  place that the window holds, the streams that ended, the life when one of its words changed, and the projects when
  they changed. A part goes at most once each 50 milliseconds, and no part goes when nothing changed. A new life is a
  new generation, which the window takes from nothing.
- The window sends calls, each answered once by its number: `open` a record, `create` a life in a project, `add` a
  project, `show` a chain, `send` a line to a chain, `answer` a question with a value of its choice, `pause`, `wake`
  or `cancel` an act, `confirm` a cancel, `takeBack` the last message of a chain, read the `files` that a rewind gives
  back, `rewind` to an act, and `resume` the work that an earlier life left. Each is the method of the `Session` of
  the TUI that does the same: `send` is its `submit`, so a line answers the question that waits on the chain and else
  goes to its model, and `takeBack` is its cancel of the last message, or its undo once that message is done.
- A cancel that ends running work waits for the operator: the host sends the window `confirm`, with the acts and the
  chains that it ends, and ends them once the window confirms. A message that comes back to the composer, after a take
  back or a rewind to a message, comes as `compose`.
- The window holds the acts in a map that grows in place, and derives its views once a second while the host still
  opens a life, so a large record opens light. Each act of a command that runs carries the tail of its output, as the
  rows of the TUI do, and a read or a write leaves its text out.
- `furb-gui --check --record RECORD` ends the app once its window drew the whole life, and prints the process of its
  web view, which the private property `_webProcessIdentifier` of WKWebView gives, and the count of its chains, its
  threads and its acts, and whether they are those of the log of the life or those that its record kept. A check keeps
  the list of its projects in a folder of its own, so the list of the operator stays as it is, and enables no extension
  that the record does not run, so the life that it draws is the one of the record. It runs as an accessory of the
  system from its launch: no tile in the Dock, no place in the switcher of apps, and no front taken from another app,
  so a quit that the Dock sends for the name of the app never reaches the app that the operator has open. It ends by
  its own exit, which ends its host first, and fails after 600 seconds. The window tells the core what it drew at its
  next frame, or after 500 milliseconds when no frame comes, since WebKit draws no frame in a window that is hidden or
  behind others.
- The test `gui/src-tauri/tests/check.rs` runs a check on a copy of the record of a life of the demo in
  `tests/fixtures`, or on the record that `FURB_RECORD` names, compares the counts with those of the log beside the
  record, which the life writes again, or with those of `furb facts` on the record when the window drew what the record
  kept, and holds the footprint of the web view of the app, and of no other WebContent of the machine, under 300 MB. A second test runs a check in a window that `FURB_GUI_HIDDEN` hides. The gates run both on macOS.
- `bun run web` shows the UI in a browser, on a host of its own, and `bun run shots` drives it in WebKit for the
  captures.

## The look

The desktop apps of Claude Code and of Codex set the bar of finish. furb takes their care of type, space and motion,
and not their layout or their look.

- Two themes: Ink, a blue slate that is not black, and Paper, its light twin.
- One family of type in two forms: Atkinson Hyperlegible Next for the interface and Atkinson Hyperlegible Mono for
  the code. The Braille Institute made it to be legible: a 0 and an O, a 1 and an l, must never be read as each
  other.
- One accent, a violet, marks the selection, the composer and a record that opens to read. Amber marks only a
  question to the operator.
- Each state has a shape and a color, so a state reads without its color: a dot for work that runs, a diamond for a
  question to the operator, a cross for a failure, a check for an end, two bars for a pause, and a ring for a wait.
- An error reads as a person reads it, once: no name of an exception around it, and no path of the directory of
  the life.
- Panes differ by tone, with a hairline between two of them. Motion is short and answers an action of the user.
- Each empty part of the header and of the top of the rail moves the window.

## The screens

- Picker: the projects and their records, a new life in each project, each record that opens, and the buttons that
  pick a folder or a record with the dialog of the system.
- Thread: one thread read top to bottom: what was asked, each step that its model took, folded to what it did and
  open to its code, the word of a model as it writes it, the output of a command as it prints it, the threads that a
  step asked, and the answer. The composer under it sends to its chain.
- The rail at the left holds the life and what it spent, the questions that wait for you, and the tree of threads,
  each with its whole title and what its steps came to in a tip.
- The header says the thread, the last notice of the session, and the mark Read only on a record of another engine.
- The menu of an act holds its controls, and a dialog asks the operator to confirm a cancel that ends running work,
  and what a rewind gives back.

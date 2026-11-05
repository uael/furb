# Ideas for later

This file keeps the ideas that furb does not build now, so that a later change can start from them. An idea here is no
promise and no plan: the issues and the design of the daemon hold the work that is planned.

## A desktop client of the daemon

furb had a desktop app on Tauri 2 until it was removed, with the decision that the TUI and the protocol of the daemon
prove the shared core, and that the cloud product needs a web view and not a desktop app. The idea stays.

A desktop client is a window that dials the daemon (D1 of #983) and holds no life of its own. It draws the tree of
nodes of the views (D19) with the DOM: a box is a flex row or column, a text is runs in the colors of the theme, and a
table, a list, an input, a button, a progress, code, a diff and a markdown are the elements of their names. It is a
client of the same protocol as the TUI, so a life that the TUI opened can be read or answered in the window, and the
other way. The door of the windows that `furb host` opens (`docs/protocol.md`, "The door of the windows") is made for
this: a WebSocket on loopback, with its token in `window.json`. A web view of the cloud product can use the same tree
of nodes in a browser.

What the removed app taught:

- Two hosts of a life are one too many. The app had `gui/host`, a program of bun that ran its own lives and imported
  the sources of `tui/src` (`Workspaces` and `Session`), so each change of the TUI could break the app, and a life that
  the app opened could not be read by a TUI. A client must dial the daemon and import no source of another client.
- The model of a life, `furb/life` in `bind/typescript/src/life`, was the right seam: the tree of threads, the source
  of a chain with each act on its line, the desk of the questions, and a message as a person reads it came from it for
  both the TUI and the app, with no native code. A new client should read the model of the views that the daemon
  serves, and add a view to the model when it needs one.
- A window must hold its acts in a map that grows in place, and derive its views once a second while a life opens, so
  a large record opens light. A part of the wire goes at most once each 50 milliseconds, and none when nothing changed.
- A page of any origin can open a socket to 127.0.0.1, so the door must take only a peer that gives the token.
- A web view draws no frame in a window that is hidden or behind others, so a check of a window cannot wait for a
  frame: it must tell what it drew after a short time. Only the runners of CI can run such a check, since a window takes
  the mouse and the keyboard of the person who works on the machine, and only macOS gave a stable proof of it.
- The look worked: two themes (a slate and its light twin), one legible family of type in two forms, one accent, and
  one shape and color for each state, so a state reads without its color. Amber marked only a question to the operator.

The git history keeps the app: `git log --diff-filter=D --stat -- gui` finds the change that removed it, and its parent
holds the whole tree.

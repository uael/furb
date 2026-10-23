# Changelog

Each release of furb is listed here, the newest first. release-please writes the entry of each release from the
titles of the pull requests that it holds, as [the template of a pull request](.github/pull_request_template.md) says.

## 0.1.0 and the steps to 0.2.0

furb 0.1.0 came out on PyPI on 2026-09-19, as one Python package. The work between 0.1.0 and 0.2.0 came before
this changelog, in commits with no type, so this short history gives its main steps, with the pull request of each.

- 2026-09-19, 0.1.0: the contract of the engine, a suite with one test for each sentence of it, the engine itself,
  and a World, a Kernel and a door in Python.
- 2026-09-22: the crate runs the engine in monty, a Python interpreter written in Rust, with a door to Python (#8).
- 2026-09-24: the bindings for TypeScript and the TUI on OpenTUI (#21), then a new design of the TUI (#37).
- 2026-09-26: the engine, derived again from its meanings: every question is an act (#43).
- 2026-09-27: a provider of models in Rust, with the claude command line as a model (#45), and `furb`, the command
  line, which opens the TUI or serves a JSON-RPC (#46).
- 2026-09-28: Rust as the only provider and a small system of extensions (#50), and an engine that the crate loads
  with no preamble (#78).
- 2026-10-20: a scratch directory for each record (#136), the models of a local server (#140), and `furb facts`
  (#174).
- 2026-10-21: the official extensions compaction (#183), ouroboros (#224) and web (#249), a store of the record on
  SQLite (#282), and the suite on Windows (#263).
- 2026-10-22: examples for each door (#253), the log of a life beside its record (#284), a snapshot that opens a life
  with no replay (#288), the doctrine in its own file (#299), a record that names its engine (#301), and wheels for
  Windows (#306).
- 2026-10-23: a retry of a model call that fails for a cause that passes (#324), a check of the setup at the first run
  (#321), the desktop preview that reads records (#325), the audit of a record (#326), and in the TUI three views at
  once (#329), parts of extensions (#330), a review of the changes (#331), files in a rewind (#332), and a mode for a
  screen reader (#334).

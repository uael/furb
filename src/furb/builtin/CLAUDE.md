# The names of the builtin extensions

The builtin extensions are `files`, `bash` and `grant`. Each is a module of form (b): a python module that imports
what it uses from the engine and from the extensions it requires, whose word the crate makes by blanking those
imports. Each has its contract beside it, `<name>.pyi`, and its suite in `test/<name>/`. They are on unless a config
turns them off, and the engine knows none of them. `docs/extensions.md` says how a host plays them.

Each name has one meaning, which the sentences of the contracts of the builtins use as given here, beside the names
of the engine in `src/furb/CLAUDE.md`.

- working directory: where the paths of a chain resolve, which `cwd` gives: the path of the closest cd back in the
  transcript of the chain, and the directory of its standing while no cd stands there.
- text: what read gives, and what write is given and gives back as it landed; a Text, with a path and a content.
- door: the name of an act, with a part after it or without; the name of a prompt is the door of its ladder, which
  read and write reach through the ladder query, the word of a rung adds a door by making an act whose ear answers
  reads of a scheme, and the World serves the rest.
- span, grep and differs: the makers of the shows of the files extension; HEAD shows the first lines and HIDDEN
  shows nothing.
- command: what bash runs on the machine.
- merged: the state of a command whose stderr flows into its stdout, in the order the command wrote them.
- exit: what a command came to, an Exit, with its code and its two streams as texts.
- grant: an act that puts a ceiling on a chain: dollars, a share of the window, or both.
- ledger: what the grant holds: the dollars of the answers since it was made and the share the last one filled.

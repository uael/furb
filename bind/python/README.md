# furb-monty

furb for python, with the engine in monty. `furb_monty.engine` gives every name of `engine.pyi` over one life of
the engine in the sandbox of monty, with the Kernel of the crate, so it needs no Kernel of its own.
`furb_monty.gate` is the gate of the crate, which the Kernel of the package `furb` reads a word with too.

`FURB_ENGINE=monty` makes `from furb import engine` give `furb_monty.engine`. The suite of furb runs on both
engines.

The extension module is the crate at the root of the repository, built with its `python` feature.

It gives the extension API of the crate too, the same for every host:

- `extensions(project, refresh=False)` gives the extensions that a host plays for a project, each an `Extension`
  with its `name`, its `root`, the `word` of its python part, its `life` word, what it `requires`, and the files of
  its parts, `world_ts`, `world_py` and `tui`. It reads the config of the user and `.furb/config.json` of the
  project, fetches into the cache once, and again on a refresh, and raises `Refused` with what failed.
- `builtin_extensions()` gives the builtins `files`, `bash` and `grant`, and `places()` gives the config directory
  and the cache directory.
- `word_of(source)` gives the word of a python part, and `missing_words(program, words)` gives the words that a
  program lacks, in their order.

The command line of furb plays the words of the extensions with these, and holds the parts for a World of the
builtins. [The guide of the extensions](../../docs/extensions.md) says more.

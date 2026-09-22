# furb-monty

furb for python, with the engine in monty. `furb_monty.engine` gives every name of `engine.pyi` over one life of
the engine in the sandbox of monty, with the Kernel of the crate, so it needs no Kernel of its own.
`furb_monty.gate` is the gate of the crate, which the Kernel of the package `furb` reads a word with too.

`FURB_ENGINE=monty` makes `from furb import engine` give `furb_monty.engine`. The suite of furb runs on both
engines.

The extension module is the crate at the root of the repository, built with its `python` feature.

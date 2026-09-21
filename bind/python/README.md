# furb-monty

furb for python, with the engine in monty: `furb_monty.engine` gives every name of `engine.pyi` over one life of
the engine running in the sandbox of monty, with the Kernel of the crate. It needs no Kernel of its own.

`FURB_ENGINE=monty` makes `from furb import engine` give it. The suite of furb runs on both engines.

The extension module is the crate at the root of the repository, built with its `python` feature.

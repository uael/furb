# furb-monty

furb for python, with the engine in monty. `furb_monty.engine` gives every name of `engine.pyi` over one life of
the engine in the sandbox of monty, with the Kernel of the crate, so it needs no Kernel of its own.
`furb_monty.gate` is the gate of the crate, which the Kernel of the package `furb` reads a word with too.

`FURB_ENGINE=monty` makes `from furb import engine` give `furb_monty.engine`. The suite of furb runs on both
engines.

`furb_monty.engine.dump()` gives the life as bytes where it stands still, stamped with the engine, the build of the
crate and the record. `furb_monty.engine.restore(dump, record, **outside)` opens a life from those bytes on the ears
it was dumped with, with nothing replayed, and refuses a dump whose stamp does not match, so the host boots on the
record. The World of `furb.world` leaves `<record>.dump` when its life stands still at its end, and opens from it.

The extension module is the crate at the root of the repository, built with its `python` feature.

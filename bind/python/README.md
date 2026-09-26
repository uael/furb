# furb-monty

furb for python, with the engine in monty. `furb_monty.engine` gives every name of `engine.pyi` over one life of
the engine in the sandbox of monty, with the Kernel of the crate, so it needs no Kernel of its own.
`furb_monty.gate` is the gate of the crate, which the Kernel of the package `furb` reads a word with too.

`furb_monty._monty` is the door itself. Its `Engine` has one method for each verb of the contract, as the `Engine`
of the crate has, and none given to a word with a default leaves the default of the engine. It also gives the ears
of the World that the crate writes: `files()`, `bash()`, `time()`, and `store(path)`, which gives the record at the
path and the ear that keeps it. Each is a `NativeEar`, which `furb_monty.engine.boot` takes beside the generators
of this interpreter:

```python
from furb_monty import _monty, engine

record, store = _monty.store("life.jsonl")
ears = {"files": _monty.files(), "bash": _monty.bash(), "time": _monty.time(), "store": store}
# `world` is an ear of the host that answers the rest, the stand and the replies of the models among it.
root = engine.boot(record, **ears, world=world)
```

A generator of this interpreter is heard on a thread of its own, so it may say a verb while it hears, as an ear of
the engine of python does.

`FURB_ENGINE=monty` makes `from furb import engine` give `furb_monty.engine`. The suite of furb runs on both
engines.

The extension module is the crate at the root of the repository, built with its `python` feature.

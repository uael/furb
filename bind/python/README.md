# furb-monty

furb for python, with the engine in monty. `furb_monty.engine` gives every name of `engine.pyi` over one life of
the engine in the sandbox of monty, with the Kernel that the crate loads there, so it needs no Kernel of its own.
`furb_monty.gate` is the gate of the crate, which the package `furb` reads a word with too.

`furb_monty._monty` is the door itself, and it gives the same host API as the package of TypeScript, each name in
the case of python. Its `Engine` has one method for each verb of the contract, which takes the words the verb needs
by position and the others by name, and gives an `Act` when the verb makes one. `_monty.opened(directory=...,
record=...)` opens a life as every host of the crate opens one: it gives the record and the ears of the crate, which
the host boots after its own ears, and `Engine.open(ears, directory=...)` boots them at once.
Those ears are the provider of the models, the extensions, which enable at the tip what the configs of the user and
of the directory turn on unless `extensions=False`, the ear of each official extension, the files, the commands,
time, and the store when the life keeps. Each is a `NativeEar`, which `furb_monty.engine.boot` takes beside the
generators of this interpreter, and which the engine of this interpreter, `furb.python`, steps as a generator of its
own:

```python
from furb_monty import _monty, engine

record, ears = _monty.opened(directory=".", record="life.jsonl", actor="claude-cli:opus")
# `world` is an ear of the host that answers the rest, a prompt to the operator among it.
root = engine.boot(record, world=world, **dict(ears))
```

The provider offers the model of its `actor` and the models of its `roster`, or the first model the catalog offers when
it names neither, and the operator alone when the roster names none and no actor is said. Its `answer`, a function,
answers each request in place of the models, which the suite of furb does. Its `stream`, a function, is told what a
model writes as it writes it, its `log`, a function, is told each report of the life that no model reads, one line
each, in place of the stderr of the process, and `images` is the directory of the images that a message names. A life that is
`inspecting` keeps nothing, enables nothing new and asks no model. `models(claude)` gives the models the catalog of this
machine offers, `model(name)` one model by its name, and `levels()` the levels of effort. `files(scratch)`,
`bash(scratch)`, `time()`, `store(path)`, `official()`, `extensions(given)`, `memory(config)` and `skills(config)` give
the ears one by one, as the suite of each extension boots them: the files and the commands share the scratch directory
that the host names, which a word reads as `scratch://` and a command as `$TMPDIR`. Only the opening of a life makes the
files of a record again in its scratch directory, and gives a life with no record a directory that goes with it.
`scratch(record)` gives the scratch directory of a record, which a host removes when it deletes the record.
`answered(shape, line)` reads a line of the operator as a value of one of the `shapes()`, by the rules that every
console of the crate reads a line by. `_monty.SYSTEM` is the system prompt of every model: the engine, minified with
its program kept as it is.

`furb_monty.engine.pending()` names the work that an earlier life left, which waits for a wake that this life says,
each act by its name and its kind, as the `Engine` of the door finds it: among each act that the outside started and
did not end, and the acts that made it, each prompt, rung, command and wait that is not done and that no pause holds.
An act that a wake puts to the outside again is pending no more.

`furb_monty.engine.parked()` names the work that a crash loop parked, each act by its name and its kind: a run, a
command or a reply that the record shows started in five lives and not done, since each of those lives ended while it
ran. A wake puts no parked act to the outside, so a host that wakes the work of a life as it opens does not run it
again and end again, and a parked act is pending no more. `furb_monty.engine.retry(id)` runs a parked act again: it is
parked no more in this life, and a wake of it puts it to the outside.

`furb_monty.engine.keyed(key, kind, *args, **kwargs)` makes the act of a verb once for a key of the client, across a
crash of the host and a retry of the client, as `keyed("k1", "prompt", int, "count the lines", on=root)`. A key fact
goes to the record before the act, and a later call with the same key on the same chain gives the act that the key
made, in this life or in a later life on the record. When the host crashed after it kept the key and before it kept
the act, the call makes the act.

A generator of this interpreter is heard where the engine runs. A verb it says while it hears goes to the life that
hears it, as a verb that an ear of the engine of python says does.

`FURB_ENGINE=monty` makes `from furb import engine` give `furb_monty.engine`. The suite of furb runs on both
engines.

The extension module is the crate at the root of the repository, built with its `python` feature.

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
model writes as it writes it, and `images` is the directory of the images that a message names. A life that is
`inspecting` keeps nothing, enables nothing new and asks no model. `models(claude)` gives the models the catalog of this
machine offers, `model(name)` one model by its name, and `levels()` the levels of effort. `files()`, `bash()`, `time()`,
`store(path)`, `official()`, `extensions(given)`, `memory(config)` and `skills(config)` give the ears one by one, as the
suite of each extension boots them. `answered(shape, line)` reads a line of the operator as a value of one of the
`shapes()`, by the rules that every console of the crate reads a line by. `_monty.SYSTEM` is the system prompt of every
model: the engine, minified in layout alone.

`furb_monty.engine.pending()` names the work that an earlier life left, which waits for a wake that this life says,
each act by its name and its kind, as the `Engine` of the door finds it: among each act that the outside started and
did not end, and the acts that made it, each prompt, rung, command and wait that is not done and that no pause holds.
An act that a wake puts to the outside again is pending no more.

A generator of this interpreter is heard where the engine runs. A verb it says while it hears goes to the life that
hears it, as a verb that an ear of the engine of python says does.

`FURB_ENGINE=monty` makes `from furb import engine` give `furb_monty.engine`. The suite of furb runs on both
engines.

The extension module is the crate at the root of the repository, built with its `python` feature.

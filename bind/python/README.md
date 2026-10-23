# furb-monty

The engine of furb for Python, run in monty, a Python interpreter written in Rust. Most programs do not import it
directly: `pip install furb` installs it, and [`furb.open`](https://github.com/uael/furb/blob/main/docs/examples/README.md) opens a life on it. Read on
when you write a host of your own, which boots an engine on ears that you choose.

## What it gives

- `furb_monty.engine` gives every name of the contract, [`engine.pyi`](https://github.com/uael/furb/blob/main/src/furb/engine.pyi), over one life of
  the engine in the sandbox of monty. The crate loads the Kernel there, so this package needs none of its own.
- `furb_monty.gate` is the gate of the crate. The package `furb` reads a word with it too.
- `furb_monty._monty` is the door itself. It gives the same host API as the package of TypeScript, each name in the
  case of Python. Its `Engine` has one method for each verb of the contract, which takes the words that the verb
  needs by position and the others by name, and gives an `Act` when the verb makes one.

`FURB_ENGINE=monty` makes `from furb import engine` give `furb_monty.engine`. The suite of furb runs on both engines.

## Open a life

`_monty.opened(directory=..., record=...)` opens a life as every host of the crate opens one. It gives the record and
the ears of the crate, which the host boots after its own ears. `Engine.open(ears, directory=...)` boots them at
once.

```python
from furb_monty import _monty, engine

record, ears = _monty.opened(directory=".", record="life.jsonl", actor="claude-cli:opus")
# `world` is an ear of the host that answers the rest, a prompt to the operator among it.
root = engine.boot(record, world=world, **dict(ears))
```

The ears of the crate are the provider of the models, the extensions, the ear of each official extension, the files,
the commands, time, and the store when the life keeps its record. The extensions enable at the start what the configs
of the user and of the directory turn on, unless `extensions=False`. Each ear is a `NativeEar`, which
`furb_monty.engine.boot` takes beside the generators of this interpreter, and which the engine of this interpreter,
`furb.python`, steps as a generator of its own.

The words of `opened`, each of which can be left out:

- `actor` is the default actor, and `roster` the models that the provider offers beside it. With no roster, it
  offers each model of the catalog, and with neither, the first of them is the default actor. A roster that names no
  model offers the model of the actor alone, and with no actor, the operator alone. [Models](https://github.com/uael/furb/blob/main/docs/models.md)
  says how a model is named.
- `keeps` false keeps nothing in the record, and `inspecting` true opens a life that keeps nothing, enables nothing
  new, and asks no model.
- `extensions` false enables no new extension, and `config` names the config directory of the user.
- `snapshot` false opens the life with a replay of its record, and keeps no snapshot of it.
- `answer`, a function, answers each request in place of the models, as the suite of furb does.
- `stream`, a function, hears what a model writes as it writes it, and `retrying` each retry of a call of a model.
- `log`, a function, hears each report of the life that no model reads, one line each, in place of the stderr of the
  process.
- `claude` names the claude command line to run, and `stall` how many seconds a turn of it may go with no progress.
- `images` is the directory of the images that a message names.

Only the opening of a life makes the files of a record again in its scratch directory, and gives a life with no record
a directory that goes with it. `scratch(record)` gives the scratch directory of a record, which a host removes when it
deletes the record.

## The parts one by one

- `models(claude)` gives the models that the catalog of this machine offers, `model(name)` one model by its name, and
  `levels()` the levels of effort.
- `files(scratch)`, `bash(scratch, record)`, `time()`, `store(path)`, `official()`, `extensions(given)`,
  `memory(config)` and `skills(config)` give the ears one by one, as the suite of each extension boots them. The files
  and the commands share the scratch directory that the host names, which a word reads as `scratch://` and a command
  as `$TMPDIR`.
- `answered(shape, line)` reads a line of the operator as a value of one of the `shapes()`, by the rules that every
  console of the crate reads a line by.
- `_monty.SYSTEM` is the system prompt of every model: the engine, minified with its program kept as it is.

## Work that waits

- `furb_monty.engine.pending()` names the work that an earlier life left, which waits for a wake that this life says,
  each act by its name and its kind. It finds each act that the outside started and did not end, and the acts that
  made it: each prompt, rung, command and wait that is not done and that no pause holds. An act that a wake puts to
  the outside again is pending no more.
- `furb_monty.engine.parked()` names the work that a crash loop parked: a run, a command or a reply that the record
  shows started in five lives and not done, since each of those lives ended while it ran. A wake puts no parked act
  to the outside, so a host that wakes the work of a life as it opens does not run it again and end again. A parked
  act is pending no more.
- `furb_monty.engine.retry(id)` runs a parked act again: it is parked no more in this life, and a wake of it puts it
  to the outside.

## Make an act once

`furb_monty.engine.keyed(key, kind, *args, **kwargs)` makes the act of a verb once for a key of the client, across a
crash of the host and a retry of the client, as `keyed("k1", "prompt", int, "count the lines", on=root)`. A key fact
goes to the record before the act, and a later call with the same key on the same chain gives the act that the key
made, in this life or in a later life on the record. When the host crashed after it kept the key and before it kept
the act, the call makes the act.

## Ears of this interpreter

A generator of this interpreter is heard where the engine runs. A verb that it says while it hears goes to the life
that hears it, as a verb that an ear of the engine of Python says does.

The extension module of this package is the crate at the root of the repository, built with its `python` feature.

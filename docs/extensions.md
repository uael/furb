# Extensions

An extension is python that extends the engine, as a word of a model can. A life plays the word of each extension
that it runs as a rung on each chain, so the model reads the word in its turns, and the names it defines are bound in
the module of the chain. An extension never enters the system prompt or the module of the engine, so a chain that
takes one grows at its end, and what a provider caches of it stands.

## What an extension is

An extension is a folder:

```
extensions/<name>/
  furb.json     the manifest
  <name>.py     the word
  <name>.pyi    the contract, which an official extension has
  test/         the suite, which an official extension has
```

The manifest is a JSON map with three keys:

```json
{ "name": "memory", "word": "memory.py", "life": "remember()" }
```

- `name` is the name of the extension: a lower case letter, then lower case letters, digits and `-`. A config names
  the extension by it, and the name must be the key of the config.
- `word` is the file of the word, relative to the folder. It is a python module that imports what it uses from
  `furb.engine`, so an editor, ruff and ty read it. The word is that file less its imports of furb, with its line
  ends made LF. It only defines, holds no comment, and binds no name of the engine. It holds no sentence but one
  doctrine of its own, a quote that it tells and that holds no law, as `engine.py` holds its own. Its contract holds
  its laws, as `engine.pyi` holds those of the engine.
- `life` is the life word, python that starts the work of the extension, such as `remember()`. It can be absent.

## Where furb finds extensions

Two configs say which extensions a life runs: `config.json` in the config directory of the user, then
`.furb/config.json` in the directory of the life, so the project wins for a name. The config directory is
`FURB_CONFIG_DIR`, else `furb` under `XDG_CONFIG_HOME`, under `%APPDATA%` on Windows, or under `~/.config`. The
`.furb` that a host makes keeps all it holds out of version control but `config.json` and `extensions/`, since the
extensions of a project are code of the project, so a project can share them.

```json
{ "extensions": { "memory": true, "skills": false, "notes": "~/src/furb-notes" } }
```

- `true` turns an extension on, and keeps the path that an earlier config gave.
- `false` turns an extension off.
- A path gives the folder of an extension that is not official. A relative path starts at the folder of the config,
  and `~` is the home.

The official extensions, `memory`, `skills`, `compaction` and `ouroboros`, are on unless a config turns them off. A config that is not of this
form, a folder with no manifest, a manifest of another name, and a missing word are each refused with the path.

## How a life runs its extensions

A life enables each extension by a fact, `enable`, which the record keeps. [`src/furb/CLAUDE.md`](../src/furb/CLAUDE.md)
says what the fact means and when a life says it, under the names enable and life word and in its laws.
The ear `extensions` of [`src/extension.rs`](../src/extension.rs) says the fact and plays it, and its documentation
gives the shape of the fact. The extensions that a host gives the ear are those that the configs turn on. In the
middle of a life, when a write of `.furb/config.json` in the directory of the life lands, the ear enables each
extension that the config turns on by its path and that the life does not run, with the same fact. A word that
the life runs already does not change in that life, and an extension that a config turns off runs on. A host
that turns the extensions off, as the DeepSWE rig does, gives the ear none.

Anything that comes from the disk reaches a chain through a question that an ear of the World answers, which the
journal answers again in a later life, so a replay is exact and the World is asked nothing twice.

## The official extensions

- `skills` defines `counted()`, which tells in one line how many skills the chain knows and that `skills()` lists
  them, and `skills()`, which reads `skills://`: one line for each skill that the World finds in
  `.furb/skills` and `.claude/skills` of the working directory of the chain and of each folder above it, then in
  `skills` of the config directory. `skill(name)` reads the `SKILL.md` file of a skill. It carries one skill of its own, `extending`, which a
  skill of that name in a folder replaces: it shows a model, with an example each, how to extend furb by a def in a
  word and by an extension, and it points to this guide for the rest. Its life word is `counted()`.
- `memory` defines `memory(path)`, which tells the chain the memory files that apply to a path: the memory of the
  user, in the config directory, then that of each folder from the root down to the working directory of the chain,
  then that of each folder under it down to the folder of the path, when the path is under it. The memory of a
  folder is its `CLAUDE.md`, or its `AGENTS.md` when the folder holds no `CLAUDE.md`. A memory file enters the chain
  whole the first time, and after that again whole, only when it changed. `remember()`
  is an act that tells the memory of the working directory when it is made, then the memory of the folder of each
  file that the chain reads, and the memory that changed at each stand. Its life word is `remember()`.

- `ouroboros` teaches a model to extend furb. Its life word, `ouroboros()`, tells each chain its doctrine once: the
  three ways to extend furb and when to take each. A def in a word lives with its chain. `extend(name, *rungs)` lives
  with the project: it writes the words of those rungs, as `program()` holds them, as the word of the extension
  `name` in `.furb/extensions/<name>`, with its manifest, and turns it on in `.furb/config.json`, so it plays on
  every chain now and in every later life. An official extension lives in furb. `enabled()` gives the names of the
  extensions that the life runs. It needs no ear of the World.

- `compaction` defines `compaction()`, an act that watches the fill of its chain. At the first answer of a model
  that reads more than `SHARE`, 0.6, of the window of its actor, it tells the model that share and the word that goes
  on in a chain with a source and a take of the old work, which drops each prompt of the chain that is done. It
  does not act for the model, and it needs no ear of the World. Its life word is `compaction()`.

Each has a contract, `extensions/<name>/<name>.pyi`, which the hygiene laws 1 to 4 of `CLAUDE.md` hold to its suite
as they hold the engine. Laws 5 to 7 are for `engine.py` alone, and laws 8 and 9 hold the word of each extension.
`memory` and `skills` also have an ear of the World in the crate, `src/extension/<name>.rs`, which answers their
questions from the disk. The suite runs on both engines.

## A third party extension

A third party extension has a word and a life word, and no ear of its own. The skill `extending` gives an example
of one. Its word reaches the machine through the
acts that the World takes already: a read, a write, a command, a wait, the clock, a chance, and a prompt to the
operator. It can also make acts with ears of its own, written in python, as `remember` does, and a door that
answers the reads of a scheme of its own. An extension that needs an ear in rust is a change of the crate.

Name only the extensions that you trust. A word runs in the sandbox of monty on the engine of the crate, and with the
rights of the user on the engine of CPython, as a word of a model does there.

## Permissions

furb has no permissions, by design. Each act runs with the rights of the user who runs furb, and the record shows who
made each act. For isolation, run furb in a virtual machine. An extension can add a check: its word can rebind a verb,
such as `bash`, so that the verb asks the operator before it makes the act. A word of the model can rebind that verb
again, so such a check stops a mistake and not an attack. furb has no such extension built in. The extensions of
a project in `.furb/extensions`, as `extend` writes them, run with the rights of the user who runs furb, with no
permission, as the rest of furb does.

## In the hosts

- `furb extensions` prints each extension that a life on a record runs, with its life word, and the state of the
  JSON-RPC names them.
- The TUI has no part of its own for an extension. `/extensions` lists what the life runs, and `/run` runs a word
  of one on the chain on screen.
- `Session` takes `extensions`, false to enable nothing new, and `config`, the config directory of the user.
- `opened` gives the record and the ears of the crate to a host of either door, and `extensions` false enables
  nothing new.
- `enabled`, on the facts of the root, gives the extensions that a life runs, in the order it enabled them, in either
  door.

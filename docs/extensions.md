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
  ends made LF. It only defines, holds no sentence and no comment, and binds no name of the engine. Its contract
  holds its laws, as `engine.pyi` holds those of the engine.
- `life` is the life word, python that starts the work of the extension, such as `remember()`. It can be absent.

## Where furb finds extensions

Two configs say which extensions a life runs: `config.json` in the config directory of the user, then
`.furb/config.json` in the directory of the life, so the project wins for a name. The config directory is
`FURB_CONFIG_DIR`, else `furb` under `XDG_CONFIG_HOME`, under `%APPDATA%` on Windows, or under `~/.config`. The
`.furb` that a host makes keeps all it holds out of version control but `config.json`, so a project can share it.

```json
{ "extensions": { "memory": true, "skills": false, "notes": "~/src/furb-notes" } }
```

- `true` turns an extension on, and keeps the path that an earlier config gave.
- `false` turns an extension off.
- A path gives the folder of an extension that is not official. A relative path starts at the folder of the config,
  and `~` is the home.

The official extensions, `memory` and `skills`, are on unless a config turns them off. A config that is not of this
form, a folder with no manifest, a manifest of another name, and a missing word are each refused with the path.

## How a life runs its extensions

A life enables each extension by a fact, `enable`, which the record keeps, with the name, the word and the life
word. The ear `extensions` of the crate says it at the tip of the life, for each extension that the configs turn on
and the record does not enable yet. While it hears that fact, it plays the extension on every chain there is: one
rung, whose word is the word of the extension and then its life word. After that, it plays the extension on each
chain at its birth. A chain with a source made the rung of its origin again, so it has the word, and the ear plays
the life word alone on it, which starts its own work there.

A later life says each `enable` again at its place, and the ear plays the same rungs there. So a life runs what its
record enables, whatever its configs say then, and the configs only add at the tip. A host that turns the extensions
off, as the DeepSWE rig does, enables nothing new, and runs what its record enables all the same.

Anything that comes from the disk reaches a chain through a question that an ear of the World answers, which the
journal answers again in a later life, so a replay is exact and the World is asked nothing twice.

## The official extensions

- `skills` defines `skills()`, which reads `skills://`: one line for each skill that the World finds in
  `.furb/skills` and `.claude/skills` of the working directory of the chain and of each folder above it, then in
  `skills` of the config directory. `skill(name)` reads the `SKILL.md` file of a skill. Its life word is `skills()`.
- `memory` defines `memory(path)`, which tells the chain the `CLAUDE.md` files that apply to a path, or the
  `AGENTS.md` of a folder that holds no `CLAUDE.md`, whole, and `remember()`, an act that tells the memory of the
  working directory when it is made, then the memory of the folder of each file that the chain reads, and the memory
  that changed at each stand. Its life word is `remember()`.

Each has a contract, `extensions/<name>/<name>.pyi`, which the hygiene laws hold to its suite as they hold the engine,
and an ear of the World in the crate, `src/extension/<name>.rs`, which answers its questions from the disk. The suite
runs on both engines.

## A third party extension

A third party extension has a word and a life word, and no ear of its own. Its word reaches the machine through the
acts that the World takes already: a read, a write, a command, a wait, the clock, a chance, and a prompt to the
operator. It can also make acts with ears of its own, written in python, as `remember` does, and a door that
answers the reads of a scheme of its own. An extension that needs an ear in rust is a change of the crate.

Name only the extensions that you trust. A word runs in the sandbox of monty on the engine of the crate, and with the
rights of the user on the engine of CPython, as a word of a model does there.

## In the hosts

- `furb extensions` prints each extension that a life on a record runs, with its life word, and the state of the
  JSON-RPC names them.
- The TUI has no part of its own for an extension. `/extensions` lists what the life runs, and `/run` runs a word
  of one on the chain on screen.
- `Session` takes `extensions`, false to enable nothing new, and `config`, the config directory of the user.
- `_monty.opened` gives the record and the ears of the crate to a python host, and `extensions=False` enables
  nothing new.

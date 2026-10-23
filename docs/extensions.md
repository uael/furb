# Extensions

An extension adds to what a model can do in furb. It is Python, as a word of a model is: a life runs the word of each
extension on each chain, before any other word, so the functions that it defines are there for the model, and the
model reads the extension in its turns as it reads its own work.

An extension never goes into the system prompt. So a chain that takes one only grows at its end, and what a provider
caches of the chain stays valid.

## The official extensions

Five extensions come with furb, and each life runs them unless a config turns them off.

- **`memory`** tells a chain the memory files of its project. The memory of a folder is its `AGENTS.md`, or its
  `CLAUDE.md` when it has no `AGENTS.md`. When a chain starts, it reads the memory of the user, in the config
  directory, then the memory of each folder from the root down to the working directory. Each time the chain reads a
  file in a folder below, it reads the memory of each folder on the way. A memory file comes whole the first time,
  and again only when it changed. `memory(path)` tells the memory of one path, and `remember()`, its life word, does
  all of this as the chain works.
- **`skills`** gives a chain the skills that it can read. `skills()` lists each skill that it finds in `.furb/skills`,
  `.agents/skills` and `.claude/skills` of the working directory and of each folder above it, then in `skills` of the config
  directory. `skill(name)` reads the `SKILL.md` of one skill. Its life word, `counted()`, says in one line how many
  skills the chain has. It carries one skill of its own, `extending`, which shows a model how to extend furb, with an
  example for each way. A skill named `extending` in a folder takes its place.
- **`compaction`** goes on in a new chain that keeps less of the old one, since a compaction in furb is a chain with
  a source and a filter. `compact()` makes that chain: its filter applies `FILTERS` in order, and its first word binds
  again what the module of the old chain holds with `restore(source)`, binds the summary that it is given, and reads
  again with `reground()` each file that it kept a read or write of, and shows only what changed. `latest()` keeps the
  last read or write of each path, `quiet()` drops the commands, `recent(n)` keeps the acts of the last `n` replies,
  `await chosen()` asks a model which acts to keep, and `together(*filters)` joins filters. `choice()` asks a model,
  on a chain of its own, which acts the work still needs and what the rest holds. `compacted()` watches one chain:
  after an answer that fills more than `SHARE`, 0.6, of the window of its model, it asks `choice()`, and at the next
  reply after the answer, it compacts the chain to what the model chose, with its summary, and moves each open prompt
  of a model to the new chain. It pauses nothing, so the workers that a conductor hired go on. No chain is watched
  unless a word asks for it, so the extension has no life word. A project changes the defaults in an extension of its
  own whose word rebinds `SHARE`, `FILTERS`, `CHOOSER`, `LISTED`, `BRIEF` or `LABEL`.
- **`ouroboros`** teaches a model to extend furb. Its life word, `ouroboros()`, tells each chain once the three ways
  to extend furb and when to take each: a def in a word, which lives with its chain; `extend(name, *rungs)`, which
  writes the words of those rungs as an extension of the project and turns it on, so every later chain runs it; and
  an official extension, which lives in furb. `extend(name, *rungs, tui=module)` also writes a
  [part for the TUI](#a-part-for-the-tui). `enabled()` gives the extensions that the life runs.
- **`web`** searches the web and reads its pages. `web_search(query)` uses the search of the provider of the model of
  the chain, or of the actor that `to` names: the tool WebSearch of the claude command line, the web search of the
  APIs of Anthropic, OpenAI and xAI, Gemini grounded in Google Search, and the search of DeepSeek. For a provider with
  no search, it uses the free search of Parallel for agents, which needs no key. `web_fetch(url)` reads a page, as
  Markdown when it is HTML. Each is an act to await, as a command is, since a page takes seconds. The extension has no
  life word.

`compaction` and `web` each hold a short guide for the model, a quote named after the extension, which the model reads
on each chain.

## Turn an extension on or off

Two configs say which extensions a life runs: `config.json` in the config directory of the user, then
`.furb/config.json` in the directory of the project. The project wins for a name. The config directory is
`FURB_CONFIG_DIR`, else `furb` under `XDG_CONFIG_HOME`, else `furb` under `~/.config`, or under `%APPDATA%` on
Windows.

```json
{ "extensions": { "memory": true, "skills": false, "notes": "~/src/furb-notes" } }
```

- `true` turns an extension on, and keeps the path that an earlier config gave.
- `false` turns an extension off.
- A path turns on the extension in that folder. A relative path starts at the folder of the config, and `~` is your
  home.

A config of another form, a folder with no manifest, a manifest of another name, and a missing word are each refused
with their path. The `.furb` of a project keeps all that it holds out of version control but `config.json` and
`extensions/`, since the extensions of a project are code of the project, which its team shares.

`furb extensions` prints the extensions that a life here runs, and `/extensions` lists them in the TUI.

## Write your own

The smallest extension is a function that a model wrote in a word. When a later chain needs it too, a model calls
`extend(name, ...)` of `ouroboros`, which writes the extension for you. You can also write one by hand: an extension
is a folder.

```
.furb/extensions/<name>/
  furb.json     the manifest
  <name>.py     the word
  tui.ts        the part for the TUI, which it can have
```

The manifest is a JSON map:

```json
{ "name": "notes", "word": "notes.py", "life": "notes()", "tui": "tui.ts" }
```

- `name` is the name of the extension: a lower case letter, then lower case letters, digits and `-`. It must be the
  key that the config gives it.
- `word` is the file of the word, relative to the folder. It is a Python module that imports what it uses from
  `furb.engine`, so an editor, ruff and ty read it. The word that a chain runs is that file less its imports of furb.
  It only defines: it holds no comment and binds no name of the engine. It can hold one quote of guidance for the
  model, which holds no law.
- `life` is the life word, a line of Python that starts the work of the extension, such as `remember()`. It can be
  absent.
- `tui` is the file of the [part for the TUI](#a-part-for-the-tui). It can be absent.

A word reaches the machine only through the acts that furb has already: a read, a write, a command, a wait, the
clock, a chance, and a prompt to the operator. It can also make acts that an ear of its own answers, written in
Python, as `remember()` does, and answer the reads of a scheme of its own. An extension that needs an ear in Rust is
a change of furb. The skill `extending` gives an example of each.

An official extension also has a contract, `<name>.pyi`, whose laws its suite in `test/` holds, as
[the hygiene laws](../AGENTS.md#the-hygiene-laws) say, and an ear of the World in `src/extension/<name>.rs` when it
reads the disk or the network, as `memory`, `skills` and `web` do.

Turn on only the extensions that you trust. A word runs in the sandbox of monty with the engine of the crate, and with
the rights of your user with the engine of CPython, as a word of a model does there.

## A part for the TUI

An extension can give the TUI a part: a module of TypeScript, which its manifest names under `tui`. A part can add a
panel over the input, a row in the feed for each act of a kind, keys, and commands. Nothing builds it: Bun loads the
module as it stands, and loads it again when it changes, with no restart.

![The part of an extension](screenshots/74-extension-part.png)

The module exports a function as its default, which the TUI calls with `furb`:

```ts
export default function (furb) {
  const items = (seen) => seen.of("todo").map((act) => ({ item: String(act.words[0]), done: Boolean(act.words[1]) }));
  furb.panel("Todo", (seen) => items(seen).map(({ item, done }) => [[done ? "✓ " : "○ ", "accent"], [item]]));
  furb.row("todo", (act) => ({ label: String(act.words[0]) }));
  furb.command("todos", "Say how many items are open", (_, seen) => seen.say(`${items(seen).length} items`));
}
```

- `furb.panel(title, draw)` adds a panel over the input while the feed shows. `draw(seen)` gives its lines, each a
  text or a list of `[text, tone]`, where a tone is `text`, `muted`, `faint`, `accent`, `success`, `warning` or
  `danger`. The TUI draws it again each time that the life changes, and shows twelve lines at most.
- `furb.row(kind, draw)` draws each act of a kind in the feed: `draw(act, seen)` gives `{ label, details }`, the
  text after the kind and the lines that its card shows when it opens. The rows of a rung and of a prompt stay the
  TUI's own.
- `furb.key(on, run)` adds a key, as `ctrl+e` or `+`, which acts while no dialog is open. A key with no ctrl, meta or
  alt acts only on an empty input.
- `furb.command(name, detail, run)` adds a command to the palette, which `/name` runs too. A key of the TUI wins over
  a key of a part, and a part that names a command of the TUI fails.
- `seen` gives the chain on screen, every act of the life, `of(kind)` for the acts of a kind, `run(word)`, which runs
  Python on the chain through the gate as `/run` does, and `say(text)`, which shows a notice in the footer.
  `furb.life` is [the model of a life](../bind/typescript/README.md), `furb/life`.

A part imports types alone, so it loads in any project. `tui/src/parts.ts` gives the types, and `tui/fixtures/todo`
is an extension with a part. The TUI loads the parts when it opens a session, and again when a file of the folder of
a part changes, or when a write of the life lands in `.furb` or in the folder of a part, as a write of `extend` does.
It loads them also for a life that enables no extension, such as the demo, since a part reads only the acts of the
life.

When a part fails, the TUI goes on. A module that does not load, that exports no function, or whose function throws
shows its error in plain words in a panel under its name. A panel or a row that throws shows its error in its place,
and a key or a command that throws shows its error in the footer.

## Permissions

furb has no permissions, by design. Each act runs with the rights of the user who runs furb, and the record shows who
made each act. For isolation, run furb in a virtual machine.

An extension can add a check: its word can define a verb again, such as `bash`, so that the verb asks you before it
makes the act. A word of the model can define that verb again too, so such a check stops a mistake and not an
attack. furb has no such extension built in.

## How a life runs its extensions

A life enables each extension by a fact, `enable`, which the record keeps, so a later life on the record runs the
same words. [`src/furb/AGENTS.md`](../src/furb/AGENTS.md) says what the fact means and when a life says it, and the
ear `extensions` of [`src/extension.rs`](../src/extension.rs) says the fact and plays it.

The extensions that a life enables at its start are those that the configs turn on. When a write of
`.furb/config.json` lands in the middle of a life, the life enables each extension that the config turns on by its
path and that it does not run yet. An extension that a config turns off runs on until the life ends.

A life reloads an extension that it runs when its word changes: at its start, or at a later stand, for the word that
the host gives, as a new build gives a new word of an official extension, and when a write of the life lands in the
folder of an extension that a config turns on, as `extend()` does. The life enables it again with the new word, and
each chain plays the definitions of that word alone: its imports, its functions, its classes, its types and its
names in capitals. So a function and a constant take their new value, a name of state such as `queue` keeps its value,
and the life word does not run again. A change of a file that no write of the life makes reaches the life at its
next start. A host that gives no extensions, as the DeepSWE rig does,
enables none.

Each thing that comes from the disk reaches a chain through a question that an ear of the World answers, and a later
life gets the same answer from the record. So a replay is exact, and the World is asked nothing twice.

## In each host

- The command line: `furb extensions` prints each extension that a life on a record runs, with its life word. The
  `state` of the JSON-RPC names them.
- The TUI: `/extensions` lists what the life runs, and `/run` runs a word of one on the chain on screen, as
  `/run skill("pdf")`.
- TypeScript: `Session` takes `extensions`, false to enable nothing new, and `config`, the config directory of the
  user.
- Both doors: `opened` gives the record and the ears of the crate, and `extensions` false enables nothing new.
  `enabled`, on the facts of the root, gives the extensions that a life runs, in the order that it enabled them.

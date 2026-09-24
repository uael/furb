# Extensions

An extension adds verbs to the engine that a model reads, and work to the World and to the TUI. This guide says what
an extension is, where furb finds one, how to write one, and how to publish one. The skills extension in
`extensions/skills` is the example that each section points to.

## What an extension is

An extension has up to three parts, and one part at least:

- A **python part**: python that the module of the engine runs after the engine, before boot. Every chain binds its
  names from its birth, as it binds the names of the engine, and the system prompt reads it after the engine. It
  defines the verbs that a model calls, such as `skills` and `skill`. The manifest may also give a **life word**,
  which the host plays as a rung, as the World, in every life on each chain without a source.
- A **part for a World**: code of the host that does the work of the extension outside the sandbox. It answers the
  questions of the extension with plain data, and it does the acts of its kinds. The TypeScript host has these
  parts now, and the manifest keeps a place for parts in python.
- A **part for the TUI**: slash commands, how the acts of its kinds show, lines of the sidebar, and what it does
  before a message is sent.

## The builtins

The builtins are `files` (read, write, cd, cwd, Text and the shows), `bash` (bash and Exit) and `grant` (grant).
They are definitions of `engine.py` itself, so the engine and its contract say what they do. Each has its parts for
a World and for the TUI in each host. They are on unless a config turns them off.

A host that turns a builtin off cuts its definitions out of the system prompt, which is the text the life runs: the
crate holds the top-level names each builtin defines, and cuts each top-level statement of the engine that binds only
such names. No statement that stays reads a name that goes, as a test of the crate proves for each set of builtins a
config may take, so the gate refuses a word that names a builtin that is off. The code of the engine that only
answers a verb of a builtin, as the chain answers a read of the door of a prompt, stays, and does nothing while no
verb asks it. The World holds no part of a builtin that is off.

`bash` requires `files`. A read and a write cross to the World and back as plain data, a path and a content, and
the engine makes the `Text`, so no class of the engine crosses to a World.

## Where furb looks

- **The config of the user**: `config.json` in the config directory. That is `FURB_CONFIG_DIR` when it is set,
  else `furb` under `XDG_CONFIG_HOME`, else `furb` under `%APPDATA%` on Windows, else `~/.config/furb`. The TUI
  keeps its `ui.json` in the same directory.
- **The config of a project**: `.furb/config.json` of the project. It names an extension over the config of the
  user, by its name. `.furb/.gitignore` holds `*` and `!config.json`, so a project shares its config in version
  control and keeps its records out of it. furb writes that file whenever it is missing.
- **The cache**: `FURB_CACHE_DIR` when it is set, else `furb` under `XDG_CACHE_HOME`, else `furb` under
  `%LOCALAPPDATA%` on Windows, else `~/.cache/furb`. Every fetched extension goes there, from every project.

## The config

A config maps the name of an extension to where it stands:

```json
{
  "extensions": {
    "skills": "../furb/extensions/skills",
    "notes": { "git": "https://github.com/example/notes", "ref": "v1", "path": "extension" },
    "review": { "npm": "@example/review", "version": "1.2.0" },
    "bash": false
  }
}
```

- `false` turns an extension off, a builtin among them.
- `true` keeps the source that an earlier config gave, and turns the extension on again.
- A string, or `{"path": ...}`, is a directory. A relative path resolves against the directory of the config file,
  and `~` is the home.
- `{"git": url, "ref"?: branch or tag, "path"?: folder}` is a git remote.
- `{"npm": package, "version"?: version}` is an npm package, or a path to a tarball or to a folder.

The manifest of an extension must give the name that the config gives it.

## The cache

furb fetches an extension once, the first time a config names it:

- A git remote goes to `<cache>/extensions/git/<sha256 of url#ref>/`, as `git clone --depth 1`, and the extension
  is its folder `path`.
- An npm package goes to `<cache>/extensions/npm/<name>@<version>/`. furb runs `npm pack` and unpacks the tarball
  itself. When the package has dependencies, furb runs `npm install --omit=dev --ignore-scripts` there.

To fetch every extension again, run `furb update` or `/extensions update` in the TUI. A new session takes the
change; a session that exists runs what its record pins.

## The manifest

An extension is a folder with a `package.json`, whose field `furb` is its manifest:

```json
{
  "furb": {
    "name": "skills",
    "python": "skills.py",
    "life": "skills()",
    "world": { "ts": "world.ts" },
    "tui": "tui.ts",
    "requires": ["files"]
  }
}
```

- `name` is the name of the extension, which a config uses.
- `python` names its python part. `life` is its life word.
- `world` names its part for a World, one file for each language that a host writes its World in: `ts` now, and
  `py` later.
- `tui` names its part for the TUI.
- `requires` names the extensions whose names its word reads.

furb takes the builtins first, then the extensions of the config of the user, then those of the config of the
project. An extension stands after each extension it requires. furb refuses an extension whose requirement is off,
and says to turn it off too, or to turn the requirement on. It refuses a cycle of requirements, and a python part
that does not parse, with its file and its line.

## The python part

A python part has one of two forms, and one rule serves both:

- **Form (a)**: a word, a python file that the host runs as it is.
- **Form (b)**: a real module, which imports what it uses from the engine, as `from furb.engine import ask, read,
  tell`. An editor, ruff and ty check it, and it has its own tests. The skills extension is of this form.

The word of a module is the module less each top-level `from furb...` import, with its line ends made LF. An import
leaves no line of its own: the empty lines where it stood are as many as stood on either side of it, and a word
starts and ends with its code. Every other byte stays as it is. The word is what the system prompt reads, so it
reads as a python file that a person wrote. A word of form (a) has no such import, so the host runs it as it is.

**Where it runs.** The module of the engine runs each word after the engine, in the order of the extensions, before
boot. Every chain copies the module of the engine at its birth, so every chain binds the names of the words, and a
later rung may bind any of them again, as it may bind a name of the engine. The gate reads a word of a model after
the system prompt, so it refuses a name that the prompt does not bind. A word defines; it asks the bus only from a verb,
since no life lives while it runs.

**What a word may use.** A word speaks through the bus: `ask` for a query, `act` for an act, `send` for a fact,
`tell` for notes. The World answers a question of an extension with plain data, and the verb makes its own values of
it.

**The pin.** A life says once, as the World, the builtins it takes and the words it runs, in a fact of the kind
`extensions` about the root, which the record keeps. A later life on that record runs the same system prompt,
whatever the configs say then, since a record is made again by running its words. A life that takes every builtin
and runs no word pins nothing, and a life on a record that pins nothing takes every builtin and runs no word. The
crate holds this rule, so every host runs the same text.

**The life word.** The host plays each life word as a rung, as the World, on every chain without a source, once
the life stands on its record, and at the birth of each such chain after. A chain with a source runs the rungs of
its origin again, so it plays nothing of its own.

**Its contract and its suite.** An extension of the repository has its contract beside its python part, as
`skills.pyi`, and its suite in `test/`. The hygiene laws hold it as they hold the engine: one test for each
sentence, whose docstring is that sentence, and no name of its word bound again beneath a name of the engine or of
another word. A test of the suite runs the words through the harness of `test/conftest.py`, which runs each test on
both engines.

## The system prompt

The system prompt of a life is the text the life runs: the engine, minified in layout alone, less the definitions of
each builtin that a config turns off, then the word of each extension the life runs, each after an empty line. A host
gives the life the engine and the builtins it takes. The life runs that text, the gate reads a word after it, and
`life.system` gives it to the host, which sends it to each model. The python host runs `engine.py` less the names
that each builtin it does not take defines, then the words, which is the same program; `system_prompt` gives it the
text.

## The part for a World

In TypeScript, the file exports a function as its default. The function takes a `WorldContext` and gives a
`WorldPart`:

```ts
import type { Fact, WorldContext, WorldPart } from "@furb/engine";

export default function skills(context: WorldContext): WorldPart {
  return {
    *hears([kind, qid]: Fact) {
      if (kind === "skills") yield ["done", qid, found(roots(context.directory, context.config))];
    },
  };
}
```

- `hears(fact)` hears every fact of the life. It is a generator: it yields a saying, `[kind, about, ...words]`,
  which the life makes a fact of, or a call, `{verb, args, kwargs}`, which the life answers before the part goes
  on. A raised call is thrown into the generator.
- `kinds` names the kinds of act that the part does. The World hands it the start of an act of its kinds through
  `hears`. It closes the start of a kind that no part does with `Refused("the World does no <kind>")`.
- `live` gives the value of an act of its kinds in `world.activity`, at its birth and as a fact changes it, which
  the TUI shows. `dispose` ends what the part holds.
- The context gives the directory of the World, the config directory, `where(on)` for the working directory of a
  chain, `at(here, path)`, `speak` and `close` to say a fact later as the World, `change` for a change of a file
  that the TUI shows, `spawn` for a command of the shell, and `refused` for a refusal.

An act that the World does is pending in a later life when the record shows it begun and not done. The TUI and
`inspectRecord` see this with no code of the extension.

Write the file in erasable TypeScript only, and import only types from `@furb/engine`, as `import type`. The file
runs from the cache, which holds no copy of the package, so every helper comes through the context. Bun imports a
`.ts` file anywhere, and Node.js 22.18 or later imports one outside `node_modules`.

## The part for the TUI

The file exports a function as its default, which gives a `TuiPart`:

- `commands`: slash commands, each with its label, its argument, what it does, the key that says it, whether its
  value is a path, the values it offers, and `run(argument, context)`.
- `prefixes`: a prefix of the input that says a command, as `!` says `/bash`.
- `acts`: how the acts of a kind show: the subject of the card, a preview, the details when the card opens, whether
  a pause holds them, whether they stand over the chain, whether they are hidden, the words that end them, and the
  names of their words.
- `quiet`: header words of notes that no card shows. `paths`: header words whose detail is a path.
- `sidebar(view)`: rows under the usage and a mark on the meter of the context.
- `prompting(message, context)`: what the part does before a message is sent, as the files extension reads each
  file that the message names with `@`.

The context gives the life, the chain on screen and its directory, the acts, `call` for a verb on that chain,
`path`, `notify`, `submit`, `track` and `show`. Two parts that claim one command, one prefix or one kind are
refused with both names. `/extensions` lists the extensions of a session. The feed shows the rungs of the life words
that the World played as one line, `extensions`, which names their extensions.

## Walkthrough: the skills extension

`extensions/skills` holds:

- `package.json`: the manifest above, and the npm package `@furb/skills`.
- `skills.py`: the python part, of form (b). `skills()` asks the World a `skills` question on its chain, and tells
  one line for each skill that is new, changed or gone since the question before it, which it finds in `asked` and
  `outcomes`. `skill(name)` reads the `SKILL.md` of a skill into the chain through `read`, so the manifest requires
  `files`.
- `skills.pyi` and `test/`: its contract and the suite of its sentences, on both engines.
- `world.ts`: the part for a World, which answers the `skills` question with the skills of `.furb/skills`,
  `.claude/skills` and the config directory, as plain data.
- `tui.ts`: `/skills`, `/skill <name>` and `/reload-skills`.
- `test/world.test.ts` and `test/tui.test.ts`: the tests of the parts.

The life word, `skills()`, runs in every life, so a chain tells its skills before its first ask, and a later life
tells what changed. The crate test and the TypeScript test prove that skills loads the same by a path, a git remote
and an npm package. The python host runs its word and holds no part of it for a World, so there `skills()` finds no
skill.

## Publishing an extension

- **git**: push the folder. A config names it as `{"git": url, "path": folder}`, with a `ref` to pin a version.
- **npm**: give the package the manifest, the `files` it ships, `@furb/engine` as an optional peer dependency for
  its types, and `"publishConfig": {"access": "public"}`. In the workspace, pack it with `bun pm pack`, which
  rewrites `workspace:*`, then run `npm publish` on the tarball. An extension with no dependencies needs no install
  in the cache.

## Records of 0.1.0

A record of furb 0.1.0 opens. It answers a read and a write with a text as the mark of its class, and the life
replays each query from the record by its name, so a read and a write of that record give the text as it is.

## The names of the extension system

- **extension**: what a host adds to the engine: a word, a life word, and parts for a World and for the TUI.
- **builtin**: files, bash or grant: definitions of `engine.py` that a host takes unless a config turns them off.
- **manifest**: the field `furb` of the `package.json` of an extension.
- **word of an extension**: the python that the module of the engine runs after the engine, before boot.
- **word of a module**: the module less its imports of furb, with its line ends made LF.
- **life word**: a word of the manifest that the host plays as a rung in every life.
- **pin**: the fact `extensions` of the World, which says the builtins a life takes and the words it runs, so a later
  life runs the same.
- **config**: `config.json` of the config directory, or `.furb/config.json` of a project.
- **cache**: where furb keeps the extensions it fetched.

## Trust

The python part runs in the sandbox, as the engine does. A part for a World and a part for the TUI run with the
rights of the user, as any program the user starts. Name only the extensions that you trust. furb runs every npm
command with `--ignore-scripts`, so a fetch runs no script of a package.

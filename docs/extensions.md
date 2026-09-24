# Extensions

An extension adds verbs to the life of furb, and work to the World and to the TUI. The engine knows no extension:
a host plays the word of each one as a rung, and holds its parts. This guide says what an extension is, where furb
finds one, how to write one, and how to publish one. The skills extension in `extensions/skills` is the example
that each section points to.

## What an extension is

An extension has up to three parts, and it has one part at least:

- A **python part**: a python file whose word the host plays as a rung on every chain without a source. It defines
  the verbs that a model calls, such as `read`, `bash` or `skill`.
- A **part for a World**: code of the host that does the work of the extension outside the sandbox. It answers the
  questions of the extension, and it does the acts of its kinds. The TypeScript host has these parts now, and the
  manifest keeps a place for parts in python.
- A **part for the TUI**: slash commands, how the acts of its kinds show, lines of the sidebar, and what it does
  before a message is sent.

The builtins are extensions too: `files` (read, write, cd, cwd, Text and the shows), `bash` (bash and Exit) and
`grant` (grant). They are on unless a config turns them off. The crate carries their words, so every host plays the
same words.

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

To fetch every extension again, run `furb update` or `/extensions update` in the TUI. A new session plays the
change.

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
- `python` names its python part. `life` is a word that the host plays in every life, after the word of the module.
- `world` names its part for a World, one file for each language that a host writes its World in: `ts` now, and
  `py` later.
- `tui` names its part for the TUI.
- `requires` names the extensions whose words its word reads.

furb plays the builtins first, then the extensions of the config of the user, then those of the config of the
project. An extension stands after each extension it requires. furb refuses an extension whose requirement is off,
and says to turn it off too, or to turn the requirement on. It refuses a cycle of requirements, and a python part
that does not parse, with its file and its line.

## The python part

A python part has one of two forms, and one rule serves both:

- **Form (a)**: a word, a python file that the host plays as it is.
- **Form (b)**: a real module, which imports what it uses from the engine and from the extensions it requires, as
  `from furb.engine import ask, tell` and `from furb.builtin.files import Text, read`. An editor, ruff and ty check
  it, and it has its own tests. The builtins and the skills extension are of this form.

The word of a module is the module with each line of a top-level `from furb...` import made an empty line, and its
line ends made LF. Every other line stays as it is, so a finding of the gate points at the line of the file. A word
of form (a) has no such import, so the host plays it as it is.

Why the imports are blanked, and the gate and the Kernel do not follow them: the module of a chain already binds
every name of the engine and of each word played before it, and the engine reads its own names through the globals
of the chain, so a name that a later rung binds again is used from its next use on. An import would pin the object
of the module it names, and a verb bound again would not reach the word. The sandbox of monty holds no package
`furb`, so the gate and the Kernel stay blind to packages, and the program that a model reads holds no import it
cannot run.

**When it plays.** The host plays the words of the extensions as the World, on every chain without a source, as
rungs. It plays nothing while boot replays the record. Once the life stands on its record, it plays each word that
the program of such a chain does not hold yet, in order, then each life word. From then on it plays them at the
birth of each such chain. A chain with a source runs the words of its origin again, so it gets nothing of its own.
The crate holds this rule, so every host plays the same. A later life plays a word that changed as one more rung,
and that word binds last. One limit: a model that opens a chain and prompts it in the same word gets its first ask
before the rungs of the words, and they run before its answer.

**What a word may use.** A word runs in the module of a chain, which binds the engine and the words before it. It
speaks through the bus: `ask` for a query, `act` for an act, `send` for a fact, `tell` for notes. The World answers
a question of an extension with plain data, and the verb makes its own values of it, so a record holds no class of
an extension. A later life reads the record before any word of an extension runs.

**Its contract and its suite.** An extension of the repository has its contract beside its python part, as
`skills.pyi`, and its suite in `test/`. The hygiene laws hold it as they hold the engine: one test for each
sentence, whose docstring is that sentence, and no name of its word bound again beneath a name of the engine or of
another word. A test of the suite plays the words through the harness of `test/conftest.py`, which runs each test
on both engines.

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
refused with both names. `/extensions` lists the extensions of a session.

## Walkthrough: the skills extension

`extensions/skills` holds:

- `package.json`: the manifest above, and the npm package `@furb/skills`.
- `skills.py`: the python part, of form (b). `skills()` asks the World a `skills` question on its chain, and tells
  one line for each skill that is new, changed or gone since the question before it. `skill(name)` reads the
  `SKILL.md` of a skill into the chain through `read`, so the manifest requires `files`.
- `skills.pyi` and `test/`: its contract and the suite of its sentences, on both engines.
- `world.ts`: the part for a World, which answers the `skills` question with the skills of `.furb/skills`,
  `.claude/skills` and the config directory, as plain data.
- `tui.ts`: `/skills`, `/skill <name>` and `/reload-skills`.
- `test/world.test.ts` and `test/tui.test.ts`: the tests of the parts.

The life word, `skills()`, runs in every life, so a chain tells its skills before its first ask, and a later life
tells what changed. The crate test and the TypeScript test prove that skills loads the same by a path, a git remote
and an npm package. The python host plays its word and holds no part of it for a World, so there `skills()` finds
no skill.

## Publishing an extension

- **git**: push the folder. A config names it as `{"git": url, "path": folder}`, with a `ref` to pin a version.
- **npm**: give the package the manifest, the `files` it ships, `@furb/engine` as an optional peer dependency for
  its types, and `"publishConfig": {"access": "public"}`. In the workspace, pack it with `bun pm pack`, which
  rewrites `workspace:*`, then run `npm publish` on the tarball. An extension with no dependencies needs no install
  in the cache.

## Records of 0.1.0

A record of furb 0.1.0 opens. Its words called `read`, `write` and `bash`, which were verbs of the engine then, and
the reader keeps the mark of a class it does not know, as `{"is": "Text", ...}`, as its plain fields. The words of
the builtins are played after the record replays, so an old word finds no `read` while it replays and the gate
refuses it. Such a record opens when nothing hangs on its old acts. A record whose later acts hang on an old read,
write or bash drifts. No tool migrates it.

## The names of the extension system

- **extension**: a word that a host plays on a chain, with a part for a World and a part for the TUI that the host
  holds.
- **builtin**: an extension that the crate carries, on unless a config turns it off.
- **manifest**: the field `furb` of the `package.json` of an extension.
- **word of a module**: the module with its imports of furb made empty lines and its line ends made LF.
- **life word**: a word of the manifest that the host plays in every life.
- **play**: to run a word as a rung on a chain, as the World.
- **config**: `config.json` of the config directory, or `.furb/config.json` of a project.
- **cache**: where furb keeps the extensions it fetched.

## Trust

The python part runs in the sandbox, as every word does. A part for a World and a part for the TUI run with the
rights of the user, as any program the user starts. Name only the extensions that you trust. furb runs every npm
command with `--ignore-scripts`, so a fetch runs no script of a package.

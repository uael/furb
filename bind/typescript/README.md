# furb for TypeScript

The same Rust crate and Monty sandbox, through N-API. Queries and controls are synchronous. An `Act` has an
`id`, converts to its name as a string, and can be awaited. Its Promise resolves with its outcome or rejects
with an error that names the engine's fault. A TUI is not part of this package.

From the repository root:

```sh
uv sync
bun install
bun run build
bun test bind/typescript/test
```

`napi-rs` generates `index.cjs` and `index.d.cts` from `src/binding/ts.rs`. The optional `typescript` feature
builds the binding in the existing crate. There is no Rust worker or second crate. TypeScript compiles the supplied
TypeScript World to JavaScript. The package runs on Bun and Node.js 22 or later, on macOS, Linux and Windows.
The supplied World runs a command with `/bin/sh`, and on Windows with the `sh` on `PATH`, such as the one of Git
for Windows. It ends a command with the processes that the command started: on Unix by the process group of the
command, and on Windows by its tree of processes, through `taskkill`. Build the native binary for the host before
using the package. Bun gives no signal on Windows for Ctrl+Break or for the close of the console, and ends the process
at once. `onConsoleEnd(callback)` hears these events there, and the system holds the process until the callback ends
it; the TUI quits through it.

```ts
import { boot } from "@furb/engine";

const session = await boot({
  cwd: process.cwd(),
  record: ".furb/work.jsonl",
  // With no model, the World offers the operator alone, and this callback answers what is put to it.
  operator: async ({ message }) => `You asked: ${message}`,
});
try {
  const { life } = session;
  console.log(life.clock()); // A synchronous query.
  console.log(life.call("cwd", [], { on: life.root })); // A verb of the files extension, by its name.
  const work = life.prompt<string>("str", "What is this project?");
  console.log(work.id);
  const answer = await work;
  console.log(answer); // You asked: What is this project?
} finally {
  await session.dispose();
}
```

The default `World` provides time, chance, records, waits, model requests, and operator questions, and it
hands every other fact to the parts of its extensions: the builtin parts of `files` and `bash` give files,
streamed shell commands, stdin, and timeouts. It knows no provider of its own: it asks the pi-ai collection it is given in
`models`, the built-in providers when it is given none, and it preserves provider response blocks in the
record. The host names what the World offers in `roster`, as `provider:model`, and the default actor in `model`,
which is the first of the roster when unsaid; a World given neither offers the operator alone, and a prompt
that names no actor goes to the operator. A name without its provider routes to the one model of that id, by
the rule that `modelNamed(models, name)` gives a host. A
reopened World offers what its host names now. It keeps the model and the effort that the host chose last, and
takes that model when the host names none and the World holds it. The record keeps the standing that each chain
was lived on, and the later life tells each chain whose standing changed with a `stood`. Configure an API
provider through its pi-ai credentials.

The Claude CLI provider, `claudeProvider` from `@furb/engine/claude`, is a pi-ai provider that a host adds to
its collection at run time. It follows the pooled session design in [dirt](https://github.com/uael/dirt/tree/main/packages/cli/src/providers).
It reads current pi-ai system messages, keeps a warm conversation for each session id, sends only new messages,
preserves text and thinking blocks, and reports the cost of each turn. The World gives each chain of each life a
session id of its own, since the ids of chains repeat in every life. It runs pure completions with CLI tools and
MCP disabled. It finds the standalone CLI (`claude.exe` on Windows) or the CLI installed by Claude Desktop. Set
`FURB_CLAUDE_BIN` to select a binary; `DIRT_CLI_BIN` is also accepted. Its pool belongs to the host that made the
provider, and its `dispose` stops it. No CLI process starts until a model is asked.

```ts
import { builtinModels } from "@earendil-works/pi-ai/providers/all";
import { World } from "@furb/engine";
import { claudeProvider } from "@furb/engine/claude";

const claude = claudeProvider();
const models = builtinModels();
models.setProvider(claude.provider);
const roster = claude.provider.getModels().map((model) => `${model.provider}:${model.id}`);
const world = new World({ models, roster });
```

Pass `answer` to replace only model requests, or `operator` to supply operator answers. The host still names
the models that `answer` stands in for, and a World given `answer` and no model is refused, since every prompt
would go to the operator. With no `operator`, questions stand in `world.prompts`; call `world.answer(id, text)`
to parse and validate an answer. `world` emits `change` and `facts`; `fault` reports a failure to deliver an
outside result. `world.facts` holds the facts of the life but the answers to the queries that the host asks
outside a rung, which no record keeps either, so a listener that asks the life hears no change of its own. A
failed outside act carries its refusal in the record. A `Keep` writes and syncs one complete record entry
before it returns. The engine phrases every turn as python, so the World renders nothing. A turn is its role,
its python, its usage and the blocks of its provider, as `life.turns(chain)` gives it. The World hands a
provider the python of each user turn as it is, and no user turn that holds nothing, and the `answer` callback
receives the same turns.

Pass `world` to `boot` to replace what every World does. It receives these operations, and the parts of the
extensions do the rest, as they do in the supplied World:

| Operation | Arguments | Answer |
| --- | --- | --- |
| Stand | none | `[roster, directory, actor]` |
| Clock, Chance | none | number |
| Keep | entry | nothing |
| Ask | rung, chain, actor, turns | Promise of a turn |
| Wait | seconds, act id | Promise that resolves when time passes |
| Prompt | id, shape, message | Promise of an operator answer |

Stand, Clock, Chance, and Keep answer synchronously. They must not call back
into the same life. For a World that needs nested engine queries, use `Ears`: its generators yield a saying
`[kind, id, ...words]`, a call `{verb, args, kwargs}`, or nothing. A yielded call is answered before the ear
continues, as in the Python binding. `Ears.callable` carries a JavaScript show or filter into the engine,
from the `Ears` the life boots on: `world.ears` for the supplied World, and `session.ears` from `boot`.
`Life.call` reaches every public engine verb beyond the named methods.

Values use the Python record form. Faults carry `is` and `args`. A value of a class that a word defined, such as
the `Text` of the files extension and the `Exit` of the bash extension, crosses out as
`{is: "instance", class: {is: "class", id, name, base}, value}`, where `value` holds its fields. `unwrapped(value)`
gives every instance in a value as its fields, `isInstance(value, name)` says whether a value is an instance of a
class of that name, and `remade(instance, fields)` gives an instance with other fields, which crosses back into the
life as its class.
Lists and tuples cross as arrays, and maps keep their order. Every value of the engine crosses to JavaScript,
so an ear hears every fact. A value that JSON holds only in part crosses as its type under `is` and what that
type makes it from under `args`, and comes back in whole: an int past the safe range as
`{"is":"int","args":["1180591620717411303424"]}`, a float that is not finite as `{"is":"float","args":["inf"]}`,
and a map with a key that is no string as `{"is":"dict","args":[[[1,"a"]]]}`. Any other value, and a value
nested beyond 64 levels, crosses as its Python representation. A map that holds the key `is` crosses as its
pairs under the same mark, `{"is":"dict","args":[[["is","x"]]]}`, both ways, so no map of a word reads as a
mark; a host marks its own such map the same way, as `world.answer` does for an answer of the operator. Into the engine, a whole JavaScript number is
an int and a number with a fraction is a float, so a whole float goes in its form, `{"is":"float","args":["2"]}`.
A whole number past the safe range is refused, since
JavaScript holds it rounded: send a BigInt, or the `int` form above. A BigInt past 64 bits reaches the engine
as its digits in a string. `inspect(name, chain)` also gives the Python type and representation of a value.

Records preserve integral floats as `{"is":"float","args":["1"]}`. The native record reader checks integer
precision before JavaScript can round a number. A query of a rung enters the record with its answer, and a
query that the host asks outside a rung enters none. To keep a program edit across a later open, write the new
program to the door of its prompt in a `rung`, as `write(Text(prompt, program))`.

A record whose replay drifts gives a life all the same, and `life.raised` holds the drift; that life keeps
nothing more. `World.open` refuses such a record with the drift.

Only one process owns a record. Its `RecordLock` holds a lock on `<record>.lock`, which the system releases
when the process ends, so a lease of a process that ended never blocks an open. The holder may move the lock file
with the record, and a process that locked the moved file opens the path again. On Windows the lock also refuses
a read of the file by any other handle; the file holds no text, and nothing reads it. A torn final line is removed
before an append; a damaged complete line fails.
The record keeps a command, a wait and a prompt to the operator from its start, and a rung from its ask. What
it shows begun and not done is pending in a later life: the engine starts none of it, and asks for no rung of
it, until a wake that this life says. `world.pending` holds that work, and `world.resume()` says a wake of each
chain that holds some. Work that a pause of the operator holds is not in `world.pending`, and it waits for the
wake of the operator. A command that an earlier World started and did not end runs again at that wake, once,
and what it told before stands in its door. Wait deadlines and partial streams live in the record's
`.world.json` companion. File snapshots append to `.changes.jsonl`; `world.changes.read` loads a page of them.
Keep both companions with the JSONL record. The World saves `.world.json` whole with `saveFile(path, text)`,
which writes `<path>.tmp` and gives it the name of the file. A save that fails throws an error that names the file,
with the error of the system as its cause. The TUI saves its own files with it.

`inspectRecord(path, models)` reads pending work through the same native replay without taking a record lock,
writing files, or starting a model or command. The TUI runs this inspection in its own worker. `World.activity` holds
the state of every act, derived once from the facts as the life hears them, so a host reads it without asking the
sandbox. It asks the engine's `covers` which live acts a pause or a wake is over, one call for each act whose state
the control would change. `World.isPaused` and `World.rungState` read it.

`world.attachImage(path)` copies an image into the record's `.images` directory and returns its name, type,
size, and `furb-image://` reference. A World with no record copies it into `.furb/images` of its directory. The
`.furb` holds a `.gitignore` that keeps it out of version control but its `config.json`, which `furbDirectory`
writes whenever it is missing.
Put that reference in the prompt as a Markdown image,
`![design](furb-image://...)`, which `imageReference(image)` writes and `imageReferences(message)` reads. The
World hands the python of the turn as it is, and adds each image that the message of a prompt of that turn
references as a pi-ai image block. The stored bytes are checked against their digest before use. PNG, JPEG, GIF, and WebP are
supported, with a 20 MiB limit per image. Keep `.images` with the record when moving a session.

## Extensions

An extension adds verbs to the engine that a model reads, and parts to the World and to the TUI. [The guide of the
extensions](../../docs/extensions.md) says how to write one and how a config names it. This package gives what a
host in TypeScript needs:

- `resolveExtensions(project, {refresh, install})` gives the extensions that a host takes for a project: the
  builtins `files`, `bash` and `grant`, and what `config.json` of the config directory and `.furb/config.json` of the
  project name, fetched once into the cache, and again on a refresh. Each `Extension` has its `name`, whether it is
  `builtin`, its `root`, the `word` of its python part and its `life` word, what it `requires`, and the files of its
  parts, `world.ts`, `world.py` and `tui`. It throws with what failed. `builtinExtensions()` gives the builtins
  alone. `configDirectory()` and `cacheDirectory()` give the directories, which `process.env` names:
  `FURB_CONFIG_DIR` and `FURB_CACHE_DIR`, then the directories of XDG, of Windows, and of the home. `wordOf(source)`
  gives the word of a python part, and `systemPrompt(engine, taken, words)` the system prompt of a life: the engine
  less the definitions of each builtin that `taken` does not name, then the words.
- `Life.boot(callback, names, record, words, lives)` runs the words in the module of the engine after the engine, or
  the words the record pins, and pins them when the record pins none; `life.words` gives the words it runs. It plays
  the life words as the World, on every chain without a source, once boot stands on its record and at the birth of
  each such chain after. The supplied World and `boot` give them, and the World builds its system prompt from them.
- `World.load(options)` resolves the extensions of the directory when the options name none, imports the part for a
  World of each one, and gives the World. `new World(options)` takes the `extensions` and the `parts` it is given,
  and the builtins when it is given none. It refuses an extension whose part for a World it does not hold.
- A part for a World is the default export of the file of the extension, a function of a `WorldContext` that gives
  a `WorldPart`: the `kinds` of act it does, `hears(fact)`, a generator that yields a saying or a call and is given
  back what the life made of it, `live`, which gives the value of its acts in `world.activity`, and `dispose`. The
  World closes the start of an act of a kind that no part does with `Refused("the World does no <kind>")`. A part
  answers a question with plain data. The context gives the directory, `where(on)`, `at(here, path)`, `speak`,
  `close`, `change`, `spawn` and `refused`. The file imports types alone, since it runs from the cache.
- A part for the TUI is the default export of its file, a function that gives a `TuiPart`: its `commands`, the
  `prefixes` of the input that say one, how the `acts` of its kinds show, the `quiet` header words, the `paths`
  header words, what it adds to the `sidebar`, and what it does in `prompting` before a message is sent.
  `loadWorldParts` and `loadTuiParts` import the parts of a list of extensions.

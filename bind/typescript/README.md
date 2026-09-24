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

const session = boot({
  cwd: process.cwd(),
  record: ".furb/work.jsonl",
  // With no model, the World offers the operator alone, and this callback answers what is put to it.
  operator: async ({ message }) => `You asked: ${message}`,
});
try {
  const { life } = session;
  console.log(life.cwd()); // A synchronous query.
  const work = life.prompt<string>("str", "What is this project?");
  console.log(work.id);
  const answer = await work;
  console.log(answer); // You asked: What is this project?
} finally {
  await session.dispose();
}
```

The default `World` provides files, streamed shell commands, stdin, timeouts, time, chance, records, model
requests, and operator questions. It knows no provider of its own: it asks the pi-ai collection it is given in
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

Pass `world` to `boot` to replace the whole World. It receives these operations:

| Operation | Arguments | Answer |
| --- | --- | --- |
| Stand | none | `[roster, directory, actor]` |
| Read | directory, path | `{path, content}` |
| Write | directory, path, content | `{path, content}` |
| Clock, Chance | none | number |
| Keep | entry | nothing |
| Ask | rung, chain, actor, turns | Promise of a turn |
| Run | `{id, here, command, fed, timeout, merged}` | nothing; send out/exited later |
| Feed | command id, text or null | nothing |
| Slay | command id | nothing |
| Wait | seconds, act id | Promise that resolves when time passes |
| Prompt | id, shape, message | Promise of an operator answer |

Stand, Read, Write, Clock, Chance, Keep, Run, Feed, and Slay answer synchronously. They must not call back
into the same life. For a World that needs nested engine queries, use `Ears`: its generators yield a saying
`[kind, id, ...words]`, a call `{verb, args, kwargs}`, or nothing. A yielded call is answered before the ear
continues, as in the Python binding. `Ears.callable` carries a JavaScript show or filter into the engine,
from the `Ears` the life boots on: `world.ears` for the supplied World, and `session.ears` from `boot`.
`Life.call` reaches every public engine verb beyond the named methods.

Values use the Python record form. Text and Exit carry `is` plus their fields. Faults carry `is` and `args`.
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
query that the host asks outside a rung enters none. To keep a program edit across a later open, perform its
`write` in a `rung`.

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
`.furb` that it makes holds a `.gitignore` that keeps it out of version control, as `furbDirectory` makes it.
Put that reference in the prompt as a Markdown image,
`![design](furb-image://...)`, which `imageReference(image)` writes and `imageReferences(message)` reads. The
World hands the python of the turn as it is, and adds each image that the message of a prompt of that turn
references as a pi-ai image block. The stored bytes are checked against their digest before use. PNG, JPEG, GIF, and WebP are
supported, with a 20 MiB limit per image. Keep `.images` with the record when moving a session.

# furb for TypeScript

The same Rust crate and Monty sandbox, through N-API. Views, queries and controls are synchronous. An `Act` has
an `id`, converts to its name as a string, and can be awaited. Its Promise resolves with its outcome or rejects
with an error that names the engine's fault. A TUI is not part of this package.

From the repository root:

```sh
uv sync
bun install
bun run build
bun test bind/typescript/test
```

`napi-rs` builds `furb.node` and generates `index.d.cts` from `src/binding/ts.rs`, and `index.cjs` loads it. The optional `typescript` feature
builds the binding in the existing crate. There is no Rust worker or second crate. The `Engine` of the package has
one method for each verb of the contract, which the build of the crate makes from `src/furb/engine.pyi`, as it
makes the methods of the crate: the words that the verb needs, in their order, then an object of the words that have
a default, where nothing leaves the default of the engine. A verb of the operator names its chain in `on`. The
package runs on Bun and Node.js 22 or later, on macOS, Linux and Windows. The ear of commands of the crate runs a
command with `/bin/sh`, and on Windows with the `sh` on `PATH`, such as the one of Git for Windows. It ends a
command with the processes that the command started: on Unix by the process group of the command, and on Windows
by its tree of processes, through `taskkill`. Build the native binary for the host before using the package. Bun
gives no signal on Windows for Ctrl+Break or for the close of the console, and ends the process at once.
`onConsoleEnd(callback)` hears these events there, and the system holds the process until the callback ends it;
the TUI quits through it.

```ts
import { boot } from "@furb/engine";

const session = boot({
  cwd: process.cwd(),
  record: ".furb/work.jsonl",
  // With no model, the provider offers the operator alone, and this callback answers what is put to it.
  operator: async ({ message }) => `You asked: ${message}`,
});
try {
  const { engine } = session;
  const on = engine.root;
  console.log(engine.cwd({ on })); // A synchronous view.
  const work = engine.prompt("str", { message: "What is this project?", on });
  console.log(work.id);
  const answer = await work;
  console.log(answer); // You asked: What is this project?
} finally {
  await session.dispose();
}
```

A `Session` opens an engine on the ears of the World: the files, the commands, time, and the store of the record,
which the crate writes and the package gives as `files()`, `bash()`, `time()` and `store(path)`, and the provider
of models and the console of the operator, which this package writes. The files read and write a path against the
directory where its chain stands. The commands stream what a command writes, feed its stdin, and end it at its
timeout. Time gives the clock and a chance, and ends a wait. The provider knows no provider of models of its own: it
asks the pi-ai collection it is given in `models`, the built-in providers when it is given none, and it preserves
provider response blocks in the record. The host names what the provider offers in `roster`, as
`provider:model`, and the default actor in `model`, which is the first of the roster when unsaid; a provider given
neither offers the operator alone, and a prompt that names no actor goes to the operator. A name without its
provider routes to the one model of that id, by the rule that `modelNamed(models, name)` gives a host. A reopened
session offers what its host names now. It keeps the model and the effort that the host chose last, and takes that
model when the host names none and the provider holds it. The journal keeps each stand and what the provider
answered it, and every life stands again as it opens, so a later life tells the new standing on each chain whose
standing changed. Configure an API provider through its pi-ai credentials.

The Claude CLI provider, `claudeProvider` from `@furb/engine/claude`, is a pi-ai provider that a host adds to
its collection at run time. It follows the pooled session design in [dirt](https://github.com/uael/dirt/tree/main/packages/cli/src/providers).
It reads current pi-ai system messages, keeps a warm conversation for each session id, sends only new messages,
preserves text and thinking blocks, and reports the cost of each turn. The provider gives each chain of each life a
session id of its own, since the ids of chains repeat in every life. It runs pure completions with CLI tools and
MCP disabled. It finds the standalone CLI (`claude.exe` on Windows) or the CLI installed by Claude Desktop. Set
`FURB_CLAUDE_BIN` to select a binary. Its pool belongs to the host that made the
provider, and its `dispose` stops it. No CLI process starts until a model is asked.

```ts
import { builtinModels } from "@earendil-works/pi-ai/providers/all";
import { Session } from "@furb/engine";
import { claudeProvider } from "@furb/engine/claude";

const claude = claudeProvider();
const models = builtinModels();
models.setProvider(claude.provider);
const roster = claude.provider.getModels().map((model) => `${model.provider}:${model.id}`);
const session = new Session({ models, roster });
```

Pass `answer` to replace only model requests, or `operator` to supply operator answers. The host still names
the models that `answer` stands in for, and a session given `answer` and no model is refused, since every prompt
would go to the operator. With no `operator`, questions stand in `session.console.prompts`; call
`session.console.answer(id, text)` to parse and validate an answer. `session` emits `change` and `facts`, and
`fault` tells what the life refused when the provider or the console said what an act came to. `session.facts`
holds every fact of the life, every act among them, since the session drives the ear that keeps them as an ear of
the engine. A view reads the life and says nothing, so a listener that reads the life hears no change of its own. A
failed outside act carries its refusal in the record. The store writes and syncs one complete record entry for
each keep of the journal. The engine phrases every turn as python, so the provider renders nothing. A turn is its
role, its python, its usage and the blocks of its provider, as `engine.turns({ on })` gives it. The provider hands a
model the python of each user turn as it is, and no user turn that holds nothing, and the `answer` callback
receives the same turns.

An ear is a generator of JavaScript or an ear of the crate, a `NativeEar`, and `Engine.boot(record, ears)` takes
them under the names the engine hears them by, in the order the engine offers them a question. A generator hears
each fact of the life. It yields a saying, `[kind, id, ...words]`, and hears back the fact the bus made of it; it
yields a call of a verb, `{ verb, args, kwargs }`, and hears back what the verb gave, or has thrown in what it
raised; and it yields nothing to hear the next fact. It hears nothing at its birth. An ear may not call the engine
while it hears, since the engine waits for it. The first ear that says started or done of a question owns it, and
the owner answers it: now with a done, or later. What the work that an ear began says later, it says under the name
of that ear, which `speaking(engine, name, action)` sets. An ear of the outside hears a question only when no ear
before it took it, so `driving(ear, name)` gives an ear that brings another to life as an ear of the engine, which
hears every question. `SessionOptions.ears` gives the ears of the host, which come before the ears of the session:
one that takes a question takes it in their place, and one that takes it and asks the same again wraps it. A
function of JavaScript crosses as a show, a filter, or a function that makes the ear of an act, and what it throws
is raised where the word called it.

Values use the Python record form. Text and Exit carry `is` plus their fields. Faults carry `is` and `args`.
Lists and tuples cross as arrays, and maps keep their order. Every value of the engine crosses to JavaScript,
so an ear hears every fact. A value that JSON holds only in part crosses as its type under `is` and what that
type makes it from under `args`, and comes back in whole: an int past the safe range as
`{"is":"int","args":["1180591620717411303424"]}`, a float that is not finite as `{"is":"float","args":["inf"]}`,
and a map with a key that is no string as `{"is":"dict","args":[[[1,"a"]]]}`. Any other value, and a value
nested beyond 64 levels, crosses as its Python representation. A map that holds the key `is` crosses as its
pairs under the same mark, `{"is":"dict","args":[[["is","x"]]]}`, both ways, so no map of a word reads as a
mark; a host marks its own such map the same way, as `console.answer` does for an answer of the operator. Into the
engine, a whole JavaScript number is an int and a number with a fraction is a float, so a whole float goes in its
form, `{"is":"float","args":["2"]}`. A whole number past the safe range is refused, since JavaScript holds it
rounded: send a BigInt, or the `int` form above. A BigInt past 64 bits reaches the engine as its digits in a
string. `inspect(name, chain)` also gives the Python type and representation of a value.

Records preserve integral floats as `{"is":"float","args":["1"]}`. The native record reader checks integer
precision before JavaScript can round a number. Every act the host makes enters the record, a query among them, and
a later life makes it again at its place. A view enters nothing.

A record whose replay drifts gives a life all the same, and `engine.raised` holds the drift; that life keeps
nothing more. `Session.open` refuses such a record with the drift.

Only one process owns a record. `store(path)` gives the record at the path and the ear of the store, which holds a
lock on `<record>.lock` for as long as it lives; the system releases the lock when the process ends, so a lease of
a process that ended never blocks an open. The holder may move the lock file with the record, and a process that
locked the moved file opens the path again. On Windows the lock also refuses a read of the file by any other
handle; the file holds no text, and nothing reads it. A torn final line is removed before an append; a damaged
complete line fails. `kept(path)` reads a record with no lock. The dispose of an engine lets every ear go: the store
lets its record go, and a command ends. `NativeEar.dispose()` lets go an ear that no engine heard.

The journal keeps a command, a wait, a prompt to the operator and a reply from the started of the ear that took it.
What the record shows started and not done is pending in a later life: the engine starts none of it until a wake that this
life says. `session.pending` holds that work, and `session.resume()` says a wake of each chain that holds some. Work
that a pause of the operator holds is not in `session.pending`, and it waits for the wake of the operator. A
command that an earlier life started and did not end runs again at that wake, once; what it told before stands in
its door until then, and it is answered with the streams of the process it runs again. The deadline of a wait is a
`due` fact that the ear of time says, which the record keeps, so a later life ends the wait at the same time. The
streams that a model wrote in part live in the record's `.session.json` companion. File snapshots append to
`.changes.jsonl`; `session.changes.read` loads a page of them. Keep both companions with the JSONL record. The
session saves `.session.json` whole with `saveFile(path, text)`, which writes `<path>.tmp` and gives it the name
of the file. A save that fails throws an error that names the file, with the error of the system as its cause. The
TUI saves its own files with it.

`inspectRecord(path, models)` reads pending work through the same native replay without taking a record lock,
writing files, or starting a model or command. The TUI runs this inspection in its own worker. `Session.activity`
holds the state of every act a person follows, derived once from the facts as the life hears them, so a host reads
it without asking the sandbox; the acts the engine asks on the way, such as a read, a run or a reply, are no rows of
it. It asks the engine's `covers` which live acts a pause or a wake is over, one call for each act whose state
the control would change. `Session.isPaused` reads it.

`session.attachImage(path)` copies an image into the record's `.images` directory and returns its name, type,
size, and `furb-image://` reference. A session with no record copies it into `.furb/images` of its directory. The
`.furb` that it makes holds a `.gitignore` that keeps it out of version control, as `furbDirectory` makes it.
Put that reference in the prompt as a Markdown image,
`![design](furb-image://...)`, which `imageReference(image)` writes and `imageReferences(message)` reads. The
provider hands the python of the turn as it is, and adds each image that the message of a prompt of that turn
references as a pi-ai image block. The stored bytes are checked against their digest before use. PNG, JPEG, GIF,
and WebP are supported, with a 20 MiB limit per image. Keep `.images` with the record when moving a session.

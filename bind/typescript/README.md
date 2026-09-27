# furb for TypeScript

The same Rust crate and Monty sandbox, through N-API. Views, queries and controls are synchronous. An `Act` has
an `id`, converts to its name as a string, and can be awaited. Its Promise resolves with its outcome or rejects
with an error that names the engine's fault. A TUI is not part of this package.

`napi-rs` builds `furb.node` and generates `index.d.cts` from `src/binding/ts.rs`, and `index.cjs` loads it. The optional `typescript` feature
builds the binding in the existing crate. There is no Rust worker or second crate. The `Engine` of the package has
one method for each verb of the contract, which the build of the crate makes from `src/furb/engine.pyi`, as it
makes the methods of the crate: the words that the verb needs, in their order, then an object of the words that have
a default, where nothing leaves the default of the engine. A verb of the operator names its chain in `on`. The
package runs on Bun and Node.js 22 or later, on macOS, Linux and Windows. Bun gives no signal on Windows for
Ctrl+Break or for the close of the console, and ends the process at once: `onConsoleEnd(callback)` hears these
events there, and the system holds the process until the callback ends it.

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

A `Session` opens an engine on the ears of the World that the crate writes, which the package gives as `files()`,
`bash()`, `time()`, `store(path)` and `provider(options)`, and on the console of the operator, which this package
writes. The files read and write a path against the directory where its chain stands. The commands stream what a
command writes, feed its stdin, and end it at its timeout. Time gives the clock and a chance, and ends a wait. The
provider asks the models of the catalog of the crate, which `models()` lists and `model(name)` finds, and it
preserves provider response blocks in the record. The catalog offers the models of a provider when a credential of
it stands in the environment, such as `ANTHROPIC_API_KEY`, and the models of the claude command line when it finds
the program. The host names what the provider offers in `roster`, as `provider:id`, and the default actor in
`model` and `effort`: the first of the roster, and `low`, when unsaid. The effort moves to the nearest one the model
takes, among `levels()`. A session given neither offers the operator alone, and a prompt that names no actor goes
to the operator. A name without its provider names the one model of that id. A reopened session offers what its
host names now. It keeps the model and the effort that the host chose last, and takes that model when the host
names none and the catalog knows it. The journal keeps each stand and what the provider answered it, and every life
stands again as it opens, so a later life tells the new standing on each chain whose standing changed.

The claude command line is the provider `claude-cli` of the catalog, with the models `opus`, `sonnet`, `haiku` and
`fable`. It follows the pooled session design in [dirt](https://github.com/uael/dirt/tree/main/packages/cli/src/providers).
It keeps a warm conversation for each chain of each life, sends only new messages, preserves text and thinking
blocks, and reports the cost of each turn. It runs pure completions with CLI tools and MCP disabled. It finds the
program that `claude` names in the options, then the one `FURB_CLAUDE_BIN` names, then the standalone CLI on PATH
or the CLI installed by Claude Desktop. No process starts until a model is asked, and each one ends with the life.

```ts
import { models, Session } from "@furb/engine";

const roster = models().map((model) => model.name);
const session = new Session({ roster, model: "claude-cli:opus", effort: "high" });
```

Pass `answer` to replace only model requests, or `operator` to supply operator answers. `answer` gets the actor,
the chain, the messages and the settings of the effort as the provider would send them, and a `write` that tells
what it writes as it writes it, and gives a turn. The host still names the models that `answer` stands in for. With
no `operator`, questions stand in `session.console.prompts`; call `session.console.answer(id, text)` to parse and
validate an answer. `session` emits `change` and `facts`, and `fault` tells what the life refused when the console
said what an act came to. `session.streams` holds what a model writes for each rung, until the done of its reply.
`session.facts` holds every fact of the life, every act among them, since the session drives the ear that keeps
them as an ear of the engine. A view reads the life and says nothing, so a listener that reads the life hears no
change of its own. A failed outside act carries its refusal in the record. The store writes and syncs one complete
record entry for each keep of the journal. The engine phrases every turn as python, so the provider renders
nothing. A turn is its role, its python, its usage and the blocks of its provider, as `engine.turns({ on })` gives
it. The provider hands a model the python of each user turn as it is, and no user turn that holds nothing, and the
`answer` callback receives the same messages.

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
a later life makes it again at its place. A view enters nothing. `Session.open` refuses a record whose replay
drifts.

`session.pending` holds the work that the record showed begun and not done when the life opened, except the work
that a pause of the operator holds, and `session.resume()` wakes each chain that holds some. The streams that a
model wrote in part live in the record's `.session.json` companion. File snapshots append to `.changes.jsonl`;
`session.changes.read` loads a page of them. Keep both companions with the JSONL record. The session saves
`.session.json` whole with `saveFile(path, text)`, which writes `<path>.tmp` and gives it the name of the file. A
save that fails throws an error that names the file, with the error of the system as its cause. The TUI saves its
own files with it.

`inspectRecord(path)` reads pending work through the same native replay without taking a record lock,
writing files, or starting a model or command. `session.activity` holds the state of every act a person follows,
and `session.isPaused(id)` reads it.

`session.attachImage(path)` copies an image into the record's `.images` directory and returns its name, type,
size, and `furb-image://` reference. A session with no record copies it into `.furb/images` of its directory. The
`.furb` that it makes holds a `.gitignore` that keeps it out of version control, as `furbDirectory` makes it.
Put that reference in the prompt as a Markdown image,
`![design](furb-image://...)`, which `imageReference(image)` writes and `imageReferences(message)` reads. The
provider hands the python of the turn as it is, and adds each image that the message of a prompt of that turn
references, to a model that takes images. The stored bytes are checked against their digest before use. PNG, JPEG, GIF,
and WebP are supported, with a 20 MiB limit per image. Keep `.images` with the record when moving a session.

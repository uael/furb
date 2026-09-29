# furb for TypeScript

The same Rust crate and Monty sandbox, through N-API. Views, queries and controls are synchronous. An `Act` has
an `id`, converts to its name as a string, and can be awaited. Its Promise resolves with its outcome or rejects
with an error that names the engine's fault. A TUI is not part of this package.

`napi-rs` builds `furb.node` from the host API in `src/binding.rs`, which the door to python gives too, and from the
door in `src/binding/ts.rs` and its module `src/binding/ts/host.rs`, and generates `index.d.cts` from them. `index.cjs` loads `furb.node`. The optional
`typescript` feature builds the binding in the existing crate. There is no Rust worker or second crate. The
`Engine` of the package has one method for each verb of the contract, which the build of the crate makes from
`src/furb/engine.pyi`, as it makes the methods of the crate: the words that the verb needs, in their order, then an
object of the words that have a default, where nothing leaves the default of the engine. A verb of the operator
names its chain in `on`. The package runs on Bun 1.4.2 or later and on Node.js 22 or later, on macOS, Linux and
Windows. Bun gives no signal on Windows for Ctrl+Break or for the close of the console, and ends the process at
once: `onConsoleEnd(callback)` hears these events there, and the system holds the process until the callback ends
it.

```ts
import { boot } from "@furb/engine";

const session = boot({
  cwd: process.cwd(),
  record: ".furb/work.jsonl",
  // A roster of no model offers the operator alone, and this callback answers what is put to it.
  roster: [],
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

A `Session` opens an engine on the ears of the World that the crate writes and on the console of the operator, which
this package writes. `Engine.open(ears, opening)` opens it as every host of the crate opens a life: on the record, on
the ears of the host, and then on the ears of the crate, which are the provider of the models, the extensions, the ear
of each official extension, the files, the commands, time, and the store of the record. Its `record` getter gives the
record it opened on. The extensions enable at the start of the life what the configs of the user and of the directory
turn on, which `docs/extensions.md` says: `SessionOptions.extensions` is false to enable nothing new, and `config` names
the config directory of the user. A life runs what its record enables either way. The package also gives the ears of the
files, the commands, time and the store one by one, as `files()`, `bash()`, `time()` and `store(path)`, for a host that
boots an engine by hand. The files read and write a path against the directory where its chain stands. The commands
stream what a command writes, feed its stdin, and end it at its timeout. Time gives the clock and a chance, and ends a
wait. The provider asks the models of the catalog of the crate, which `models(claude?)` lists and `model(name)` finds,
and it preserves provider response blocks in the record. The catalog offers the models of a provider when a credential
of it stands in the environment, such as `ANTHROPIC_API_KEY`, and the models of the claude command line when it finds
the program. The host names what the provider offers beside the model of the default actor in `roster`, as
`provider:id`, and the default actor in `model` and `effort`: the first of the roster when unsaid. When the roster is
unsaid too, the session stands on the default of the crate, the first model that the catalog offers. A roster that names
no model, with no model, offers the operator alone, and a prompt that names no actor goes to the operator. The standing
of every chain tells the roster, so a session offers the models it names and not the whole catalog. An actor that names
no effort takes `high`, and an effort moves to the nearest one the model takes, among `levels()`. A name without its
provider names the one model of that id. A reopened session offers what its host names now. It keeps the model and the
effort that the host chose last, and takes that model when the host names none and the catalog knows it. The journal
keeps each stand and what the provider answered it, and every life stands again as it opens, so a later life tells the
new standing on each chain whose standing changed.

The claude command line is the provider `claude-cli` of the catalog, with the models `opus`, `sonnet`, `haiku` and
`fable`. It follows the pooled session design in [dirt](https://github.com/uael/dirt/tree/main/packages/cli/src/providers).
It keeps a warm conversation for each chain of each life, sends only new messages, preserves text and thinking
blocks, and reports the cost of each turn. It runs pure completions with CLI tools and MCP disabled. It finds the
program that `claude` names in the options, then the one `FURB_CLAUDE_BIN` names, then the standalone CLI on PATH
or the CLI installed by Claude Desktop. No process starts until a model is asked, and each one ends with the life.

```ts
import { models, Session } from "@furb/engine";

// The models the catalog offers, which a host names as its default actor and in its roster.
const [first, ...others] = models().map((model) => model.name);
const session = new Session({ model: first, effort: "high", roster: others.slice(0, 2) });
```

Pass `answer` to replace only model requests, or `operator` to supply operator answers. `answer` gets the actor,
the chain, the messages and the settings of the effort as the provider would send them, and a `write` that tells
what it writes as it writes it, and gives a turn. The host still names the models that `answer` stands in for. With
no `operator`, questions stand in `session.console.prompts`; call `session.console.answer(id, text)` to read an
answer by the rules that every console of the crate reads a line by, which `answered(shape, line)` gives, for each
of the `shapes()` that the operator answers. `session` emits `change` and `facts`, and `fault` tells what the life refused when the console
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
each fact of the life. It yields a saying, `[kind, id, ...words]`, and hears back the fact the bus made of it, and
it yields nothing to hear the next fact. It hears nothing at its birth. While it hears, it calls a verb of the
engine by yielding `call(verb, args, kwargs)`, and hears back what the verb gave, or has what it raised thrown where
it yielded, since the engine answers the ear that it waits for. The first ear that says started or done of a question owns it, and
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
mark; a host marks its own such map the same way, as `answered` does for an answer of the operator. Into the
engine, a whole JavaScript number is an int and a number with a fraction is a float, so a whole float goes in its
form, `{"is":"float","args":["2"]}`. A whole number past the safe range is refused, since JavaScript holds it
rounded: send a BigInt, or the `int` form above. A BigInt past 64 bits reaches the engine as its digits in a
string. `inspect(name, chain)` also gives the Python type and representation of a value.

Records preserve integral floats as `{"is":"float","args":["1"]}`. The native record reader checks integer
precision before JavaScript can round a number. Every act the host makes enters the record, a query among them, and
a later life makes it again at its place. A view enters nothing. `Session.open` refuses a record whose replay
drifts.

`session.pending` names the work that an earlier life left, which waits for a wake that this life says, by the kind of
each act, as `engine.pending()` finds it: among each act that the outside started and did not end, and the acts that
made it, each prompt, rung, command and wait that is not done and that no pause holds. `session.resume()` wakes each
chain that holds some, and an act that a wake puts to the outside again is pending no more. The streams that a
model wrote in part live in the record's `.session.json` companion. File snapshots append to `.changes.jsonl`;
`session.changes.read` loads a page of them. Keep both companions with the JSONL record. The session saves
`.session.json` whole with `saveFile(path, text)`, which writes `<path>.tmp` and gives it the name of the file. A
save that fails throws an error that names the file, with the error of the system as its cause. The TUI saves its
own files with it.

`inspectRecord(path)` reads pending work through the same native replay without taking a record lock,
writing files, enabling an extension, or starting a model or command. `session.activity` holds the state of every act
a person follows, and `session.isPaused(id)` reads it. `session.interrupt(chain)` cancels the work of a chain, each
prompt, rung, command and wait on it that is not done, and not what an extension started on it.

`session.attachImage(path)` copies an image into the record's `.images` directory and returns its name, type,
size, and `furb-image://` reference. A session with no record copies it into `.furb/images` of its directory. The
`.furb` that it makes holds a `.gitignore` that keeps it out of version control but its `config.json`, as
`furbDirectory` makes it.
Put that reference in the prompt as a Markdown image,
`![design](furb-image://...)`, which `imageReference(name, uri)` writes and `imageReferences(message)` reads. The
provider hands the python of the turn as it is, and adds each image that the message of a prompt of that turn
references, to a model that takes images. The stored bytes are checked against their digest before use. PNG, JPEG, GIF,
and WebP are supported, with a 20 MiB limit per image. Keep `.images` with the record when moving a session.

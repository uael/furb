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
files, the commands, time and the store one by one, as `files(scratch)`, `bash(scratch, record)`, `time()` and
`store(path)`, for a host that boots an engine by hand. The files read and write a path against the directory where its chain stands,
and a path of `scratch://` in the scratch directory that the host names. The commands stream what a command writes,
feed its stdin, and end it at its timeout, and each runs with `TMPDIR` set to that scratch directory. They are given
the record that the life opens on, and at a wake they close each command that the record shows started and not done
with a refusal that says it was interrupted, and run it no more. Only the opening
of a life makes the files of a record again in its scratch directory, and gives a life with no record a directory that
goes with it. A session has the scratch directory of its record, in the cache of furb, which `scratch(record)` gives,
so a host removes it when it deletes the record. Time gives the clock and a chance, and ends a wait. The provider asks
the models of the catalog of the crate, which `models(claude?)` lists and `model(name)` finds, and it preserves
provider response blocks in the record. The catalog offers the models of a provider when a credential
of it stands in the environment, such as `ANTHROPIC_API_KEY`, the models of the claude command line when it finds
the program, and the models of the local server that `FURB_LOCAL_URL` names, each named `local:` and its id. The host
names what the provider offers beside the model of the default actor in `roster`, as `provider:id`, and the default
actor in `model` and `effort`: the first of the roster when unsaid. When the roster is unsaid too, the session stands on
the default of the crate: a model of the claude command line, then of a provider of the network, then of the local
server. A session whose roster is
unsaid offers every model that the catalog offers, after the model of the default actor. A roster that names models
offers those and no more, since the standing of every chain tells the roster, and a roster that names no model, with no
model, offers the operator alone, and a prompt that names no actor goes to the operator. An actor that names
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
the chain, the messages and the settings of the request as the provider would send them, its effort, its conversation
and the most tokens of thought of a claude model among them, and a `write` that tells
what it writes as it writes it, and gives a turn. The host still names the models that `answer` stands in for. With
no `operator`, questions stand in `session.console.prompts`; call `session.console.answer(id, text)` to read an
answer by the rules that every console of the crate reads a line by, which `answered(shape, line)` gives, for each
shape that `answers(shape)` holds for: the `shapes()` that the operator offers, and a `Literal` of values that a line
names, as `Literal['a', 'b']`. A prompt of `None` asks nothing: the console closes it with `None` as
it takes it, so no word waits for the operator, and keeps it in `session.console.notices` until
`session.console.dismiss(id)`, or gives it to `operator` and waits for no answer. `session` emits `change` and `facts`, and `fault` tells what the life refused when the console
said what an act came to. `session.streams` holds what a model writes for each rung, until the done of its reply.
When the call of a model fails for a cause that passes, the provider asks the model again after a wait, and the
stream of its rung holds the `retry`: the attempt that comes next, the most attempts, when the wait ends, and why,
with none of what the model wrote before the failure. The option `retrying` of `Engine.open` is told each retry.
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
is raised where the word called it. It answers from its words alone: a verb of the engine that it calls while the
engine runs it is refused, since only an ear, which yields, can wait for the answer of the engine.

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
chain that holds some, and an act that a wake puts to the outside again is pending no more. The engine reads the
pending work off the transcripts, so the session asks it again only after a pause, a wake, or a start or a done of
an act of that work, and keeps what it gave until then.

`session.parked` names the work that a crash loop parked, by the kind of each act, as `engine.parked()` finds it: a
run or a reply that the record shows started in five lives and not done, since each of those lives ended
while it ran. A wake puts no parked act to the outside, so `session.resume()` does not run it again and end again,
and a parked act is pending no more. `session.retry(id)` runs a parked act again: it is parked no more in this life,
and a wake of it puts it to the outside.

`engine.keyed(key, kind, args, kwargs)` makes the act of a verb once for a key of the client, across a crash of the
host and a retry of the client, as `engine.keyed("k1", "prompt", ["int"], { message: "count", on })`. A key fact goes
to the record before the act, and a later call with the same key on the same chain gives the act that the key made,
in this life or in a later life on the record. When the host crashed after it kept the key and before it kept the
act, the call makes the act. The record's
`.session.json` companion holds the directory, the model and the effort that the host chose last, and no stream,
since a stream lives only while its reply runs. File snapshots append to `.changes.jsonl`;
`session.changes.read` loads a page of them, and throws a `DamagedJournal` when the page holds a line that is no
change. `session.changes.repair()` writes the journal again with each change that it can read, and
`session.changes.renew()` starts a new journal and keeps the damaged one beside it. Keep both companions with the JSONL
record.

The ear of the log writes the log of the life beside its record, `<record>.log`, in each life that keeps its record,
and `log(record)` gives it to a host that boots an engine by hand: every fact that the life said, in the order
in which every ear heard it, with the rungs of each ladder, each run and each tell of a chain, which the record leaves
out. The fact of a module stands with no words. It is a cache: each life on the record writes it again from its first
line, and a host can delete it, since the next life writes it again. `logged(record)` gives its facts, as
`session.facts` holds them, while the log agrees with the record, and `null` otherwise, so a host shows a life whole
with no life, and opens an inspection when the log does not agree. An `Activity` reads them as it reads a live life,
but for the question `covers`, which a reader with no life answers from the acts of the log, as the engine does: a
pause, a wake or a cancel covers the act that it names, each act that this act made at any depth, and each act on the
chain that it names; a close covers the act that it names, each rung that this act made, and each reply that one of these made.
`retold(record)` reads a record with no life, so a record of any engine opens: it gives the engine that made the
record, as the record names it, and `null` when it names none, the engine of this build, whether its facts are those
of the log, and the facts. The facts are those of the log while the log agrees with the record, and else those that
the record kept, with the facts that the engine said and the record does not keep, as far as the kept facts show
them: each chain and each rung that a kept act names, the run of the word of each rung that the gate let run, the
findings of a word that the gate refused, and the done of each act that the operator closed or cancelled.

A life that keeps its record keeps a snapshot of its state beside it, `<record>.snapshot`: the session of the sandbox
and what the engine holds of its ears, with the key of the build and of the ears, and digests of the record and of
the log that it covers. A later life on the record opens from it with no replay when it agrees with all of them,
tells each ear the past of the life from the log, and stands again at its tip; else it replays the record, and keeps
a new snapshot. A life writes it when it opens, at most once each thirty seconds while it does work, and when it
ends, but not while work of the outside runs, as a command or a reply. `snapshot: false` in the opening turns it off.
A `Session` keeps none yet: its observer is an ear that a verb was given, which a later process cannot give again. The session saves
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

The model of a life is the views that a host derives from the acts of a life, in `src/life`, which `@furb/engine/life`
gives. It holds no native code and no module of node, so a terminal and a window of the web read the same views:

- `acts`: the act as a view reads it, `LiveAct`, the kinds of work, `WORK`, what a command came to, `Exit`, the
  measure of a chain, `Measure`, which `measureOf` gives from the answers of its models, and whether an act failed or
  was cancelled.
- `asks`: the desk of the operator, `desk`: each question that waits for him and each notice that he did not dismiss,
  the oldest first, each with its chain and the choices of its shape, `choicesOf`: yes and no for a bool, and each
  value of a Literal. `aimed` gives the ask that the operator acts on, on the chain that he reads, `beside` the ask a
  step after or before it, and `wants` what an ask wants of him, in words.
- `loom`: the rows of the loom, one for the operator and one for each chain in the order of the tree, with the cells
  of their moments, what each chain does now, `nowOf`, in words that a person reads, a count of seconds as a person
  reads it, `duration`, and the acts as they stood at a moment, `asOf`.
- `ladder`: the lines of the thread of a chain, each prompt with the rungs of its ladder and their acts, and a long
  thread as a person reads it at a glance, `condense`: the last steps at each level, and one line in place of each
  run of earlier ones.
- `threads`: the tree of the threads of a life, where a thread is a prompt and a prompt that a rung of a thread makes
  is its sub-thread, each with its anchor, its messages, its progress, what waits for the operator and its pull
  requests.
- `map`: the place of each chain on the map, which no later chain moves, the messages that flow between chains, and
  the way of a message along the tree of hires.
- `source`: the words of the ladders of a chain as one program, with each act on the line that made it and the line
  that runs, and what each act came to in a few words.
- `plain`: a message as a person reads it, with no name of its exception around it and no path of the directory of
  the life, and what a failed act says in such words.
- `time`: the marks of a life on its axis of time: each step, each question to the operator, and each end of a thread.
- `watch`: the last value of each name that a chain told by a moment, under the act whose name it starts with.
- `debugger`: the call stack of a chain, and the program of the ladder of a prompt, which its door holds.

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
TypeScript World to JavaScript. The package runs on Bun and Node.js 22 or later. The supplied shell World
uses `/bin/sh`, so it supports macOS and Linux. Build the native binary for the host before using the package.

```ts
import { boot } from "@furb/engine";

const session = boot({ cwd: process.cwd(), record: ".furb/work.jsonl" });
try {
  const { life } = session;
  console.log(life.cwd()); // A synchronous query.
  const work = life.prompt<string>("str", "Read the README and explain this project.");
  console.log(work.id);
  const answer = await work;
  console.log(answer);
} finally {
  await session.dispose();
}
```

The default `World` provides files, streamed shell commands, stdin, timeouts, time, chance, records, model
requests, and operator questions. It knows no provider of its own: it asks the pi-ai collection it is given in
`models`, the built-in providers when it is given none, and it preserves provider response blocks in the
record. The host names what the World offers in `roster`, as `provider:model`, and the default actor in `model`,
which is the first of the roster when unsaid; a World given neither offers the operator alone. A name without
its provider routes to the one model of that id. A reopened record keeps its roster and model, and a model the
host offers since then joins the roster. Configure an API provider through its pi-ai credentials.

The Claude CLI provider, `claudeProvider` from `@furb/engine/claude`, is a pi-ai provider that a host adds to
its collection at run time. It follows the pooled session design in [dirt](https://github.com/uael/dirt/tree/main/packages/cli/src/providers).
It reads current pi-ai system messages, keeps warm conversations by chain, sends only new messages, preserves
text and thinking blocks, and reports the cost of each turn. It runs pure completions with CLI tools and MCP
disabled. It finds the standalone CLI or the CLI installed by Claude Desktop. Set `FURB_CLAUDE_BIN` to select
a binary; `DIRT_CLI_BIN` is also accepted. Its pool belongs to the host that made the provider, and its
`dispose` stops it. No CLI process starts until a model is asked.

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

Pass `answer` to replace only model requests, or `operator` to supply operator answers. With no `operator`, questions stand in `world.prompts`; call `world.answer(id, text)`
to parse and validate an answer. `world` emits `change` and `facts`; `fault` reports a failure to deliver an
outside result. A failed outside act carries its refusal in the record. A `Keep` writes and syncs
one complete record entry before it returns. The `answer` callback also receives the exact rendered turns
as its fifth argument. `life.rendered(chain)` gives that text from the native values, so Python floats,
tuples, and instances retain their representations before they cross to JavaScript. `life.rendering(chain)`
gives the turns and that text from one question.

Pass `world` to `boot` to replace the whole World. It receives these operations:

| Operation | Arguments | Answer |
| --- | --- | --- |
| Stand | none | `[roster, directory, actor]` |
| Read | directory, path | `{path, content}` |
| Write | directory, path, content | `{path, content}` |
| Clock, Chance | none | number |
| Keep | entry | nothing |
| Ask | rung, chain, actor, turns, rendered text | Promise of a turn |
| Run | `{id, here, command, fed, timeout, merged}` | nothing; send out/exited later |
| Feed | command id, text or null | nothing |
| Slay | command id | nothing |
| Wait | seconds, act id | Promise that resolves when time passes |
| Prompt | id, shape, message | Promise of an operator answer |

Stand, Read, Write, Clock, Chance, Keep, Run, Feed, and Slay answer synchronously. They must not call back
into the same life. For a World that needs nested engine queries, use `Ears`: its generators yield a saying
`[kind, id, ...words]`, a call `{verb, args, kwargs}`, or nothing. A yielded call is answered before the ear
continues, as in the Python binding. `Ears.callable` carries a JavaScript show or filter into the engine.
`Life.call` reaches every public engine verb beyond the named methods.

Values use the Python record form. Text and Exit carry `is` plus their fields. Faults carry `is` and `args`.
Lists and tuples cross as arrays, and maps keep their order. Every value of the engine crosses to JavaScript,
so an ear hears every fact. A value that JSON holds only in part crosses as its type under `is` and what that
type makes it from under `args`, and comes back in whole: an int past the safe range as
`{"is":"int","args":["1180591620717411303424"]}`, a float that is not finite as `{"is":"float","args":["inf"]}`,
and a map with a key that is no string as `{"is":"dict","args":[[[1,"a"]]]}`. Any other value, and a value
nested beyond 64 levels, crosses as its Python representation. Into the engine, a whole JavaScript number is
an int and a number with a fraction is a float. A whole number past the safe range is refused, since
JavaScript holds it rounded: send a BigInt, or the `int` form above. A BigInt past 64 bits reaches the engine
as its digits in a string. `inspect(name, chain)` also gives the Python type and representation of a value.

Records preserve integral floats as `{"is":"float","args":["1"]}`. The native record reader checks integer
precision before JavaScript can round a number. Queries are not journal entries. To keep a program edit
across a later open, perform its `write` in a `rung`.

A record whose replay drifts gives a life all the same, and `life.raised` holds the drift; that life keeps
nothing more. `World.open` refuses such a record with the drift.

Only one process owns a record. Its `RecordLock` holds a lock on `<record>.lock`, which the system releases
when the process ends, so a lease of a process that ended never blocks an open. The lock file stays beside the
record. A torn final line is removed before an append; a damaged complete line fails.
Closing a durable World pauses its chains. A later open holds unfinished work before any model or command
runs, and `world.resume()` explicitly releases it. Interrupted commands keep their recorded output and end
with a refusal on resume; they are never run twice without a new act. Wait deadlines and partial streams
live in the record's `.world.json` companion. File snapshots append to `.changes.jsonl`; `world.changes.read`
loads a page of them. Keep both companions with the JSONL record.

`inspectRecord(path, models)` reads pending work through the same native replay without taking a record lock,
writing files, or starting a model or command. The TUI runs this inspection in its own worker. `World.activity` holds
the state of every act, derived once from the facts as the life hears them, so a host reads it without asking the
sandbox. `World.isPaused` and `World.rungState` read it.

`world.attachImage(path)` copies an image into the record's `.images` directory and returns its name, type,
size, and `furb-image://` reference. Put that reference in the prompt, for example
`![design](furb-image://...)`. The World keeps the exact native text and adds the referenced image as a pi-ai
image block. The stored bytes are checked against their digest before use. PNG, JPEG, GIF, and WebP are
supported, with a 20 MiB limit per image. Keep `.images` with the record when moving a session.

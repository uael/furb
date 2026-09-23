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
requests, and operator questions. It uses pi-ai's model collection and preserves provider response blocks in
the record. Its default actor is `claude-cli:sonnet/low`. Other models use `provider:model/effort` in the
roster, for example `anthropic:claude-sonnet-4-6/low`. Configure an API provider through its pi-ai credentials.
Add models at boot with `roster`; the engine owns the standing of each chain.

The Claude CLI provider follows the pooled session design in [dirt](https://github.com/uael/dirt/tree/main/packages/cli/src/providers).
It reads current pi-ai system messages, keeps warm conversations by chain, sends only new messages, preserves
text and thinking blocks, and reports the cost of each turn. It runs pure completions with CLI tools and MCP
disabled. It finds the standalone CLI or the CLI installed by Claude Desktop. Set `FURB_CLAUDE_BIN` to select
a binary; `DIRT_CLI_BIN` is also accepted. Its pool is owned by one
World and stops when that World closes. No CLI process starts until a model is asked.

Pass `answer` to replace only model requests, `operator` to supply operator answers, or `models` to use your
own pi-ai collection. With no `operator`, questions stand in `world.prompts`; call `world.answer(id, text)`
to parse and validate an answer. `world` emits `change` and `facts`; `fault` reports a failure to deliver an
outside result. A failed outside act carries its refusal in the record. A `Keep` writes and syncs
one complete record entry before it returns. The `answer` callback also receives the exact rendered turns
as its fifth argument. `life.rendered(chain)` gives that text from the native values, so Python floats,
tuples, and instances retain their representations before they cross to JavaScript.

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
Lists and tuples cross as arrays; maps keep their order and require string keys. Unsupported values, unsafe
integers, and nesting beyond 64 levels are refused. `inspect(name, chain)` also gives the Python type and
representation of a value that cannot cross as plain data.

Records preserve integral floats as `{"is":"float","args":["1"]}`. The native record reader checks integer
precision before JavaScript can round a number. Queries are not journal entries. To keep a program edit
across a later open, perform its `write` in a `rung`.

Only one process owns a record. A torn final line is removed before an append; a damaged complete line fails.
Closing a durable World pauses its chains. A later open holds unfinished work before any model or command
runs, and `world.resume()` explicitly releases it. Interrupted commands keep their recorded output and end
with a refusal on resume; they are never run twice without a new act. Wait deadlines and partial streams
live in the record's `.world.json` companion. File snapshots append to `.changes.jsonl`; `world.changes.read`
loads a page of them. Keep both companions with the JSONL record.

`inspectRecord(path)` reads pending work through the same native replay without taking a record lock,
writing files, or starting a model or command. The TUI runs this inspection in its own worker. `World.activity` holds
the state of every act, derived once from the facts as the life hears them, so a host reads it without asking the
sandbox. `World.isPaused` and `World.rungState` read it.

`world.attachImage(path)` copies an image into the record's `.images` directory and returns its name, type,
size, and `furb-image://` reference. Put that reference in the prompt, for example
`![design](furb-image://...)`. The World keeps the exact native text and adds the referenced image as a pi-ai
image block. The stored bytes are checked against their digest before use. PNG, JPEG, GIF, and WebP are
supported, with a 20 MiB limit per image. Keep `.images` with the record when moving a session.

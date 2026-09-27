# The JSON-RPC of furb

`furb --mode rpc` serves one life on stdin and stdout, with no TUI. A client, in any language, sends commands and
reads responses and events. The shape follows the RPC mode of Pi, `pi --mode rpc`: one JSON object on each line, in
each direction.

```sh
furb --mode rpc --record session.jsonl --cwd path/to/project
```

- `--record` is the record to keep, and to resume from. With no record, the life keeps nothing.
- `--cwd` is the directory the chains of the life start in. When you do not give it, it is the current directory.

The life runs on the engine of the crate and on the ears of the World that the crate writes: the files, the
commands, time, the store of the record, the provider of models, and the extensions, which the life enables at its
start as the configs of the user and of the directory say, and whose words it plays as rungs on each chain. The models are every model that the catalog of
the crate offers, each named `provider:id`: those of the claude command line, `claude-cli:opus`, `claude-cli:sonnet`,
`claude-cli:haiku` and `claude-cli:fable`, when furb finds the program, and those of each provider whose credential,
such as `ANTHROPIC_API_KEY`, stands in the environment. The roster also holds the operator, which is the client. A
prompt that names no actor goes to the first model at its least effort, `claude-cli:opus/low` when furb finds the
claude command line. An actor may name its model by the id alone, when one model alone holds it, and furb names it
as the roster does. `FURB_CLAUDE_BIN` names the claude command line, and the `claude` on PATH is used when it is not
set.

## Framing

- Each record is one JSON object and a line feed. furb splits its input at a line feed alone, and removes a carriage
  return before it. It ignores a blank line.
- Stdout holds the records of the protocol and nothing else. furb writes what it has to say to a person on stderr.
- Close stdin to end the life. furb ends the commands and the models of the life, lets go of the record, and exits
  with 0. The record keeps the work that is not done, and a later life resumes it at a wake.

## Values

A value crosses as JSON, in the form the record keeps:

- None, a bool, a number, a string and a list are the JSON value. A dict whose keys are strings is a JSON object.
- A number with a point is a float, and a whole number is an int. furb reads each number in a command exactly, before
  anything rounds it.
- An object with the key `is` names a class, and holds what furb makes the value again from:
  - `{"is": "Text", "path": "a.txt", "content": "one\n"}` is a text.
  - `{"is": "Refused", "args": ["why"]}` is an exception, and each exception has this form.
  - `{"is": "int", "args": ["9007199254740993"]}` is an int past the range that JavaScript holds exactly.
  - `{"is": "float", "args": ["inf"]}` is a float that JSON cannot hold.
- A value that JSON cannot hold otherwise, such as a function, crosses as the text that python shows of it.

## Commands

Each command is an object whose `type` names it. An `id` is optional, and the response gives it back as it came.
`on` names a chain, as `chain2`, and the root, `chain1`, is used when a command does not give it.

| `type` | Fields | `data` of the response |
| --- | --- | --- |
| `prompt` | `message`, `shape`, `to`, `on` | `{"act": "prompt1"}` |
| `rung` | `word`, `on` | `{"act": "rung1"}` |
| `close` | `act`, `value` | none |
| `cancel` | `act` | none |
| `pause` | `act` | none |
| `wake` | `act` | none |
| `turns` | `on` | `{"turns": [...]}` |
| `transcript` | `on` | `{"facts": [...]}` |
| `peek` | `act` | `{"done": false}`, or `{"done": true, "value": ...}`, or `{"done": true, "raised": ...}` |
| `state` | none | `{"root", "record", "standing", "extensions", "paused", "prompts", "acts"}` |

- `prompt` prompts an actor on a chain. `message` is what the actor reads, and the actor reads the transcript alone
  when there is no message. `shape` is the name of the type of the response, such as `int`, `str` or `list[str]`,
  and a prompt with no shape, or with `None`, wants nothing. `to` is the actor, as `opus/high` or `operator`, and a
  prompt with no actor goes to the default actor of its chain.
- `rung` runs `word`, which is python, as a rung on a chain, in the globals of that chain. The act is done with what
  the word gave to `close`, and with nothing when the word runs to its end.
- `close` ends an act with `value`, which is none when the command does not give it. It is how the client answers a
  prompt to the operator. An exception, such as `{"is": "Refused", "args": ["no"]}`, ends the act with that
  exception. The operator answers a prompt of the shape `float` with any number, so a whole number closes it as a
  float. A value of another shape is refused, and the prompt stays open.
- `cancel` ends an act and everything that it made, each with `CancelledError`.
- `pause` holds an act, or every act on a chain, until a `wake` of the same act or chain.
- `turns` gives the turns of a chain, as the model reads them: each is its role, its python, its usage and its blocks.
- `transcript` gives the facts on a chain, in the order of the log.
- `peek` gives what an act came to, when it is done.
- `state` gives the root, the path of the record, the standing, which is the roster, the directory and the default
  actor, the names of the extensions that the life runs, whether a pause stands over the root, each prompt to the
  operator that waits for a close, and each act that a `prompt` or a `rung` of the client made that is not done.

## Responses

Each command gets one response. `success` says whether the command did what it says. It does not say that an act is
done: the `done` event says that.

```json
{"id": "1", "type": "response", "command": "prompt", "success": true, "data": {"act": "prompt1"}}
{"id": "2", "type": "response", "command": "close", "success": false, "error": "The command needs act."}
```

A line that is not a JSON object gets a response with no `id`:

```json
{"type": "response", "command": "parse", "success": false, "error": "Failed to parse command: ..."}
```

## Events

The life says its events as they happen, after the response of the command that caused them.

- `fact`: each fact that the life says after it opens, in the order of the log, the acts among them. A fact is a list:
  its kind, the act it is about, who said it, and its words, as `src/furb/engine.pyi` says each kind. A `keep` is the
  entry that the journal gives the record.

  ```json
  {"type": "fact", "fact": ["prompt", "prompt1", "operator", "chain1", "int", "count the lines", ""]}
  ```

- `prompt`: each prompt to the operator, which the client answers with `close`. It comes from a `prompt` of the
  client to `operator`, or from a word of a model that asks the operator.

  ```json
  {"type": "prompt", "act": "prompt2", "on": "chain1", "shape": "int", "message": "how many?"}
  ```

- `done`: the end of each act that a `prompt` or a `rung` of the client made, once, with its value or the exception
  it raised.

  ```json
  {"type": "done", "act": "prompt1", "value": 3}
  {"type": "done", "act": "prompt4", "raised": {"is": "CancelledError", "args": []}}
  ```

## A resumed life

When furb opens on a record, the life says the record again before it serves the first command, and it streams no
event for what it says again. `transcript` and `turns` read that part. furb wakes nothing when it opens: when the
record holds a pause of the root, or work that an earlier life started and did not end, that work waits for a `wake`
of the client. `state` says whether a pause stands over the root.

## A session

The lines that start with `>` are the lines of the client, and the others are the lines of furb. Most `fact` events
are not shown.

```text
> {"id": "1", "type": "prompt", "message": "ask me", "shape": "int"}
{"id": "1", "type": "response", "command": "prompt", "success": true, "data": {"act": "prompt1"}}
{"type": "fact", "fact": ["prompt", "prompt1", "operator", "chain1", "int", "ask me", ""]}
{"type": "prompt", "act": "prompt2", "on": "chain1", "shape": "int", "message": "how many?"}
> {"id": "2", "type": "close", "act": "prompt2", "value": 7}
{"id": "2", "type": "response", "command": "close", "success": true}
{"type": "done", "act": "prompt1", "value": 7}
```

## How it differs from Pi

- A `prompt` of Pi starts a run of the agent, and `agent_settled` says that the run ended. A `prompt` of furb makes an
  act, and its `done` event says what it came to.
- The `abort` of Pi is `cancel`. A question of Pi to the user is an `extension_ui_request`, and its answer is an
  `extension_ui_response`. In furb, the question is a `prompt` event, and the answer is `close`.
- Pi streams messages and the parts of each message. furb streams facts, which are what the transcript holds.

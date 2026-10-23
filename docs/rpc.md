# The JSON-RPC of furb

`furb --mode rpc` serves one life on stdin and stdout, with no TUI. Use it to drive furb from a program in any
language: the program sends commands, and reads responses and events. The shape follows the RPC mode of Pi, `pi --mode rpc`: one JSON object on each line, in
each direction.

```sh
furb --mode rpc --record session.jsonl --cwd path/to/project --model claude-cli:sonnet/high --roster claude-cli:haiku
```

- `--record` is the record to keep, and to resume from. With no record, the life keeps nothing.
- `--cwd` is the directory the chains of the life start in. When you do not give it, it is the current directory.
- `--demo` and the words after `--` are for the TUI. With `--mode rpc`, furb refuses them and exits with an error.

The life runs on the engine of the crate and on the World of the crate: the files, the commands, time, the store of
the record, the models, and the extensions that the configs turn on, as [Extensions](extensions.md) says. The roster
of the life holds the model of the default actor, the models that `--roster` names, or each model that furb offers
when `--roster` names none, and the operator, which is the client. `--model` names the default actor, and a prompt
that names no actor goes to it. [Models](models.md) says how furb finds a model and how to name one. The life refuses
a prompt to a model that its roster does not hold.

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
| `prompt` | `message`, `shape`, `to`, `on`, `key` | `{"act": "prompt1"}` |
| `rung` | `word`, `on`, `key` | `{"act": "rung1"}` |
| `close` | `act`, `value` | none |
| `cancel` | `act` | none |
| `pause` | `act` | none |
| `wake` | `act` | none |
| `retry` | `act` | none |
| `turns` | `on` | `{"turns": [...]}` |
| `transcript` | `on` | `{"facts": [...]}` |
| `peek` | `act` | `{"done": false}`, or `{"done": true, "value": ...}`, or `{"done": true, "raised": ...}` |
| `state` | none | `{"root", "record", "standing", "extensions", "paused", "prompts", "acts", "pending", "parked"}` |

- `prompt` prompts an actor on a chain. `message` is what the actor reads, and the actor reads the transcript alone when
  there is no message. `shape` is the name of the type of the response, such as `int`, `str` or `list[str]`. A prompt
  with no shape wants a `str`, so the model works until it closes the prompt with its report, and a prompt with the
  shape `None` wants nothing. `to` is the actor, as `opus/high` or `operator`, and a prompt with no actor goes to the
  default actor of its chain.
- `rung` runs `word`, which is python, as a rung on a chain, in the globals of that chain. The act is done with what
  the word gave to `close`, and with nothing when the word runs to its end.
- `key` makes a `prompt` or a `rung` once, across a crash of furb and a retry of the client. It is any
  string that the client chooses. A command with a key that a command of the same type
  on the same chain had before makes no act: its response names the act that the key made, in this life or in a later
  life on the record, and the client gets the `done` event of that act again. furb keeps the key in the record before
  the act, as the fact `["key", "chain1", "operator", "k1", "prompt"]`, so a crash between the two leaves a key with
  no act, and the retry makes the act.
- `close` ends an act with `value`, which is none when the command does not give it. It is how the client answers a
  prompt to the operator. An exception, such as `{"is": "Refused", "args": ["no"]}`, ends the act with that
  exception. The operator answers a prompt of the shape `float` with any number, so a whole number closes it as a
  float. A value of another shape is refused, and the prompt stays open.
- `cancel` ends an act and everything that it made, each with `CancelledError`.
- `pause` holds an act, or every act on a chain, until a `wake` of the same act or chain.
- A `close`, a `cancel`, a `pause` or a `wake` of an id that names no act fails, and its error names the id, as
  `Refused: no act 'bash9'`.
- `retry` runs again an act that a crash loop parked, which the part on a resumed life says.
- `turns` gives the turns of a chain, as the model reads them: each is its role, its python, its usage and its blocks.
- `transcript` gives the facts on a chain, in the order of the log.
- `peek` gives what an act came to, when it is done.
- `state` gives the root, the path of the record, the standing, which is the roster, the directory and the default
  actor, the names of the extensions that the life runs, whether a pause stands over the root, each prompt to the
  operator that waits for a close, each act that a `prompt` or a `rung` of the client made that is not done, and the
  work that an earlier life left pending or that a crash loop parked, which the part on a resumed life says.

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
  client to `operator`, or from a word of a model that asks the operator. The operator answers the shapes `str`,
  `None`, `bool`, `int`, `float`, `list` and `dict`, and a `Literal` whose values are strings, whole numbers, truths
  or `None`, as `Literal['a', 'b']`, as at the terminal. A prompt to the operator of another shape
  is not sent: furb closes it at once with a refusal, such as `the operator answers no set`. A prompt of the shape
  `None` asks nothing: furb closes it with `None` as it sends it, so the client shows it as a notice and answers it
  not.

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
of the client. `state` says whether a pause stands over the root, and `pending` names the work of an earlier life that
waits, each act as its name and its kind, such as `["bash1", "bash"]`: among each act that the World started and did
not end, and the acts that made it, each prompt, rung, command and wait that is not done and that no pause holds. An
act is pending no more once a `wake` puts it to the World again. A `wake` runs no command again: the World closes a
command that an earlier life did not end with a refusal that says it was interrupted, and the model runs it again if it
wants.

A run of a word or a reply that the record shows started in five lives and not done is parked, since each
of those lives ended while it ran, so it may be what ended them. `parked` names each such act as `pending` names an
act, and a parked act is pending no more. A `wake` puts no parked act to the World, so a client that wakes the work of
a life as it opens does not run it again and end furb again. `retry` runs a parked act again: it is parked no more in
this life, and a `wake` of it puts it to the World. A `retry` of an act that is not parked fails.

## A session

The lines that start with `>` are the lines of the client, and the others are the lines of furb. The `fact` events
are not shown. This word asks the operator a question, and gives the answer as its own: it asks no model, so you can
run it as it stands.

```text
> {"id": "1", "type": "rung", "word": "close(await prompt(int, \"how many?\", to=OPERATOR))"}
{"type":"response","command":"rung","success":true,"id":"1","data":{"act":"rung7"}}
{"act":"prompt1","on":"chain1","shape":"int","message":"how many?","type":"prompt"}
> {"id": "2", "type": "close", "act": "prompt1", "value": 7}
{"type":"response","command":"close","success":true,"id":"2"}
{"type":"done","act":"rung7","value":7}
```

The act is `rung7`, since the extensions ran the rungs before it.

## How it differs from Pi

- A `prompt` of Pi starts a run of the agent, and `agent_settled` says that the run ended. A `prompt` of furb makes an
  act, and its `done` event says what it came to.
- The `abort` of Pi is `cancel`. A question of Pi to the user is an `extension_ui_request`, and its answer is an
  `extension_ui_response`. In furb, the question is a `prompt` event, and the answer is `close`.
- Pi streams messages and the parts of each message. furb streams facts, which are what the transcript holds.

# The protocol of the host

`furb-host` is the daemon of a machine: it holds each life, and each window or skill reaches it over one protocol. This
page says that protocol. Its version is `PROTOCOL`, now 1. The crate gives its JSON Schema in
`host/protocol.schema.json`, and `bind/typescript/src/protocol.ts` holds the types that `bun run protocol` makes from
that schema. A test of each side fails when the schema or the types do not agree with the code.

## Transports

A transport has two parts: a dialer, which reaches the daemon, and an identity, which says who a peer is.

- On macOS and Linux, the daemon listens on the socket `host.sock` in a folder of mode 0700 that the user owns. The
  daemon refuses a folder that another user owns or can open, and a peer of another user.
- On Windows, the daemon listens on a named pipe whose access list lets in the user alone, and which takes no peer of
  another machine.
- The identity of a local peer is the login of the user, and it has the role to write.
- A second daemon cannot listen where a daemon listens. A socket that no daemon answers is stale, and gives its place.

## Framing

- Each frame is one JSON object and a line feed, in each direction. A blank line is no frame.
- A frame holds at most 16 MiB. A frame past the bound gets a refusal, and the daemon closes the peer.
- A frame that the daemon cannot read gets a refusal with no id, and the peer stays.

## The hello

The first frame of a peer is its hello:

```json
{"id": 0, "verb": "hello", "protocol": 1, "kind": "window", "name": null}
```

- `kind` is `window`, `skill`, or `daemon` for the daemon of a new build that asks the handover, and a skill gives
  its `name`.
- The daemon answers with its protocol and the peer as it knows it: its login, its role, its kind and its name.

  ```json
  {"frame": "reply", "id": 0, "answer": "welcome", "data": {"protocol": 1, "peer": {"login": "uael", "role": "write", "kind": "window", "name": null}}}
  ```

- A hello of another protocol gets a refusal that names both versions, and the daemon closes the peer. A first frame
  that is no hello gets the refusal `first`, and the daemon closes the peer. A second hello gets the refusal `again`.

## Requests and answers

A request is an object with an `id`, which the answer gives back, and a `verb`, with the fields of that verb. A verb
that names an act names it as `act`. A life is named by the path of its record, and `on` names a chain, the root when
it is not given. The daemon answers requests in any order.

| Verb | Fields | Role |
| --- | --- | --- |
| `list` | | read |
| `open` | `life`, `cwd`, `actor`, `roster`, `extensions`, `answers` | write |
| `attach` | `life`, `from` | read |
| `detach` | `life` | read |
| `pin`, `unpin` | `life`, `holder` | write |
| `presence` | `life` | read |
| `get`, `peek` | `life`, `act` | read |
| `transcript` | `life`, `on`, `of` | read |
| `turns`, `program`, `cwd` | `life`, `on` | read |
| `standing`, `pending` | `life` | read |
| `inspect` | `life`, `name`, `on` | read |
| `prompt` | `life`, `shape`, `message`, `to`, `on` | write |
| `answer` | `life`, `act`, `line` | write |
| `pause`, `wake`, `cancel` | `life`, `act` | write |
| `chain` | `life`, `label`, `on` | write |
| `run` | `life`, `word`, `on` | write |
| `close` | `life`, `act`, `value` (a value as the record writes it) | write |
| `grant` | `life`, `usd`, `share`, `on` | write |
| `bash` | `life`, `command`, `on` | write |
| `read` | `life`, `path`, `on` | write |
| `write` | `life`, `path`, `content`, `on` | write |
| `names` | `life`, `on` | read |
| `gate` | `life`, `word`, `on` | read |
| `stand` | `life` | write |
| `seen` | `life`, `about` | write |
| `queue` | `life`, `entry`, `hash`, `shape`, `message`, `on` | write |
| `answered` | `life`, `ask`, `turn`, `why` | write |
| `wrote` | `life`, `ask`, `text`, `thinking` | write |
| `handover` | | write |
| `view.mount`, `view.tree`, `view.event`, `view.effect`, `view.dispose` | see below | read |

- The role to write holds each right of the role to read. A verb needs the role to write unless this table says read,
  and the daemon refuses it to a reader with the refusal `role`, which names both roles.
- An answer is a frame `reply` with the `id`, an `answer` that names its kind, and its `data`: `done`, `welcome`,
  `lives`, `act` (the name of the act that the verb made), `value`, `values`, `inspected` or `presence`.
- A refusal is a frame `refused` with the `id`, or none for a frame that could not be read, the `error` as data, and
  `why`, the text that a person reads.
- An `open` that makes a life gives it the `actor`, the `roster` and the `extensions` that it names, and the settings
  of the daemon give each one that it does not name. An open of a life that lives changes none of them.
- An `open` with `answers` makes its peer answer the models of the life in their place, as a demo or a test does. The
  daemon pushes each request of a model to that peer as a frame `ask`, with the `life`, the number `ask` and the
  `request`: its `actor`, `chain`, `messages` and `settings`. The peer tells what it writes with `wrote`, and gives the
  turn, written as the record writes a value, or `why` it gives none, with `answered`. A request fails once its peer
  is gone.
- `queue` sends a message that a window queued once: the record keeps its `entry` and the `hash` of its words, so a
  second `queue` of the same entry, from any window, gives the act of the first.
- After `attach`, the daemon pushes each event of the life as a frame `event`, with the `life`, the `seq` of the event
  in the log of the life and the `event`: `fact`, `act`, `text`, `notice`, `changed`, `reborn` or `ended`. After
  `reborn`, the life goes on from its checkpoint, and its acts come again as they stand.
- After each `attach` and `detach`, the daemon pushes to each window of the life a frame `presence` with the `life`
  and its `peers`, each window that is attached now. Presence is no fact of the record.

## The daemon

`furb host` serves the daemon of the machine in the folder `host` of the cache of furb, and says where it listens
once it listens. A second daemon refuses to start there.

- `open` gives the life of a record, by its absolute path, in the folder `cwd`, the folder of the record when it is
  not given. Each life opens on a thread of its own, so the daemon answers each verb while a life opens, and a fault
  of one life ends that life alone. Each verb of a life refuses with `closed` when no life lives on its record.
- Each life keeps the journal of the changes of its files beside its record, as `<record>.changes.jsonl`: one line
  for each write of a file, with its `path`, the text `before` and `after` it, its `act`, and `made` when the write
  made the file. A window reads it to show and to undo what each write changed.
- A holder keeps a life: each window that attached it, and each pin, by the name of its holder. Work that runs holds
  nothing. A life with no holder for 30 seconds writes its checkpoint and ends, and `list` gives it as not `open`
  from then on. An `open` of a life that ends waits for its end, then opens it again from its checkpoint.
- `detach` lets go of the window that says it alone, as `/end` does, and a window whose peer leaves is detached.
- `furb host upgrade` serves the daemon of a new build in the place of the daemon that runs, which has no takeover.
  The new daemon says `handover`. The daemon that runs writes the checkpoint of each life, ends it, stops to listen
  and answers `handed` with each life, its folder and its pins. The new daemon listens and opens each life again with
  its pins, then the old one closes each window and ends. Each window dials again and attaches, and the life goes on
  from its checkpoint.
- A frame of a view gets the refusal `viewless` until the daemon routes views.

## Views

A view is a tree of nodes that the process of a skill with code serves, and a window draws. The types of this section
are versioned with the protocol, and `@uael/furb/protocol` gives them to TypeScript.

- `view.mount {viewer, life, slot, skill}`: a window mounts a view of a skill in a slot of a life. A slot follows a
  chain or the life:
  - `dock {on}` follows the chain `on`.
  - `row {act}`, `detail {act}` and `mark {act}` follow the chain of the act: its row, its detail, or a mark in the
    feed after it, for an act of a kind that the skill draws.
  - `sidebar`, a section of the sidebar, `footer`, `screen` and `overlay` follow the life. An overlay takes the place of
    a choice, a form or a confirmation.
- `view.tree {viewer, slot, version, tree}`: the tree of a view, which takes the place of each tree of a lower
  version.
- `view.event {viewer, node, kind, value}`: a person pressed, changed or submitted a node, by its `id`, with the value
  of an input.
- `view.effect {viewer, effect}`: the skill asks the one window of the view to open a URL, to set the draft, or to show a
  notice.
- `view.dispose {viewer}`: the view ends in its window.

Each node has an `id`, and its `node` names one of a closed set: `box` (a row or a column, with a gap and a border),
`text` (runs, each in a tone), `list`, `table`, `input`, `button`, `progress`, `code`, `diff` and `markdown`. A window
that reads gets each tree with its acting nodes inert.

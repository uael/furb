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

### The tailnet

The daemon also listens on the address of the machine on the tailnet of Tailscale when `host.json`, in the config
folder of furb (`~/.config/furb` by default), turns it on. WireGuard encrypts each frame between two machines.

```json
{"tailnet": {"tcp": 7860, "websocket": 7861}, "roles": {"alice@github": "write", "bob@github": "read"}}
```

- `tailnet` gives the port of each transport. The daemon listens by TCP, by WebSocket, or both, on the first address
  that Tailscale gives the machine, IPv4 before IPv6. With no `tailnet`, the daemon listens on the machine alone.
- The identity of a peer is the login that the local API of Tailscale gives for its address. `roles` gives each login
  its role, and the daemon closes the peer of a login that it does not name, before its hello.
- The daemon reads `host.json` when it starts. A change takes effect at the next start or upgrade.
- The daemon finds the local API on the socket of `tailscaled`, or on the port of the app for macOS from the
  standalone download. It does not find the app of the App Store.
- Plain TCP listens only on loopback or on the tailnet, `100.64.0.0/10` and `fd7a:115c:a1e0::/48`, since no other
  network encrypts the frames.

## Framing

- Each frame is one JSON object and a line feed, in each direction. A blank line is no frame. On WebSocket, each
  frame is one text message, with no line feed.
- A frame holds at most 16 MiB. A frame past the bound gets a refusal, and the daemon closes the peer. A message of
  WebSocket past the bound breaks the stream.
- A frame that the daemon cannot read gets a refusal with no id, and the peer stays.

## The hello

The first frame of a peer is its hello:

```json
{"id": 0, "verb": "hello", "protocol": 1, "kind": "window", "name": null}
```

- `kind` is `window`, `skill`, or `daemon` for the daemon of a new build that asks the handover, and a skill gives
  its `name`, which it must.
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
| `open` | `life`, `cwd` | write |
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
| `handover` | | write |
| `view.mount`, `view.dispose` | see below | read |
| `view.event` | see below | write |
| `view.tree`, `view.effect` | see below | read |

- The role to write holds each right of the role to read. A verb needs the role to write unless this table says read,
  and the daemon refuses it to a reader with the refusal `role`, which names both roles.
- An answer is a frame `reply` with the `id`, an `answer` that names its kind, and its `data`: `done`, `welcome`,
  `lives`, `act` (the name of the act that the verb made), `value`, `values`, `inspected` or `presence`.
- A refusal is a frame `refused` with the `id`, or none for a frame that could not be read, the `error` as data, and
  `why`, the text that a person reads.
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
- A holder keeps a life: each window that attached it, and each pin, by the name of its holder. Work that runs holds
  nothing. A life with no holder for 30 seconds writes its checkpoint and ends, and `list` gives it as not `open`
  from then on. An `open` of a life that ends waits for its end, then opens it again from its checkpoint.
- `detach` lets go of the window that says it alone, as `/end` does, and a window whose peer leaves is detached.
- `furb host upgrade` serves the daemon of a new build in the place of the daemon that runs, which has no takeover.
  The new daemon says `handover`. The daemon that runs writes the checkpoint of each life, ends it, stops to listen
  and answers `handed` with each life, its folder and its pins. The new daemon listens and opens each life again with
  its pins, then the old one closes each window and ends. Each window dials again and attaches, and the life goes on
  from its checkpoint.
- A skill attaches a life as a window does, under its name, and hears its events from the place that it asks. It holds no
  life, so a life with no window and no pin ends while a skill is attached. A skill that attaches again under its name takes
  the place of the first, and its views end. A skill that detaches, or whose peer leaves, ends its views.

## Views

A view is a tree of nodes that the process of a skill with code serves, and a window draws. The types of this section
are versioned with the protocol, and `@uael/furb/protocol` gives them to TypeScript.

- `view.mount {viewer, life, slot, skill}`: a window that attached the life mounts a view of the skill of that name, which
  is attached to it, in a slot of the life. The skill hears the frame. A viewer names one view of the daemon, so a viewer
  that is mounted refuses `mounted`, and a skill that is not attached refuses `no_skill`. A slot follows a chain or the
  life:
  - `dock {on}` follows the chain `on`.
  - `row {act}`, `detail {act}` and `mark {act}` follow the chain of the act: its row, its detail, or a mark in the
    feed after it, for an act of a kind that the skill draws.
  - `sidebar`, a section of the sidebar, `footer`, `screen` and `overlay` follow the life. An overlay takes the place of
    a choice, a form or a confirmation.
- `view.tree {viewer, slot, version, tree}`: the tree of a view, which takes the place of each tree of a lower
  version. The skill says it, and only the window of that viewer gets it.
- `view.event {viewer, node, kind, value}`: a person pressed, changed or submitted a node, by its `id`, with the value
  of an input. The window says it, and the skill of the view hears it. It is an act, so a window that reads is refused
  it with `role`.
- `view.effect {viewer, effect}`: the skill asks the one window of the view to open a URL, to set the draft, or to show a
  notice.
- `view.dispose {viewer}`: the view ends in its window. The window says it, and the skill hears it. When the skill goes,
  the daemon says it to the window.

A window says `view.mount`, `view.event` and `view.dispose`, and a skill says `view.tree` and `view.effect`. A peer of
the other kind is refused with `speaker`, and a frame of a viewer that the peer does not hold with `unmounted`.

Each node has an `id`, and its `node` names one of a closed set: `box` (a row or a column, with a gap and a border),
`text` (runs, each in a tone), `list`, `table`, `input`, `button`, `progress`, `code`, `diff` and `markdown`. A window
that reads gets each tree with its `input` and `button` nodes `inert`: it draws them and takes no gesture.

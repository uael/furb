# The protocol of the host

`furb-host` is the daemon of a machine: it holds each life, and each window or skill reaches it over one protocol. This
page says that protocol. Its version is `PROTOCOL`. The crate gives its JSON Schema in
`host/protocol.schema.json`, and `bind/typescript/src/protocol.ts` holds the types that `bun run protocol` makes from
that schema, with `src/furb/protocol.py` for Python, which it makes through `cargo xtask protocol`. A test of each side fails when the schema or the types do not agree with the code.

## Transports

A transport has two parts: a dialer, which reaches the daemon, and an identity, which says who a peer is. A peer is
an operator: the login that its transport proved. The record never names a window, and two windows of one operator
are one operator.

- On macOS and Linux, the daemon listens on the socket `host.sock` in a folder of mode 0700 that the user owns. The
  daemon refuses a folder that another user owns or can open, and a peer of another user.
- On Windows, the daemon listens on a named pipe whose access list lets in the user alone, and which takes no peer of
  another machine.
- The operator of a local peer is the user of the machine, and it has the role to write. The name of the user is
  the login of the system, or, when the daemon listens on the tailnet, the login that Tailscale gives the machine:
  one person is then one operator from each of its machines. A machine that turns the tailnet on later names its user
  with the new login, and a record keeps the name of before.
- A second daemon cannot listen where a daemon listens. A socket that no daemon answers is stale, and gives its place.

### The door of the windows

A web view reaches no socket and no pipe, so `furb host` also opens a door of the windows: a WebSocket on a free port
of loopback. A page of any origin can reach loopback, so the door lets in only a peer that gives the token of the
daemon.

- The daemon writes the address of the door, with its token, in `window.json` beside its socket, which only the user
  reads: `{"url": "ws://127.0.0.1:52011/?token=..."}`. Each start and each upgrade draws a new port and a new token.
- The daemon makes the folder of that file on a new cache. On Unix the file is of mode 0600 in a folder of mode 0700; on
  Windows the file and the folder that the daemon makes hold an access list that lets in their owner alone.
- A peer that gives no token in the query of its upgrade, or another one, gets the HTTP status 403 and no socket.
- The operator of a peer of the door is the user of the machine, under the same name, and it has the role to write.
- A window dials the socket first: `window.json` stays after a daemon that a kill ended, and only a daemon that
  answers on its socket holds the door that the file names.

### The tailnet

The daemon also listens on the address of the machine on the tailnet of Tailscale when `host.json`, in the config
folder of furb (`~/.config/furb` by default), turns it on. WireGuard encrypts each frame between two machines.

```json
{"tailnet": {"tcp": 7860, "websocket": 7861}, "roles": {"alice@github": "write", "bob@github": "read"}}
```

- `tailnet` gives the port of each transport. The daemon listens by TCP, by WebSocket, or both, on the first address
  that Tailscale gives the machine, IPv4 before IPv6. With no `tailnet`, the daemon listens on the machine alone.
- The operator of a peer is the login that the local API of Tailscale gives for its address. `roles` gives each operator
  its role, and the daemon closes the peer of an operator that it does not name, before its hello.
- `stall` gives the seconds after which the daemon tells that a life with an open act and no new fact stalled, as
  the section on attach says. With no `stall`, the bound is 1200 seconds.
- The daemon reads `host.json` when it starts and at each change of the file, and a file that does not parse keeps
  the list before. A change of `roles` takes effect at once on each connected peer: an operator that the file drops gets
  a refusal and loses its connection, and an operator whose role changes holds the new role from its next verb. A change
  of `tailnet` or `stall` takes effect at the next start or upgrade.
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
{"id": 0, "verb": "hello", "protocol": 3, "kind": "window", "name": null, "role": null}
```

- `kind` is `window`, `skill`, or `daemon` for the daemon of a new build that asks the handover, and a skill gives
  its `name`, which it must.
- `role` is optional. The daemon gives the peer the lesser of the role that it asks and the role that its transport
  proved, so a peer that asks `read` is a reader on any transport, and a peer never has more than its transport
  proved. With no `role`, the peer has the role that its transport proved.
- The daemon answers with its protocol and the peer as it knows it: its `operator`, its role, its kind and its name.
  The answer also names the `machine` of the daemon, without its domain, and the `build` of the daemon, which is its
  version of furb.

  ```json
  {"frame": "reply", "id": 0, "answer": "welcome", "data": {"protocol": 3, "peer": {"operator": "uael", "role": "write", "kind": "window", "name": null}, "machine": "studio", "build": "0.4.2"}}
  ```

- A hello of another protocol gets a refusal that names both versions, and the daemon closes the peer. A skill is the
  exception: a kit works from the moment that it is written, so the daemon takes a skill of each version from `KITS`
  to its own. What a skill says keeps its meaning across those versions, or the daemon maps it, and
  `host/tests/kits/<version>.jsonl` holds the lines that a skill of each version says, which a test of the crate
  replays on each later daemon. `host/tests/kits/<version>.heard.jsonl` holds the frames that a skill of each version
  reads, and the test holds each later daemon to tell a frame of each with each key of it, of the same type, and the
  same name for a variant. A change of the protocol that breaks one of those lines adds its mapping. A first frame
  that is no hello gets the refusal `first`, and the daemon closes the peer. A second hello gets the refusal `again`.
- The version rises with each change of the schema, `host/protocol.schema.json`: a test of the crate keeps the digest
  of the schema of each version. A window that meets a daemon of another version does not wait and asks no handover,
  so it ends no life of another window: it says that the daemon of this machine is of another build. That daemon ends
  once its last window quits and its grace passes, and the next window starts one of its own build.
- A request that the daemon cannot read, as a verb of a later version, gets the refusal `unread` under its `id`, with
  why it names the verb, so no request waits for an answer that never comes.

## Requests and answers

A request is an object with an `id`, which the answer gives back, and a `verb`, with the fields of that verb. A verb
that names an act names it as `act`. A life is named by the path of its record, which the daemon reads with no link in
it, so each path of one record names one life and its cards, and `on` names a chain, the root when it is not given. The
daemon answers requests in any order.

| Verb | Fields | Role |
| --- | --- | --- |
| `list` | `config` | read |
| `projects` | `config` | read |
| `models` | `cwd`, `config`, `claude` | read |
| `files` | `cwd` | write |
| `changes` | `cwd`, `path` | read |
| `cards` | | read |
| `history` | `life` | read |
| `skills` | `life` | read |
| `plays` | `life` | read |
| `open` | `life`, `cwd`, `actor`, `roster`, `skills`, `config`, `claude`, `answers`, `attach` | write |
| `attach` | `life`, `from` | read |
| `detach` | `life` | read |
| `pin`, `unpin` | `life`, `holder` | write |
| `end` | `life`, `cancel` | write |
| `presence` | `life` | read |
| `writes` | `life`, `draft` (`{"on": thread or null}`, or null) | write |
| `get`, `peek` | `life`, `act` | read |
| `ended` | `life`, `act` | read |
| `transcript` | `life`, `on`, `of` | read |
| `turns`, `program`, `cwd` | `life`, `on` | read |
| `standing`, `pending` | `life` | read |
| `filled` | `life`, `act` | read |
| `inspect` | `life`, `name`, `on` | read |
| `prompt` | `life`, `shape`, `message`, `to`, `on` | write |
| `answer` | `life`, `act`, `line` | write |
| `pause`, `cancel` | `life`, `act`, `why` | write |
| `wake` | `life`, `act` | write |
| `run` | `life`, `word`, `on` | write |
| `close` | `life`, `act`, `value` (a value as the record writes it) | write |
| `names` | `life`, `on` | read |
| `roster` | `life`, `roster` (names of models) | write |
| `cancel_sign_in` | `life` | write |
| `config` | `life`, `config`: `{"action": "get"}`, `{"action": "set", "key", "value", "layer"}` | read for get, else write |
| `image` | `life`, `name`, `data` (the bytes of a PNG, a JPEG, a GIF or a WebP, in base64): the life keeps it beside its record and answers the text `![name](furb-image://…)` by which a message names it | write |
| `seen` | `life`, `about` | write |
| `resolve` | `life`, `thread` | write |
| `queue` | `life`, `entry`, `shape`, `message`, `on` | write |
| `unqueue` | `life`, `entry` | write |
| `queued` | `life`, `on` | read |
| `desk` | `life`, `notices` | read |
| `threads` | `life`, `on`, `at`, `read` | read |
| `as_of` | `life`, `at` | read |
| `moments` | `life`, `on` | read |
| `tracks` | `life` | read |
| `thread` | `life`, `thread`, `before`, `limit`, `at` | read |
| `answered` | `life`, `ask`, `turn`, `why` | write |
| `wrote` | `life`, `ask`, `text`, `thinking` | write |
| `handover` | | write |
| `view.mount`, `view.dispose` | see below | read |
| `view.event` | see below | write |
| `view.tree`, `view.effect` | see below | read |

- `ended` gives the act as the life folds it, once it is done, as `acts` with that one act: at once when it is done,
  else when its done comes. A window that ran a word asks it of the rung that `run` gave, and asks it again after a
  reconnect, since the record keeps the end. The `run` of the act says how the word ran: `done`, or `failed` with the
  fault that a person reads. A name that the fold of the life does not hold is refused.
- `run` is the one path of each act of a person on a chain: a new chain, a fork, a `cd`, a stand, an actor, a
  command, a read, a write, a grant. A window sends the word that a model would write for the act, as `Door` of
  `furb-host` gives it, so the transcript of the chain shows his act as it shows the act of a model.
- `roster` names the models that the life offers, in place of its roster, as the catalog names them, so a window
  offers a model that the roster lacks with no new life. No word says it, since the roster is the World's: the life
  stands on it, the answer is the act of the stand, and the record keeps the answer, so a later life of the record
  goes on with that roster.
- `config` reads and changes the configs of the folder of a life, which the World writes, so no window writes a disk.
  A get gives `values`, each key with the value that wins for it and its layer, `user`, `project` or `local`. A key has
  one or two names, as `["providers", "codex"]`. A key that a schema names comes with its `property`, as
  [`config.md`](config.md#the-schema) says, and with the `skill` that adds it, when one does; such a key that no
  layer gives comes with its default and no layer, and an object of furb, as `providers`, comes with the object of
  the values that win and no layer, then with a value for each of its names. A set refuses a value that the property
  of its key does not take, and writes nothing. A set writes the key in its layer, `project` when none is named, or
  takes it out when no value is given, and then the life stands, so the answer is the act of the stand. A change that
  the catalog cannot read puts its config back as it was, and the life does not stand.
  A set of `["providers", "codex"]` to `true` starts the sign-in of codex on the machine of the daemon, unless an
  account is connected or a sign-in runs. The daemon pushes to the peer that asked the frame `sign_in` at the current
  state and at each change of it, with the `state` (`disconnected`, `waiting`, `connecting`, `connected`,
  `needs-login`, `failed` or `canceled`), the `url` of the browser while it waits, and the `message` that a person
  reads when it needs login or failed. The answer of the set comes when the sign-in ends: the act of the stand when
  it connects, and a refusal when the operator cancels it or it fails. A set to `false` cancels a sign-in that runs. A
  window keeps no state of the sign-in beside these frames and asks for none. The verb `cancel_sign_in` cancels the
  sign-in that the daemon runs for a life and writes no config, so the set that asked it is refused.
- `list` gives each record of the machine as `lives`: each record of the sessions of each project that
  `workspaces.json` in the folder `config` names, the config folder of the daemon when the peer names none, in the
  `.furb/sessions` of its main checkout and of each worktree of it, with that main checkout as `project`, and each
  record that a life of the daemon holds, whose `project` is the checkout whose `.furb/sessions` holds it when the
  list names no project for it. `used` is the second of the epoch of the last write of the record, when the disk
  holds it. `held` says whether a life holds the record now, and `open` whether that
  life lives and does not end, and `watched` whether a window of any connection holds the life, which a pin
  does not make, so a window finds the sessions that no window holds from the one list. `pins` names the holder of
  each pin of the life: a life that a pin holds and no window holds stands in the background, and a life that
  nothing holds is in its grace, and ends. A window lists its sessions from it, so a window on another machine lists them the
  same way.
- `projects` gives the projects of the records that `list` gives with the same `config`: each `folder`, the main
  checkout of a project, once, with the count of its records as `sessions` and the latest `used` of them, the one used
  last first. A record of no project stands in none.
- `models` gives the models that the catalog of the machine offers to a life in the folder `cwd`, with the settings
  of the folder `config` and the claude command line at `claude`, each as `open` takes it, the ones of the daemon
  when the peer names none. It needs no life. The first is the default actor. Each model has its `name`, which
  `actor` of `open` takes, its `efforts`, the `effort` that a life takes when its actor names none, the tokens of
  its `window`, and whether it sees `images`.
- `files` gives the files of the folder `cwd`, where a `~` at the start names the home of the user of the machine, and of each folder in it, which a mention of the input names: each
  path from `cwd`, with `/` between its parts, dot files too, less what an ignore file of git leaves out, the
  folder `.git` and the records of the sessions of furb in `.furb/sessions`, in the order of a walk by name, at most 20000 of them, and `more` when the folder holds more. It needs no life, and its walk holds back no other verb.
- `changes` gives the changes of the checkout of git that holds the folder `cwd` against its last commit: its `branch`,
  none when its head names no branch, each file that differs, by its `path` from the top of the checkout, with the
  lines that it has `added` and `removed` and its `state` (`added`, `modified`, `deleted`, `renamed`, or `untracked`
  for a file that git does not track yet), and, when the peer names a `path`, the `diff` of that file as git writes a
  unified diff, an untracked file added whole. A folder in no checkout has no branch and no change. It needs no life
  and runs git on the machine.
- The role to write holds each right of the role to read. A verb needs the role to write unless this table says read,
  and the daemon refuses it to a reader with the refusal `role`, which names both roles.
- An answer is a frame `reply` with the `id`, an `answer` that names its kind, and its `data`: `done`, `welcome`,
  `lives`, `projects`, `models`, `files`, `changes`, `act` (the name of the act that the verb made), `value`, `values`, `inspected` or
  `presence`.
- A refusal is a frame `refused` with the `id`, or none for a frame that could not be read, the `error` as data, and
  `why`, the text that a person reads.
- An `open` that makes a life gives it the `actor`, the `roster` and the `skills` that it names, and the settings
  of the daemon give each one that it does not name. It reads the settings of the user in the folder `config` and runs
  the claude command line `claude` that it names, as the window that opens it reads them. An open of a life that
  lives changes none of them.
  A verb that names a field that it does not take is refused, and does nothing: an `open` that names `extensions`, the
  field that protocol 4 renamed `skills`, is refused with a refusal that names `extensions` and the fields that the
  verb takes, `skills` among them, and opens no life. The hello alone takes a field that it does not know, so that a peer
  of another protocol reaches the refusal that names both versions. The schema does not show this rule, since JSON
  Schema cannot close the fields of a request that flattens its verb.
- An `open` with `answers` makes its peer answer the models of the life in their place, as a demo or a test does. The
  daemon pushes each request of a model to that peer as a frame `ask`, with the `life`, the number `ask` and the
  `request`: its `actor`, `chain`, `messages` and `settings`. The peer tells what it writes with `wrote`, and gives the
  turn, written as the record writes a value, or `why` it gives none, with `answered`. That peer alone speaks for each
  request of the life, by its connection, for the whole life: the daemon refuses a `wrote` or an `answered` of any
  other peer with `foreign`, and the request still waits for its peer. Once that peer is gone, a later `open` with `answers` of the life makes its peer answer each later request; while it lives, such an `open` is refused with `answering`. A
  request ends at its turn, when the life drops it, or when the connection of its peer ends, and fails in the last
  case. A `wrote` or an `answered` after its end is refused with `unasked`.
- An `open` with `attach` attaches its peer in the same step, from that place in the log, as `attach` does. The life
  then has a holder before its grace can pass, even a grace of none.
- `resolve` resolves a `thread` of the operator, as its model does when it closes its prompt: it closes each act that
  keeps the thread open, its prompt and each message on it that is not done, the last first, with `None` for a shape
  `None` and with the text `resolved` for a shape that takes a text. A close cancels the work under each act. An act of
  another shape refuses the resolve, since only its model gives that value. A message on the thread opens it again.
- `queue` puts a follow-up of the operator in the queue of a chain, under an `entry` that the window chooses, and a
  second `queue` of the same entry, from any window, changes nothing. The life sends the first follow-up of a chain as
  a prompt of its `shape` and its `message` once the chain rests: no pause holds it, it asks the operator nothing, no
  work of it lives, and the prompt of its last follow-up is done. `unqueue` takes out a follow-up that waits, and
  `queued` gives the follow-ups that wait on a chain, in order. A send that fails holds the queue of its chain, with
  its cause in `failed`, until the operator takes the follow-up out. The record keeps each step as a fact `queue` of
  the operator, so a reopen finds the queue as it stood.
- `desk`, `threads`, `as_of`, `moments` and `tracks` give the meaning of the acts that the life told as events, which
  each window draws the same way: `desk` each question that the operator owes and each of the `notices` that the window
  still shows, the oldest first, with the lines that answer each; `threads` the tree of the threads of a chain, its
  asides, its runs of notes, what the tree says to the operator and the state of the chain, now or just after the moment
  `at`; `as_of` the acts as they stood just after a moment; `moments` each moment at which an act began or ended, of the
  life, of the chain `on`, or of the acts that the pages of the thread `on` show; `tracks` each chain of the life in the
  order it began, with its label, the state of its work, the moments of its acts and the moment each of its threads
  began. A run of notes is the asides that are no question and follow one another in the feed of the chain, with its
  `count` and its `unread` count: the notes that began or ended after the place `read`, which the window says since only
  it knows what the operator saw. A `threads` with no `read` reads nothing, so each note is unread.
- After `attach`, the daemon pushes each event of the life as a frame `event`, with the `life`, the `seq` of the event
  in the log of the life and the `event`: `fact`, `act`, `text`, `retried`, `queued`, `notice`, `changed`, `reborn` or
  `ended`. A `retried` names the `rung` and the `chain` of a reply whose call of its model failed for a cause that
  passes, and its `retry`: the `attempt` that comes, the `most` attempts, the seconds that the provider `wait`s before
  that attempt and the second of the epoch `at` which it comes, and `why` the attempt before it failed; what the model
  wrote for that rung before it is void. After `reborn`, which a life tells once it told the past of its record, the
  life goes on from the carry across its last crossing, and its acts come again as they stand. A peer that attaches from a place past
  the tip, as a window that reads the cards and the pages and no event, hears the event `ended` alone, so it learns
  that the life ended and why, as when its open failed. A life that a fault ended answers each view of what it told,
  as `threads` and `thread`, read only, until the window leaves it, and refuses each other verb with that fault. A
  record from before the stable cut, which `docs/promise.md` says, opens in no life: its life tells each fact of the
  record, then `ended` with why, and a window reads its threads that way.
- The log of a life that opens on a record that crossed starts with each fact that the record told before its last
  crossing. The log lets go of each of them once it told it, since the record holds it, and of each fact that an open
  told, which the file of the log of the record holds, and a window that attaches before them reads them again from the
  record or from that file. So a window of a life that a later open, a handover or an end
  brought back across a crossing hears the life whole, and hears no fact twice.
- `cards` gives the cards of the machine as `cards`, and the peer then hears a frame `cards` at each change of them,
  until it leaves. A card is one thread that an operator started: a prompt of his at the top of the threads of its
  life, with all that answers it. The answer holds the cards of each record that `list` gives with no `config`. A
  card holds:
  - `life`, the record of its session, `session`, the name of the session when it has one, and `project`, the main
    checkout of the project whose sessions hold the record, when the list of the machine names one, else for a
    record that a life of the daemon holds, the checkout whose `.furb/sessions` holds it;
  - `prompt`, the name of the prompt of the thread, and `title`, the first line of its message;
  - `standing`, how the thread stands, as `{"is": ...}`: `asks` with `of`, the login of the one operator that its
    question waits for, or null when it waits for each operator; `failed`; `paused` with `by`, who said the pause;
    `thinks`, `runs` or `delegates` while it works; `stopped` after a cancel; `rests` while it is open and nothing
    works; and `resolved` once its model closed its prompt;
  - `since`, the second of the epoch since which the thread stands so. A thread that works keeps the time of the
    start of its work while its model and its commands take turns;
  - `model`, the model that wrote its last word, and `agents`, how many threads work under it;
  - `readers`, the site of each operator who read the thread to its end: `operator` for each of them, or
    `operator/<login>` for one member;
  - `held`, whether a life holds the record now, `viewers`, each operator whose window holds that
    life, and `pins`, the holder of each pin of it, as `list` gives them. A card that is `held` with no viewer works
    with no view.

  A frame `cards` names one `life`, each card of it that is new or that `changed`, and the `prompt` of each card
  that is `gone`. A life tells its cards once in 250 ms at most, whatever the count of its events.

  The cards of a record at rest come from the index of its threads, the file `<record>.threads.json` beside it,
  which the life writes at its end, before its image. The index is a cache: it names the tip of the record that it
  was made from, which is the length of the record and the sha256 of its last 4096 bytes, and the daemon drops an
  index whose tip is not the tip of the record. So the list reads the index and the end of each record, and never the past of a
  record. A record with no index that agrees with it gives one card with an empty `prompt`, under the name of its
  file. The daemon then makes its index from a fold of the record with no life, one record at a time and off the
  path of each verb, and pushes the cards in a frame `cards` once the index stands. While it reads the record, it
  pushes at most four frames `indexing` each second, with the `life` and the thousandths of the read `done`, `of`
  1000, which never go back: the two passes of the reader over the record make the first two thirds, and the fold
  of the facts that it tells the last third. The frame with `done` equal to `of` comes before the frame `cards`.
  A record tells no time, so each
  card of such an index stands since the record last changed. Nothing works in a record at rest, so a thread that
  worked when its life ended rests there, and counts no agent that works.
- `history` gives what the peer sent in the project of the record `life` as `history`, the newest first and each text
  once: what it sent to that record, then to each other record of the project, the record that changed last first.
  Each entry holds `by`, the site of the peer, `text`, and `python`, whether it is a word of Python that ran or a
  message. The record is the one truth of what was sent: a live life gives what it holds, and a record at rest gives
  what the index of its threads keeps, so a record whose index does not agree with it gives nothing until the daemon
  made its index. An index of a build before this list reads as no index.
- `skills` gives the skills of the record `life` as `skills`, in the order of their names, each with its `name` and its
  `description`: the skills that the last read of `skills://` on the root of its life told, as the record keeps that
  read, else those of each other record of its project, the record that changed last first, so a new session offers
  the skills of the last session of its project. A record whose index does not agree with it gives none until the
  daemon made its index, and an index of a build before the skills reads as no index.
- `plays` gives the skills of the life `life` as `skills`, as its root finds them now, in the order of their names:
  each with its `name` and its `description`, `plays`, whether it plays in the life now, `on`, whether the configs
  turn it on, `default`, whether it plays by default, `turned_by`, the value of its name in `skills` of a config
  that turns it on, `true` for an official one, the path of its folder for another with code, and null for a skill
  with no code, and `from`, `furb` or its folder. A change of `skills` through `config` plays at once the skill that
  it turns on, and one that it turns off plays on until the life ends.
- `thread` gives a page of one thread of a life as `page`: the exchanges of the thread before the one at the place
  `before`, or before its end when `before` is null, at most `limit` of them, the oldest first, as they stood just after
  the moment `at`, or now when `at` is null. The thread is named by its prompt. Nobody reads the past of a record for
  it: the life holds each act, and a page reads only the acts of its thread, by the list of the acts that each act made.
  A page holds:
  - `above`, how many exchanges of the thread stand before the first of the page, so a window asks the older page
    with `before` set to that place when the reader scrolls to it;
  - `exchanges`, each a message of the thread with all that answers it: the prompt of the thread, then each message
    that an operator said on it. A wake or an acknowledgment that the engine says on the thread is no exchange. What
    answers a message is what came after it and before the next message, whichever prompt of the thread each rung
    advances. An exchange holds `at`, its place in the thread, 0 for the prompt; `message`, its act; `said_at`, the
    moment that it was said; `by`, who said it; `text`; `runs`, whether its act still runs, which a `cancel` takes back, since a message said on a thread
    that works is done at once; `block`, the code that answers it once a model answers; and `said`, what the answer says to the
    operator, in the order it was said: each `notice` and `question` of its words, and each `answer`, the text that
    closed a message there. A `question` holds the `shape` of its answer and, once it is closed,
    `answered`: `by`, who closed it, and `reply`, its `value` as a person reads it or the reason that `refused` it.
    A question that is not closed waits for the operator: `answer` closes it with a line of its shape, and `close`
    with a refusal, `{"is": "Refused", "args": ["why"]}`, closes it with words that its model reads;
  - a `block` holds `maker`, the model that wrote its last word, `more`, whether more comes since the answer works,
    `thought`, the `text` that its model thought before it wrote, `began`, the millisecond of the epoch at which the
    daemon saw it begin, and the seconds that it `took`, null while it thinks, and `lines`. A line has its `number` in
    the thread, which goes on from block to block, its `text`, in which a quote stands as the binding that it makes and
    a string of many lines as one line with its escapes, and its `mark`: `ran`; `runs` with the `work` that it waits
    for, `writes` while its model writes it, `command` or `agent`, and `began`, the millisecond of the epoch at which
    the daemon saw that work begin; `asks`; `retries`, the line that its model writes while its provider waits to ask it
    again, with the `attempt`, the `most`, the `wait`, the `at` and the `why` of the retry; `failed` with `why`, which
    the line that its model writes takes when the call of its model failed and no attempt is left; or `stopped` with
    `at`, the millisecond of the epoch at which the daemon saw the stop, null when it opened the life after it. The line
    that a model writes runs from the moment its model is asked, before its first part. A line that awaited a command
    has its `output`, which is absent on each other line: the `tail`, the last 3 lines that the command wrote, those of
    its stdout then those of its stderr, each as a terminal shows it, with no escape and no control; `more`, how many
    lines came before them; and the seconds that it `took` once it ended, null while it runs and when the daemon
    opened the life after it began. The tail moves as the command writes. A line that made acts has `made`, which is
    absent on each other line: what each act but a chain came to, in the order the line made them, one for each pass
    of a loop: `works`, `failed`, or `done` with its value as a literal of python of at most 80 characters, the last
    of which is the value that the line bound;
  - `agents`, each thread that a word of the thread started on another chain, in the order the thread made them, as
    a card without its life, with `began`, the millisecond of the epoch at which the daemon saw it begin, `took`, the
    seconds that it took once it ended, `line`, the line of its last word where it stands: the line that runs or
    asks, else its last line, and `at`, the number in the thread of the line of the word that made it, absent when
    no line of a word of the thread made it. A note to a chain that works is no agent: its model reads it at its
    next step;
  - `context`, null before the first answer on the chain of the thread: what its last answer `read`, its `tokens` in
    all, of which it read `cached` from the cache and `written` to it, the `window` that the roster of the standing
    gives the actor of that answer, null when the roster names no such actor, and the `dollars` of each answer on the
    chain.

  The page of the end of now also makes the peer hear a frame `thread` with the `life`, the `thread` and a `page` at
  each change of that thread, until it reads the end of another thread or leaves: a peer reads one thread at a time. The
  page of a frame holds each exchange of the end that is new or that changed, by its `at`, with `above`, `agents` and
  `context` as they stand. The daemon asks the life a page at once after an act of the life, a part that a model wrote
  or a retry of its call, and then at most once in 33 ms. What a model thought, when each work began and when each agent
  ended are no facts of the record: the life holds them while it lives, so a life that opens again shows no thought of
  the words before it.
- After each `attach`, `detach` and `writes`, the daemon pushes to each window of the life a frame `presence` with
  the `life` and its `operators`, each operator whose window holds it now, once, in the order that his first window
  attached: his `operator`, the greatest `role` of his windows, and `writes`, the draft that one of his windows
  writes, in the thread `on`, or for a new thread when `on` is null, or null when none writes. A window says `writes`
  when its draft starts and again with null when it empties, and the daemon drops the draft of a window that leaves.
  Presence is no fact of the record, and no model is told of a connect or a leave. A model is told of an operator that
  the access list adds or drops, as the stand that follows names it in the roster.
- While a life opens, the daemon pushes to each window of the life a frame
  `opening` with the `life`, the `stage` of the open, and two numbers: `done`, the work done of that stage, and `of`,
  the work known of it. A window draws a progress bar from them.

  ```json
  {"frame": "opening", "life": "/work/a.jsonl", "stage": "boot", "done": 1730, "of": 2649}
  ```

  - The stages are a closed list, and they come in this order. An open skips a stage that it has no work for.

    | Stage | The work | `done` of `of` |
    | --- | --- | --- |
    | `recall` | The daemon reads what the record told before its last crossing, for the log of the life. | Thousandths of the read, of 1000, counted as the frame `indexing` counts them. Both are 0 until the daemon found the crossings. |
    | `find` | The record is read, for the build of its last life. | Bytes passed, of the bytes of the record. |
    | `cross` | A life that another build of furb ran is replayed on the engine of that build, which writes the fact made of this build and tells each chain the lines of the engine that differ. | Both are 0: the stage counts nothing. |
    | `read` | The record is read again under its lease, when it changed after `find`. | Bytes passed, of the bytes of the record. |
    | `seed` | The sandbox loads the engine and makes the carry across each crossing of the record, or it loads the image of the life. When the image does not load, the record is read in this stage. | Both are 0: the stage counts nothing. |
    | `boot` | The life replays the entries that it boots on. | Entries that the replay passed, of those entries. |
    | `log` | The daemon puts what the open told in the log of the life, which its windows read. | Events put, of those events. |

  - The numbers of a stage never go back, `done` never passes `of`, and a stage never comes again in one open. A
    record in a database tells no bytes, so its `find` stays at 0 of the bytes of the record. The
    replay passes at once the entries that it says to no ear, so `done` of `boot` can go forward by many entries.
  - The first frame comes when the open starts. Then the window hears a frame at once for each stage that the open
    reaches, with the last numbers of each stage before it, and at most four frames each second for a stage that goes
    on. A window that attaches while the life opens hears one frame for each stage that the open reached.
  - The frame of `log` with `done` equal to `of` ends the open. It comes before the first event that the open tells,
    so a window ends its bar there. An open that fails sends no such frame, and the life ends with the event `ended`.
    A window that attaches to a life that stands hears no frame `opening`.
  - The frame is no event of the log of the life and no fact of the record.
- A verb of a life that a peer says while the life opens waits for the life, which answers it once it stands, in the
  order that it heard each verb. A life that ends on a fault, as an open that fails, refuses with that fault each
  verb that waited for it, and each such verb that comes after its end, in the place of `closed`, until the next
  open of the record: a peer that heard the answer to its open cannot know when the open failed. The daemon answers
  each other verb at once.
- The daemon watches each life that it holds, from outside the life: no word and no ear of the life runs the watch. When
  a life holds an open act, a run of a word, a reply of a model or a command, and says no fact, and no model writes or
  is asked again, for the bound of a stall, 20 minutes unless `stall` in `host.json` gives other seconds, the daemon
  pushes to each window of the life a frame `stalled` with the `life`, its open `acts`, each with its `act`, its `kind`
  and its `chain`, and the `notice` that a window shows as it stands. It also sends the notice to the root chain of the
  life as a prompt of `None`, so that its actor wakes.
- The watch also holds the last change of each chain. A chain stalls when it holds a rung that no run, reply, command or
  wait lives beside on it, or a prompt to a model with no rung and nothing else on it, and no act of it changes for 60
  seconds, or when a reply of a model on it writes nothing, is not asked again, and no act of it changes for 5 minutes,
  each bound or the bound of a stall when that is shorter, even while other chains work. Work that waits for the
  operator never stalls a chain or the life: a paused act, a prompt to the operator, an act on a chain that asks him,
  and a rung or a run whose word awaits such a prompt. The daemon pushes the frame `stalled` with the acts of that chain
  alone, and sends the notice as a prompt of `None` on that chain, so that its own actor wakes and no other.
- The watch tells each stall once: nothing more until a fact of an act that its message did not make, on that chain
  for the stall of a chain, and a new stall after that fact. A stall is no fact of the record.

## The daemon

`furb host` serves the daemon of the machine in the folder `host` of the cache of furb, and says where it listens
once it listens. A second daemon refuses to start there.

- `open` gives the life of a record, by its absolute path, in the folder `cwd`, the folder of the record when it is
  not given. Each life opens on a thread of its own, so the daemon answers each verb while a life opens, and a fault
  of one life ends that life alone. Each verb of a life refuses with `closed` when no life lives on its record.
  The daemon answers an `open` only once its life holds the lease of the record, before the life reads it, so a
  process that opens the record after that answer, as `furb run` or another daemon, is refused with the reason, and
  the life goes on. An `open` of a record that another process holds is refused at once with the reason.
- Each life keeps the journal of the changes of its files beside its record, as `<record>.changes.jsonl`: one line
  for each write of a file, with its `path`, the text `before` and `after` it, its `act`, `made` when the write
  made the file, and `unread` with why when the file could not be read as text, whose `before` is then empty and
  no undo puts back. A window reads it to show and to undo what each write changed. One journal serves each birth
  of the life, in the order of the writes, and the life ends, and replies to its end, only once the disk holds each
  change; a fault of the disk is told as a notice when the journal begins to lose changes, and again when it takes changes again, and the end of the life says what the journal lost.
- A holder keeps a life: each window that attached it, and each pin, by the name of its holder. Work that runs holds
  nothing. A life with no holder for the grace of the daemon, 30 seconds unless `furb host --grace <seconds>` says
  otherwise, writes its image and ends, and `list` gives it as not `open` from then on. An `open` of a life
  that ends waits for its end, then opens it again from its image.
- `end` ends a life now, as its grace would: it writes its image, each pin of it goes, each of its windows hears the
  event `ended`, and its record opens again at its tip, where the next life takes its open work again, so an end is
  a pause. With `cancel` true, the end is a kill: the operator first cancels each open work of the life, each act
  that is not done and is no chain and that no other open work made, so the next life takes none of it. It is
  refused with `shown` while a window of another peer holds the life, so no view ends a life that another view
  shows.
- The daemon of a machine ends once it holds no peer and no life for its grace, so it does not outlive the windows
  that started it; a window that dials later starts a new one. A daemon that no peer dials within a minute of its
  start ends too, so a program that starts it and ends before it dials leaves no daemon. A daemon with a door on the
  tailnet does not end by either bound.
- A life whose record does not hold each fact that it said yet, as on a full disk, does not end: its end, after the
  grace, at a handover or at the end of the daemon, waits until the record holds them, and the life tells why in a
  notice. Past 20000 facts that the record lacks, the life pauses each chain that lives and each chain
  that opens while it holds, and tells in a notice the folder of the record and the space left on its disk; once the record holds each
  fact, it wakes each chain that it paused, and a pause of the operator stays. The life drops no fact.
- `detach` lets go of the window that says it alone, as `/end` does, and a window whose peer leaves is detached.
- `furb host upgrade` serves the daemon of a new build in the place of the daemon that runs, which has no takeover.
  The new daemon says `handover`. The daemon that runs ends each life, which writes its image, stops to listen
  and answers `handed` with each life, its folder and its pins. The new daemon listens and opens each life again with
  its pins, then the old one closes each window and ends. Each window dials again and attaches, and the life goes on
  from its image when the new build reads it, else from its record. A life that ran another engine, Kernel or
  checkpointer reaches its tip on them, writes the fact made of the new build, tells each chain the lines of the
  engine that differ, and goes on with those of the new build, at the stage `cross`. The handover waits for each life to end, however long it takes, and the log of the daemon
  names each life that it waits for: a life given up before its end would still hold its record and the facts that
  its record does not hold yet. A life that ends already ends there and goes to no daemon, and the next `open` opens
  it from its record.
- A skill attaches a life as a window does, under its name, and hears its events from the place that it asks. Each verb of
  a skill that says an act or a fact, as `run`, `prompt` or `close`, speaks in the life under `skill/<name>`, but `seen`, `queue`, `unqueue` and `config` speak as the operator, whoever says them, since they are the operator's own, and
  the verbs of a window speak under `operator/<login>`, the member that its transport proved, so a model asks one
  person with `to="operator/<login>"`. It holds no
  life, so a life with no window and no pin ends while a skill is attached. A skill that attaches again under its name takes
  the place of the first, and its views end. A skill that detaches, or whose peer leaves, ends its views.
- The process of a kit does not dial the daemon: the life that plays the kit starts it at the first act of a kind
  that the manifest of the kit names under `furb-acts`, and speaks this protocol to it on its stdin and its stdout, one
  frame for each line. The process says `hello` as a `skill` of a version from 44, `attach`, and then `peek`, `close`,
  `run` and `prompt`, each with the field `life` that `FURB_LIFE` gives. The life tells it each fact that it hears
  after its attach as a frame `event`, the act that the process answers among them, and the done of that act. A process
  draws nothing, so the life refuses `view.tree` and `view.effect` from it. The variables `FURB_LIFE` and `FURB_SKILL`
  of the process give the record of the life and the name of the kit. `@uael/furb/skill` gives its client in
  TypeScript, `furb.skill` in Python, and `docs/skills.md` says the fields.

## Views

A view is a tree of nodes that a kit of a session serves from a value of its life, and a window draws. The types of this section
are versioned with the protocol, and `@uael/furb/protocol` gives them to TypeScript.

- `served {life, skills}`: the names of the skills attached to a life, which a window mounts views of. The daemon pushes it
  to each window of the life when it attaches, and after each attach and each end of a skill.
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
that reads gets each tree with its `input` and `button` nodes `inert`: it draws them and takes no gesture. The set and the fields of each node only grow within a major: `cargo xtask contract` reads them from
host/protocol.schema.json, and a node or a field that goes is a break.

# One life

This is a design for the ruling of the owner. It changes no code. Each later PR of the plan at the end builds one
part of it, with its tests.

The owner, on 10-23: "I want next life to be the last one: one session, only one, to manage furb, for weeks and
years, never a new session for a lack of features in furb."

The measure of done: on the demo, one life upgrades its engine twice and reloads an extension, with no drift, no
chain lost, no act run twice, and a chain that compacts.

Each claim below has its evidence: a file and its line, a PR, or a command on a copy of a real record. A claim that
is derived and not seen is marked [inferred]. The copies were made with `cp -c` in the scratch directory, and the
originals were not touched.

## 1. Why a life ends today

The three lives of 10-20 to 10-23, in `.furb/sessions`:

| Life | Record | How it ended |
| --- | --- | --- |
| 1 | `d3630cdb`, 78 MB | The engine changed, and the record could not open on the new one (reborn.md:3). |
| 2 | `f6e8243e`, 185 MB, 96322 entries | Its old engine froze on a prompt on a prompt, and the new engine could not replay it (reborn3.md:7 to :10). |
| 3 | `c3978f7f`, 423 MB, log 576 MB | It still runs. It crashed at 06:08 on 10-23, and it went on only after a person cut its record and wrote the fact `made` again by hand. |

The five causes that the owner named:

- C1. The engine changes. A record replays only on the engine that made it (#301). Four digests of the engine came
  in one day. With the fact `made` taken out, the record of life 3 drifts at `reply67` on main 4640b1a8, since #342
  numbers the chains in another order. So the refusal of #301 is true, and a hand edit of `made` no longer works.
- C2. An extension changes. The ear `extensions` plays a word only for an extension that the life does not run yet
  (`src/extension.rs`, `runs`). Life 3 played the word of the conductor of 10-22 16:54 on 112 chains while four PRs
  changed it on main, and its model defined `sweep` and `tend` again by hand in 16 replies.
- C3. A chain fills its window. The chain of the orchestrator filled to 70 percent in a day in each life
  (reborn2.md:3, reborn4.md:3), and life 2 handed over to chain98 by hand.
- C4. A record grows without end. Life 3 holds 423 MB in 21 hours, its log 576 MB and its journal of changes
  130 MB, and each open replays every entry.
- C5. A host stops. The process of the TUI is the parent of each model and each command, so a new build of the TUI,
  a quit or a crash ends the life.

The causes that the owner did not name:

- C6. A defect of the engine that the running life cannot fix: a freeze (reborn3.md:9). #302 bounds a word, but a
  spin in the engine itself still ends the life.
- C7. The provider lost the conversation of chain1 (`No conversation found with session ID`, life 3, 06:08). The
  chain paused, and the resume needed a cut of 380 entries and the four answers of the owner given again by hand.
- C8. A cancel reaches every act under it. One key of the TUI cancelled the prompt under which the loop and five
  workers ran (ops-decisions.md:232). #314 fixed the key, not the shape.
- C9. The state of the orchestrator lives in its module, which no new life loads, so each handover wrote
  `state.json` by hand: 6 queued items, 6 in flight, 25 efforts, 9 briefs (reborn4.md:8).
- C10. A record of an older form drifts: the tuples before #96, and a record of 09-27 that drifts at `memory1` on
  two builds of 10-23. No file of a life names the version of its form, but the snapshot.
- C11. The interpreter changes under the same digest. Five pins of monty came in two days, and four of them changed
  no byte of `engine.py` or `kernel.py`, so the digest stayed the same while what a word computes changed.
- C12. The TUI slows with the life. Its bench fails at 1000000 facts (`Failed to create TextBuffer`), and a real
  life says 1000000 facts in 3.4 days [inferred from 12200 facts each hour].
- C13. No life opens from a snapshot. The log of a life disagrees with its record after one reopen: the log of
  life 3 lacks the 3669 questions `merged` that the record kept before its last open, so `logged` gives nothing and
  `snapshot::restore` never restores. And a TypeScript `Session` writes none, since its observer is a foreign ear
  that never ends. `.furb/sessions` holds 47 records and no snapshot.
- C14. The bound of a turn is of the wall clock (30 s, `src/sand.rs`), so a word near the bound can raise on a
  replay on a slower machine where it did not raise live, and the replay drifts.
- C15. The heap of a life holds every fact since its birth (#288: "The memory of a life does not shrink"). The TUI
  of life 3 held 1.6 GB.
- C16. Not all the parts that make acts are in the digest: the revision of monty, `play` and the ears of rust that
  make acts, and the names of the ears (a host that renames an ear makes the journal drift).

So the design must give: an engine, a Kernel, an extension and an interpreter that change inside a running life; a
life that reads only its recent past; a host that outlives each window and each build; and formats that each later
build reads.

## 2. One primitive: the successor

A successor of a chain is a chain with a source, a filter, and the state that it carries, which takes over the work
of its source. Today a chain with a source copies its origin and stands beside it. A successor supersedes it and
takes its name: the chain goes on under its name, and only its transcript is cut to the prefix that the filter kept.
So no name of a live act changes, and a brief that names `chain1` or a value that names `prompt1177` stays true.

- In the same life, on the same engine: the chain keeps its module, its ladders and its open acts. So a word that
  runs on, as `while True: await conduct()`, goes on with the same values, and each open prompt stays the same act.
  Nothing is moved, stopped, paused or run twice.
- On another engine: the chain is one that a checkpoint carries as plain data, and the new engine makes it again
  under its name, with the carried turns as its prefix, in a module made of the definitions and the plain values
  that the checkpoint carries (section 3.2).

Where the one primitive serves:

| Need | As a successor |
| --- | --- |
| Compaction (#317) | `compacted` makes a successor of its chain with the filter that the model chose. The restore of the module, the move of each prompt, the close of each rung that runs and the rung that keeps an old prompt quiet all go. |
| A chain that supersedes another | A word makes the successor of its own chain, which keeps its name. |
| An upgrade of the engine at a checkpoint | The new engine makes each chain that the checkpoint carries again, under its name. |
| A record that grows | The checkpoint, which carries the successors, is where a later life starts to read. |
| A chain of another record | `carry` answers with a carried chain, and the new chain is its successor (section 3.7). |

Where it serves in part: a fork for each question. A fork is a chain with a source that does not supersede it. It
keeps the prefix of its source whole, so its first turn reads that prefix from the cache, and the source never
grows. This is `chain(label, source)` as it stands. The gap is in the provider (section 4).

Where it does not serve, and what serves:

- A host that stops, or a new build of the TUI: it is the same life on the same chains, so no chain changes. The
  host of its own (section 3.5) and the snapshot serve.
- A reload of an extension: it changes the module of each chain, not the chain. A fact of the record serves
  (section 3.4).
- The versions of the formats: section 3.6.

## 3. The parts

### 3.1 Supersede

The engine gives one law (#371): a prefix said on a chain takes the place of what its transcript held, so the chain
goes on under its name with its module, its ladders and its acts. It costs 23 tokens of the engine. The successor
itself is `supersede(keep)` of the extension compaction (#375): it says on the chain a prefix of what the filter
kept, all that made it, and every act of the chain, so each view that reads the acts reads as before, and only the
turns are cut. A first shape put `supersede=True` on `chain` and cost 71 tokens; the law of the prefix alone serves
the same need, and a checkpoint says a prefix too.

The extension compaction becomes smaller: `compacted` asks `choice`, then at the next reply makes the successor with
that filter. The conductor wraps its own chain with `compacted()` in its word. So the chain of an orchestrator is
wrapped by default, and compaction stays a wrapper that a word puts on one chain (the ruling of reborn.md:9).

### 3.2 Segments and the checkpoint

The record becomes a sequence of segments. The first starts at the root. Each later one starts at a checkpoint, a
fact of the World on the root. The fact `made` names the version of the build that wrote the segment (section 3.3),
and each stand says it again, so each part of the record names the build that made it.

A checkpoint carries, as plain data:

- the count of the names of each kind, so no later act takes a name of the past;
- each chain that lives, which is a chain with an act that is not over, as its turns read it: its tells and the
  answers of its replies;
- for each such chain, the plain values of its module and the definitions of its program: the statements at the
  top of each word that define (`def`, `class`, `import`, and an assignment to a name in capitals), in the order of
  the program, which the parser of monty finds;
- each act that a carried chain, a carried value or an open act names, with its answer when it has one.

A plain value is one that the record can hold. A name whose value is neither plain nor a definition is not carried,
and the checkpoint names it, so the model sees what it lost.

A new engine boots from the last checkpoint. The journal holds each carried act under its name, counts each kind on
from the carried count, makes each carried chain again under its name, makes each open act again through its verb under
its own name, and then says again only the entries after the checkpoint. A word that ran when the checkpoint was
written is cut: its run is done with a refusal that says the checkpoint cut it and which act it awaited, so its
model sees that outcome and runs it again if it wants. This is the law of #290 for a command that a crash cut. So
the model of the conductor runs its loop again, on the carried state.

The World writes a checkpoint at an upgrade, when a host ends, and when the tail since the last one passes a count
that the host sets: 50000 facts by default, about four hours of life 3. A checkpoint in a running life also lets the
host start the sandbox again from it, so the heap holds only the live state and the tail (C15).

The form of a checkpoint is one fact of the record, whose values are in the plain form of the record (the marks of
#96 for a tuple, a `Text`, an `Exit` and a whole float):

```
["checkpoint", "chain1", "store", {
  "version": 1,
  "build": "<the version of the build that wrote it>",
  "before": "<the sha256 of the entry before it>",
  "counts": {"chain": 118, "prompt": 1206, "bash": 7414, ...},
  "acts": [[<act>, <its done, or null>], ...],
  "chains": [{"chain": "chain1", "turns": [<tell or done of a reply>, ...],
              "values": {"queue": [...], "working": {...}, ...},
              "definitions": ["def hire(...): ...", "CAP = 6", ...],
              "lost": ["<a name whose value is not plain>", ...]}, ...],
  "open": ["prompt1177", "bash7394", ...]
}]
```

- `acts` holds each act that a carried chain, a carried value or an open act names, with its answer, so `get`,
  `peek` and `over` of that name give what they gave.
- `chains` holds each chain with an act that is not over: its turns as its model read them, the plain values of its
  module, the definitions of its program, and each name that it lost.
- `open` names the acts that are not over, in the order of the record, which the next build makes again through
  their verbs under their names. An open run is not in it: the checkpoint cuts it.

As built (#this PR), the fact carries the same state in another form: its words hold `entries`, facts that the journal of
any engine already replays, so the checkpoint costs no token of the engine. The root, each chain that lives, the last
stand, each act that a carried value names with its answer, each open prompt as an open act, and a reserve of the
names of each kind that no entry carries. Each carried chain then takes a rung that binds the definitions and the plain
values of its module, and says a prefix of its turns, which keeps its module. A chain with an open prompt opens paused,
and its prompt asks its model again at a wake. `src/checkpoint.rs` holds the shape.

The entries before the last checkpoint are history. No life replays them, a reader reads them, and the store can
move them to a file of their own beside the record.

### 3.3 Versions, the store of builds, and live patching

The owner, on 10-23: "An engine version is a distribution: it is small, only Rust and embedded python. Maybe we keep
a store of versions, replay each part of a record on the same one that made it, then stand on the new version in the
new life. Each stand discloses a version, and we reuse the exact same binary. Keep few versions: increment only on
breaking changes."

- A version is a build that replays: the engine, the Kernel, the sheet, monty and the ears of rust that make acts. A
  build names its version as a number, `VERSION` in the crate, and the fact `made` names that number and the commit
  of the build.
- Few versions, by a rule that a machine checks. A gate replays a fixed corpus of records on the build of each PR and
  compares the facts with the log of each record. A change that replays each record to the same facts keeps the
  version. A change that fails the check adds one to `VERSION` and records the corpus again, in the same PR. The
  corpus holds records that the repository can hold: the demo, the fixtures of the TUI, and a life of
  `script/play.py` on a claude that answers from a script. So most PRs keep the version, and a life goes on with
  them on a new build with no switch at all.
- The store of builds: each build that a record names stands as one program in
  `<cache of furb>/builds/<version>/furb`. A build puts itself there at its first run. A build that the store lacks
  is fetched from the release of its version, or built from its commit with `script/worktree.sh` and cargo. So any
  later build replays any part of any record on the exact program that made it, and no replay drifts: this is the
  rule of the owner that a drift is an internal hard error (R2).
- A stand on a new version: when a life opens on a build of another version than the last segment, the old build
  from the store replays the tail of that segment with no model and no command (`furb checkpoint --record <path>`),
  and writes a checkpoint at the tip. The new build starts from that checkpoint, with the successor of each chain. A
  dump of the old interpreter does not load in the new one, so the checkpoint is plain data.
- Live patching: an upgrade is an act on the root, which the host takes, so the record keeps its place. In a host of
  its own (section 3.5), the old host ends its life, and the new host opens it again: the old build from the store
  writes the checkpoint, and the new build boots from it. In one
  process, for an engine or a Kernel of the same build of rust, the host makes a new sandbox from the checkpoint and
  drops the old one. The World is of the host and not of the sandbox, so each command and each reply that runs goes
  on and lands in the new segment: only the words that run are cut. A change of the same version needs no
  checkpoint: the new build replays the tail as it stands.
- The owner weighed a record that holds the text of each engine. The store of builds replaces it: a text of python
  does not hold monty and the ears of rust, which make acts too (C11, C16), and the exact program does.

### 3.4 A reload of an extension

When the word of an extension that the life runs changes, by a write of the life or by a change of its file that
the host sees, the ear `extensions` says the fact `enable` again with the new word. Each chain plays the definitions
of the new word at its next step. So a function, a class and a constant in capitals take their new value, and a
name of state, as `queue` or `working`, keeps its value. The record keeps the fact at its place, so a later life
plays the same change at the same place.

### 3.5 A host of its own

One process holds one life: the record, the lease, the World, the provider, the commands and the views. The TUI and
the app attach to it and detach, so a new build of the TUI restarts only the window. The worker of the TUI and
`gui/host` of #361 already hold a life behind a protocol, so the host is that worker behind a socket in the cache of
furb, with a cursor, so a window that comes back reads only what it missed. A new build of the host takes over: a window
whose build differs from the build that the host says asks the host to hand the life over. The host waits up to 10
seconds for each command and each reply of a model that runs, so each that ends in time lands in the record. Then it
ends the life, so the record and the lease are free, tells each other window that a window of that build took the
life, and exits. The window starts a host of its own build on the same record, which opens the life again: a replay
on the same version, or an upgrade through a checkpoint on a new version (section 3.3). A command or a reply that
runs past the wait is cut: the new host holds it as pending, and the next wake ends it as interrupted, as a command
that a crash cut (#290), so its model runs it again if it wants. The ask to hand over is the end of the life with the
build in its arguments, so a host that does not know it, as a host of the protocol 1 before it, ends the life as at
any end. On the demo, a takeover takes about 0.3 seconds, and a first host about 0.18 seconds. An agent of the user starts the host again after a reboot.

### 3.7 A successor of a chain of another record

The owner, on 10-23: "import chains from a previous journal." This is one more source of a successor: a chain of
another record. `carry(record, chain)` is an act that the World takes. It reads the other record with no life, and
answers with that chain in the form of one item of `chains` of a checkpoint, and the record keeps that answer.
`chain(label, source=<that act>)` makes the successor of the carried chain. So a replay of the new life reads the
same turns from its own record and opens no other file. The act names its source by the path, the count of its
entries and the digest of its last entry, which the chain of digests of the version 2 of the record proves.

- What it carries: the turns of the chain as its model read them, from the log of the record or from `retold`
  (#311), and each open prompt of a model, which goes on as a prompt of the successor with its shape, its message and
  its actor, under a new name. When the store holds the build that made the record, that build writes a checkpoint
  at its tip first (section 3.3), and the import carries the plain values and the definitions of the module too.
  When no store holds that build, as for the records of 10-20 and 10-21, the import carries the turns alone, and the
  successor names each value that it lost.
- What it cannot carry: a generator, a word that runs and an open command, which ended with the life that ran them.
  An act of another record keeps its name only in the turns, since a name of this record never names an act of
  another one.
- What it serves: the life after this one, if one comes, takes chain1, chain8 and each worker with their context,
  and a record that no build of the store opens any more still gives its chains.

### 3.6 The versions of the formats

The version 2 of the record holds a chain of digests, which the owner ruled for the item bench-audit: the store
writes beside each entry the sha256 of the entry before it, and a host that ends writes a closing fact with the
digest of the last entry. So a record proves by itself that no entry was changed or cut, and a cut by hand, as the
one of 10-23 (C7), shows. A checkpoint holds the digest of the entry before it, so the chain goes on across each
segment, and a segment that the store moves to a file of its own stays proved by the checkpoint after it. The cut of
C7 is not needed any more: the provider makes a lost conversation again (PR 10), and an upgrade replaces the hand edit
of the fact `made`.

Each file of a life names the version of its form. Each later build reads each older version, or migrates it with no
loss, with a test for each version:

| File | Version |
| --- | --- |
| record | the fact `made` gains the version of the form of the record: 2 with segments and a chain of digests, and a record with no version is 1 |
| log `<record>.log` | its head line gains `version` |
| snapshot | `version`, as now, and its key gains the revision of monty |
| journal of changes | a first line `{"version": n}` |
| `<record>.session.json`, `<record>.ui.json`, `ui.json`, `workspaces.json` | a key `version` |

## 4. Tokens and the prefix cache, measured

On a copy of the record and the log of life 3, 6280 replies read 1245 million tokens: 1198 million of them from the
cache, and 47 million written to it, for 732 dollars. A write costs 12.5 times a read.

- A fork: `chain29` from `chain12` read 12259 tokens from the cache in its first reply and wrote 157486, for 1.28
  dollars. Its second reply read 169745 and wrote 880, for 0.05. So the provider did not fork the conversation of the
  source. It forks only when the messages of a held conversation are a prefix of the new ones, and the filter of
  that chain dropped acts. A fork with no filter keeps the prefix as it stands, and its first turn then costs about
  a tenth of the prefix [inferred].
- A successor: `chain114`, the compaction of `chain73` by the old extension, wrote 693388 tokens in its first reply,
  for 5.56 dollars, more than the 507261 of the last reply of its source. The successor of #317 kept 48709 tokens of
  62330 for chain1, and 37689 of 306071 for chain12.
- So a fork costs about 0.1 P, and a successor 1.25 S once and then 0.1 S at each turn, where P is the prefix and S
  the successor. A fork costs less while P is less than 12.5 S. A chain kept for questions holds a small P, so each
  question is a fork. A chain that works on is a successor. An upgrade changes the system prompt, which is the
  engine, so it writes each prefix once in any case.
- A defect that this design does not fix, for a PR of the provider: chain1 wrote 21.8 million tokens to the cache in
  574 replies, about 136 of its 226 dollars, and 49 of those replies wrote more than half of their prompt less than
  a minute after the reply before. So the prefix of a chain changes inside, and not only at its end, and the cause
  is not the five minutes of the cache.

## 5. What the interpreter can still force

- A change of what a word computes: each part of the record replays on the build that made it, from the store of
  builds, so a new monty changes no replay. A pin of monty that changes a fact of the corpus is a new version, by the
  gate.
- The form of its dump: a checkpoint holds no dump, so a new `DUMP_VERSION` drops only the snapshot, which is a
  cache.
- The bound of a turn (C14): a replay runs with no bound a word whose end the record holds.
- The heap (C15): the checkpoint bounds it.
- Its own binary: a new monty is a new build of the host, which takes over through a checkpoint (section 3.5).

## 6. The cost on a record of a year

From life 3, scaled to a year at its rate [inferred]: 12200 facts and 4600 entries each hour, a record of 175 GB, a
log of 236 GB and a journal of changes of 53 GB.

| Part | Today, for a year | With this design |
| --- | --- | --- |
| Open of a life | a replay of 40 million entries, about 4 hours (35 s for 96322 entries, #302) | the last checkpoint and a tail of at most 50000 facts: a few seconds, and less than one second from the snapshot |
| A checkpoint | none | at the tip of life 3, 11 chains with an open prompt of a model hold 53 MB of the 574 MB of its log, and their turns alone are less, about 15 MB [inferred] |
| Disk of the checkpoints | none | about 90 MB each day at 15 MB each four hours, so 33 GB a year beside 175 GB of record |
| Heap | grows with the life: 1.6 GB at 250000 facts | the live state and the tail since the last checkpoint |
| A supersede | none | no copy: the successor shares the module and the open acts |
| An upgrade | a new life | a checkpoint, a new sandbox and a boot from it: a few seconds [inferred: a parse of 60000 lines of the log of life 3 takes 1.9 s in python, and its live chains hold about 23000 lines] |
| A build in the store | none | one program `furb`: 64 MB in the dev profile today (`target/debug/furb`), less in a release |
| A stand on a new version after a crash | a new life | the old build replays a tail of at most 50000 facts, about 7 s at the rate of #302, then writes the checkpoint |
| The TUI while it switches | none | the host tells each window that the life switches, with each step (the tail, the checkpoint, the boot); the feed stays readable from the log, and what the operator sends waits in the queue of follow-ups |
| A reload | none | the definitions of one word on each chain at its next step |
| TUI | 1.6 GB and slow at 250000 facts, and no draw at 1000000 | the item bigtui: it reads the summary, the table of chains and the pages in view, with a memory that does not grow with the age |

Each PR that builds a part measures it again on copies of the real records.

## 7. The sentences of the contracts that change

`src/furb/engine.pyi`, `chain`, new:


- A source is a chain of the life, or an act of carry that the World answered with a chain of another record, of
  which the new chain is the successor, with the turns, the open prompts and the plain values that the answer
  carries.

`chain`, changed (#371): was "Nothing leaves a transcript once the transcript holds it." Now: "Nothing leaves a
transcript once the transcript holds it, but by a prefix said on its chain, which takes the place of what the
transcript held."

`Prefix`, new (#371): "A prefix takes the place of what the transcript of the chain that it is said on held, so a word
that says a prefix on its own chain cuts its turns, and the chain goes on under its name, with its module, its
ladders and its acts."

`extensions/compaction/compaction.pyi`, new (#375): `supersede(keep)`, "supersede(keep) says on the chain of the
caller a prefix of what the filter keep kept of its acts, everything that made what it kept, and every act of the
chain, and gives the chain." and "So the chain goes on under its name with its module, its ladders and its acts,
and only its turns are cut."

`Standing`, was: "A standing holds no source: the engine is one file the model imports, and a record made by another
engine is a drift." Now: "A standing holds no source: the engine is one file the model imports, and a segment of the
record replays only on a build of the version that wrote it, which the fact made names."

`Entry`, new: "The record is a sequence of segments: the first starts at the root and each later one at a
checkpoint, and the fact made names the version of the build that wrote each segment, which each stand says
again."

A new type, `Checkpoint`, with these sentences:

- A checkpoint is a fact of the World on the root, which starts a segment of the record.
- It carries, as plain data, the count of the names of each kind, each chain with an act that is not over as its
  turns read it, the plain values of the module and the definitions of the program of each such chain, and each act
  that a carried chain, a carried value or an open act names, with its answer when it has one.
- The definitions of a program are the statements at the top of each of its words that define: def, class, import,
  and an assignment to a name in capitals, in the order of the program.
- A name of a module whose value is neither plain nor a definition is not carried, and the checkpoint names it.
- An act of carry is answered with one chain of another record in the form that a checkpoint carries it, and the
  record keeps that answer, so a replay reads the same turns.

`boot`, new:

- Given a record that holds a checkpoint, the journal reads from the last one and says again no entry before it.
- It holds each act that the checkpoint carries under its name, with its answer, and counts each kind on from the
  count that it carries, so no later act takes a name of the past.
- For each chain that the checkpoint carries, it makes that chain again under its name, whose prefix is the carried
  transcript and whose module is made of the carried definitions and then the carried plain values.
- It makes each open act that the checkpoint carries again through its verb, under the site of its maker and under
  its own name, so the ear of the outside that runs it goes on with it, and a command or a reply that runs across a
  checkpoint ends once.
- A word that ran when the checkpoint was written is cut: its run is done with a refusal that says the checkpoint
  cut it and the act that it awaited, so its model sees that outcome and runs it again if it wants.

`boot`, changed:

- Was: "The engine makes a chain from the record and in no other way, by running its rungs again." Now: "The engine
  makes a chain from the record and in no other way: by running its rungs again, or from the checkpoint that carries
  it."
- Was: "What the module holds is not in the record." Now: "What the module holds is not in the record, but what a
  checkpoint carries of it."
- Was: "In a later life the rungs of every chain run again from the record that the World kept." Now: "In a later
  life the rungs of every chain run again from the last checkpoint of the record that the World kept, or from its
  start when it holds none."

`extensions/compaction/compaction.pyi`:

- `restore` goes, with its three sentences.
- `compact`, was: "The first word of that chain restores the chain of the caller, binds the summary that compact is
  given as the quote summary when there is one, and regrounds the new chain." Now: "compact() makes the successor of
  the chain of the caller, and its first word binds the summary that compact is given as the quote summary when
  there is one, and regrounds the new chain."
- `compacted` loses six sentences: "It moves each open prompt of a model to the new chain, and closes the old prompt
  with what the new one comes to.", "It stops each open rung of a prompt that it moves with a close, ...", "A rung of
  its own on each prompt that it moves keeps that prompt from asking its model again.", "The summary names each rung
  that it stopped, ...", "A prompt of a model that asks its model on a chain that it compacted moves to the newest
  chain at that reply." and "It moves no prompt to the operator and no prompt that made it, and it never completes."
  It gains: "The chain supersedes itself, so each open prompt, each rung that waits and each word that runs
  goes on in it, and nothing is moved, stopped or paused." and "It never completes."

`extensions/ouroboros/ouroboros.pyi`, `extend`, new: "A life that runs the extension already reloads it, so each
chain plays the definitions of the new word at its next step."

Outside the contracts, the laws that change: `CLAUDE.md`, the meaning of the record, where "loaded from the snapshot"
gains "or from the last checkpoint"; `src/furb/CLAUDE.md`, the law "A chain is a function of the record";
`src/life.rs`, where the refusal of a record of another engine becomes a boot from its last checkpoint and an upgrade
at the tip; and `src/extension.rs`, where the ear `extensions` reloads a word that changed.

## 8. The PRs, in order

Each enables the next, and each carries its tests.

1. This design, with the label owner-merge.
2. fix(crate): the log agrees with its record after a reopen (C13), so a snapshot restores.
3. fix(typescript): a `Session` keeps a snapshot, with its observer as an ear of the host by its name (C13).
4. feat(crate): each format names its version, the record holds its chain of digests with the work of the item
   bench-audit, and each reader reads each older version, with a test for each (C10).
5. feat(crate): the version of a build, which each stand names, the gate of the corpus, and the store of builds
   (C1, C11, C16).
6. feat(engine): a chain supersedes its source, and compaction makes a successor (C3, C9).
7. feat(engine): a record goes on from its last checkpoint, and a chain from a chain of another record (C4, C9,
   C15).
8. feat(crate): the upgrade of a running life at a checkpoint, and a stand on a new version (C1, C6, C14).
9. feat(extensions): the reload of an extension (C2).
10. feat(tui): a host of its own, to which the TUI and the app attach (C5, and C12 with bigtui).
11. fix(provider): a lost conversation is made again from the turns (C7), and the prefix of a chain stays as it
    stood.
12. The measure on the demo: one life upgrades its engine twice, reloads an extension and compacts a chain, with no
    drift, no chain lost and no act run twice.

C8, a cancel that reaches every act under it, stays the law of the engine. The round runs on a chain of its own,
under no prompt of the operator, which the conductor does in PR 6.

## 9. The decisions taken for the owner

- A successor takes the name of its source, so no name of a live act changes, no scope moves, and the engine pays
  23 tokens. A copy of the module, as `restore` of #317, goes, since it is a second truth of the same state.
- A checkpoint carries plain data and no dump of monty, so no form of the interpreter binds a later build. Its data is
  facts that the journal of any engine replays, so the engine pays no token for it.
- A word that a checkpoint cuts ends as interrupted, as a command that a crash cut (#290), and its model runs it
  again.
- A takeover waits up to 10 seconds for the commands and the replies that run, then ends the life, and each that
  runs past the wait ends as interrupted at the next wake, as after a crash (#290). The window that attaches last
  takes the life over, and a host that it started keeps its build until another window attaches.
- A version is a build, kept in a store, and a gate on a corpus of records keeps the versions few. The text of the
  engine in the record goes, since it does not hold monty and the ears of rust.
- A chain of another record is carried by an act whose answer the record keeps, so no replay reads another file.
- A reload rebinds the definitions and the names in capitals, and keeps each other name, so a reload keeps the state.
- The word of the conductor wraps the chain of the orchestrator, and the engine wraps no chain.
- The engine stays under the wall of 6500 (R3). PR 6 and PR 7 each fit it, or go back to the owner with the cost.

# The names and the laws of the engine

## Technical names

Each name has one meaning, which the contract's sentences use as given here.

`AGENTS.md` at the root gives the meanings of fact, question, act, owner, started, query and view, under The
meanings. The entry of such a name here gives only its form in the engine.

- engine: the program in engine.py, which is the system prompt of a model.
- operator: the person or program that calls the engine from outside a rung.
- outside: a host that makes acts under a site of its own, which is neither the operator nor an act; an act of the
  outside is one that neither the operator nor an act made.
- model: a language model that answers with python code.
- provider: the service that runs a model.
- actor: the operator, or a model at an effort, named model/effort; it answers prompts and makes acts.
- effort: one of the levels of reasoning the World offers, ordered from least to most.
- roster: the actors the World offers, each as its name, its efforts and its window.
- window: the context size of a model in tokens, as its roster entry says.
- share: the part of a window that one model response used.
- standing: what the chains stand on, which the World answers a stand with: the roster, the directory, the default actor
  and the path of the doctrine, each path with `~` in place of the home of the user, as every path that the World writes
  for a model; its one home is the transcript of the root, where the done of the last stand holds it. A chain tells no
  standing of its own: a stand that a word makes tells the roster and the actor, and a cd tells the directory, and each
  chain whose standing names a doctrine plays both on itself, just before its open and again when a later life stands on
  another standing. The engine reads the directory of the standing nowhere, and the World resolves against it the paths
  of a chain that ran no cd.
- stand: the act that asks the World for the standing, which boot makes on the root as a life opens and again at
  the tip of a later life, and which tells nothing then. A stand that a word makes tells its answer in a paragraph
  headed by its id, which binds the roster and the actor. When the standing at the tip of a later life differs from
  the one before it, each chain whose standing names a doctrine plays `stand()` on itself, as a rung with that word
  and its default actor, and `cd('<directory>')` after it in the same word when the directory changed and no rung
  with no actor, as a word of a model or of the operator, ran a cd on it: a cd that a stand of a chain made is no cd
  of its own.
- tip: the point of a later life where the journal has said the record again whole, at which boot stands the life
  again, and after which the life goes on past it.
- catch-up: what the official extension catchup does on each chain at each stand, which `extensions/catchup/catchup.pyi`
  says.
- default actor: the actor a prompt goes to when the prompt leaves the actor unsaid, bound as `actor` in the chain.
- World: the ears of the outside that serve the machine, one or many, which reach the disk, the machine, the actors,
  the operator and the record; they answer what is theirs to answer, take what takes time, a command, a wait, a
  prompt to the operator and a reply, and say each done when it ends, and they keep what the journal says to keep.
- Kernel: the ear of the outside that runs rungs: it takes a run, makes a wants as the run while the word waits for
  an act that is not done, and says the run done with what the word gave.
- gate: the ear of the outside that answers a gate, apart from the Kernel, so a word may ask it while the Kernel
  runs that word; what it reads with is its own. It reads the word after the program of its chain, and it finds
  nothing when the word may run.
- boot: the life, given the record and the generators of the outside, which binds say, act, drive, get, peek and
  transcript, and gives the root.
- bus: what boot binds for every verb to speak through: say for a fact, act for an act, and drive for a generator;
  it keeps what each ear waits for, and gives each fact to the ears that wait for it.
- fact: a tuple of its kind, the act it is about, who said it, and its words.
- site: who is speaking, which every fact is said from: the generator while it speaks, the run while it is
  stepped, the operator otherwise.
- kind: the first slot of a fact; for a question, the verb that made it.
- question: a fact that is asked, whose first word is the chain or the thread it is on.
- act: a question, which is over at its done.
- life: one run of the engine, which boot opens and a second boot ends; every act, name and outcome is of it.
- ear: a generator that hears the facts of the life and speaks by yielding one; the ear of an act is given the name
  of the act, and a verb makes it with controlled, so the controls over the act reach it. An ear hears each fact that
  it waits for, and every control; an ear of the engine hears each act that it waits for, and an ear of the outside,
  the World, the Kernel and every other generator that boot is given, hears an act only while no ear before it took
  it, but a chain, which it hears at its birth.
- wait: what an ear says it waits for when it yields no saying: a set of names, so it hears each fact that names one
  of them by its kind, its act, or the kind, the maker or the chain of that act, or the chain of the maker of that
  act; or nothing, so it hears every fact.
- done: the fact that settles the act it names with what the act came to.
- outcome: what an act came to, held under its name once its done lands: the value or the exception.
- id: the name of an act, as a string: its kind and its number among the acts of that kind, as bash1 or read3,
  which python can bind.
- maker: the one that made a question, which its fact says as who said it; for a rung that retells, the rung it
  retells.
- lineage: the act and its makers, one under the other, which the life holds and lineage(name) walks, up to the
  operator or an ear of the outside; under and covers read it, and chains form a tree by maker, from the operator.
- under: what an act is to its ancestors.
- scope: the chain an act is on, or the scope of the thread it is on, and itself for a chain.
- root: chain1, which boot opens.
- chain: an act, chainN, that holds the facts on it, with a module and a working directory of its own;
  with a source it holds, besides, the acts it inherited.
- origin: the chain that a chain with a source stands on.
- source: the chain a new chain retells, by its id, which means the transcript of that chain as it stands with the
  filter applied.
- filter: a callable that says which acts of the transcript up to the source the turns of the new chain keep; take
  makes the filter of the file.
- transcript: the facts on a chain, in the order of the log, which boot adds as they are said; it is the whole state
  of the chain, and its module, its program, its working directory and its turns are read off it. The view gives a
  new list of them at each call, and the view of a kind or of a speaker gives the facts on the chain of that kind
  or that it said, which boot keeps as they are said.
- module: the fact that carries the globals of a chain, which the chain says at its birth and at each replay.
- prefix: the fact that carries what a chain with a source holds of the transcript of its origin, which the chain
  says at its birth, before its open; the transcript of the chain holds those facts where the prefix stands.
- carried turn: a fact on a chain about a name that no act of the life holds, as a turn that the chain took from
  another record or from a checkpoint; a chain with a source holds it whole.
- note: one thing a tell says: a paragraph, or the binding of an act.
- paragraph: what one fact that tells stands as in a turn: its notes, with a blank line between two paragraphs.
- header: the first line of a paragraph: # and, with no space, the id of the act it is of, then what happened to
  the act, or the maker of a rung whose word it tells, and no text that the act tells.
- advance: the paragraph the chain tells last before a reply, whose header names the rung it asks for and the prompt
  that rung advances, and which binds the share that the last reply read.
- binding: a statement that binds the name of an act to the act, or a value that the act tells to the id of the act
  and the word of the value, as bash1_command, which a turn shows and a rung the chain writes runs.
- tell: the one fact that carries notes, about the act it is of, which the turns are folded from; a control tells
  its header by one, and the journal keeps a tell only when no act said it.
- turn: one item of what a model reads of a transcript, folded from the tells: a role, python, a usage and blocks.
- prompt: the act that sends a message to an actor and wants a response of a shape.
- message: the text a prompt carries.
- shape: the python type of a response, which a prompt carries by its name.
- response: what a prompt completes with.
- acknowledgment: the prompt of nothing a chain makes when an act a rung made is done, or the chain of a prompt that
  a rung made pauses, and no reply has shown it, on the thread that made that rung, so it opens that thread again.
- thread: a prompt and the prompts on it, in order, each of which names the one before it as its on; a prompt of
  None on a thread that is open is a message of it, and any other prompt on it is answered in steps, and opens it
  again when it closed.
- rung: the run of one word on a chain, and the verb that makes it: with a word its caller wrote, or with no word,
  for a turn of a model.
- reply: the act a chain makes, under the site of a rung, to ask the model of that rung for its word, which the
  World takes and answers with the turn of the model.
- run: the act a chain makes to run the word of a rung, which the Kernel takes and says done with what the word
  gave.
- writing: the fact the World says of the part of the word of a rung that its model finished, while the model
  writes the rest, which the Kernel hears and runs as that rung. It is no act, since the World that awaited one would
  stand in its own call while the part prompts a chain that asks the World for a reply.
- ran: the fact the Kernel says, as a rung, once the part of a writing of that rung is over, with what the part
  raised, which the World hears to end a reply at a part that raised.
- wants: the act a run makes when its word waits for an act that is not done, which the rung takes and answers
  with what that act came to.
- step: one turn of a model and the run of its word, which is one rung.
- ladder: the rungs of one prompt, in order, which the name of that prompt is the door of.
- program: the words of rungs, in order, each under the name of its rung; the program of a ladder is the words of
  its rungs, the words the gate refused among them, which its door shows, and the program of a chain is the words
  of its runs since its last module, as python, which the gate reads a word after.
- replay: the making of the rungs of a chain again from a donor, which is how a chain with a source stands on its
  origin and how a write of a door edits the program of a ladder.
- donor: the rungs a replay retells, and, for one rung, the rung of the record it stands for, whose acts it shares
  and whose run a Kernel may answer from what it kept.
- globals: python's globals, the dict a rung runs in, which is the engine's module copied for the chain and carried
  by its module.
- command: what bash runs on the machine.
- merged: the state of a command whose stderr flows into its stdout, in the order the command wrote them.
- door: the name of an act, with a part after it or without; the name of a prompt is the door of its ladder, and the
  name of a command with /stdout, /stderr or /stdin after it is a door of that command: the chain of a read serves it
  while the command runs and after it, and the World serves a write of it; the word of a rung adds a door by making an
  act whose ear answers reads of a scheme, and the World serves the rest.
- text: what read gives, and what write is given and gives back as it landed; a Text, with a path and a content.
- show: a callable given the lines of a text, which gives the numbers of the lines the engine tells; span, grep and
  differs make the shows of the file.
- quote: a string between two marks, <s:name> and </s:name>, which binds its name when it runs; a word writes one,
  and the transcript tells each string of more than one line as one.
- doctrine: prose that says how a model works in furb, and holds no law: `DOCTRINE.md` at the root. The standing names
  the file that a chain reads: `.furb/DOCTRINE.md` of the directory when it holds one, the `DOCTRINE.md` of the checkout
  of git that furb was built from in dev mode, else the official copy, `~/.furb/DOCTRINE.md`, which furb writes from
  the doctrine that it carries. The World reads it in the first turn of the setup of the root.
- crafted rung: a rung with a word and an actor, a turn of that actor that its caller wrote; the crafted rungs of a
  chain take their turns one at a time, in the order they were made, a rung that one of them makes takes its turn
  within the turn of its maker, and every other word of the chain and every prompt to a model wait until none is open.
- setup: the turns that the engine crafts on the root of a new record at its first stand, after the bare turn
  `chain()`, from the standing alone, in `seeded()` of `engine.py`: the World gives in the standing, after the path of
  the doctrine, each extension that the life runs as `[name, file, life word, root alone]`, and the root, when it holds
  no reply, makes crafted rungs of its default actor: the read of the doctrine, the turn
  `rung(read('<file>', HIDDEN).content)` of each extension of every chain that has a file, in the order of the life,
  whose rung tells the code of the file alone in the turn after it and runs that code in the module of the root, the
  file being the copy of the word in `~/.furb/extensions/<name>/`, then one turn of their life words, which read the
  place against the directory of the standing, as every chain that ran no cd, then the fact setup,
  `("setup", "chain1", "chain1")`, then the same turns for the extensions of the root alone. No ear makes a rung of the
  setup, so the transcript shows each word that ran, and the record holds only the standing that the World gave. Then
  the root plays on itself one turn `stand()` and `cd('<the directory of the standing>')`, a rung with that word and
  its default actor, just before its open, as each chain with no source does at its birth, after the setup that it
  holds. The reads of the files come first, so their turns are the same text whatever the place and the actors, then
  the life words, so their turns are the same text in one place whatever the actors, and a provider caches them across
  records. The engine makes each of these rungs at once, so the root plays its stand after the last of them and tells
  its open after its stand, in the same turn, just before its first prompt. Each chain with no source holds the setup
  of the root as its prefix, with the same names, less the acts that the World, the operator or the root itself made
  after the fact setup, as the stand of the root, and each act under one, and runs its words again, so every chain
  shares one prefix, which the provider ends with a block that it marks for the cache. The fact teach,
  `("teach", "chain1", "extensions", actor, doctrine, count)`, which the World says when the engine seeded the root,
  before the count of enables, tells the ear of the extensions that the setup holds them, so it plays them on no chain.
  A standing that gives no extensions, as that of a World that names no doctrine, as the World of the suite, seeds no
  setup: the root tells its open at its first stand, the extensions play as rungs of the ear, and its chains run no
  cd, so the World resolves their paths against the directory of the standing. A later life replays the setup from
  the standing that the record holds.
- template: a python template string, each interpolation of which carries an expression and its value.
- debug: the verb that tells the interpolations of a template, a tell and no act.
- control: a pause, a wake, a cancel or a close; a close is a cancel with a value, and a cancel a close with
  CancelledError.
- paused: the state of an act while a pause over it stands with no wake after it.
- grant: an act that puts a ceiling on a chain: dollars, a share of the window, or both.
- ledger: what the grant holds: the dollars of the answers since it was made and the share the last one filled.
- journal: the ear of the life that hears everything and says a keep for each entry of the record.
- record: the entries the journal keeps, in the order they were said.
- scratch directory: the directory of the machine that the World keeps for a record, which every chain of the record
  shares, which a path of scratch:// names and a command reads as TMPDIR; the opening of a life takes it, and makes
  its files again from the record when it is missing.
- entry: one line of the record: one fact, an act among them.
- plain: the form of a value on the wire, which the World may keep an entry as: nothing, a boolean, a number, a
  string, a list of plain, or a table from a string to plain.
- drift: an act that, made again, does not agree with the record.
- pending: what the record shows started and not done when boot returns: a command, a wait, a prompt to the operator
  or a reply that the outside took, which the life holds with no fact until a wake that the life says.
- extension: python that extends the engine from a rung: its word, which defines its names, and, for one that a host
  gives a life, its life word, which starts its work; an official one has an ear of the World that answers the
  questions that its word defines.
- enable: the fact by which the World turns an extension on in a life, with its name, its word, its life word and, for
  an extension that plays on the root alone, root, which the record keeps; the World says it at the tip, and when a
  write of the config of the project lands that turns on an extension by its path, and it plays the extension as a rung
  on every chain there is where it plays when it hears the fact, and after it on each chain at its birth where it plays,
  before the call that made the chain returns. An extension plays on every chain, or on the root alone and on each chain
  whose source had it. The World says it again for an extension that the life runs when its word changed: at a stand,
  for the word that the host gives, and when a write of the life lands in the folder of an extension that a config turns
  on. That fact reloads the extension: each chain that has the extension plays the definitions of the new word, its
  imports, functions, classes, types and names in capitals, and each name of state that it assigns, as
  `name = globals().setdefault("name", value)`, so a new name takes its value, each name that the chain holds keeps its
  value, and the life word does not run again. An extension whose word is blank, as one that holds only a part for
  the TUI, plays no word: no turn of its file in a setup, and no rung on a chain.
- made: the fact by which the store names in the record the build that made it, the digest of the engine and of the
  Kernel and the version of the build, `VERSION` in `src/world/store.rs`, which the store says as it hears the root of
  a new record at its birth. A life on a record that names another version is refused before any replay, since a
  build of that version can make other acts from the same record. The command line of each version puts its program
  in the store of builds of the machine. A life that keeps a record whose last segment an older version made is
  upgraded as it opens: the program of that version from the store writes a checkpoint at the tip of the record, and
  the life goes on from it on this build. A command on a record of a newer version runs the program of that version in
  its place when the store holds it. A record of a build before the versions names an
  engine alone, and a life on another engine refuses it. A build keeps the version while each record of the corpus,
  `tests/corpus`, replays on it with no drift. After a checkpoint, only the fact made of the segment after it counts,
  and the store says it again at the root of the first life that goes on from the checkpoint.
- checkpoint: the fact by which a record carries the state of its life to a later life of any build, at its tip:
  the facts that the journal of any engine replays, of each chain that lives, its turns, its module and its open
  prompts, and the names that each kind took. `src/checkpoint.rs` holds its shape. A later life boots on these facts
  and on the record after the checkpoint, and replays nothing before it. Each name of the past stays taken, an act
  that a value names keeps its name and its answer, each chain with an open prompt opens paused, and its prompt asks
  its model again at a wake. A name of the module whose value the record cannot hold is lost, and the chain is told.
- life word: the python of an extension that its rung runs after its word, and that the World plays alone on a chain
  whose origin had the extension, since that chain made the rung of its origin again. In a setup, the life words of
  the extensions run as one turn of the default actor, after the turns of their files.

## Laws no test holds

- A model does everything with the python that the model writes.
- The engine hosts itself.
- The system prompt a model reads is the engine, minified with its program kept as it is, and nothing else.
- The engine phrases everything that a model reads, as python, and the World renders nothing.
- Everything that the file defines is public, and what is in engine.py is the API, the same for the model and for the
  operator.
- The defaults of every verb are public names in the file.
- A model knows from the source alone what the engine does with what the model writes.
- What the engine does to the word of a rung it does to itself.
- The prompt is the one prose channel of the engine: every exchange of prose between the operator, the models and a
  chain is a prompt.
- A model extends the engine from a step: an extension writes verbs and ears, shows and filters as callables, and
  rebinds names, and it tells notes of its own, from a door or from a rebound verb.
- The engine has no registry, no plugin surface, no permission and no REPL: the record shows who made each act.
- Within a chain, a rung binds, passes and returns any python value, plain or not.
- When its share of the window is high, a model opens a chain with a source and a take that is not inside.
- Compaction is not in the core.
- engine.pyi is the specification of engine.py: it says what the engine is, what is always true of it and what its
  surface is, and never how it is made; it names the ears and the facts as python reads them, and holds every law, so
  engine.py holds no sentence and no comment.
- engine.py depends only on the python interpreter and on the ears of the outside that boot is given, the World, the
  Kernel and the gate among them, which are held by no verb and bound to no name of the engine.
- A fact is a tuple, its kind first, deconstructed only by match, and nothing of the engine is a class but a text, an
  exit, the name of an act, and the two exceptions.
- The verbs and Text carry their signatures, typed, since a model reads them, but a parameter whose default says its
  type, as on="", carries no annotation; nothing else in engine.py has a type annotation but what ty and a dataclass
  need.
- engine.py has no private names, and a name one thing alone uses lives inside it.
- Each technical name has one meaning.
- ty check passes on engine.py.
- There is one root per record, and one process at a time owns a record.
- A chain is a function of the record, and a later boot must offer the same interpreter and the same outside, since a
  record is made again by running its words, or loaded from the snapshot that a run of those words made on the same
  record, interpreter and outside, which the opening of the life checks against the record. A record names the
  version of the build that made it, and a life on a build of another version is refused before any replay.
- A chain is one conversation, and the transcript that a provider caches grows at one end.
- Nothing is told twice, and nothing tells what it does not know: an act tells of itself, and a chain of the rungs it
  asks a model for, with the message of the prompt each advances, of a word that told nothing, of the prompts and the
  words it refuses, and of the done of an act that one of its rungs made on another chain, or of the pause of the
  chain of such a prompt.
- A recoverable error is one a model can correct on another turn: a cut turn, context overflow, transient provider failure, or refusal of its word. The provider gives the next turn the cause, and never pauses the chain for these errors. It retries transient failures within its backoff first. A failed setting, missing model, unsupported input or a fault of the host is not recoverable; do not retry it as a model error.
- A drift is a hard error.
- A change that makes room under the wall and rewrites the code of a fix names that fix, and shows that the test
  of the fix fails on the old form of its cause, since a test that does not hold the failure lets a later change undo
  the fix.
- The engine owns the order of every run, and nothing of a model runs on the loop of the outside.
- A chain with a source reads the transcript of its origin as it stands, where the record is what a later life should
  make it from.
- An extension enters a life as rungs that the World plays, and never enters the module of the engine or the system
  prompt, so a chain that takes one grows at its end.
- A life runs every extension that its record enables, whatever the configs say then, since a record is made again by
  running its words; the World enables at the tip each extension that the host gives it and the record does not hold
  as the host gives it, so a new word of an extension that the life runs reloads it there.
- An ear that makes acts makes each one while it hears the fact that causes it, from what it heard alone and never
  from the disk, so a later life makes it again at the same place; what the disk holds reaches a chain through a
  question that the World answers and that the journal answers again.
- The word of an extension only defines, holds no comment, and binds no name of the engine, and it holds no sentence but
  one doctrine of its own, a quote that holds no law, as furb holds its own in DOCTRINE.md; its contract holds its laws.

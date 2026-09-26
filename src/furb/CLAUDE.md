# The names and the laws of the engine

## Technical names

Each name has one meaning, which the contract's sentences use as given here.

- engine: the program in engine.py, which is the whole system prompt of a model.
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
- standing: what the chains stand on, which the World answers a stand with: the roster, the directory and the default
  actor; its one home is the transcript of the root, where the done of the last stand holds it, and a chain tells
  each standing it takes in one paragraph, under the headers roster, cwd and actor.
- stand: the act that asks the World for the standing, which boot makes on the root as a life opens and again at
  the tip of a later life.
- tip: the point of a later life where the journal has said the record again whole, at which boot stands the life
  again, and after which the life goes on past it.
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
- bus: what boot binds for every verb to speak through: say for a fact, act for an act, and drive for a generator.
- view: a read of the life that makes nothing and keeps nothing: get, peek and transcript, and every read derived
  from them.
- fact: what is said to the life, inert, with no response, which every ear hears in the order of the log; a tuple
  of its kind, the act it is about, who said it, and its words.
- site: who is speaking, which every fact is said from: the generator while it speaks, the run while it is
  stepped, the operator otherwise.
- kind: the first slot of a fact; for a question, the verb that made it.
- question: a fact that is asked, which is an act: it takes a name when it is said, its first word is the chain it
  is on, and what it came to the life holds under its name.
- act: a question, answered now or later, which goes to the ears in turn until one owns it, and is over at its
  done.
- owner: the ear that took an act by saying a started or a done about it, which answers it.
- started: the fact by which an ear takes an act whose done comes later.
- query: an act that no ear started, which its owner answers with a done at once.
- life: one run of the engine, which boot opens and a second boot ends; every act, name and outcome is of it.
- ear: a generator that hears the facts of the life and speaks by yielding one; the ear of an act is given the name
  of the act, and a verb makes it with pausing and ending. An ear of the engine hears every fact and every act; an
  ear of the outside, the World, the Kernel and every other generator that boot is given, hears every fact, and an
  act only while no ear before it took it.
- done: the fact that settles the act it names with what the act came to.
- outcome: what an act came to, held under its name once its done lands: the value or the exception.
- id: the name of an act, as a string: its kind and its number among the acts of that kind, as bash1 or read3,
  which python can bind.
- maker: the one that made a question, which its fact says as who said it; for a rung that retells, the rung it
  retells.
- lineage: the makers of an act, one under the other, which the life holds and under walks; chains form a tree by
  maker, from the operator.
- under: what an act is to its ancestors.
- scope: the chain an act is on, and itself for a chain.
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
  new list of them at each call.
- module: the fact that carries the globals of a chain, which the chain says at its birth and at each replay.
- prefix: the fact that carries what a chain with a source holds of the transcript of its origin, which the chain
  says at its birth, before its open; the transcript of the chain holds those facts where the prefix stands.
- note: one thing a tell says: python as it stands, or a text and its show.
- paragraph: what one fact that tells stands as in a turn: its notes, with a blank line between two paragraphs.
- header: the first line of a paragraph: # and, with no space, the id of the act it is of, or the kind of a query,
  whose id no chain binds, then its words.
- advance: the header the chain tells last before a reply, which names the rung it asks for and the prompt that
  rung advances.
- binding: a statement that binds the name of an act to the act, which a turn shows and a rung the chain writes
  runs.
- tell: a fact that carries notes about the act it is of, which the turns are folded from and the journal keeps
  none of.
- turn: one item of what a model reads of a transcript, folded from the tells: a role, python, a usage and blocks.
- known: a line that a model was told, in the transcript of its chain.
- prompt: the act that sends a message to an actor and wants a response of a shape.
- message: the text a prompt carries.
- shape: the python type of a response, which a prompt carries by its name.
- response: what a prompt completes with.
- acknowledgment: the prompt of nothing a chain makes when an act a rung made is done and no reply has shown it.
- rung: the run of one word on a chain, and the verb that makes it: with a word its caller wrote, or with no word,
  for a turn of a model.
- reply: the act a chain makes, under the site of a rung, to ask the model of that rung for its word, which the
  World takes and answers with the turn of the model.
- run: the act a chain makes to run the word of a rung, which the Kernel takes and says done with what the word
  gave.
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
- door: the name of an act, with a part after it or without; the name of a prompt is the door of its ladder, the
  word of a rung adds a door by making an act whose ear answers reads of a scheme, and the World serves the rest.
- text: what read gives, and what write is given and gives back as it landed; a Text, with a path and a content.
- show: a callable given the lines of a text, which gives the numbers of the lines the engine tells; span, grep and
  differs make the shows of the file.
- quote: a string a word writes between two marks, <Sn> and </Sn>, which binds Sn when the word runs.
- template: a python template string, each interpolation of which carries an expression and its value.
- debug: the verb that tells the interpolations of a template, a tell and no act.
- control: a pause, a wake, a cancel or a close; a close is a cancel with a value, and a cancel a close with
  CancelledError.
- paused: the state of an act while a pause over it stands with no wake after it.
- grant: an act that puts a ceiling on a chain: dollars, a share of the window, or both.
- ledger: what the grant holds: the dollars of the answers since it was made and the share the last one filled.
- journal: the ear of the life that hears everything and says a keep for each entry of the record.
- record: the entries the journal keeps, in the order they were said.
- entry: one line of the record: one fact, an act among them.
- plain: the form of a value on the wire, which the World may keep an entry as: nothing, a boolean, a number, a
  string, a list of plain, or a table from a string to plain.
- drift: an act that, made again, does not agree with the record.
- pending: what the record shows started and not done when boot returns: a command, a wait, a prompt to the operator
  or a reply that the outside took, which the life holds with no fact until a wake that the life says.

## Laws no test holds

- A model does everything with the python that the model writes.
- The engine hosts itself.
- The system prompt a model reads is the engine, minified in layout alone, and nothing else.
- The engine phrases everything that a model reads, as python, and the World renders nothing.
- Everything that the file defines is public, and what is in engine.py is the API, the same for the model and for the
  operator.
- The defaults of every verb are public names in the file.
- A model knows from the source alone what the engine does with what the model writes.
- What the engine does to the word of a rung it does to itself.
- The prompt is the one channel of the engine: every exchange between the operator, the models and a chain is a prompt.
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
- The verbs and Text carry their signatures, typed, since a model reads them; nothing else in engine.py has a type
  annotation but what ty and a dataclass need.
- engine.py has no private names, and a name one thing alone uses lives inside it.
- Each technical name has one meaning.
- ty check passes on engine.py.
- There is one root per record, and one process at a time owns a record.
- A chain is a function of the record, and a later boot must offer the same interpreter and the same outside, since a
  record is made again by running its words.
- A chain is one conversation, and the transcript that a provider caches grows at one end.
- Nothing is told twice, and nothing tells what it does not know: an act tells of itself, and a chain of the rungs it
  asks a model for, with the message of the prompt each advances, and of the prompts and the words it refuses.
- A drift is a hard error.
- The engine owns the order of every run, and nothing of a model runs on the loop of the outside.
- A chain with a source reads the transcript of its origin as it stands, where the record is what a later life should
  make it from.

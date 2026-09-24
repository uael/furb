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
- standing: what a chain stands on, which the World answers it, or its origin for a chain with a source: the roster,
  the directory and the default actor; the record holds each standing a chain took, at its place, and the chain
  tells it in one paragraph, under the headers roster, cwd and actor.
- tip: the point of a later life where the journal has said the record again whole, after which the life goes on
  past it.
- stood: the fact that says what a chain stands on from its place in the record on, which the journal says at the
  tip for a chain whose standing the World changed.
- default actor: the actor a prompt goes to when the prompt leaves the actor unsaid, bound as `actor` in the chain.
- World: the ear of the outside that boot takes under the name WORLD, which reaches the actors and the record, and
  the disk and the machine through the parts of the extensions; it keeps what the journal says it keeps, does what
  it hears it is started to do, and answers a question of an extension with plain data.
- Kernel: the ear of the outside that runs rungs: it hears a run, and says wants and ran.
- gate: the ear of the outside that answers a gate, apart from the Kernel, so a word may ask it while the Kernel
  runs that word; what it reads with is its own. It reads the word after the program of its chain, and it finds
  nothing when the word may run.
- boot: the life, given the record and the generators of the outside, which gives the root.
- bus: the three entries every verb speaks through: send for a fact, ask for a query, act for an act.
- fact: what is said to the life, inert, with no response, and queried later; a tuple of its kind, the act it is
  about, who said it, and its words.
- site: who is speaking, which every fact is said from: the generator while it speaks, the run while it is
  stepped, the operator otherwise.
- kind: the first slot of a fact; for a question, the verb that made it.
- question: a fact that is asked; it takes a name when it is said, its first word is the chain it is on, and what
  it was answered the life holds under its name; an act if it lives, a query if it does not.
- query: a synchronous question, answered now or with nothing.
- act: an asynchronous question, answered now or later, which an ear lives, opened by the call and done by its done.
- life: one run of the engine, which boot opens and a second boot ends; every act, name and outcome is of it.
- ear: a generator that hears every fact of the life and speaks by yielding one; the ear of an act is given the
  name of the act, and a verb makes it with pausing, ending and started; the World, the Kernel and every other
  generator that boot is given from the outside are ears too.
- done: the fact that settles the act it names, or answers the question it names.
- outcome: what a question came to, held under its name once its done lands: the value or the exception.
- id: the name of a question, as a string. An act is its kind and its number among the acts of that kind, as
  bash1, which python can bind; a query is its kind, @, its maker and its number among what that maker made, as
  read@rung1.2.
- maker: the one that made a question, which its fact says as who said it; for a rung that retells, the rung it
  retells.
- lineage: the makers of an act, one under the other, which the life holds and under walks; chains form a tree by
  maker, from the operator.
- under: what an act is to its ancestors.
- scope: the chain an act is on, and itself for a chain.
- root: chain1, which boot opens.
- chain: an act, chainN, that holds the facts on it, with a module of its own; with a source it holds, besides,
  the acts it inherited.
- origin: the chain that a chain with a source stands on.
- source: the chain a new chain retells, by its id, which means the transcript of that chain as it stands with the
  filter applied.
- filter: a callable that says which acts of the transcript up to the source the turns of the new chain keep; take
  makes the filter of the file.
- transcript: the facts a chain holds, in the order it heard them.
- note: one thing a tell says: python as it stands, or a showing.
- showing: a note of a path, a content and a show, which the chain tells by the lines of the content that the model
  has not seen.
- paragraph: what one fact that tells stands as in a turn: its notes, with a blank line between two paragraphs.
- header: the first line of a paragraph: # and, with no space, the id of the act or the kind of the query it is
  of, then its words.
- advance: the header the chain tells last before an ask, which names the rung it asks for and the prompt that
  rung advances.
- binding: a statement that binds the name of an act to the act, which a turn shows and a rung the chain writes
  runs.
- tell: a fact that carries notes about the act it is of, which the turns are folded from and the record keeps
  none of.
- turn: one item of what a model reads of a transcript, folded from the tells: a role, python, a usage and blocks.
- known: a line that a model was told, in the transcript of its chain.
- prompt: the act that sends a message to an actor and wants a response of a shape.
- message: the text a prompt carries.
- shape: the python type of a response, which a prompt carries by its name.
- response: what a prompt completes with.
- acknowledgment: the prompt of nothing a chain makes when an act a rung made is done and no ask has shown it.
- rung: the run of one word on a chain, and the verb that makes it: with a word its caller wrote, or with no word,
  for a turn of a model.
- step: one turn of a model and the run of its word, which is one rung.
- ladder: the rungs of one prompt, in order, and the query that the chain of that prompt answers with their
  program, or answers by a replay with a program it is given.
- program: the words of rungs, in order, each under the name of its rung; the program of a ladder is the words of
  its rungs, the words the gate refused among them, which the ladder query answers, and the program of a chain is
  the words of every rung that runs, as python, which the gate reads a word after.
- replay: the making of the rungs of a chain again from a donor, which is how a chain with a source stands on its
  origin and how a ladder given a word edits the program of that ladder.
- donor: the rungs a replay retells, and, for one rung, the rung of the record it stands for, whose acts it shares
  and whose run a Kernel may answer from what it kept.
- globals: python's globals, the dict a rung runs in, which is the engine's module copied for the chain.
- show: a callable given the lines of the content of a showing, which gives the numbers of the lines the chain
  tells.
- extension: a word that a host plays on a chain as a rung, as the World, with a part for the World and a part for
  the TUI that the host holds; the engine knows no extension, and `src/furb/builtin/CLAUDE.md` holds the names of the
  builtin extensions.
- quote: a string a word writes between two marks, <Sn> and </Sn>, which binds Sn when the word runs.
- template: a python template string, each interpolation of which carries an expression and its value.
- debug: the verb that tells the interpolations of a template, a tell and no act.
- control: a pause, a wake, a cancel or a close; a close is a cancel with a value, and a cancel a close with
  CancelledError.
- paused: the state of an act while a pause over it stands with no wake after it.
- journal: the ear of the life that hears everything and says a keep for each entry of the record.
- record: the entries the journal keeps, in the order they were said.
- entry: one line of the record: the fact, and for a query of a run its answer beside.
- plain: the form of a value on the wire, which the World may keep an entry as: nothing, a boolean, a number, a
  string, a list of plain, or a table from a string to plain.
- drift: an act that, made again, does not agree with the record.
- pending: what the record shows begun and not done when boot returns: a wait, a prompt to the operator or an act
  of an extension that the World started, or a rung that its chain asked for; it waits for a wake that the life
  says.

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
  rebinds names, and it tells notes of its own, from an act it makes or from a rebound verb.
- The engine has no registry, no plugin surface, no permission and no REPL: the record says who made each act.
- Within a chain, a rung binds, passes and returns any python value, plain or not.
- When its share of the window is high, a model opens a chain with a source and a take that is not inside.
- Compaction is not in the core.
- engine.pyi is the specification of engine.py: it says what the engine is, what is always true of it and what its
  surface is, and never how it is made; it names the ears and the facts as python reads them, and holds every law, so
  engine.py holds no sentence and no comment.
- engine.py depends only on the python interpreter and on the ears of the outside that boot is given, the World, the
  Kernel and the gate among them, which are held by no verb and bound to no name of the engine.
- A fact is a tuple, its kind first, deconstructed only by match, and nothing of the engine is a class but the name
  of an act and the two exceptions.
- The verbs carry their signatures, typed, since a model reads them; nothing else in engine.py has a type
  annotation but what ty and a dataclass need.
- engine.py has no private names, and a name one thing alone uses lives inside it.
- Each technical name has one meaning.
- The engine knows no extension: a host plays the word of each one as a rung, as the World, on every chain without
  a source, once the life stands on its record and at the birth of each such chain.
- The word of a module is the module with each line of a top-level import from furb made an empty line.
- ty check passes on engine.py.
- There is one root per record, and one process at a time owns a record.
- A chain is a function of the record, and a later boot must offer the same interpreter and the same outside, since a
  record is made again by running its words.
- A chain is one conversation, and the transcript that a provider caches grows at one end.
- Nothing is told twice, and nothing tells what it does not know: an act tells of itself, and a chain of the rungs it
  asks for, with the message of the prompt each advances, and of the prompts and the words it refuses.
- A drift is a hard error.
- The engine owns the order of every run, and nothing of a model runs on the loop of the outside.
- A chain with a source still asks its origin for its transcript, where the record is what a later life should make it
  from.

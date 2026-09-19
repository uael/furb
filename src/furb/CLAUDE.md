## Technical names

Each name has one meaning, which the contract's sentences use as given here.

- engine: the program in engine.py, which is the whole system prompt of a model.
- operator: the person or program that calls the engine from outside a rung.
- model: a language model that answers with python code.
- provider: the service that runs a model.
- actor: the operator, or a model at an effort, named model/effort; it answers prompts and makes acts.
- effort: one of the levels of reasoning the World offers, ordered from least to most.
- roster: the actors the World offers, each as its name, its efforts and its window.
- window: the context size of a model in tokens, as its roster entry says.
- share: the part of a window that one model response used.
- standing: what a chain stands on, which the World answers it: the roster, the directory and the default actor.
- default actor: the actor a prompt goes to when the prompt leaves the actor unsaid, bound as `actor` in the chain.
- World: the interface to the disk, the machine, the actors and the record; it keeps what the journal says it keeps,
  and does what it hears it is started to do.
- Kernel: the interface that gates and runs rungs, and what it gates with is its own.
- boot: the life, given the record and the generators of the outside, which gives the root.
- bus: the three entries every verb speaks through: send for a fact, ask for a query, act for an act.
- fact: what is said to the life, inert, with no response, and queried later; a tuple of its kind, the act it is
  about, who said it, and its words.
- kind: the first slot of a fact; for a question, the verb that made it.
- question: a fact that is asked; it takes a name when it is said, its first word is the chain it is on, and what
  it was answered the life holds under its name; an act if it lives, a query if it does not.
- query: a synchronous question, answered now or with nothing.
- act: an asynchronous question, answered now or later, which a life lives, opened by the call and done by its done.
- life: the generator of an act, given its name, which hears every fact and speaks by yielding one; a verb makes it
  with pausing, ending and started.
- outcome: what a question came to, held under its name once its done lands: the value or the exception.
- id: the name of a question, as a string, kind://lineage.n: its kind, the lineage of its maker, and its number
  among that maker's.
- maker: the one whose lineage the name of an act hangs off.
- lineage: the makers of an act, one under the other, which its name holds after its kind; chains form a tree by
  name, from the operator through their makers.
- under: what an act is to its ancestors.
- scope: the chain an act is on, and itself for a chain.
- root: chain://operator.1, which boot opens.
- chain: an act, chain://lineage.n, that holds the facts on it, with a module and a working directory of its own; with
  a source it holds, besides, the acts it inherited.
- source: the chain a new chain retells, by its id, which means the transcript of that chain as it stands with the
  filter applied.
- filter: a callable that says which acts of the transcript up to the source the turns of the new chain keep; take
  makes the filter of the file.
- transcript: the facts a chain holds, in the order it heard them.
- turn: one item of what a model reads of a transcript, folded from the tells: a role, a content, a usage and blocks.
- known: a line that a model was told, in the transcript of its chain.
- prompt: the act that sends a message to an actor and wants a response of a shape.
- message: the text a prompt carries.
- shape: the python type of a response, which a prompt carries by its name.
- response: what a prompt completes with.
- acknowledgment: the prompt of nothing a chain makes when an act a rung made is done and no ask has shown it.
- rung: the run of one word on a chain, and the verb that runs a word its caller wrote.
- step: one turn of a model and the run of its word, which is one rung.
- ladder: the rungs of a chain in record order.
- program: the words the rungs of a chain accepted, the model's and its caller's alike, in the order of acceptance.
- globals: python's globals, the dict a rung runs in, which is the engine's module copied for the chain.
- gate: what the Kernel reads a word against before it runs: the rungs of its chain before it and the shape it must
  give.
- command: what bash runs on the machine.
- merged: the state of a command whose stderr flows into its stdout, in the order the command wrote them.
- door: the name of an act, with a part after it or without; the word of a rung adds a door by making an act whose
  life answers reads of a scheme, and the World serves the rest.
- text: what read gives, and what write is given and gives back as it landed; a Text, with a path and a content.
- show: a callable given the lines of a text, which gives the numbers of the lines the engine tells; span, grep and
  differs make the shows of the file.
- template: a python template string, each interpolation of which carries an expression and its value.
- debug: the verb that tells the interpolations of a template, a tell and no act.
- control: a pause, a wake, a cancel or a close; a close is a cancel with a value, and a cancel a close with
  CancelledError.
- paused: the state of an act while a pause over it stands with no wake after it.
- grant: an act that puts a ceiling on a chain: dollars, a share of the window, or both.
- ledger: what the grant holds: the dollars of the answers since it was made and the share the last one filled.
- record: the entries the journal keeps, in the order they were said.
- entry: one line of the record: the act made last before its fact, and the fact; for a query of a run, its answer
  beside.
- drift: an act that, made again, does not agree with the record.

## Laws no test holds

A model does everything with the python that the model writes.
The engine hosts itself.
The system prompt a model reads is the engine, minified in layout alone, and nothing else.
The engine phrases everything else that a model reads, and the World renders any tag.
Everything that the file defines is public, and what is in engine.py is the API, the same for the model and for the operator.
The defaults of every verb are public names in the file.
A model knows from the source alone what the engine does with what the model writes.
What the engine does to the word of a rung it does to itself.
The prompt is the one channel of the engine: every exchange between the operator, the models and a chain is a prompt.
A model extends the engine from a step: an extension writes verbs and lives, shows and filters as callables, and rebinds names, and it tells tags of its own, from a door or from a rebound verb.
The engine has no registry, no plugin surface, no permission and no REPL: the record says who made each act.
Within a chain, a rung binds, passes and returns any python value, plain or not.
When its share of the window is high, a model opens a chain with a source and a take that is not inside.
Compaction is not in the core.
engine.pyi is the specification of engine.py: it says what the engine is, what is always true of it and what its surface is, and never how it is made; it names the interfaces and the facts as python reads them, and holds every law, so engine.py holds no sentence and no comment.
engine.py depends only on the python interpreter and on the two interfaces the contract names, World and Kernel, which are generators given to boot, held by no verb and bound to no name of the engine.
A fact is a tuple, its kind first, deconstructed only by match, and nothing of the engine is a class but a text, an exit, the name of an act, and the two exceptions.
The verbs and Text carry their signatures, typed, since a model reads them; nothing else in engine.py has a type annotation but what ty and a dataclass need.
engine.py has no private names, and a name one thing alone uses lives inside it.
Each technical name has one meaning.
ty check passes on engine.py.
There is one root per record, and one process at a time owns a record.
A chain is a function of the record, and a later boot must offer the same interpreter and the same outside, since a record is made again by running its words.
A chain is one conversation, and the transcript that a provider caches grows at one end.
Nothing is told twice, and nothing tells what it does not know: an act tells of itself, and a chain of the rungs it asks for and of the words it refuses.
A drift is a hard error.
The engine owns the order of every run, and nothing of a model runs on the loop of the outside.
A chain with a source still asks its origin for its transcript, where the record is what a later life should make it from.

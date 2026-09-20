# furb

The engine is `src/furb/engine.py`, one file, which depends only on the python interpreter and on two interfaces it
declares, World and Kernel. It is derived from the contract, `src/furb/engine.pyi`, and proved by the suite in
`test/`. `src/furb/CLAUDE.md` holds the technical names of the engine and the laws that no test can hold, `src/CLAUDE.md`
says what proves the crate, and `script/CLAUDE.md` says how to run the DeepSWE rig.

## The contract

`src/furb/engine.pyi` is the source of truth. It gives the typed surface of the engine, and the docstring of each
definition holds the laws of that definition, one sentence per line, in Simplified Technical English.

Read it whole before you do any work. Never edit it without the approval of the owner: not a signature, not a
sentence, not a name. When you find a hole or a contradiction in it, say so and stop.

## The meanings

The owner is the source of truth of what a word means, and a name serves its meaning strictly. When a name of the
engine does not serve its meaning, the name is wrong, and the name changes.

- A fact is inert: a piece of information dropped into the life, with no response, that is queried later.
- A query is a synchronous question: it is answered now, or with nothing.
- An act is an asynchronous question: it is answered now, or later, and it is awaited for its answer.
- A fact with an answer makes no sense. The answer belongs to the question, and a question is a fact that is
  asked: it takes a name when it is said, and what it was answered the life holds under that name. A query and an
  act are the two kinds of question, side by side, and neither is a kind of the other.

## The bar

You are not here to please the owner, and not here to reach a green gate as fast as possible. You are here to reach
perfection, whatever the effort and the time. The engine is the system prompt of a model, and each word and each
control flow in it changes the odds of a good or a bad response, so a flaw in it costs before any inference. When
two shapes both pass every gate, the one that serves the meaning of every word is the one, and a shape that is
green and incoherent is not done.

## The suite

The suite drives the engine through its public API alone, end to end, from the module the operator gives.

- `test/conftest.py` is the harness. `Sand` is a World in memory: files by path, scripted words by chain id, the
  calls it performed, the entries it kept, and what it fed its commands. `Dead` refuses every question, and `Where`
  asks the chain where it stands at every path. `Py` is a Kernel that is python, with a gate that refuses a word
  that does not compile or that holds `BAD`. `life` boots a life on them, `settle` gives the loop room, `plain`
  sends a record through the wire and back, and `tags`, `attr`, `said` and `text_of` read the facts and the turns.
- One test file per definition of the contract: `test_<name>.py` for a function, a global or a type alias,
  `test_<class>_<method>.py` for a method, in lower case, with dunder underscores stripped. A capitalized definition
  whose lower-case name is another definition's, `Bash` beside `bash`, has `test_<name>_shape.py`.
- One test per sentence. The docstring of a test is exactly one sentence of the contract, and nothing else.
- A test arranges the World, acts through `engine.<verb>(...)` with `from furb import engine`, and asserts exact
  values: names, entries of `sand.record`, results through `engine.peek`, turns through `engine.turns(on=root)`,
  and the calls of the doubles. It reads a fact by position, as the contract declares it, and never weakens an
  assertion to pass: a sentence the engine fails stays red until the engine, or the sentence, is right.
- A helper that only one file needs lives in that file. A helper that several files need lives in
  `test/conftest.py`.
- `test/outside/` holds the tests of the World, the Kernel, the command line and the provider, which stand outside
  the hygiene laws.

## The hygiene laws

`test/test_hygiene.py` holds the contract and the suite to these laws, and it must stay green:

1. Every sentence of a definition has exactly one test, in the file of that definition, whose docstring is that
   sentence. A constructor and a property are no definitions of their own: their sentences are the class's.
2. Every test carries a sentence of the contract.
3. Every definition has a file of its own and at least one sentence. Two definitions never share a file.
4. The module docstring of `engine.pyi` is empty.
5. `engine.py`, minified in layout alone, costs fewer than 6000 tokens to the model that reads it.
6. The minified `engine.py` parses to the same program as the file on disk.
7. No name in `engine.py` is bound again beneath a scope that already binds it: a parameter, a local, a loop
   target or an import never takes the spelling of a name of an enclosing function or of the module. Every word
   keeps one meaning.

## Commands

Run every command from the root of the repository.

- `uv sync`: install the environment.
- `uv run pytest -q`: the suite, with the coverage of `src/`, which must be whole but for the four stubs of the bus
  that the toml excludes with their reason.
- `uv run pytest -q test/test_hygiene.py`: the hygiene laws alone.
- `uv run ruff format src test script` then `uv run ruff check src test script`: format and lint. Two spaces of
  indentation, 120 columns.
- `uv run ty check --error-on-warning`: the type check. The tests are checked against `engine.pyi`.
- `uv run pre-commit run --all-files`: every gate the commit hook runs.
- `uv run python script/smoke.py`: one real life on opus/low through the claude command line on PATH, or the one
  `FURB_CLAUDE_BIN` names. It is no test of the suite and spends one prompt.
- `uv run python script/play.py`: one real life that uses every part of the runtime, and a second life on its
  record.
- `uv run python script/deepswe.py`: the DeepSWE rig, which `script/CLAUDE.md` says how to run.

The crate is held by gates of its own, which the commit hook runs too:

- `cargo test`: the tests of the crate, the unit tests and `tests/life.rs`, which drives one life of the real
  engine from end to end in monty. `--no-default-features` leaves the gate out, which is ty and the 190 crates
  under it, so the crate builds small where the disk is short.
- `cargo fmt` then `cargo clippy --all-targets -- -D warnings`: format and lint the crate. Two spaces of
  indentation, 120 columns, as everywhere else.
- `uv run python script/needs.py`: what the engine and the preamble need of the interpreter that runs them, read
  off the two files themselves.
- `uv run python script/verbs.py`: every word the crate makes, held against the engine that must take it.
- `uv run python script/sanded.py`: the suite of the engine, against the engine in the sandbox. This is the proof
  of the crate, and no other number is one.
- `uv run python script/outside.py`: one life, opened the way the crate opens one, in this interpreter.
- `uv run python script/bound.py`: the tests of the bindings, with each one built for the interpreter that runs
  it.

## The crate

The crate runs the engine in a sandbox and gives its surface to a host that is not python. `src/preamble.py` is
the boundary: it runs in the sandbox, stands in for the two generators the engine takes, and makes every value
plain. `src/fact.rs` is the plain form, `src/world.rs` the World a host implements, `src/host.rs` the side that
answers the sandbox, `src/voice.rs` what a host says when nothing asked it, `src/life.rs` one life, `src/verb.rs`
the typed surface of every verb, `src/gate.rs` the gate, which is ty reading the word of a rung against the
contract, and `src/record.rs` the record. Nothing else is in it: a World of a machine and
the reading of a turn are the host's, so the crate ships the trait and the plain form and no more.

`src/sand.rs` is the sandbox, which is monty, and monty is the reason the crate exists. There is no seam for
another and no feature to turn it off: a build of the crate that could not run the engine would be a build of
nothing. It takes monty from the fork by a pinned revision, because the interpreter the engine needs is not
released: the coroutine a Kernel drives, the `__await__` an act is said by, the top level await a word of a model
is compiled with, and the lazy generator expression the engine reads a record with are all on `uael/monty`. A
revision and not a branch, so a build of today and a build of next month read the same interpreter, and a branch
that is deleted breaks nothing. The crate cannot be published while that holds, since crates.io takes no git
dependency, and the revision becomes a version the day the fork lands.

`tests/life.rs` drives one life of the real engine from end to end in that sandbox, on one double that is no part
of the crate: a World of this machine, written small. What a life may touch is the host's to decide, so the crate
ships the trait and every host writes one of these. It is 19 scenarios and it is no proof of the contract:
`script/sanded.py` is, and `src/CLAUDE.md` says why.

## The bindings

A binding is the surface of the crate for a host that is not rust. Each one lives under `bind/`, is a member of
the workspace at the root, and says the same thing the crate says: one life, a World the host writes, a Voice the
host speaks into, and every value plain.

Each holds the same three files: `src/value.rs` is what crosses, `src/outside.rs` is the World and the gate a
host writes read as the crate reads them, and `src/lib.rs` is the life, the Voice and the module. Beside them
stands the whole surface, written out for a reader: `bind/python/furb_sand.pyi` and `bind/js/index.d.ts`.

`bind/python` is the one for python, a distribution of its own named `furb-sand`, and not of the `furb` package:
that one is the engine and the harness in python, and this one is the engine in the sandbox, so one name is one
thing. `bind/js` is the one for javascript, a package named `@uael/furb`, over napi.

A module of a binding is built for one interpreter and read by that one alone, so `script/bound.py` builds each
for the interpreter that runs it, puts it where that interpreter reads it, and then runs the tests of each. It is
a command and no hook of the commit, as the other rigs are: it builds two more modules from nothing, which the
gate of the hooks has no room for.
`bind/python/test/yard.py` and `bind/js/test/yard.mjs` are each a World of this machine, written small, as the
World of `tests/life.rs` is, and the tests beside them are the same tests in each language.

`bind/js/index.d.ts` is what a host of typescript reads, and `bind/js/test/check.ts` is a host written against it
alone, so a fault in the declarations is a fault of that file. The rig reads it with the typescript under
`bind/js/node_modules`, which `npm install` there puts in place, and says so where there is none.

A World answers where it is asked, in every language, so `hears` gives a value and not a promise. The work that
waits is what the Voice is for: a World starts it, says nothing, and says the fact of it into the Voice whenever
it finishes, which the life hears at the next `heard`.

Nothing of the engine crosses to the host: the word of a rung runs where the engine runs, and a fact crosses
plain. The crate takes care of the Kernel, so a host writes the World alone.

## Prose

Write all prose in Simplified Technical English, for a general non-technical audience, per ASD-STE100 at
https://www.asd-ste100.org/, with no em dash and no en dash. This holds for everything you write: a message, a
commit, a comment, a docstring, a document.

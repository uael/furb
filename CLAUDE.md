# furb

The engine is `src/furb/engine.py`, one file, which depends only on the python interpreter and on two interfaces it
declares, World and Kernel. It is derived from the contract, `src/furb/engine.pyi`, and proved by the suite in
`test/`. `src/furb/CLAUDE.md` holds the technical names of the engine and the laws that no test can hold,
`src/CLAUDE.md` says what the crate is and what proves it, and `script/CLAUDE.md` says how to run the DeepSWE rig.

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
- `uv run pytest -q`: the suite, on both engines, the one of this interpreter and the one in the sandbox of monty,
  with the coverage of `src/` and of `furb_monty`, which must be whole but for the four stubs of the bus that the
  toml excludes with their reason.
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

- `cargo test`: the tests of the crate, and `tests/life.rs`, which drives one life of the real engine from end to
  end in monty on an ear of this machine.
- `cargo fmt` then `cargo clippy --all-targets -- -D warnings`: format and lint the crate. Two spaces of
  indentation, 120 columns, as everywhere else.
- `uv run python script/needs.py`: what the engine and the preamble need of the interpreter that runs them, read
  off the two files themselves, which is what the fork of monty must offer and no more.

## The crate

The crate runs the engine in monty, a python interpreter written in rust for untrusted code, and gives one life of
it to a host that is not python. `src/lib.rs` stands beside `src/furb/engine.py`, and the crate embeds that same
file, so the engine a host runs and the engine a model reads are one thing. `src/preamble.py` is the boundary: it
runs in the sandbox, stands in for every ear of the host, holds the Kernel, and makes every value plain.
`src/fact.rs` is the plain form, `src/ear.rs` the ear a host writes and the one call the sandbox reaches it by,
`src/life.rs` one life, `src/gate.rs` the gate of the Kernel, which is ty reading the word of a rung against the
contract, and `src/sand.rs` the session of monty. The Kernel runs the word of a rung where the engine runs, so
nothing of it crosses to a host and no host writes one. `src/CLAUDE.md` says the rest.

Monty comes from the fork by a pinned revision, because the interpreter the engine needs is not released: the
coroutine a Kernel drives, the `__await__` an act is said by, the top level await a word of a model is compiled
with, and the lazy generator expression the engine reads a record with are all on `uael/monty`. A revision and
not a branch, so a build of today and a build of next month read the same interpreter. The crate cannot be
published while that holds, since crates.io takes no git dependency, and the revision becomes a version the day
the fork lands. `rust-toolchain.toml` names the one rust every build reads, and `Cargo.lock` the crates.

## The engine of monty in python

`bind/python` is `furb-monty`, a distribution of its own and a member of the workspace, which `uv sync` builds
with maturin. `furb_monty.engine` holds every name of `engine.pyi`, one to one, over one life of the crate: a verb
runs its word in the sandbox, `acts`, `asked`, `outcomes` and `modules` read the maps of the life where they
stand, and a value crosses plain and comes back as the shape the engine of this interpreter holds it in. A World
of python is a generator, and it is heard from a thread of its own, so it reads the engine while it answers. A
show, a filter or an ear a caller hands a verb is called back across the boundary. A life of monty needs no
Kernel, since the crate holds one, and `boot` refuses a generator under that name.

`FURB_ENGINE=monty` makes `from furb import engine` give it, read once at import. The suite runs every test on
both engines, and a test that binds the Kernel double of the harness is skipped on monty by name, with the reason,
in `test/conftest.py`.

## Prose

Write all prose in Simplified Technical English, for a general non-technical audience, per ASD-STE100 at
https://www.asd-ste100.org/, with no em dash and no en dash. This holds for everything you write: a message, a
commit, a comment, a docstring, a document.

# furb

The engine is `src/furb/engine.py`, one file. It depends only on the python interpreter and on the ears of the outside
that boot is given, each a generator under a name: the World, the Kernel and the gate among them. It is derived from the
contract, `src/furb/engine.pyi`, and the suite in `test/` proves it. `src/furb/CLAUDE.md` holds the technical names of
the engine and the laws that no test can hold. `script/CLAUDE.md` says how to run the DeepSWE rig.

The crate at the root, `furb`, runs the same file in monty, a python interpreter written in rust, behind an async
API of its own:

- `src/lib.rs` says what the crate gives: `Life`, whose methods are the verbs of the contract, and `World`, the one
  trait that a host writes.
- `src/preamble.py` runs in the sandbox and stands in for the ears of a host.
- The Kernel and the gate are the crate's. The gate is the type checker of monty. It reads a word on the sheet of
  the engine, `src/furb/sheet.py`, against the typeshed of the sandbox. The gate of the python package reads
  through it too.
- `src/binding/py.rs`, behind the `python` feature, is the door to python. `bind/python` is the package
  `furb-monty`. Its module `furb_monty.engine` gives every name of the contract over one life in the sandbox, and
  `FURB_ENGINE=monty` makes `from furb import engine` give it.

The suite runs on both engines. `test/outside/test_monty.py` proves what the door carries that no sentence of the
contract says.

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

Read the whole file and list each incoherence: a truth with two homes, a word with two meanings, a part held by the
wrong owner. Probe each, build its reshape, and measure its size and the suite. Keep what adds coherence, then read
the whole file again.

## The suite

The suite drives the engine through its public API alone, end to end, from the module the operator gives.

- `test/conftest.py` is the harness. `Sand` is a World in memory: files by path, scripted words by chain id, the calls
  it performed, the entries it kept, and what it fed its commands. `Dead` refuses every question but the standing, and
  `Where` asks the chain where it stands at every path. `Py` is a Kernel that is python, with the gate of the crate,
  which refuses a word that is not python or that names what nothing binds, such as `BAD`. `life` boots a life on them,
  `settle` gives the loop room, `plain` sends a record through the wire and back, `said` reads the facts, and
  `paragraphs` and `heads` read the turns, which are python.
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

- `uv sync`: install the environment, which builds the crate with its `python` feature into the package
  `furb-monty`. After a change of the crate, `uv sync --reinstall-package furb-monty` builds it again.
- `uv run pytest -q`: the suite on both engines, with the coverage of `furb` and of `furb_monty`, which must be
  whole but for the four stubs of the bus that the toml excludes with their reason.
- `uv run pytest -q test/test_hygiene.py`: the hygiene laws alone.
- `uv run ruff format src test script` then `uv run ruff check src test script`: format and lint. Two spaces of
  indentation, 120 columns.
- `uv run ty check --error-on-warning`: the type check. The tests are checked against `engine.pyi`.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test`: the gates of the crate. The
  tests of a module stand beside it, `src/life.test.rs` beside `src/life.rs`, and those of the life drive the real
  engine on a World in rust.
- `uv run pre-commit run --all-files`: every gate the commit hook runs.
- `uv run python script/smoke.py`: one real life on opus/low through the claude command line on PATH, or the one
  `FURB_CLAUDE_BIN` names. It is no test of the suite and spends one prompt.
- `uv run python script/play.py`: one real life that uses every part of the runtime, and a second life on its
  record.
- `uv run python script/deepswe.py`: the DeepSWE rig, which `script/CLAUDE.md` says how to run.

## Prose

Write all prose in Simplified Technical English, for a general non-technical audience, per ASD-STE100 at
https://www.asd-ste100.org/, with no em dash and no en dash. This holds for everything you write: a message, a
commit, a comment, a docstring, a document.

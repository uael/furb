# furb

The engine is `src/furb/engine.py`, one file, which depends only on the python interpreter and on two interfaces it
declares, World and Kernel. It is derived from the contract, `src/furb/engine.pyi`. Every call in the engine raises NotImplementedError: the contract and the suite come first. `src/furb/CLAUDE.md` holds the technical names of the engine and the laws that no test can hold.

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

## Commands

Run every command from the root of the repository.

- `uv sync`: install the environment.
- `uv run ruff format src test script` then `uv run ruff check src test script`: format and lint. Two spaces of
  indentation, 120 columns.
- `uv run ty check --error-on-warning`: the type check. The tests are checked against `engine.pyi`.
- `uv run pre-commit run --all-files`: every gate the commit hook runs.

## Prose

Write all prose in Simplified Technical English, for a general non-technical audience, per ASD-STE100 at
https://www.asd-ste100.org/, with no em dash and no en dash. This holds for everything you write: a message, a
commit, a comment, a docstring, a document.

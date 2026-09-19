# furb

The core of an AI harness, in one python file, `src/furb/engine.py`. A model does everything with python that it
writes: the engine, minified, is its whole system prompt, and each word it answers with runs on a chain of the
engine. The contract is `src/furb/engine.pyi`, whose docstrings hold every law, one sentence per line, and the suite
in `test/` holds one test per sentence.

- `uv sync` installs the environment, and `uv run pytest -q` runs the suite.

Copyright (C) 2026 Abel Lucas. furb is free software under the GNU Affero General Public License, version 3, which
`LICENSE` holds: you may use, study, change and share it, and anyone who ships it or runs a changed furb as a
service must offer the source under the same terms. The engine carries no notice of its own, since its text is the
system prompt of a model and every word of it counts.

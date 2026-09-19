"""Every word the crate makes, held against the engine that must take it.

The crate is the one source of the words: `cargo run --quiet --example verbs` prints one word for each verb of
the contract, and this reads them. For every call in every word, the name must be a name of the engine, and the
words of the call must be ones the engine takes, in the place it takes them, which the signature of the verb
says. A keyword the engine does not take, a word in the wrong place and a name that is no verb all fail here.

    uv run python script/verbs.py

This spends nothing and asks no model: it reads the engine and never runs a life.
"""

import ast
import inspect
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
"""ROOT is the root of the repository, which holds the crate and the engine."""

sys.path.insert(0, str(ROOT / "src"))

from furb import engine  # noqa: E402

WORDS = ("cargo", "run", "--quiet", "--example", "verbs")
"""WORDS is how the crate is asked for the word of every verb it makes."""


def stood(node: ast.expr) -> object:
  """One word of a call, as something the signature can be bound to, which is a mark and never a value."""
  return ast.unparse(node)


def bound(call: ast.Call) -> str:
  """One call, held against the verb it names, and the fault of it when the engine takes it not."""
  if not isinstance(call.func, ast.Name):
    return f"{ast.unparse(call)} calls no name"
  name = call.func.id
  verb = getattr(engine, name, None)
  if verb is None:
    return f"{name} is no name of the engine"
  if not callable(verb):
    return f"{name} is no verb of the engine"
  args = [stood(one) for one in call.args]
  keys = {one.arg: stood(one.value) for one in call.keywords if one.arg}
  try:
    inspect.signature(verb).bind(*args, **keys)
  except TypeError as no:
    return f"{ast.unparse(call)} is no call of {name}: {no}"
  return ""


def held(word: str) -> list[str]:
  """Every fault of one word, which is every call in it that the engine does not take."""
  try:
    tree = ast.parse(word, mode="eval")
  except SyntaxError as no:
    return [f"{word} is no expression: {no}"]
  found = [bound(one) for one in ast.walk(tree) if isinstance(one, ast.Call)]
  names = [one.id for one in ast.walk(tree) if isinstance(one, ast.Name)]
  found += [f"{one} is no name of the engine" for one in names if not hasattr(engine, one)]
  # One name that is wrong is one fault, whether it stands alone or carries a call.
  return list(dict.fromkeys(one for one in found if one))


def main() -> int:
  """Every word of the crate, read and held, and the faults of all of them at once."""
  got = subprocess.run(WORDS, capture_output=True, text=True, cwd=ROOT, check=False)  # noqa: S603
  if got.returncode:
    sys.stderr.write(got.stderr)
    return 1
  words = [one for one in got.stdout.split("\n") if one.strip()]
  faults = [(word, one) for word in words for one in held(word)]
  for word, one in faults:
    sys.stdout.write(f"{word}\n  {one}\n")
  sys.stdout.write(f"{len(words) - len({word for word, _ in faults})} of {len(words)} words the engine takes\n")
  return 1 if faults else 0


sys.exit(main())

"""Print the cost of the engine and of the doctrine in tokens of o200k_base, against their walls.

The hygiene laws hold the same count, from the same code: `test/test_hygiene.py` imports it from here. It builds
nothing. `--doctrine` and `--engine` print one count, and `--stdin` counts a draft of the doctrine read from
stdin. The engine that a model reads is the minified `engine.py`, which only the build of the crate makes, so this
script looks for the newest copy of it that is the program on disk: the one in the installed `furb_monty`, then each
`system.py` that cargo left in `target`. When no copy is current, it says which command makes one.
"""

import ast
import sys
from pathlib import Path

import tiktoken

try:
  from furb_monty._monty import SYSTEM
except ImportError:
  SYSTEM = ""

ROOT = Path(__file__).resolve().parents[1]
ENGINE = ROOT / "src" / "furb" / "engine.py"
# What the engine may cost the model that reads it, counted with o200k_base as law 5 says: a wall, and a shape that
# will not fit under it is a shape not found yet. Seven thousand, by the owner's word, for the engine, and one
# thousand for the doctrine of furb, which every chain reads in a turn of its setup.
BUDGET = 7_000
DOCTRINE = 1_000
QUOTE = (ROOT / "DOCTRINE.md").read_text(encoding="utf-8")


MISSING = (
  "no minified engine is the program of src/furb/engine.py: run `cargo check`, whose build script makes it, "
  "or `uv sync`, and run this again\n"
)


def tokens(text: str) -> int:
  return len(tiktoken.get_encoding("o200k_base").encode(text))


def engine_tokens(system: str) -> int:
  return tokens(system)


def doctrine_tokens(quote: str = QUOTE) -> int:
  return tokens(f"<s:doctrine>\n{quote}</s:doctrine>")


def current(system: str) -> bool:
  return ast.dump(ast.parse(system)) == ast.dump(ast.parse(ENGINE.read_text(encoding="utf-8")))


def copies() -> list[tuple[str, str]]:
  found: list[tuple[str, str]] = []
  if SYSTEM:
    found.append(("the installed furb_monty", SYSTEM))
  built = sorted(ROOT.glob("target/*/build/furb-*/out/system.py"), key=lambda one: one.stat().st_mtime, reverse=True)
  return found + [(str(one.relative_to(ROOT)), one.read_text(encoding="utf-8")) for one in built]


def engine_count() -> tuple[int, str] | None:
  for where, system in copies():
    try:
      if current(system):
        return engine_tokens(system), where
    except SyntaxError:
      continue
  return None


def main(args: list[str]) -> int:
  if args in (["--stdin"], ["--doctrine"]):
    spent = doctrine_tokens(sys.stdin.read() if args == ["--stdin"] else QUOTE)
    sys.stdout.write(f"{spent}\n")
    return int(spent > DOCTRINE)
  if args == ["--engine"]:
    found = engine_count()
    if found is None:
      sys.stderr.write(MISSING)
      return 1
    sys.stdout.write(f"{found[0]}\n")
    return int(found[0] >= BUDGET)
  if args:
    sys.stderr.write("usage: tokens.py [--doctrine | --engine | --stdin]\n")
    return 2
  spent = doctrine_tokens()
  over = spent > DOCTRINE
  sys.stdout.write(f"the doctrine costs {spent} tokens, against the wall of {DOCTRINE}{' (over)' if over else ''}\n")
  found = engine_count()
  if found is None:
    sys.stderr.write(MISSING)
    return 1
  sys.stdout.write(f"the engine costs {found[0]} tokens, against the wall of {BUDGET}, counted in {found[1]}\n")
  return int(over or found[0] >= BUDGET)


if __name__ == "__main__":
  sys.exit(main(sys.argv[1:]))

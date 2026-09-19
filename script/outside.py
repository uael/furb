"""One life opened the way the crate opens one, in this interpreter instead of a sandbox.

The crate runs `src/preamble.py` in the session it makes, gives the engine a module of its own, and opens a life
on the two generators the preamble makes. Every one of those steps is python, so all of it can be done here, with
a host of a few lines where the crate has Rust. What this proves is the shape: the engine boots through the
preamble, a World outside it answers in plain data alone, the Kernel gates a word with the host and runs it, and a
fact the host says later is heard.

    uv run python script/outside.py

A sandbox changes where this runs and nothing of what it does.
"""

import asyncio
import builtins
import sys
from collections import ChainMap
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
"""ROOT is the root of the repository, which holds the engine and the preamble."""
SRC = ROOT / "src"
"""SRC holds the preamble and the package the engine is in."""

if str(SRC) not in sys.path:
  sys.path.insert(0, str(SRC))

from preamble import asked, does, module, opened, unwire, wire  # noqa: E402

WORD = "close(len(read('a.txt').lines))"
"""WORD is what the model of this rig answers with: it reads a file and closes with how many lines it holds."""
STANDS = ((("m", ("low",), 200000),), "/w", "m/low")
"""STANDS is what the chains of this life stand on: one actor, a directory, and the actor a prompt goes to."""
FILES = {"/w/a.txt": "one\ntwo\nthree\n"}
"""FILES is the disk of this World, by path."""


def say(text: str) -> None:
  """One line to the operator, said as it happens."""
  sys.stdout.write(f"{text}\n")
  sys.stdout.flush()


def hosted(engine: dict[str, object], said: list) -> object:
  """The host of this rig: a World of a few facts, and a gate that finds nothing.

  It answers what the crate answers in Rust, in the same plain form: facts to say, or the findings of the gate.
  """
  kind = said[0]
  if kind == "gate":
    return []
  if kind == "stand":
    return ("say", [wire(("done", said[1], STANDS))])
  if kind == "read":
    path = unwire(said[4], engine)
    got = engine["Text"](f"/w/{path}", FILES[f"/w/{path}"])  # ty: ignore[call-non-callable]
    return ("say", [wire(("done", said[1], got))])
  if kind == "ask":
    return ("say", [wire(("answer", said[1], ("assistant", [WORD], (0, 0, 0, 0, 0.0), [])))])
  return ("say", [])


async def main() -> None:
  """One life, opened through the preamble, prompted once, and read."""
  engine = module((SRC / "furb" / "engine.py").read_text(encoding="utf-8"))
  names = ChainMap(engine, vars(builtins))

  def host(_name: str, one: object) -> object:
    """Every call the preamble makes, answered."""
    got = unwire(one, names)
    assert isinstance(got, tuple)
    return hosted(engine, list(got))

  root = opened(engine, [], host)
  say(f"root: {root}")

  got = asked(engine, f"prompt(int, 'count the lines', on={root!r})")
  prompt = unwire(got, names)
  say(f"the prompt is: {prompt}")
  for _ in range(200):
    await asyncio.sleep(0)
  say(f"the prompt gave: {unwire(asked(engine, f'peek({prompt!r})'), names)}")
  held = unwire(asked(engine, f"turns(on={root!r})"), names)
  assert isinstance(held, list)
  say(f"turns: {len(held)}")

  does(engine, [wire(("fact", "tell", root, "world", [[("noted", [], "a fact the host said later")]]))])
  again = unwire(asked(engine, f"turns(on={root!r})"), names)
  assert isinstance(again, list)
  tags = [tag[0] for _, content, _, _ in again for tag in content]
  say(f"the host was heard: {'noted' in tags}")


asyncio.run(main())

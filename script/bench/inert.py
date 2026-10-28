"""The proof that a replay is inert: a life that only inspects a record starts nothing outside before its tip.

    python script/bench/inert.py <record>

It opens a life that only inspects a copy of the record, on the ears of the crate, the Kernel and the gate, with one
more ear of the outside first, which hears each fact until the journal says the last entry of the record again. The
proof fails when, before that tip, the life puts an act on to the outside that the record answers, when an ear of the
World says that it started an act, or when the journal keeps a fact, which a life that keeps would add to the record.
A run, which the Kernel takes, and a writing, which the Kernel runs, are no work of the World. It exits 0 when the
life reaches the tip inert, 1 when it is not inert, and 2 when the replay stops before the tip.
"""

import asyncio
import re
import shutil
import sys
import tempfile
from collections.abc import Generator
from pathlib import Path

import furb_monty
from furb import engine, python, sheet
from furb.kernel import kernel
from furb.world import entries
from furb_monty import _monty

ACT = re.compile(r"\w+?\d+")
"""ACT matches the name of an act, which a maker of an act of the life is."""

OWN = ("journal", "kernel", "gate", "")
"""OWN names the sayers that are no ear of the World: the journal, the Kernel, the gate, and the life itself."""


def spy(last: tuple, seen: dict[str, list]) -> Generator[None, tuple | None]:
  """An ear of the outside that notes each act put on to it, each start that an ear of the World says and each keep,
  until it hears the last entry of the record again."""
  while True:
    a = yield
    if not a or seen["tip"]:
      continue
    if a[:2] == last:
      seen["tip"].append(a)
    elif a[0] == "keep":
      seen["kept"].append(a)
    elif (a[0] not in ("chain", "run", "writing") and engine.question(a) and engine.get(a[1]) == a) or (
      a[0] == "started" and not ACT.fullmatch(str(a[2])) and a[2] not in OWN
    ):
      seen["put"].append(a)


async def replayed(record: Path, directory: Path) -> dict[str, list]:
  """What the life that inspects the record put on to the outside and kept before its tip."""
  stored, pairs = _monty.opened(
    directory=str(directory), record=str(record), inspecting=True, extensions=False, roster=[]
  )
  ears = dict(pairs)
  held = entries(stored)
  seen: dict[str, list] = {"tip": [], "kept": [], "put": []}
  try:
    engine.boot(
      held,
      spy=spy(tuple(held[-1][0][:2]), seen),
      kernel=kernel(vars(python)),
      gate=sheet.gating(vars(python), furb_monty.gate),
      **ears,
    )
    await asyncio.sleep(0)
  finally:
    for one in ears.values():
      one.dispose()
  return seen


def say(*words: object) -> None:
  """Write a line of the proof on stdout."""
  sys.stdout.write(" ".join(map(str, words)) + "\n")


def main() -> int:
  """Prove the record given on the command line inert, and print what the life did otherwise."""
  with tempfile.TemporaryDirectory() as tmp:
    copy = Path(tmp) / "record.jsonl"
    shutil.copyfile(sys.argv[1], copy)
    try:
      seen = asyncio.run(replayed(copy, Path(tmp)))
    except Exception as no:
      say(f"the replay stops before the tip: {type(no).__name__}: {str(no)[:600]}")
      return 2
  say(f"the replay reached the tip, {seen['tip'][0][:2]}")
  say(f"acts put on to the outside before the tip: {len(seen['put'])}")
  for a in seen["put"][:25]:
    say("  ", repr(a)[:200])
  say(f"facts kept before the tip: {len(seen['kept'])}")
  for a in seen["kept"][:25]:
    say("  ", repr(a)[:200])
  return 1 if seen["put"] or seen["kept"] else 0


if __name__ == "__main__":
  sys.exit(main())

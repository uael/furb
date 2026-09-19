"""The smoke of a real life: one model, one record, and the record answering the same prompt the second time.

Run it from the root of the repository, as `uv run python script/smoke.py`. It spends the dollars of one turn of
one model and no more: the first life asks the model, and the second life is given the record of the first and is
answered out of it, so no model is asked again.
"""

import asyncio
import os
import shutil
import sys
import tempfile
from pathlib import Path

from furb import engine
from furb.cli import again, lived, say
from furb.engine import Act
from furb.provider.claude import BIN, cool
from furb.world import kept

MESSAGE = "How many lines does the file a.txt hold?"
"""MESSAGE is what the model is asked, of a file it must read to answer."""
HELD = "one\ntwo\nthree\n"
"""HELD is what the file holds."""
LINES = len(HELD.splitlines())
"""LINES is how many lines the file holds, which is what the model must answer."""
TO = "opus/low"
"""TO is the actor the smoke asks, which is opus at the least effort it takes."""
STALL = 300.0
"""STALL is the seconds the smoke waits for a prompt, since a chain the World paused would wait for ever."""
CEILING = 1.0
"""CEILING is the dollars the first life may spend, ten answers or so, so that a model that loops is paused."""


def told(record: Path) -> list[list]:
  """Every answer of a model that the record holds."""
  return [entry[1] for entry in kept(record) if entry[1][0] == "answer"]


def spent(record: Path) -> float:
  """The dollars the answers of the record cost."""
  return sum(one[3][2][4] for one in told(record) if one[3] and one[3][2])


def asks(calls: list[tuple]) -> list[tuple]:
  """Every ask the World performed in one life."""
  return [one for one in calls if one[0] == "ask"]


async def first(yard: Path, record: Path) -> None:
  """The first life: the model reads the file and answers with the number of the lines it holds."""
  world, root, held = lived(record, yard, TO)
  assert held == [], "the first life is opened on no record"
  engine.grant(usd=CEILING, on=root)
  try:
    got = await asyncio.wait_for(engine.prompt(int, MESSAGE, TO, on=root), STALL)
  except TimeoutError:
    say(f"no answer in {STALL:.0f} seconds: {len(asks(world.calls))} ask(s) cost {spent(record):.4f} dollars")
    # A chain at the ceiling of its grant is paused and answers nothing more, which is why nothing came back.
    shown = [tag for turn in engine.turns(on=root) for tag in turn[1] if isinstance(tag, tuple)]
    if [tag for tag in shown if tag[0] == "paused"]:
      say(f"the chain is paused: the grant of {CEILING} dollars holds it at its ceiling")
    raise
  finally:
    await cool()
  say(f"the first life gave {got!r}, after {len(asks(world.calls))} ask(s)")
  for one in told(record):
    _, content, usage, _ = one[3]
    say(f"the word of the model:\n{'\n'.join(x for x in content if isinstance(x, str))}")
    say(f"the usage of the answer: {usage}")
  assert got == LINES, f"the model answered {got!r} and not {LINES}"


async def second(yard: Path, record: Path) -> None:
  """The second life, on the record of the first: it asks no model, since the record answers the prompt."""
  world, root, held = lived(record, yard, TO)
  name = again(held, root, int, MESSAGE, TO)
  assert name, "the record holds no prompt of the operator"
  try:
    got = await asyncio.wait_for(Act(name), STALL)
  finally:
    await cool()
  say(f"the second life gave {got!r}, after {len(asks(world.calls))} ask(s), from {name}")
  assert got == LINES, f"the record answered {got!r} and not {LINES}"
  assert asks(world.calls) == [], "the second life asked a model for what the record holds"


def main() -> int:
  """The two lives, in a directory of their own, and what the answer of the model cost."""
  if shutil.which(os.environ.get(BIN) or "claude") is None:
    say("no claude on PATH, so no model can be asked and the smoke stops here.")
    return 1
  yard = Path(tempfile.mkdtemp(prefix="furb-smoke-"))
  record = yard / "record.jsonl"
  (yard / "a.txt").write_text(HELD, encoding="utf-8")
  say(f"the directory of the smoke is {yard}")
  asyncio.run(first(yard, record))
  asyncio.run(second(yard, record))
  say(f"the smoke spent {spent(record):.6f} dollars, and the record holds {len(kept(record))} entries")
  say("SMOKE OK")
  return 0


if __name__ == "__main__":
  sys.exit(main())

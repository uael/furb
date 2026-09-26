"""Saying, what an ear says and the bus makes a fact of."""

from collections.abc import Generator

from conftest import born, said
from furb import engine


async def test_what_an_ear_says() -> None:
  """What an ear says: the kind of the fact, the act it is about, and the words, and nothing of who says it, which the bus fills in from whoever is speaking."""
  _, log, root = born()
  kept: list[tuple] = []

  def note() -> Generator[tuple | None, tuple]:
    """An ear that says one saying and keeps it beside the fact the bus made of it."""
    saying = ("tell", root, ["#chain1 noted"])
    kept.append(saying)
    kept.append((yield saying))
    while True:
      yield

  engine.drive(note(), "note")
  saying, fact = kept
  assert saying == ("tell", "chain1", ["#chain1 noted"])
  assert fact == ("tell", "chain1", "note", ["#chain1 noted"]) == [a for a in log if a[0] != "keep"][-1]
  assert len(fact) == len(saying) + 1 and fact[2] == "note"
  act = engine.bash("echo hi", on=root)
  opened = next(one for one in said(log, "tell") if one[1] == act)
  assert opened == ("tell", "bash1", "bash1", ["#bash1 echo hi", "bash1: Act[Exit] = Act('bash1')"])

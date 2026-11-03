"""automatic, the actor of automatic work."""

from conftest import STANDS, born
from furb import engine


async def test_automatic_gives_the_actor_of_automatic_work() -> None:
  """automatic gives the actor of automatic work: the model of the roster of the standing with the first place in the list of automatic work, which the entry of each model of that list gives as its fifth field, else the first cheap model of the roster, else an empty str."""
  plain = [*STANDS[0], ["h", ["low"], 200000, "cheap"], ["s", [], 200000, "dear"], ["k", [], 200000, "cheap"]]
  placed = [*STANDS[0], ["h", ["low"], 200000, "cheap", 1], ["s", [], 200000, "dear", 0], ["k", [], 200000, "cheap"]]
  got = []
  for offers in (placed, plain, STANDS[0]):
    born(stands=[offers, "/z", "m/low"])
    got.append(engine.automatic())
  assert got == ["s", "h", ""]


async def test_the_operator_is_no_such_model() -> None:
  """The operator is no such model."""
  born(stands=[[[engine.OPERATOR, [], 200000, "cheap", 0], ["h", [], 200000, "dear", 1]], "/z", "m/low"])
  assert engine.automatic() == "h"

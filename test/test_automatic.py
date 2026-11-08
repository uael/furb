"""automatic, the actor of automatic work."""

from conftest import STANDS, born
from furb import engine


async def test_automatic_gives_the_actor_of_automatic_work() -> None:
  """automatic gives the actor of automatic work: the name at place n of the list of automatic work, which holds the models of the roster of the standing that have a place in that list, which the entry of each model gives as its fifth field, in the order of their places, then each cheap model of the roster, each name once."""
  plain = [*STANDS[0], ["h", ["low"], 200000, "cheap"], ["s", [], 200000, "dear"], ["k", [], 200000, "cheap"]]
  placed = [*STANDS[0], ["h", ["low"], 200000, "cheap", 1], ["s", [], 200000, "dear", 0], ["k", [], 200000, "cheap"]]
  got = []
  for offers in (placed, plain):
    born(stands=[offers, "/z", "m/low"])
    got.append([engine.automatic(n) for n in range(3)])
  assert got == [["s", "h", "k"], ["h", "k", ""]]
  assert engine.automatic() == engine.automatic(0) == "h"


async def test_past_the_end_of_the_list_it_gives_an_empty_str() -> None:
  """Past the end of the list it gives an empty str, so a use whose model refuses it asks automatic(1), then the next, until the list ends."""
  born(stands=[[*STANDS[0], ["h", [], 200000, "cheap"]], "/z", "m/low"])
  assert [engine.automatic(n) for n in range(3)] == ["h", "", ""]
  born(stands=[STANDS[0], "/z", "m/low"])
  assert engine.automatic() == ""


async def test_the_operator_is_no_such_model() -> None:
  """The operator is no such model."""
  born(stands=[[[engine.OPERATOR, [], 200000, "cheap", 0], ["h", [], 200000, "dear", 1]], "/z", "m/low"])
  assert engine.automatic() == "h"

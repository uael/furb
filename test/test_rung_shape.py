"""Rung, the word, the rung it retells, the actor and the shape of the run of one word."""

from conftest import STANDS, Sand, life, said, settle
from furb import engine


async def test_a_rung_carries_the_word_the_rung_it_retells_the_actor_and_the_name_of_the_shape_it_must_give() -> None:
  """A rung carries the word, the rung it retells, the actor and the name of the shape it must give."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  assert await engine.rung("k = 21", on=root) is None
  sand.script[root] = ["close(k + 1)"]
  assert await engine.prompt(int, "count", to="m/high", on=root) == 22
  await settle()
  mine, theirs = said(log, "rung")
  assert mine[4:] == ("k = 21", "", "", "")
  assert theirs[4:] == ("", "", "m/high", "int")
  twin = engine.chain("twin", source=root)
  await settle(300)
  assert [a[4:] for a in said(log, "rung") if a[3] == twin] == [
    ("k = 21", mine[1], "", ""),
    ("close(k + 1)", theirs[1], "", ""),
  ]

"""door, whether a path is the door of a ladder of its chain."""

from conftest import FILES, STANDS, Sand, life, said, settle
from furb import engine
from furb.engine import OPERATOR


async def test_whether_a_path_is_the_door_of_a_ladder_of_its_chain() -> None:
  """Whether a path is the door of a ladder of its chain: the name of a prompt that the chain it is on has heard on itself, which that chain answers a ladder of."""
  sand = Sand(stands=STANDS, words=FILES)
  _, root = life(sand)
  act = engine.prompt(int, "count", to=OPERATOR, on=root)
  assert await engine.rung(f"close(door({act!r}))", on=root) is True
  assert await engine.rung(f"close(door({act!r}, on=__name__))", on=root) is True
  assert engine.ask("ladder", root, act)[1] == ""


async def test_a_path_of_no_name_and_the_name_of_a_prompt_of_another_chain_is_no_door_of_the_chain() -> None:
  """A path of no name, and the name of a prompt of another chain, is no door of the chain."""
  sand = Sand(stands=STANDS, words=FILES)
  log, root = life(sand)
  two = engine.chain("two")
  await settle()
  theirs = engine.prompt(int, "count", to=OPERATOR, on=two)
  names = ["", "a.txt", "nowhere://x", root, theirs, said(log, "rung")[0][1]]
  assert await engine.rung(f"close([door(x) for x in {names!r}])", on=root) == [False] * len(names)
  assert await engine.rung(f"close(door({theirs!r}))", on=two) is True

"""get, which gives an act again from the name of the act."""

import pytest

from conftest import STANDS, Sand, life, said, settle
from furb import engine
from furb.engine import OPERATOR, WORLD


async def test_the_act_again_from_its_name() -> None:
  """The act again, from its name: whoever holds the name of an act is given the act the life holds under it, whole as it stands."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = wait(0)\nclose(x)"]
  which = await engine.prompt(str, "start one", on=root)
  await settle()
  step = said(log, "rung")[0][1]
  assert engine.get(which) == said(log, "wait")[0]
  assert engine.get(which) == ("wait", which, step, root, 0)
  assert which in engine.outcomes and engine.peek(which, on=root) is None


async def test_get_gives_an_act_again_from_the_id_of_the_act() -> None:
  """get gives an act again from the id of the act."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = engine.wait(0, on=root)
  again = engine.get(act)
  assert again[:4] == ("wait", act, OPERATOR, root) and again == engine.get(act)
  assert (await act) is None and engine.get(act) == again


async def test_get_and_peek_enter_nothing_in_the_record() -> None:
  """get and peek enter nothing in the record."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = engine.prompt(int, "how many?", to=OPERATOR, on=root)
  await settle()
  engine.close(3, act)
  await act
  kept = len(sand.record)
  assert engine.get(act)[1] == act
  assert engine.peek(act, on=root) == 3
  assert len(sand.record) == kept


async def test_get_and_peek_read_the_record_as_it_stands_where_the_call_is_made() -> None:
  """get and peek read the record as it stands where the call is made."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = engine.wait(100, on=root)
  await settle()
  match engine.get(act):
    case ("wait", _, _, _, seconds):
      word = seconds
    case _:
      word = None
  assert word == 100 and act not in engine.outcomes
  assert engine.peek(act, on=root) is None
  engine.send("done", act, 3, by=WORLD)
  await settle()
  assert act in engine.outcomes and engine.peek(act, on=root) == 3


async def test_a_name_of_no_act_of_the_life_raises_keyerror() -> None:
  """A name of no act of the life raises KeyError, since the life holds nothing under it."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = engine.wait(0, on=root)
  assert engine.get(act)[1] == act
  with pytest.raises(KeyError):
    engine.get("wait://nobody")
  assert all(isinstance(fact, tuple) for fact, *_ in sand.record)

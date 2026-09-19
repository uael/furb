"""get, which gives an act again from the name of the act."""

import pytest

from conftest import STANDS, Sand, life, said, settle
from furb import engine
from furb.engine import OPERATOR, WORLD, Exit, Text


async def test_the_act_again_from_its_name() -> None:
  """The act again, from its name: whoever holds the name of an act is given the act the life holds under it, whole as it stands."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose(x)"]
  which = await engine.prompt(str, "start one", on=root)
  await settle()
  step = said(log, "rung")[0][1]
  assert engine.get(which) is said(log, "bash")[0]
  assert engine.get(which) == ("bash", which, step, root, "echo hi", False, 600.0)
  ended = engine.peek(which, on=root)
  assert isinstance(ended, Exit) and ended.code == 0


async def test_get_gives_an_act_again_from_the_id_of_the_act() -> None:
  """get gives an act again from the id of the act."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = engine.bash("echo hi", on=root)
  again = engine.get(act)
  assert again[:4] == ("bash", act, OPERATOR, root) and again is engine.get(act)
  assert (await act).code == 0 and engine.get(act) is again


async def test_get_and_peek_enter_nothing_in_the_record() -> None:
  """get and peek enter nothing in the record."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = engine.bash("echo hi", on=root)
  await act
  kept = len(sand.record)
  assert engine.get(act)[1] == act
  assert isinstance(engine.peek(act, on=root), Exit)
  assert len(sand.record) == kept


async def test_get_and_peek_read_the_record_as_it_stands_where_the_call_is_made() -> None:
  """get and peek read the record as it stands where the call is made."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  act = engine.bash("slow", on=root)
  await settle()
  match engine.get(act):
    case ("bash", _, _, _, command, *_):
      word = command
    case _:
      word = None
  assert word == "slow" and act not in engine.outcomes
  assert engine.peek(act, on=root) == Exit(None, Text(f"{act}/stdout"), Text(f"{act}/stderr"))
  engine.send("exited", act, 3, by=WORLD)
  await settle()
  got = engine.peek(act, on=root)
  assert act in engine.outcomes and isinstance(got, Exit) and got.code == 3


async def test_a_name_of_no_act_of_the_life_raises_keyerror() -> None:
  """A name of no act of the life raises KeyError, since the life holds nothing under it."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = engine.bash("echo hi", on=root)
  assert engine.get(act)[1] == act
  with pytest.raises(KeyError):
    engine.get("bash://nobody")
  assert all(isinstance(fact, tuple) for _, fact, *_ in sand.record)

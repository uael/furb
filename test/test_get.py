"""get, which gives an act again from the name of the act."""

from conftest import STANDS, Sand, life, said, settle
from furb import engine
from furb.engine import OPERATOR, Exit


async def test_the_act_again_from_its_name() -> None:
  """The act again, from its name: whoever holds the name of an act is given the act the life holds under it, whole as it stands."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose(x)"]
  which = await engine.prompt(str, "start one", on=root)
  await settle()
  step = said(log, "rung")[0][1]
  assert engine.get(which) == said(log, "bash")[0]
  assert engine.get(which) == ("bash", which, step, root, "echo hi", False, 600.0)
  ended = engine.peek(which)
  assert isinstance(ended, Exit) and ended.code == 0


async def test_get_gives_an_act_again_from_the_id_of_the_act() -> None:
  """get gives an act again from the id of the act."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = engine.bash("echo hi", on=root)
  again = engine.get(act)
  assert again[:4] == ("bash", act, OPERATOR, root) and again == engine.get(act)
  assert (await act).code == 0 and engine.get(act) == again


async def test_get_and_peek_enter_nothing_in_the_record() -> None:
  """get and peek enter nothing in the record."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = engine.bash("echo hi", on=root)
  await act
  kept = len(sand.record)
  assert engine.get(act)[1] == act
  assert isinstance(engine.peek(act), Exit)
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
  assert word == "slow" and engine.peek(act, ...) is ... and engine.peek(act) is None
  sand.exits(act, 3)
  await settle()
  got = engine.peek(act)
  assert engine.peek(act, ...) is not ... and isinstance(got, Exit) and got.code == 3


async def test_a_name_of_no_act_of_the_life_gives_nothing() -> None:
  """A name of no act of the life gives nothing, since the life holds nothing under it."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = engine.bash("echo hi", on=root)
  assert engine.get(act)[1] == act
  assert engine.get("bash://nobody") is None and engine.get("") is None

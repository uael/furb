"""covers, whether a control is over an act."""

from conftest import Sand, life, said, settle
from furb import engine
from furb.engine import OPERATOR


async def test_whether_a_control_is_over_an_act() -> None:
  """Whether a control is over an act: over the act it names and everything under it, and over every act on the chain it names; a close is over the act it names, the words running under it and the replies that ask for those words, where a cancel is over everything under it."""
  sand = Sand(auto=False)
  log, root = life(sand)
  sand.script[root] = ["x = bash('slow')\nclose((await x).code)"]
  act = engine.prompt(int, "go", on=root)
  await settle()
  step, command = said(log, "rung")[0][1], said(log, "bash")[0][1]
  mine = engine.bash("mine", on=root)
  shut, over, whole = ("close", act, OPERATOR, 21, []), ("cancel", act, OPERATOR, []), ("cancel", root, OPERATOR, [])
  assert [engine.covers(shut, one) for one in (act, step, command, mine)] == [True, True, False, False]
  assert [engine.covers(over, one) for one in (act, step, command, mine)] == [True, True, True, False]
  assert [engine.covers(whole, one) for one in (act, step, command, mine)] == [True, True, True, True]
  quiet = engine.prompt(int, "quiet", on=root)
  await settle()
  asking = said(log, "reply")[-1][1]
  assert engine.get(engine.get(asking)[2])[2] == quiet
  assert engine.covers(("close", quiet, OPERATOR, 1, []), asking) and not engine.covers(shut, asking)

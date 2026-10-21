"""covers, whether a control is over an act."""

from conftest import born, said, settle, stalled
from furb import engine
from furb.engine import OPERATOR


async def test_whether_a_control_is_over_an_act() -> None:
  """Whether a control is over an act: over the act it names and everything under it, and over every act on the chain it names; a close is over the act it names, each rung that act made and each reply that asks for the word of one of those, where a cancel is over everything under it."""
  _, log, root, act, step, command = await stalled()
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


async def test_a_rung_that_a_word_makes_is_an_act_of_that_word() -> None:
  """A rung that a word makes is an act of that word, as a command is, so a close of the prompt of that word is not over it, on whatever chain it runs."""
  _, log, root = born("c = chain('other')\nrung('k = 1', on=c)\nrung('j = 2')\nclose(1)")
  assert await engine.prompt(int, "count", on=root) == 1
  await settle(300)
  (other,) = [a[1] for a in said(log, "chain") if a[4] == "other"]
  made = [a[1] for a in said(log, "rung") if a[4] in ("k = 1", "j = 2")]
  assert [engine.peek(one) for one in made] == [None, None]
  assert (engine.module(other)["k"], engine.module(root)["j"]) == (1, 2)

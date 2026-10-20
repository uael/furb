"""asked, the questions that an act asked."""

from conftest import born, said
from furb import engine


async def test_the_names_of_the_questions_that_an_act_asked_on_a_chain_in_the_order_that_it_asked_them() -> None:
  """The names of the questions that an act asked on a chain, in the order that it asked them."""
  _, log, root = born("read('a.txt')\nx = bash('true')\nclose(1)")
  act = engine.prompt(int, "read", on=root)
  assert await act == 1
  step = said(log, "reply")[0][2]
  assert engine.asked(act, root) == [step]
  assert engine.asked(step, root) == ["reply1", "read1", "bash1"]
  assert engine.asked(step, "chain9") == []

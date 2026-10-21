"""over, whether an act is over."""

from conftest import settle, slow
from furb import engine
from furb.engine import Exit


async def test_whether_an_act_is_over_its_done_stands_so_the_life_holds_its_outcome_under_its_name_and_a_name_of_no_act_is_never_over() -> (
  None
):
  """Whether an act is over: its done stands, so the life holds its outcome under its name, and a name of no act is never over."""
  sand, _, _, act = slow()
  await settle()
  assert engine.over(act) is False
  sand.exits(act, 0)
  await settle()
  assert engine.over(act) is True
  assert isinstance(done := engine.peek(act), Exit) and done.code == 0
  assert engine.over("bash9") is False

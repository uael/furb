"""ended, what an act a control is over is done with."""

from asyncio import CancelledError

from conftest import said, settle, stalled
from furb import engine


async def test_what_an_act_a_control_is_over_is_done_with() -> None:
  """What an act a control is over is done with: the value a close carries for the act it names, and a CancelledError for every other."""
  _, log, _, act, step, _ = await stalled()
  engine.close(21, act)
  await settle()
  shut = said(log, "close")[0]
  assert engine.ended(shut, act) == 21
  assert isinstance(engine.ended(shut, step), CancelledError)
  assert engine.peek(act) == 21 and isinstance(engine.peek(step), CancelledError)

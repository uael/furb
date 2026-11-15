"""held, the acts that the ears of the outside took and that are not done, which the journal holds."""

import pytest

from conftest import last_journal, said, settle, stalled
from furb import checkpoint, engine
from furb.engine import Refused


async def test_held_makes_the_journal_hold_each_act_that_an_ear_of_the_outside_took_and_that_is_not_done() -> None:
  """held makes the journal that a boot took hold each act that an ear of the outside took and that is not done, as it holds each act that a record shows started and not done, and gives the name of each act that it holds, so a life that opens from an image puts each act that the ears of the life before held in flight to its fresh ear at a wake, as a life that opens by its record does."""
  _, log, _, _, _, command = await stalled()
  flying = [x[1] for x in said(log, "started") if x[2] == "world" and engine.peek(x[1], ...) is ...]
  assert command in flying
  assert checkpoint.held(last_journal()) == flying


async def test_a_wake_of_an_act_that_the_journal_holds_puts_it_to_the_ears_of_the_outside_again() -> None:
  """A wake of an act that the journal holds puts it to the ears of the outside again, as an act that an earlier life took."""
  sand, log, _, _, _, command = await stalled()
  asked = [a for a in sand.calls if a[1] == command]
  checkpoint.held(last_journal())
  engine.wake(command)
  await settle()
  assert [a for a in sand.calls if a[1] == command] == [*asked, *asked]
  assert [x[:2] for x in log if x[1] == command and x[0] == "bash"] == [("bash", command)] * 2


async def test_held_on_a_journal_that_no_boot_took_is_refused() -> None:
  """held on a journal that no boot took is refused, since nothing holds an act there."""
  with pytest.raises(Refused, match=r"^no boot took this journal, so it holds no act$"):
    checkpoint.held(checkpoint.journal(vars(engine)))

"""Close, the control that carries what the act it names is done with."""

from asyncio import CancelledError

import pytest

from conftest import STANDS, Sand, life, said, settle
from furb import engine
from furb.engine import OPERATOR


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_close_is_a_cancel_that_carries_what_the_act_it_names_is_done_with() -> None:
  """A close is a cancel that carries what the act it names is done with, and it is a kind of its own, since a tuple has no slot that may be empty."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  act = engine.bash("slow", on=root)
  gone = engine.bash("other", on=root)
  engine.close(21, act)
  engine.cancel(gone)
  await settle()
  shut, over = said(log, "close")[0], said(log, "cancel")[0]
  assert shut == ("close", act, OPERATOR, 21, [("closed", [("over", act)], "21")])
  assert over == ("cancel", gone, OPERATOR, [("cancelled", [("over", gone)], None)])
  assert (len(shut), len(over)) == (5, 4)
  assert engine.outcomes[act] == 21 and isinstance(engine.outcomes[gone], CancelledError)


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_close_is_over_the_act_it_names_and_the_words_running_under_it() -> None:
  """A close is over the act it names and the words running under it, where a cancel is over everything under it."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  sand.script[root] = ["x = bash('slow')\nclose((await x).code)"]
  shut = engine.prompt(int, "go", on=root)
  await settle()
  step, command = said(log, "rung")[0][1], said(log, "bash")[0][1]
  engine.close(21, shut)
  await settle()
  assert engine.outcomes[shut] == 21
  assert isinstance(engine.outcomes[step], CancelledError) and command not in engine.outcomes
  sand.script[root] = ["y = bash('slower')\nclose((await y).code)"]
  gone = engine.prompt(int, "go again", on=root)
  await settle()
  theirs = said(log, "bash")[1][1]
  engine.cancel(gone)
  await settle()
  assert isinstance(engine.outcomes[gone], CancelledError)
  assert isinstance(engine.outcomes[theirs], CancelledError)

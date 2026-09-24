"""Close, the control that carries what the act it names is done with."""

from asyncio import CancelledError

from conftest import STANDS, Sand, life, said, settle
from furb import engine
from furb.engine import OPERATOR


async def test_a_close_is_a_cancel_that_carries_what_the_act_it_names_is_done_with() -> None:
  """A close is a cancel that carries what the act it names is done with, and it is a kind of its own, since a tuple has no slot that may be empty."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.wait(100, on=root)
  gone = engine.wait(200, on=root)
  engine.close(21, act)
  engine.cancel(gone)
  await settle()
  shut, over = said(log, "close")[0], said(log, "cancel")[0]
  assert shut == ("close", act, OPERATOR, 21, [f"#{act} closed 21"])
  assert over == ("cancel", gone, OPERATOR, [f"#{gone} cancelled"])
  assert (len(shut), len(over)) == (5, 4)
  assert engine.outcomes[act] == 21 and isinstance(engine.outcomes[gone], CancelledError)


async def test_a_close_is_over_the_act_it_names_and_the_words_running_under_it() -> None:
  """A close is over the act it names and the words running under it, where a cancel is over everything under it."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = wait(100)\nclose(await x)"]
  shut = engine.prompt(int, "go", on=root)
  await settle()
  step, command = said(log, "answer")[0][1], said(log, "wait")[0][1]
  engine.close(21, shut)
  await settle()
  assert engine.outcomes[shut] == 21
  assert isinstance(engine.outcomes[step], CancelledError) and command not in engine.outcomes
  sand.script[root] = ["y = wait(200)\nclose(await y)"]
  gone = engine.prompt(int, "go again", on=root)
  await settle()
  theirs = said(log, "wait")[1][1]
  engine.cancel(gone)
  await settle()
  assert isinstance(engine.outcomes[gone], CancelledError)
  assert isinstance(engine.outcomes[theirs], CancelledError)

"""cancel, which ends an act and everything it made."""

from asyncio import CancelledError

import pytest

from conftest import born, heads, said, settle
from furb import engine
from furb.engine import OPERATOR


async def test_a_cancel_of_that_prompt_reaches_the_acts_that_its_rungs_made_on_the_chain_with_a_source() -> None:
  """A cancel of that prompt reaches the acts that its rungs made on the chain with a source."""
  sand, log, root = born(auto=False)
  twin = engine.chain("twin", source=root)
  await settle()
  sand.script[root] = [f"y = bash('there', on={twin!r})\nclose((await y).code)"]
  one = engine.prompt(int, "run it there", on=root)
  await settle()
  command = said(log, "bash")[0]
  assert command[3] == twin
  engine.cancel(one)
  await settle()
  assert isinstance(engine.peek(one), CancelledError)
  assert isinstance(engine.peek(command[1]), CancelledError)


async def test_a_cancel_is_over_the_act_it_names_and_everything_that_act_made() -> None:
  """A cancel is over the act it names and everything that act made, and each of them is done with CancelledError."""
  _, log, root = born("x = bash('slow')\nclose((await x).code)", auto=False)
  one = engine.prompt(int, "go", on=root)
  await settle()
  step, command = said(log, "rung")[0][1], said(log, "bash")[0][1]
  engine.cancel(one)
  await settle()
  assert isinstance(engine.peek(one), CancelledError)
  assert isinstance(engine.peek(step), CancelledError)
  assert isinstance(engine.peek(command), CancelledError)
  assert [a[1] for a in said(log, "cancel")] == [one]
  waiting = engine.rung("await wait(30)\nclose(1)", on=root)
  await settle()
  (run,) = [a[1] for a in said(log, "run") if a[4] == waiting]
  engine.cancel(root)
  await settle()
  assert isinstance(engine.peek(waiting), CancelledError) and isinstance(engine.peek(run), CancelledError)


async def test_cancel_is_given_the_id_of_an_act_and_says_a_cancel_over_it() -> None:
  """cancel is given the id of an act, and says a cancel over it."""
  _, log, root = born(auto=False)
  one = engine.bash("slow", on=root)
  engine.cancel(one)
  assert said(log, "cancel") == [("cancel", one, OPERATOR, [f"#{one} cancelled"])]


async def test_a_cancelled_act_completes_with_cancellederror() -> None:
  """A cancelled act completes with CancelledError."""
  _, _, root = born(auto=False)
  one = engine.bash("slow", on=root)
  engine.cancel(one)
  await settle()
  assert isinstance(engine.peek(one), CancelledError)


async def test_a_cancelled_prompt_raises_cancellederror_to_whoever_awaits_it() -> None:
  """A cancelled prompt raises CancelledError to whoever awaits it."""
  _, _, root = born()
  one = engine.prompt(int, "how many?", to=OPERATOR, on=root)
  await settle()
  engine.cancel(one)
  with pytest.raises(CancelledError):
    await one


async def test_a_cancel_touches_nothing_else_on_the_chain() -> None:
  """A cancel touches nothing else on the chain."""
  _, _, root = born()
  one = engine.prompt(int, "one", to=OPERATOR, on=root)
  two = engine.prompt(int, "two", to=OPERATOR, on=root)
  await settle()
  engine.cancel(one)
  await settle()
  assert isinstance(engine.peek(one), CancelledError) and engine.peek(two) is None
  engine.close(21, two)
  await settle()
  assert (await two) == 21


async def test_the_awaiter_of_a_cancelled_command_raises_cancellederror_in_its_step() -> None:
  """The awaiter of a cancelled command raises CancelledError in its step."""
  _, log, root = born("x = bash('slow')\nclose((await x).code)", auto=False)
  one = engine.prompt(int, "go", on=root)
  await settle()
  step, command = said(log, "reply")[0][2], said(log, "bash")[0][1]
  engine.cancel(command)
  await settle()
  assert isinstance(engine.peek(step), CancelledError)
  assert [line for line in heads(engine.turns(on=root)) if " raised " in line] == [f"#{step} raised CancelledError()"]
  assert engine.peek(one) is None
  engine.cancel(one)

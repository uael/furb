"""ask, the way to put a question that is answered now."""

from collections.abc import Generator

import pytest

from conftest import Dead, Sand, acts, life, said
from furb import engine
from furb.engine import OPERATOR, Refused


def boom() -> Generator[tuple | None, tuple]:
  """A door of the suite, which answers a read of boom:// with an exception."""
  while True:
    match (yield):
      case ("read", name, _, _, "boom://x"):
        yield "done", name, ValueError("boom")


async def test_the_way_to_put_a_question_that_is_answered_now() -> None:
  """The way to put a question that is answered now: it makes the act, and gives back what the act came to."""
  sand = Sand()
  _, root = life(sand)
  assert engine.ask("clock", root) == 1001.0
  assert engine.get("clock1") == ("clock", "clock1", OPERATOR, root)
  assert engine.peek("clock1") == 1001.0


async def test_it_is_no_entry_of_the_bus() -> None:
  """It is no entry of the bus: it makes the act through act and reads what it came to through peek."""
  sand = Sand()
  log, root = life(sand)
  made = len(acts(log))
  assert engine.ask("chance", root) == 1 / 7
  assert len(acts(log)) == made + 1 and said(log, "ask") == []
  assert [a[1] for a in said(log, "done") if a[1].startswith("chance")] == ["chance1"]


async def test_the_call_raises_the_refusal_an_act_came_to() -> None:
  """The call raises the refusal an act came to, and gives back anything else, an exception among it."""
  dead = Dead()
  _, root = life(dead)
  with pytest.raises(Refused, match="a dead World answers no read"):
    engine.ask("read", root, "a.txt")
  engine.drive(boom(), "boom")
  got = engine.ask("read", root, "boom://x")
  assert isinstance(got, ValueError) and str(got) == "boom"


async def test_an_act_that_is_not_done_when_it_is_made_is_no_answer_now() -> None:
  """An act that is not done when it is made is no answer now, so the call raises Refused."""
  sand = Sand()
  _, root = life(sand)
  with pytest.raises(Refused, match="wait1 not done"):
    engine.ask("wait", root, 5.0)
  assert engine.get("wait1") == ("wait", "wait1", OPERATOR, root, 5.0) and engine.peek("wait1", ...) is ...

"""ask, the way to put a question that lives not."""

from asyncio import CancelledError

import pytest

from conftest import STANDS, Dead, Sand, life, said, settle
from furb import engine
from furb.engine import OPERATOR, WORLD, Refused


async def test_the_way_to_put_a_question_that_lives_not() -> None:
  """The way to put a question that lives not: it takes a name when it is put, it is put to the living generators in turn, the acts first and the outside last, it stops at the first answer, and the question and its answer are given back."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  here, where = engine.ask("stand", root)
  assert here == ("stand", "stand@operator.2", OPERATOR, "chain1") and where == STANDS
  assert engine.asked[here[1]] == here
  when, at = engine.ask("clock", root)
  assert at == 1001.0
  assert [a[1] for a in log if a[0] in ("stand", "clock") and a[2] == OPERATOR] == [when[1]]


async def test_a_query_is_put_at_once_and_is_no_event_of_the_log() -> None:
  """A query is put at once and is no event of the log, since what answers it is; it is answered while the one that asked waits, so nothing awaits it, and the done that names it is its answer."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  mark = len(log)
  here, where = engine.ask("stand", root)
  assert where == STANDS and engine.asked[here[1]] == here
  assert log[mark:] == [("done", here[1], root, STANDS)]
  assert engine.outcomes[here[1]] == where


async def test_the_call_raises_what_the_engine_will_not_make_and_gives_back_anything_else() -> None:
  """The call raises what the engine will not make, and gives back anything else; a peek gives back whatever it was answered, since what an act came to, a cancel and a raise among it, is no refusal of the peek."""
  dead = Dead(stands=STANDS)
  _, root = life(dead)
  with pytest.raises(Refused, match="a dead World answers no clock"):
    engine.ask("clock", root)
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  assert engine.ask("stand", root)[1] == STANDS
  one = engine.wait(100, on=root)
  engine.cancel(one)
  two = engine.prompt(int, "how many?", to=OPERATOR, on=root)
  ghost = engine.prompt(int, "how many?", to="ghost/low", on=root)
  await settle()
  engine.close(ValueError("boom"), two)
  await settle()
  assert isinstance(engine.ask("peek", root, one)[1], CancelledError)
  assert isinstance(engine.ask("peek", root, two)[1], ValueError)
  assert isinstance(engine.ask("peek", root, ghost)[1], Refused)


async def test_the_chain_a_query_is_on_is_the_chain_named_to_the_call() -> None:
  """The chain a query is on is the chain named to the call, or the scope of the one that asked when the call names none, so a query said from a run is on the chain of that run."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  two = engine.chain("two")
  there, _ = engine.ask("clock", two)
  assert there[3] == two
  sand.script[root] = ["close(clock())"]
  assert await engine.prompt(float, "when", on=root) == 1002.0
  assert [a[3] for a in engine.asked.values() if a[0] == "clock"] == [two, root]


async def test_a_question_that_nobody_answers_is_answered_with_nothing() -> None:
  """A question that nobody answers is answered with nothing."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  merged, got = engine.ask("merged", root, "bash9")
  assert got is None and merged[1] not in engine.outcomes
  assert engine.ask("nothing", "")[1] is None


async def test_a_query_the_operator_asks_from_outside_a_run_that_names_no_chain() -> None:
  """A query the operator asks from outside a run that names no chain is put to every generator, and no chain answers it."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  two = engine.chain("two")
  when, at = engine.ask("clock", "")
  assert when == ("clock", "clock@operator.3", OPERATOR, "") and at == 1001.0
  assert when in log
  assert [a[2] for a in said(log, "done") if a[1] == when[1]] == [WORLD]
  held, theirs = engine.ask("transcript", root, root)[1], engine.ask("transcript", two, two)[1]
  assert isinstance(held, list) and isinstance(theirs, list)
  assert [a for a in held if a[1] == when[1]] == []
  assert [a for a in theirs if a[1] == when[1]] == []

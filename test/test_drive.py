"""drive, the other way to speak: a generator brought to life under a name."""

from collections.abc import Generator

import pytest

from conftest import STANDS, Sand, keeping, life, said, settle
from furb import engine
from furb.engine import Refused


def ends(mark: list[tuple]) -> Generator[tuple | None, tuple | None]:
  """A generator that says a done of its own and returns at the first fact it hears after it."""
  yield "done", "none://one", None
  if (word := (yield)) is not None:
    mark.append(word)


def breaks() -> Generator[tuple | None, tuple | None]:
  """A generator that raises the first fact it hears."""
  yield
  raise ValueError("boom")


async def test_the_other_way_to_speak_a_generator_is_brought_to_life_under_a_name() -> None:
  """The other way to speak: a generator is brought to life under a name, and from then it hears every fact that is said and says its own."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  heard: list[tuple] = []
  at = len(log)
  engine.drive(keeping(heard, ("done", "none://one", None)), "keeper")
  engine.wait(0, on=root)
  assert said(log, "done")[-1][1] == "none://one" or heard
  assert heard == log[at:]


async def test_one_that_returns_is_over_and_lives_no_more() -> None:
  """One that returns is over and lives no more, which is how a thing that watches for one fact alone is dropped the moment it hears it."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  mark: list[tuple] = []
  engine.drive(ends(mark), "once")
  engine.wait(0, on=root)
  engine.wait(0, on=root)
  assert len(mark) == 1 and len(said(log, "wait")) == 2


async def test_one_that_raises_while_it_hears_is_broken_the_same_way() -> None:
  """One that raises while it hears is broken the same way, and what went wrong goes to the one that spoke."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  heard: list[tuple] = []
  engine.drive(keeping(heard), "before")
  engine.drive(breaks(), "broken")
  with pytest.raises(ValueError, match="boom"):
    engine.wait(0, on=root)
  after: list[tuple] = []
  engine.drive(keeping(after), "broken")
  engine.wait(0, on=root)
  assert len(said(log, "wait")) == 2
  assert said(heard, "wait") == said(log, "wait") and said(after, "wait") == said(log, "wait")[1:]
  assert [e[0] for e in sand.record if e[0][0] == "wait"] == said(log, "wait")


async def test_a_generator_brought_to_life_under_a_name_and_nothing_more() -> None:
  """A generator brought to life under a name and nothing more: it hears from the tip and runs to its first wait, and one born while a fact goes round hears from the next."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  fresh: list[tuple] = []
  later: list[tuple] = []
  at = len(log)
  engine.drive(keeping(fresh, ("done", "none://tip", None)), "tip")
  assert said(log, "done")[-1][1] == "none://tip"

  def bearing() -> Generator[tuple | None, tuple | None]:
    """An ear that brings another to life while a fact goes round, so the newborn hears from the next fact on."""
    while (heard := (yield)) is not None:
      if heard[0] == "wait":
        engine.drive(keeping(later), "later")

  engine.drive(bearing(), "bearing")
  engine.wait(0, on=root)
  command = said(log, "wait")[0]
  assert fresh == log[at:] and command in fresh
  assert later and command not in later and later == log[log.index(command) + 1 :]


async def test_it_lives_until_it_returns() -> None:
  """It lives until it returns, and an act that hears nothing more returns at the first fact it hears after its own end."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  heard: list[tuple] = []
  engine.drive(keeping(heard), "keeper")
  act = engine.prompt(None, "hi", to="operator", on=root)
  await settle()
  with pytest.raises(Refused, match="hears"):
    engine.drive(engine.idle(act), act)
  engine.close(None, act)
  await settle()
  engine.drive(engine.idle(act), act)
  assert heard and heard[-1] == log[-1]


async def test_a_generator_that_yields_a_saying_is_given_the_fact_as_the_bus_said_it() -> None:
  """A generator that yields a saying is given the fact as the bus said it, and one that yields nothing waits for the next fact said."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  got: list[object] = []

  def asking() -> Generator[tuple | None, tuple | None]:
    """An ear that says one saying of its own and then waits for what is said after it."""
    got.append((yield "tell", root, [("noted", [], None)]))
    while True:
      got.append((yield))

  at = len(log)
  engine.drive(asking(), "asker")
  assert got[0] == ("tell", root, "asker", [("noted", [], None)]) == log[at]
  engine.wait(0, on=root)
  await settle()
  assert got[1:] == log[at:]

"""drive, the way to bring an ear to life under a name."""

from collections.abc import Generator

import pytest

from conftest import STANDS, Sand, keeping, life, said, settle
from furb import engine
from furb.engine import OPERATOR, WORLD, Refused


def ends(mark: list[tuple]) -> Generator[None, tuple | None]:
  """A generator that says a done of its own and returns at the first fact it hears after it."""
  engine.say("done", "none://one", None)
  if (word := (yield)) is not None:
    mark.append(word)


def breaks() -> Generator[None, tuple | None]:
  """A generator that raises the first fact it hears."""
  yield
  raise ValueError("boom")


async def test_the_way_to_bring_an_ear_to_life() -> None:
  """The way to bring an ear to life: a generator is brought to life under a name, and from then it hears every fact that is said and every question offered to it."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  heard: list[tuple] = []
  at = len(log)
  engine.drive(keeping(heard), "keeper")
  engine.bash("echo hi", on=root)
  note = engine.act("note", root, None)
  await settle()
  assert said(heard, "bash") == [] and said(heard, "note") == [engine.get(note)]
  assert heard == [a for a in log[at:] if not engine.question(a) or a[1] == note]


async def test_one_that_returns_is_over_and_lives_no_more() -> None:
  """One that returns is over and lives no more, which is how a thing that watches for one fact alone is dropped the moment it hears it."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  mark: list[tuple] = []
  engine.drive(ends(mark), "once")
  engine.bash("echo hi", on=root)
  engine.bash("echo again", on=root)
  assert len(mark) == 1 and len(said(log, "bash")) == 2


async def test_one_that_raises_while_it_hears_is_broken_the_same_way() -> None:
  """One that raises while it hears is broken the same way, and what went wrong goes to the one that spoke."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  heard: list[tuple] = []
  engine.drive(keeping(heard), "before")
  engine.drive(breaks(), "broken")
  with pytest.raises(ValueError, match="boom"):
    engine.bash("echo hi", on=root)
  after: list[tuple] = []
  engine.drive(keeping(after), "broken")
  engine.bash("echo again", on=root)
  await settle()
  first, again = said(log, "bash")
  assert [a[1] for a in said(heard, "started")] == [a[1] for a in said(after, "started")] == [first[1], again[1]]
  assert [e[0] for e in sand.record if e[0][0] == "bash"] == [first, again]


async def test_a_generator_brought_to_life_under_a_name_and_nothing_more() -> None:
  """A generator brought to life under a name and nothing more: it hears from the tip and runs to its first wait, and one born while a fact goes round hears from the next."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  fresh: list[tuple] = []
  later: list[tuple] = []
  at = len(log)
  engine.drive(keeping(fresh, ("done", "none://tip", None)), "tip")
  assert said(log, "done")[-1][1] == "none://tip"

  def bearing() -> Generator[None, tuple | None]:
    """An ear that brings another to life while a fact goes round, so the newborn hears from the next fact on."""
    while (heard := (yield)) is not None:
      if heard[0] == "started":
        engine.drive(keeping(later), "later")

  engine.drive(bearing(), "bearing")
  engine.bash("echo hi", on=root)
  await settle()
  started = said(log[at:], "started")[0]
  assert fresh == [a for a in log[at:] if not engine.question(a)] and started in fresh
  assert later and started not in later
  assert later == [a for a in log[log.index(started) + 1 :] if not engine.question(a)]


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


async def test_an_ear_yields_nothing() -> None:
  """An ear yields nothing, and each yield waits for what it hears next, the next fact said or the next question offered to it."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  got: list[object] = []

  def asking() -> Generator[None, tuple | None]:
    """An ear that says one fact of its own by say, and then waits for what it hears after it."""
    got.append(engine.say("tell", root, [("noted", [], None)]))
    while True:
      got.append((yield))

  at = len(log)
  engine.drive(asking(), "asker")
  assert got[0] == ("tell", root, "asker", [("noted", [], None)]) == log[at]
  engine.bash("echo hi", on=root)
  note = engine.act("note", root, None)
  await settle()
  assert got[1:] == [a for a in log[at:] if not engine.question(a) or a[1] == note]


async def test_drive_refuses_a_name_that_an_ear_hears_by_already() -> None:
  """drive refuses a name that an ear hears by already, and the name of the operator, which is a site and never an ear."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  heard: list[tuple] = []
  for name in ("world", "record", OPERATOR):
    with pytest.raises(Refused, match=rf"^{name} hears$"):
      engine.drive(keeping(heard), name)
  engine.drive(keeping(heard), "keeper")
  with pytest.raises(Refused, match=r"^keeper hears$"):
    engine.drive(keeping([]), "keeper")
  engine.bash("echo hi", on=root)
  await settle()
  assert said(sand.calls, "bash") == said(log, "bash") and said(heard, "started") == [("started", "bash1", WORLD)]

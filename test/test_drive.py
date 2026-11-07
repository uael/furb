"""drive, the other way to speak: a generator brought to life under a name."""

from collections.abc import Generator

import pytest

from conftest import Sand, born, keeping, relived, said, settle, verbs, viewed
from furb import engine
from furb.engine import OPERATOR, Refused


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
  """The other way to speak: a generator is brought to life under a name, and from then it hears every fact that it waits for and says its own."""
  _, log, root = born()
  heard: list[tuple] = []
  at = len(log)
  engine.drive(keeping(heard, ("done", "none://one", None)), "keeper")
  engine.bash("echo hi", on=root)
  assert log[at] == ("done", "none://one", "keeper", None)
  assert heard == log[at:]


async def test_one_that_returns_is_over_and_lives_no_more() -> None:
  """One that returns is over and lives no more, which is how a thing that watches for one fact alone is dropped the moment it hears it."""
  _, log, root = born()
  mark: list[tuple] = []
  engine.drive(ends(mark), "once")
  engine.bash("echo hi", on=root)
  engine.bash("echo again", on=root)
  assert len(mark) == 1 and len(said(log, "bash")) == 2


async def test_one_that_raises_while_it_hears_is_dropped_at_once_and_the_one_that_spoke_goes_on() -> None:
  """One that raises while it hears is dropped at once, and the one that spoke goes on: the engine says the done of the act it is the ear of, with a refusal that names it and what it raised, or the fact dropped that names them when no open act is its own, so a replay that gives it the same facts says the same."""
  sand, log, root = born()
  await verbs(root, "touchy", "shut")
  heard: list[tuple] = []
  engine.drive(keeping(heard), "before")
  engine.drive(breaks(), "broken")
  engine.bash("echo hi", on=root)
  (drop,) = [a for a in log if a[0] == "dropped"]
  assert drop[1:3] == ("broken", "broken") and str(drop[3]) == "broken raised ValueError('boom')"
  after: list[tuple] = []
  engine.drive(keeping(after), "broken")
  engine.bash("echo last", on=root)
  assert said(heard, "bash") == said(log, "bash") and said(after, "bash") == said(log, "bash")[1:]

  def touchy(id: str) -> Generator[tuple | None, tuple | None]:
    yield "started", id
    yield
    raise TypeError("touchy")

  owned = engine.act("touchy", root, touchy)
  sand.files["/w/a.txt"] = "one\n"
  assert engine.read("a.txt", on=root).content == "one\n"
  got = engine.peek(owned)
  assert isinstance(got, Refused) and str(got) == f"{owned} raised TypeError('touchy')"
  assert [a for a in log if a[0] == "dropped"] == [drop]

  def shut(id: str) -> Generator[tuple | None, tuple | None]:
    yield "started", id
    yield
    engine.close(7, id)

  closed = engine.act("shut", root, shut)
  engine.read("a.txt", on=root)
  # monty knows no CancelledError, so there the close ends the act with the refusal of that fault.
  assert engine.peek(closed) == 7 or "CancelledError is no class" in str(engine.peek(closed))
  assert [a for a in log if a[0] == "dropped"] == [drop]
  await raises_again()


async def test_a_generator_brought_to_life_under_a_name_and_nothing_more() -> None:
  """A generator brought to life under a name and nothing more: it hears from the tip and runs to its first wait, and one born while a fact goes round hears from the next."""
  _, log, root = born()
  fresh: list[tuple] = []
  later: list[tuple] = []
  at = len(log)
  engine.drive(keeping(fresh, ("done", "none://tip", None)), "tip")
  assert said(log, "done")[-1][1] == "none://tip"

  def bearing() -> Generator[tuple | None, tuple | None]:
    """An ear that brings another to life while a fact goes round, so the newborn hears from the next fact on."""
    while (heard := (yield)) is not None:
      if heard[0] == "bash":
        engine.drive(keeping(later), "later")

  engine.drive(bearing(), "bearing")
  engine.bash("echo hi", on=root)
  command = said(log, "bash")[0]
  assert fresh == log[at:] and command in fresh
  assert later and command not in later and later == log[log.index(command) + 1 :]


async def test_it_lives_until_it_returns() -> None:
  """It lives until it returns, and an act that hears nothing more returns at the first fact it hears after its own end."""
  _, log, root = born()
  heard: list[tuple] = []
  engine.drive(keeping(heard), "keeper")
  act = engine.prompt(int, "hi", to="operator", on=root)
  await settle()
  with pytest.raises(Refused, match="hears"):
    engine.drive(engine.idle(act), act)
  engine.close(1, act)
  await settle()
  engine.drive(engine.idle(act), act)
  assert heard and heard[-1] == log[-1]


async def test_a_generator_that_yields_a_saying_is_given_the_fact_as_the_bus_said_it() -> None:
  """A generator that yields a saying is given the fact as the bus said it, one that yields a set of names waits for the next fact that names one of them, and one that yields nothing waits for the next fact said."""
  _, log, root = born()
  got: list[object] = []

  def asking() -> Generator[tuple | None, tuple | None]:
    """An ear that says one saying of its own and then waits for what is said after it."""
    got.append((yield "tell", root, ["noted"]))
    while True:
      got.append((yield))

  at = len(log)
  engine.drive(asking(), "asker")
  assert got[0] == ("tell", root, "asker", ["noted"]) == log[at]
  engine.bash("echo hi", on=root)
  await settle()
  assert got[1:] == log[at:]


async def raises_again() -> None:
  """A replay of a life whose ears raised, in a delivery and in an act made again, says the same facts as that life, also at the last entry, and ends at a raise before the act that it makes again exists."""
  probe, _, top = born()
  word = (
    "b = bash('echo hi')\n"
    "def noisy(id):\n  yield 'started', id\n  while True:\n    a = yield {b}\n"
    "    if a[0] == 'done':\n      raise TypeError('noisy')\n"
    "n = act('noisy', '', noisy)\nawait wait(0.05, b)\nwent = peek(b).code\nlast = bash('echo ' + str(went))\nawait last"
  )
  await engine.rung(word, on=top)
  await settle()
  live, b = viewed(top), engine.module(top)["b"]
  assert live["modules"][top]["went"] == "0" and isinstance(engine.peek(engine.module(top)["n"]), Refused)
  _, over = await relived(Sand(), list(probe.record))
  later = viewed(over)
  assert {k: v[: len(live["facts"][k])] for k, v in later["facts"].items() if k in live["facts"]} == live["facts"]
  assert live["modules"] == later["modules"]
  assert {k: later["outcomes"].get(k) for k in live["outcomes"]} == live["outcomes"]
  cut = next(n for n, (e,) in enumerate(probe.record) if e[:2] == ("done", b))
  door, _, top = born()
  await engine.rung(
    "def touchy(id):\n  yield 'started', id\n  yield {'read'}\n  raise TypeError('touchy')\nt = act('touchy', '', touchy)",
    on=top,
  )
  await settle()
  with pytest.raises(Refused, match="nothing takes read"):
    engine.read("a", on=top)
  assert str(engine.peek(engine.module(top)["t"])).endswith("raised TypeError('touchy')")
  after = engine.bash("echo after", on=top)
  await settle()
  live = viewed(top)
  _, over = await relived(Sand(), list(door.record))
  later = viewed(over)
  assert {k: v[: len(live["facts"][k])] for k, v in later["facts"].items() if k in live["facts"]} == live["facts"]
  assert {k: later["outcomes"].get(k) for k in live["outcomes"]} == live["outcomes"] and after in live["outcomes"]
  sand, _, top = born()
  with pytest.raises(Refused):
    engine.read("missing", on=top)
  record = [((*e, "x", "y") if e[0] == "read" and e[2] == OPERATOR else e,) for (e,) in sand.record]
  with pytest.raises(TypeError):
    await relived(Sand(), record)
  first, second = list(probe.record)[: cut + 1], Sand()
  _, again = await relived(second, first)
  live = viewed(again)
  _, over = await relived(Sand(), [*first, *second.record])
  later = viewed(over)
  assert {k: v[: len(live["facts"][k])] for k, v in later["facts"].items() if k in live["facts"]} == live["facts"]
  assert engine.module(over)["last"] == engine.module(again)["last"]

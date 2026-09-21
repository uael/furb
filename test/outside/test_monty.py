"""The door to the engine of monty: what it carries that no sentence of the contract says.

The suite proves the contract on both engines, sentence for sentence. What is proved here is the door itself: an
ear of this interpreter that says a verb from its thread and is answered with what the verb raised, a show the
engine made that an ear calls back from its thread, a map of the life read where it stands, and the Kernel of
this interpreter refused, since the engine of monty holds its own. And what that Kernel makes of a word, which
the contract leaves to it: a run that retells a rung it kept is answered from what the rung left, so the word
runs no second time, a life dumped carries what the Kernel kept, and a run it could not keep runs its word.
"""

from collections.abc import Generator

import pytest

import furb
import furb_monty.engine
from conftest import OPERATOR, STANDS, Dead, Py, Sand, settle, swapped
from furb import engine
from furb.engine import Refused


def asking() -> Generator[tuple | None, tuple | None]:
  """An ear that hears and says nothing."""
  while True:
    yield


def hearing(log: list[tuple]) -> Generator[tuple | None, tuple | None]:
  """An ear that keeps every fact it hears and says nothing."""
  while True:
    if (a := (yield)) is not None:
      log.append(a)


@pytest.fixture(autouse=True)
def on_monty() -> Generator[None]:
  """Every name this module reads the engine by, bound to the engine of monty for the length of the test."""
  swapped(furb_monty.engine)
  yield
  swapped(furb.python)


async def test_an_ear_that_says_a_verb_the_world_refuses_is_answered_with_the_refusal() -> None:
  """A verb an ear says from its thread raises in the ear what it raised in the life, where the ear said it."""
  caught: list[str] = []

  def poked() -> Generator[tuple | None, tuple | None]:
    while True:
      if (a := (yield)) is not None and a[0] == "poke":
        try:
          engine.read("a.txt", on=a[1])
        except Refused as no:
          caught.append(str(no))

  root = engine.boot((), world=Dead(stands=STANDS).hears(), asking=poked())
  engine.send("poke", root, by=OPERATOR)
  await settle()
  assert caught == ["a dead World answers no read"]


async def test_a_show_the_engine_made_is_called_back_from_the_thread_of_an_ear() -> None:
  """A show the engine made crosses to an ear as a callable, which the ear calls back from its own thread."""
  sand = Sand(files={"/w/n.txt": "one\ntwo\n"}, stands=STANDS)
  picked: list[list[int]] = []

  def looking() -> Generator[tuple | None, tuple | None]:
    while True:
      if (a := (yield)) is not None and a[0] == "tell":
        picked.extend(show(text.lines) for tag in a[3] if tag[0] == "read" for text, show in tag[2])

  root = engine.boot((), world=sand.hears(), looking=looking())
  sand.script[root] = ["read('n.txt', span(2, 2))\nclose(1)"]
  assert await engine.prompt(int, "read it", on=root) == 1
  assert picked == [[2]]


async def test_a_map_of_the_life_refuses_a_key_it_does_not_hold() -> None:
  """A map of the life, read where it stands, raises KeyError for a key it does not hold, and says which map it is."""
  engine.boot((), world=Sand(stands=STANDS).hears())
  with pytest.raises(KeyError):
    engine.modules["chain://none"]
  with pytest.raises(KeyError):
    engine.acts["none://x"]
  assert repr(engine.acts) == "acts of the life"


async def test_a_life_dumped_where_it_stands_still_is_restored_and_goes_on() -> None:
  """A life dumped where it stands still is restored on ears under the names it was dumped with, and goes on from
  there: nothing of the record is replayed, no model is asked, and an ear hears from the next fact on."""
  sand = Sand(files={"/w/a.txt": "one\ntwo\n"}, stands=STANDS)
  root = engine.boot((), world=sand.hears())
  await engine.rung("k = len(read('a.txt').lines)", on=root)
  await settle()
  dump = furb_monty.engine.dump()
  later = Sand(stands=STANDS)
  assert furb_monty.engine.restore(dump, world=later.hears()) == root
  assert engine.modules[root]["k"] == 2
  assert await engine.rung("close(k + 1)", on=root) == 3
  assert later.calls == []
  with pytest.raises(Refused, match="names it was dumped with"):
    furb_monty.engine.restore(dump, world=Sand(stands=STANDS).hears(), probe=asking())


async def test_a_dump_is_no_dump_of_a_life_when_it_is_not_one() -> None:
  """What is not a dump of a life is refused as one."""
  with pytest.raises(Refused, match="no dump of a life"):
    furb_monty.engine.restore(b"not a dump", world=Sand(stands=STANDS).hears())


async def test_boot_refuses_a_kernel_or_a_gate_of_this_interpreter() -> None:
  """boot refuses a Kernel or a gate of this interpreter, since the engine of monty holds its own."""
  with pytest.raises(Refused, match="kernel hears: the engine of monty holds its Kernel and its gate"):
    engine.boot((), world=Sand(stands=STANDS).hears(), kernel=Py().kernel())
  with pytest.raises(Refused, match="gate hears: the engine of monty holds its Kernel and its gate"):
    engine.boot((), world=Sand(stands=STANDS).hears(), gate=Py().gating())


def debugged(log: list[tuple]) -> list[str]:
  """Every value a word debugged in the life, in order, which a word says once for each time it runs."""
  return [
    value for a in log if a[0] == "tell" for name, attrs, _ in a[3] if name == "debugged" for _, value in attrs[1:]
  ]


async def test_a_run_that_retells_a_rung_the_kernel_kept_is_answered_from_what_it_left() -> None:
  """A run that retells a rung the Kernel kept is answered from what the rung left: the module of the chain holds
  the bindings again, a copy of them, and the word runs no second time, so what it debugs is debugged once."""
  sand = Sand(files={"/w/a.txt": "one\ntwo\n"}, stands=STANDS)
  log: list[tuple] = []
  root = engine.boot((), world=sand.hears(), probe=hearing(log))
  await engine.rung("xs = [len(read('a.txt').lines)]\ndebug(t\"{xs}\")", on=root)
  await engine.rung('xs.append(3)\ndebug(t"{xs}")', on=root)
  twin = engine.chain("twin", source=root)
  await settle(300)
  assert debugged(log) == [[2], [2, 3]]
  assert engine.modules[twin]["xs"] == [2, 3]
  assert [a[5] != "" for a in log if a[0] == "run"] == [False, False, True, True]
  assert await engine.rung("xs.append(4)\nclose(xs)", on=twin) == [2, 3, 4]
  assert engine.modules[root]["xs"] == [2, 3]


async def test_a_run_that_a_close_stopped_is_answered_as_a_run_that_came_to_nothing() -> None:
  """A run that a close stopped is kept with nothing, so the copy of it comes to nothing, as it does under a Kernel
  that runs the word; a run that raised is kept with what it raised, so the copy raises it again."""
  sand = Sand(stands=STANDS)
  log: list[tuple] = []
  root = engine.boot((), world=sand.hears(), probe=hearing(log))
  sand.script[root] = ["k = 21\nclose(k)", "close(None)"]
  assert await engine.prompt(int, "count", on=root) == 21
  with pytest.raises(ValueError, match="boom"):
    await engine.rung("raise ValueError('boom')", on=root)
  twin = engine.chain("twin", source=root)
  await settle(300)
  theirs = [a[1] for a in log if a[0] == "rung" and a[3] == twin]
  assert [type(engine.outcomes[one]).__name__ for one in theirs] == ["NoneType", "ValueError"]
  assert [a[5] != "" for a in log if a[0] == "run"] == [False, False, True, True]
  assert type(engine.modules[twin]["raised"]).__name__ == "ValueError" and engine.modules[twin]["k"] == 21


async def test_a_run_the_kernel_could_not_keep_runs_its_word_again() -> None:
  """A run whose bindings cannot be copied is kept not, so a run that retells it runs the word, and the life is the
  one a Kernel that runs every word gives."""
  sand = Sand(stands=STANDS)
  log: list[tuple] = []
  root = engine.boot((), world=sand.hears(), probe=hearing(log))
  await engine.rung('g = (n for n in range(3))\nk = next(g)\ndebug(t"{k}")', on=root)
  twin = engine.chain("twin", source=root)
  await settle(300)
  assert debugged(log) == [0, 0]
  assert engine.modules[twin]["k"] == 0
  assert await engine.rung("close(next(g))", on=twin) == 1


async def test_a_life_dumped_carries_what_the_kernel_kept() -> None:
  """A life dumped carries what the Kernel kept of every run, so a chain with a source made in the life restored
  is answered from it, and no word of the life before runs again."""
  sand = Sand(files={"/w/a.txt": "one\ntwo\n"}, stands=STANDS)
  before: list[tuple] = []
  root = engine.boot((), world=sand.hears(), probe=hearing(before))
  await engine.rung("k = len(read('a.txt').lines)\ndebug(t\"{k}\")", on=root)
  await settle()
  assert debugged(before) == [2]
  dump = furb_monty.engine.dump()
  log: list[tuple] = []
  later = Sand(stands=STANDS)
  assert furb_monty.engine.restore(dump, world=later.hears(), probe=hearing(log)) == root
  twin = engine.chain("twin", source=root)
  await settle(300)
  assert engine.modules[twin]["k"] == 2 and debugged(log) == [] and later.calls == []
  assert [a[5] != "" for a in log if a[0] == "run"] == [True]

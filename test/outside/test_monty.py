"""The door to the engine of monty: what it carries that no sentence of the contract says.

The suite proves the contract on both engines, sentence for sentence. What is proved here is the door itself: an
ear of this interpreter that says a verb from its thread and is answered with what the verb raised, a show the
engine made that an ear calls back from its thread, a map of the life read where it stands, and the Kernel of
this interpreter refused, since the engine of monty holds its own.
"""

from collections.abc import Generator

import pytest

import furb
import furb_monty.engine
from conftest import OPERATOR, STANDS, Dead, Py, Sand, settle, swapped
from furb import engine
from furb.engine import Refused


@pytest.fixture(autouse=True)
def on_monty() -> Generator[None]:
  """Every name this module reads the engine by, bound to the engine of monty for the length of the test."""
  swapped(furb_monty.engine)
  yield
  swapped(furb.python)


async def test_an_ear_that_says_a_verb_the_world_refuses_is_answered_with_the_refusal() -> None:
  """A verb an ear says from its thread raises in the ear what it raised in the life, where the ear said it."""
  caught: list[str] = []

  def asking() -> Generator[tuple | None, tuple | None]:
    while True:
      if (a := (yield)) is not None and a[0] == "poke":
        try:
          engine.read("a.txt", on=a[1])
        except Refused as no:
          caught.append(str(no))

  root = engine.boot((), world=Dead(stands=STANDS).hears(), asking=asking())
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


async def test_boot_refuses_a_kernel_or_a_gate_of_this_interpreter() -> None:
  """boot refuses a Kernel or a gate of this interpreter, since the engine of monty holds its own."""
  with pytest.raises(Refused, match="kernel hears: the engine of monty holds its Kernel and its gate"):
    engine.boot((), world=Sand(stands=STANDS).hears(), kernel=Py().kernel())
  with pytest.raises(Refused, match="gate hears: the engine of monty holds its Kernel and its gate"):
    engine.boot((), world=Sand(stands=STANDS).hears(), gate=Py().gating())

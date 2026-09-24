"""The door to the engine of monty: what it carries that no sentence of the contract says.

The suite proves the contract on both engines, sentence for sentence. What is proved here is the door itself: an
ear of this interpreter that says a verb from its thread and is answered with what the verb raised, a show the
engine made that an ear calls back from its thread, a class a word defined held as a type of this interpreter
and its instances as objects of it, both ways, a map of the life read where it stands, and the Kernel of this
interpreter refused, since the engine of monty holds its own.
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
        picked.extend(show(text.lines) for note in a[3] if isinstance(note, tuple) for text, show in [note])

  root = engine.boot((), world=sand.hears(), looking=looking())
  sand.script[root] = ["read('n.txt', span(2, 2))\nclose(1)"]
  assert await engine.prompt(int, "read it", on=root) == 1
  assert picked == [[2]]


async def test_a_class_a_word_defined_is_a_type_of_this_interpreter_and_its_instances_are_objects_of_it() -> None:
  """A class a word defined crosses as a type of this interpreter, one for the class, whose call makes the instance
  in the sandbox; an instance of it crosses as an object of that type with its fields; and both go back in as
  themselves, the class by its handle and the instance made from its fields."""
  sand = Sand(stands=STANDS)
  root = engine.boot((), world=sand.hears())
  word = "class Plan:\n  n = 21\n  def __init__(self, x=1):\n    self.x = x\n  def total(self):\n    return self.n + self.x\np = Plan(3)"
  await engine.rung(word, on=root)
  plan = engine.modules[root]["Plan"]
  assert isinstance(plan, type) and plan.__name__ == "Plan" and plan is engine.modules[root]["Plan"]
  p = engine.modules[root]["p"]
  assert isinstance(p, plan) and vars(p) == {"x": 3}
  made = plan(5)
  assert isinstance(made, plan) and vars(made) == {"x": 5}
  assert await engine.rung("close(p.total())", on=root) == 24
  act = engine.prompt(None, "give", to=OPERATOR, on=root)
  engine.close(made, act)
  await settle()
  back = engine.outcomes[act]
  assert isinstance(back, plan) and vars(back) == {"x": 5}
  assert await engine.rung(f"close((isinstance(outcomes[{act!r}], Plan), outcomes[{act!r}].total()))", on=root) == (
    True,
    26,
  )
  cls = engine.prompt(None, "which", to=OPERATOR, on=root)
  engine.close(plan, cls)
  await settle()
  assert engine.outcomes[cls] is plan
  assert await engine.rung(f"close(outcomes[{cls!r}] is Plan)", on=root) is True
  twin = engine.chain("twin", source=root)
  await settle(300)
  theirs = engine.modules[twin]["Plan"]
  assert isinstance(theirs, type) and theirs is not plan and isinstance(engine.modules[twin]["p"], theirs)


async def test_a_class_a_word_defined_derives_from_the_type_its_base_is_here() -> None:
  """The type a class a word defined is here derives from the type its base is here: a builtin, a class of the
  engine, or the type of another class a word defined; so an exception class raises what the builtin it derives
  from catches, a subclass of a class of a word is one of its type, and a string of a class that inherits str is
  a string."""
  sand = Sand(stands=STANDS)
  root = engine.boot((), world=sand.hears())
  word = (
    "class Boom(ValueError):\n  pass\nclass Plan:\n  n = 21\nclass Sub(Plan):\n  pass\nclass Hurt(Refused):\n  pass\n"
    "class Word(str):\n  pass\ne = Boom('boom')\ns = Sub()\nw = Word('x')"
  )
  await engine.rung(word, on=root)
  boom, plan, sub, hurt = (engine.modules[root][name] for name in ("Boom", "Plan", "Sub", "Hurt"))
  assert isinstance(boom, type) and issubclass(boom, ValueError) and boom.__name__ == "Boom"
  assert isinstance(plan, type) and isinstance(sub, type) and issubclass(sub, plan) and sub is not plan
  assert isinstance(hurt, type) and issubclass(hurt, Refused)
  assert isinstance(engine.modules[root]["s"], sub) and isinstance(engine.modules[root]["s"], plan)
  assert engine.modules[root]["w"] == "x" and isinstance(engine.modules[root]["w"], str)
  with pytest.raises(ValueError, match="boom") as caught:
    await engine.rung("raise Boom('boom')", on=root)
  assert isinstance(caught.value, boom) and caught.value.args == ("boom",)
  e = engine.modules[root]["e"]
  assert isinstance(e, boom) and e.args == ("boom",)
  act = engine.prompt(None, "give", to=OPERATOR, on=root)
  engine.close(caught.value, act)
  await settle()
  told = await engine.rung(f"close((isinstance(outcomes[{act!r}], Boom), outcomes[{act!r}].args))", on=root)
  assert told == (True, ("boom",))


async def test_a_map_that_holds_the_key_is_crosses_both_ways_as_the_map_it_is() -> None:
  """A map of this interpreter and a map of a word that hold the key `is` cross as the maps they are, both ways, and
  never as a mark: the word reads the map a close of the operator gave, and a word gives its own map back."""
  root = engine.boot((), world=Sand(stands=STANDS).hears())
  refusal = {"is": "Refused", "args": ["x"]}
  act = engine.prompt(None, "give", to=OPERATOR, on=root)
  engine.close(refusal, act)
  await settle()
  assert engine.outcomes[act] == refusal
  assert await engine.rung(f"close(outcomes[{act!r}] == {refusal!r})", on=root) is True
  verb = {"is": "name", "name": "bash"}
  assert await engine.rung(f"close({verb!r})", on=root) == verb


async def test_a_map_of_the_life_refuses_a_key_it_does_not_hold() -> None:
  """A map of the life, read where it stands, raises KeyError for a key it does not hold, and says which map it is."""
  engine.boot((), world=Sand(stands=STANDS).hears())
  with pytest.raises(KeyError):
    engine.modules["chain9"]
  with pytest.raises(KeyError):
    engine.acts["none9"]
  assert repr(engine.acts) == "acts of the life"


async def test_boot_refuses_a_kernel_or_a_gate_of_this_interpreter() -> None:
  """boot refuses a Kernel or a gate of this interpreter, since the engine of monty holds its own."""
  with pytest.raises(Refused, match="kernel hears: the engine of monty holds its Kernel and its gate"):
    engine.boot((), world=Sand(stands=STANDS).hears(), kernel=Py().kernel())
  with pytest.raises(Refused, match="gate hears: the engine of monty holds its Kernel and its gate"):
    engine.boot((), world=Sand(stands=STANDS).hears(), gate=Py().gating())


async def test_the_gate_of_the_sandbox_finds_what_the_run_finds_of_a_name_the_program_bound_again() -> None:
  """A rung that bound a name of the engine again leaves that value to the word after it in the sandbox too, so the
  word raises when it calls it, and the gate of the sandbox refuses a word that calls it where it can see it."""
  root = engine.boot((), world=Sand(stands=STANDS).hears())
  assert await engine.rung("read = 1", on=root) is None
  with pytest.raises(TypeError, match="not callable"):
    await engine.rung("f: Any = read\nclose(f('a'))", on=root)
  assert engine.gate("close(read('a'))", on=root)[0].startswith("line 1: error[call-non-callable]")
  with pytest.raises(Refused):
    await engine.rung("close(read('a'))", on=root)

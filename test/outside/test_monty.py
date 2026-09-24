"""The door to the engine of monty: what it carries that no sentence of the contract says.

The suite proves the contract on both engines, sentence for sentence. What is proved here is the door itself: an
ear of this interpreter that says a verb from its thread and is answered with what the verb raised, a show the
engine made that an ear calls back from its thread, a class a word defined held as a type of this interpreter
and its instances as objects of it, both ways, a map of the life read where it stands, the Kernel of this
interpreter refused, since the engine of monty holds its own, and a gate that accepts a builtin or a name of a module
exactly when the sandbox runs it.
"""

import builtins
from collections.abc import Generator

import pytest

import furb
import furb_monty.engine
from conftest import OPERATOR, STANDS, Dead, Py, Sand, plain, said, settle, swapped
from furb import engine, kernel
from furb.engine import Drift, Refused


def asking() -> Generator[tuple | None, tuple | None]:
  """An ear that hears and says nothing."""
  while True:
    yield


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


async def test_a_life_dumped_where_it_stands_still_is_restored_and_goes_on() -> None:
  """A life dumped where it stands still is restored on ears under the names it was dumped with and on the record
  the World holds, and goes on from there: nothing of the record is replayed, no model is asked, the World is asked
  what it stands on, as at the tip of a later life, and an ear hears from the next fact on."""
  sand = Sand(files={"/w/a.txt": "one\ntwo\n"}, stands=STANDS)
  root = engine.boot((), world=sand.hears())
  await engine.rung("k = len(read('a.txt').lines)", on=root)
  await settle()
  dump = furb_monty.engine.dump()
  record = plain(sand.record)
  later = Sand(stands=STANDS)
  assert furb_monty.engine.restore(dump, record, world=later.hears()) == root
  assert engine.modules[root]["k"] == 2
  assert await engine.rung("close(k + 1)", on=root) == 3
  assert [a[0] for a in later.calls] == ["stand"]
  with pytest.raises(Refused, match="names it was dumped with"):
    furb_monty.engine.restore(dump, record, world=Sand(stands=STANDS).hears(), probe=asking())


async def test_a_dump_is_no_dump_of_a_life_when_it_is_not_one() -> None:
  """What is not a dump of a life is refused as one."""
  with pytest.raises(Refused, match="no dump of a life"):
    furb_monty.engine.restore(b"not a dump", world=Sand(stands=STANDS).hears())


def restamped(dump: bytes, name: bytes) -> bytes:
  """A dump whose stamp says another value on the line of this name."""
  head, _, session = dump.partition(b"\n\n")
  lines = [
    line.split(b" ")[0] + b" " + b"0" * len(line.split(b" ")[1]) if line.split(b" ")[0] == name else line
    for line in head.split(b"\n")
  ]
  return b"\n".join(lines) + b"\n\n" + session


async def test_a_dump_that_does_not_match_its_record_its_engine_or_its_build_is_refused() -> None:
  """A dump is restored only on the record it was dumped on, by the engine and the build of the crate that made it,
  and a restore refuses any other with each part that differs, so the host boots instead."""
  sand = Sand(stands=STANDS)
  root = engine.boot((), world=sand.hears())
  engine.rung("k = 1", on=root)
  await settle()
  record = plain(sand.record)
  dump = furb_monty.engine.dump()
  with pytest.raises(Refused, match=r"^the dump does not match: it is of a record of \d+ entries that end with "):
    furb_monty.engine.restore(dump, record[:-1], world=Sand(stands=STANDS).hears())
  with pytest.raises(Refused, match=r"^the dump does not match: it holds another engine$"):
    furb_monty.engine.restore(restamped(dump, b"engine"), record, world=Sand(stands=STANDS).hears())
  with pytest.raises(Refused, match=r"^the dump does not match: another build of the crate made it$"):
    furb_monty.engine.restore(restamped(dump, b"build"), record, world=Sand(stands=STANDS).hears())
  assert furb_monty.engine.restore(dump, record, world=Sand(stands=STANDS).hears()) == root


async def test_a_restored_life_holds_its_pending_work_unstarted_until_a_wake_as_a_booted_life_does() -> None:
  """A life restored from a dump of a life that boot left with pending work holds that work unstarted until a wake
  this life says, as a life booted on the same record does, and at the wake both start the same work and ask the
  same model with the same turns."""
  first = Sand(stands=STANDS, auto=False)
  root = engine.boot((), world=first.hears())
  engine.bash("sleep 9", on=root)
  engine.wait(100.0, on=root)
  engine.prompt(str, "why?", to=OPERATOR, on=root)
  engine.prompt(int, "count", on=root)
  await settle()
  record = plain(first.record)
  engine.boot(record, world=Sand(stands=STANDS).hears())
  await settle()
  dump = furb_monty.engine.dump()
  lives: list[tuple[Sand, list]] = []
  for restored in (False, True):
    sand = Sand(stands=STANDS, script={root: ["close(4)"]})
    if restored:
      furb_monty.engine.restore(dump, record, world=sand.hears())
    else:
      engine.boot(record, world=sand.hears())
    await settle()
    assert said(sand.calls, "start") == [] and said(sand.calls, "ask") == []
    engine.wake(root)
    await settle()
    lives.append((sand, engine.turns(on=root)))
  (booted, turns), (again, returns) = lives
  assert [a[1] for a in said(booted.calls, "start")] == ["bash1", "wait1", "prompt1"]
  assert said(again.calls, "start") == said(booted.calls, "start")
  assert len(said(booted.calls, "ask")) == 1
  assert said(again.calls, "ask") == said(booted.calls, "ask")
  assert again.record == booted.record
  assert returns == turns


async def test_a_life_whose_boot_raised_is_dumped_never() -> None:
  """A life whose boot raised a drift keeps nothing more, and a restore raises nothing, so it is refused its dump."""
  sand = Sand(stands=STANDS)
  root = engine.boot((), world=sand.hears())
  await engine.rung("bash('echo a')", on=root)
  await settle()
  record: list = [((*e[0][:4], "echo b", *e[0][5:]),) if e[0][0] == "bash" else e for e in plain(sand.record)]
  with pytest.raises(Drift, match="bash1 drifts"):
    engine.boot(record, world=Sand(stands=STANDS).hears())
  with pytest.raises(
    Refused, match=r"^a life is dumped where it stands still, and its boot raised Drift: bash1 drifts$"
  ):
    furb_monty.engine.dump()


async def test_a_life_that_holds_an_ear_the_host_gave_it_after_its_boot_is_dumped_never() -> None:
  """An ear of the host that crossed in after the boot is no ear a restored life is given, so a life that holds one
  alive is refused its dump, and one whose ear is over is dumped."""

  def watching() -> Generator[tuple | None, tuple | None]:
    while (a := (yield)) is None or a[0] != "poke":
      pass

  root = engine.boot((), world=Sand(stands=STANDS).hears())
  engine.drive(watching(), "watching")
  with pytest.raises(Refused, match=r"an ear the host gave it lives: ear:\d+$"):
    furb_monty.engine.dump()
  engine.send("poke", root)
  await settle()
  assert furb_monty.engine.dump().startswith(b"furb dump\n")


async def test_boot_refuses_a_kernel_or_a_gate_of_this_interpreter() -> None:
  """boot refuses a Kernel or a gate of this interpreter, since the engine of monty holds its own."""
  with pytest.raises(Refused, match="kernel hears: the engine of monty holds its Kernel and its gate"):
    engine.boot((), world=Sand(stands=STANDS).hears(), kernel=Py().kernel())
  with pytest.raises(Refused, match="gate hears: the engine of monty holds its Kernel and its gate"):
    engine.boot((), world=Sand(stands=STANDS).hears(), gate=Py().gating())


async def test_the_gate_accepts_a_builtin_or_a_name_of_a_module_exactly_when_a_rung_runs_it() -> None:
  """The gate reads a word against the typeshed of the sandbox and a chain of the sandbox, which is a module, so it
  accepts a name of the builtins of python, or a name ty gives every module, exactly when a rung runs a word that
  names it. A refused word never runs, so a refused name is run as the Kernel runs a word, in the globals of the
  chain. A word is judged the same on both engines, so the gate of python finds the same of every name, though
  python runs some that it refuses."""
  root = engine.boot((), world=Sand(stands=STANDS).hears())
  # The names ty gives every module are those of module_type_implicit_global_symbol in ty_python_semantic: the names
  # that the typeshed of the sandbox declares in the class types.ModuleType, but __dict__, __init__ and __getattr__,
  # and __builtins__, __debug__ and __warningregistry__, which ty adds itself.
  module = {"__name__", "__file__", "__loader__", "__package__", "__path__", "__spec__", "__doc__", "__annotations__"}
  module |= {"__annotate__", "__builtins__", "__debug__", "__warningregistry__"}
  names = sorted({*vars(builtins), *module})
  # Every rung grows the program that each later sheet reads again, so a sheet and a rung for each name cost
  # seconds. One word holds every name, one on each line, and the line of a finding is the name it refuses.
  word = "\n".join(f"got = {name}" for name in names)
  found = engine.gate(word, on=root)
  assert kernel.gate(word, []) == found
  refused = sorted({names[int(one.split(":")[0].removeprefix("line ")) - 1] for one in found})
  probe = (
    f"ran = []\nfor name in {refused!r}:\n  try:\n    eval(compile('got = ' + name, 'probe', 'exec'), dict(globals()))\n"
    "    ran.append(name)\n  except NameError:\n    pass\nclose(ran)"
  )
  tries = "".join(
    f"try:\n  got = {name}\nexcept NameError:\n  unbound.append({name!r})\n" for name in names if name not in refused
  )
  refused_yet_ran = await engine.rung(probe, on=root)
  accepted_yet_unbound = await engine.rung(f"unbound = []\n{tries}close(unbound)", on=root)
  assert (refused_yet_ran, accepted_yet_unbound) == ([], [])


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

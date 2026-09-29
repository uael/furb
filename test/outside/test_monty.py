"""The door to the engine of monty: what it carries that no sentence of the contract says.

The suite proves the contract on both engines, sentence for sentence. What is proved here is the door itself: an
ear of this interpreter that says a verb while it hears and is answered with what the verb raised, a show the
engine made that an ear calls back while it hears, a class a word defined held as a type of this interpreter
and its instances as objects of it, both ways, the Kernel of this interpreter refused, since the engine of monty
holds its own, a gate that accepts a builtin or a name of a module exactly when the sandbox runs it, an ear the
crate writes, which serves a life of either engine, and a life longer than the tables of its session could count.
What the ears of the crate do, the crate proves.
"""

import builtins
from collections.abc import Generator
from pathlib import Path

import pytest

import furb
import furb_monty.engine
from conftest import OPERATOR, STANDS, Dead, Py, Sand, settle, swapped
from furb import engine
from furb.engine import Refused
from furb_monty import _monty


@pytest.fixture(autouse=True)
def on_monty() -> Generator[None]:
  """Every name this module reads the engine by, bound to the engine of monty for the length of the test."""
  swapped(furb_monty.engine)
  yield
  swapped(furb.python)


async def test_an_ear_that_says_a_verb_the_world_refuses_is_answered_with_the_refusal() -> None:
  """A verb an ear says while it hears raises in the ear what it raised in the life, where the ear said it."""
  caught: list[str] = []

  def asking() -> Generator[tuple | None, tuple | None]:
    while True:
      if (a := (yield)) is not None and a[0] == "poke":
        try:
          engine.read("a.txt", on=a[1])
        except Refused as no:
          caught.append(str(no))

  root = engine.boot((), world=Dead(stands=STANDS).hears(), asking=asking())
  engine.say("poke", root)
  await settle()
  assert caught == ["a dead World answers no read"]


async def test_a_show_the_engine_made_is_called_back_by_an_ear_while_it_hears() -> None:
  """A show the engine made crosses to an ear as a callable, which the ear calls back while it hears."""
  sand = Sand(files={"/w/n.txt": "one\ntwo\n"}, stands=STANDS)
  picked: list[list[int]] = []

  def looking() -> Generator[tuple | None, tuple | None]:
    while True:
      if (a := (yield)) is not None and a[0] == "shown":
        picked.append(a[3](["one", "two"]))

  root = engine.boot((), world=sand.hears(), looking=looking())
  sand.script[root] = ["say('shown', acting(), span(2, 2))\nclose(1)"]
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
  plan = engine.module(root)["Plan"]
  assert isinstance(plan, type) and plan.__name__ == "Plan" and plan is engine.module(root)["Plan"]
  p = engine.module(root)["p"]
  assert isinstance(p, plan) and vars(p) == {"x": 3}
  made = plan(5)
  assert isinstance(made, plan) and vars(made) == {"x": 5}
  assert await engine.rung("close(p.total())", on=root) == 24
  act = engine.prompt(None, "give", to=OPERATOR, on=root)
  engine.close(made, act)
  await settle()
  back = engine.peek(act)
  assert isinstance(back, plan) and vars(back) == {"x": 5}
  assert await engine.rung(f"got = peek({act!r})\nclose(isinstance(got, Plan) and got.total())", on=root) == 26
  cls = engine.prompt(None, "which", to=OPERATOR, on=root)
  engine.close(plan, cls)
  await settle()
  assert engine.peek(cls) is plan
  assert await engine.rung(f"close(peek({cls!r}) is Plan)", on=root) is True
  twin = engine.chain("twin", source=root)
  await settle(300)
  theirs = engine.module(twin)["Plan"]
  assert isinstance(theirs, type) and theirs is not plan and isinstance(engine.module(twin)["p"], theirs)


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
  boom, plan, sub, hurt = (engine.module(root)[name] for name in ("Boom", "Plan", "Sub", "Hurt"))
  assert isinstance(boom, type) and issubclass(boom, ValueError) and boom.__name__ == "Boom"
  assert isinstance(plan, type) and isinstance(sub, type) and issubclass(sub, plan) and sub is not plan
  assert isinstance(hurt, type) and issubclass(hurt, Refused)
  assert isinstance(engine.module(root)["s"], sub) and isinstance(engine.module(root)["s"], plan)
  assert engine.module(root)["w"] == "x" and isinstance(engine.module(root)["w"], str)
  with pytest.raises(ValueError, match="boom") as caught:
    await engine.rung("raise Boom('boom')", on=root)
  assert isinstance(caught.value, boom) and caught.value.args == ("boom",)
  e = engine.module(root)["e"]
  assert isinstance(e, boom) and e.args == ("boom",)
  act = engine.prompt(None, "give", to=OPERATOR, on=root)
  engine.close(caught.value, act)
  await settle()
  told = await engine.rung(f"got = peek({act!r})\nclose(isinstance(got, Boom) and got.args)", on=root)
  assert told == ("boom",)


async def test_a_map_that_holds_the_key_is_crosses_both_ways_as_the_map_it_is() -> None:
  """A map of this interpreter and a map of a word that hold the key `is` cross as the maps they are, both ways, and
  never as a mark: the word reads the map a close of the operator gave, and a word gives its own map back."""
  root = engine.boot((), world=Sand(stands=STANDS).hears())
  refusal = {"is": "Refused", "args": ["x"]}
  act = engine.prompt(None, "give", to=OPERATOR, on=root)
  engine.close(refusal, act)
  await settle()
  assert engine.peek(act) == refusal
  assert await engine.rung(f"close(peek({act!r}) == {refusal!r})", on=root) is True
  verb = {"is": "name", "name": "bash"}
  assert await engine.rung(f"close({verb!r})", on=root) == verb


@pytest.mark.timeout(600)
async def test_a_life_runs_a_word_after_more_names_and_functions_than_a_u16_counts() -> None:
  """A life is one session of monty for its whole length, and the session keeps a name for each piece of code the host
  fed it, and each name, literal and function that a rung compiled. After more of each than a u16 counts, a word that
  names what the session never saw still runs, since the interpreter bounds an operand by the code that holds it."""
  root = engine.boot((), world=Sand(stands=STANDS).hears())
  # Each saying of the operator is one piece of code that the host feeds the session.
  for _ in range(66_000):
    engine.say("tick", root)
  # Each rung makes 10 000 functions that each read a new attribute and hold a function of their own. The source is
  # built as the rung runs, since the gate reads the whole program again for each word.
  for k in range(7):
    word = (
      f"exec(''.join(f'def f{k}_{{i}}(x):\\n  def g():\\n    pass\\n  return x.a{k}_{{i}}\\n'"
      " for i in range(10_000)), {})"
    )
    await engine.rung(word, on=root)
  word = "class Fresh:\n  fresh_attr = 1\ndef never_seen():\n  return Fresh()\nclose(never_seen().fresh_attr)"
  assert await engine.rung(word, on=root) == 1
  assert engine.module(root)["raised"] is None


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
  assert Py().gate(word, []) == found
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


async def test_an_ear_of_the_crate_serves_a_life_of_this_interpreter_and_what_it_says_from_a_thread_drives_it() -> None:
  """An ear of the crate crosses to the boot of the door beside the generators the contract says, and what it says
  from a thread of its own drives the life from the loop: time ends a wait from its thread."""
  root = furb_monty.engine.boot((), time=_monty.time(), world=Dead(stands=STANDS).hears())
  assert await engine.wait(0.01, on=root) is None


async def test_the_engine_of_this_interpreter_steps_an_ear_of_the_crate_as_a_generator_of_its_own(
  tmp_path: Path,
) -> None:
  """The engine of this interpreter steps an ear of the crate as it steps a generator: a verb the ear says is said to
  that engine, what the work of the ear says from a thread drives the life, and the record the store keeps reads
  back as the values it held."""
  swapped(furb.python)
  path = str(tmp_path / "record.jsonl")
  _, store = _monty.store(path)
  ears = {"files": _monty.files(), "time": _monty.time(), "store": store}
  world = Dead(stands=[STANDS[0], str(tmp_path), STANDS[2]]).hears()
  root = furb.python.boot((), kernel=Py().kernel(), gate=Py().gating(), **ears, world=world)
  furb.python.write(furb.python.Text("a.txt", "one\n"), on=root)
  assert furb.python.read("a.txt", on=root).content == "one\n"
  assert await furb.python.wait(0.01, on=root) is None
  for one in ears.values():
    one.dispose()
  kept = [fact for (fact,) in _monty.kept(path)]
  assert [fact[3] for fact in kept if fact[:2] == ["done", "read1"]] == [
    furb.python.Text(str(tmp_path / "a.txt"), "one\n")
  ]


async def test_the_door_opens_a_life_as_every_host_opens_one(tmp_path: Path) -> None:
  """A life opens on its record and on the ears of the crate, in the order every host boots them after its own, and
  enables at its tip what the configs turn on; a life that turns the extensions off enables nothing new."""
  path = str(tmp_path / "record.jsonl")
  record, ears = _monty.opened(directory=str(tmp_path), record=path, config=str(tmp_path / "config"))
  assert (record, [name for name, _ in ears]) == (
    [],
    ["provider", "extensions", "memory", "skills", "files", "bash", "time", "store"],
  )
  root = furb_monty.engine.boot(record, **dict(ears), world=Dead(stands=[STANDS[0], str(tmp_path), STANDS[2]]).hears())
  await settle()
  assert [one["name"] for one in _monty.enabled(engine.transcript(on=root))] == ["memory", "skills"]
  assert {a[0] for a in engine.transcript(on=root) if a[2] == "memory"} == {"done"}, "the memory ear answered"
  for _, one in ears:
    one.dispose()
  record, ears = _monty.opened(directory=str(tmp_path), record=path, keeps=False, extensions=False)
  assert [name for name, _ in ears] == ["provider", "extensions", "memory", "skills", "files", "bash", "time"]
  root = furb_monty.engine.boot(record, **dict(ears), world=Dead(stands=[STANDS[0], str(tmp_path), STANDS[2]]).hears())
  assert [one["name"] for one in _monty.enabled(engine.transcript(on=root))] == ["memory", "skills"]


async def test_the_door_names_the_work_that_an_earlier_life_left_pending(tmp_path: Path) -> None:
  """A later life on the record of a life that left a wait names that wait, which waits for a wake that this life
  says, as the engine of the crate finds it."""
  path = str(tmp_path / "record.jsonl")
  record, store = _monty.store(path)
  root = furb_monty.engine.boot(record, time=_monty.time(), store=store, world=Dead(stands=STANDS).hears())
  engine.wait(600, on=root)
  furb_monty.engine.boot(_monty.kept(path), time=_monty.time(), world=Dead(stands=STANDS).hears())
  assert furb_monty.engine.pending() == [("wait1", "wait")]


def test_the_door_gives_the_catalog_of_the_crate(tmp_path: Path) -> None:
  """The door gives the catalog of the crate, as the door to TypeScript gives it: the levels of effort, a model by
  its name whether this machine offers it or not, and the models this machine offers."""
  assert _monty.levels() == ["off", "minimal", "low", "medium", "high", "xhigh", "max"]
  efforts = ["low", "medium", "high", "xhigh", "max"]
  haiku = {"name": "claude-cli:haiku", "efforts": efforts, "window": 200000, "images": True, "price": None}
  assert _monty.model("claude-cli:haiku") == haiku
  assert _monty.model("nobody:none") is None
  claude = tmp_path / "claude"
  claude.write_text("#!/bin/sh\n")
  claude.chmod(0o755)
  opus = {"name": "claude-cli:opus", "efforts": efforts, "window": 1000000, "images": True, "price": None}
  assert opus in _monty.models(str(claude))

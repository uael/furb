"""The suite, with the engine itself in the sandbox, driven from here through the binding.

`wired.py` puts the Worlds of the harness behind the plain boundary and `kerneled.py` puts the Kernel of the crate
under every life, but both leave the engine in this interpreter. This rig moves the engine: every life the suite
opens is a life of `furb_sand`, which is the engine running in monty with the Kernel of the crate, and every verb
the suite calls is said as a word in there. The suite is not edited and never enters the sandbox: the tests, the
World doubles and the assertions all stand here, in python, as they always did.

    uv run python script/sanded.py

It takes the arguments of pytest after its own, so `uv run python script/sanded.py test/test_read.py` runs one
file. The binding must be built for this interpreter first, which `script/bound.py` does.

A World double of the harness calls back into the engine while it answers, which is what `engine.cwd(on=...)` in
the middle of a read is. Nothing may call into a life that stands waiting for it, so the double runs on a thread
of its own: it blocks where it asks, the boundary is told the question, and the answer wakes it. What the double
says when nothing asked it, which is every `loop.call_soon(engine.send, ...)`, goes through the Voice, which is
what the Voice is for.
"""

import asyncio
import os
import queue
import subprocess
import sys
import threading
from pathlib import Path

HERE = Path(__file__).resolve().parent
"""HERE is the directory of the rig, which holds the plugin it runs pytest with."""
ROOT = HERE.parent
"""ROOT is the root of the repository."""
SRC = ROOT / "src"
"""SRC holds the preamble, whose plain form both sides of the boundary read."""
BOUND = ROOT / "target" / "python"
"""BOUND is where `script/bound.py` puts the module of the binding for this interpreter."""

for one in (str(SRC), str(BOUND)):
  if one not in sys.path:
    sys.path.insert(0, one)

import furb_sand  # noqa: E402

from furb import engine as real  # noqa: E402

NAMES = dict(vars(real))
"""NAMES are the names of the engine, which is what a shape that crosses is made again by."""
PURE = ("question", "covers", "site", "modules")
"""PURE are the names of the engine that read no life, so this side answers them itself.

`scope`, `acts`, `asked` and `outcomes` are not among them: each reads the life, which stands in the sandbox.
"""
HELD = ("acts", "asked", "outcomes")
"""HELD are the maps of the life, which a test and a World both read, and which the sandbox holds."""
ASKED = {"cwd": 1, "merged": 3}
"""ASKED are the verbs a World reaches the engine by while it answers, and how many words each one carries."""


class Source(str):
  """One expression, already written out, which stands in a word exactly as it reads.

  A show and a filter never cross the boundary: they are made where the word runs. So `span(2, 2)` here is not a
  show but the text `span(2, 2)`, which the word carries and the engine reads in there.
  """


def made(got: object) -> object:
  """One plain value of the sandbox, made again as the engine of this interpreter holds it."""
  if isinstance(got, furb_sand.Shape):
    return NAMES[got.name](*[made(one) for one in got.fields.values()])
  if isinstance(got, furb_sand.Fault):
    return NAMES[got.name](*[made(one) for one in got.args])
  if isinstance(got, list):
    return [made(one) for one in got]
  if isinstance(got, tuple):
    return tuple(made(one) for one in got)
  if isinstance(got, dict):
    return {key: made(one) for key, one in got.items()}
  return got


def plainly(got: object) -> object:
  """One value of this interpreter, made plain, as the boundary carries it."""
  if isinstance(got, str):
    return str(got)
  if isinstance(got, (list, tuple)):
    return [plainly(one) for one in got]
  if isinstance(got, dict):
    return {key: plainly(one) for key, one in got.items()}
  if hasattr(got, "__dataclass_fields__") and type(got).__name__ in NAMES:
    fields = {one: plainly(getattr(got, one)) for one in got.__dataclass_fields__}
    return furb_sand.Shape(type(got).__name__, fields)
  if isinstance(got, BaseException):
    return furb_sand.Fault(type(got).__name__, *[plainly(one) for one in got.args])
  return got


def written(got: object) -> str:
  """One value of this interpreter, written out as the expression a word carries."""
  if isinstance(got, Source):
    return str(got)
  return held_out(got) if isinstance(got, (list, tuple, dict)) else alone(got)


def held_out(got: object) -> str:
  """One list, tuple or map, written out, with every value of it written out in turn."""
  if isinstance(got, (list, tuple)):
    held = ", ".join(written(one) for one in got)
    return f"[{held}]" if isinstance(got, list) else (f"({held},)" if len(got) == 1 else f"({held})")
  return "{" + ", ".join(f"{key!r}: {written(one)}" for key, one in got.items()) + "}"


def alone(got: object) -> str:
  """One value that holds no other, written out: a shape of the engine, an exception, or a plain one."""
  if isinstance(got, type):
    return got.__name__
  name = type(got).__name__
  if hasattr(got, "__dataclass_fields__") and name in NAMES:
    held = ", ".join(written(getattr(got, one)) for one in got.__dataclass_fields__)
    return f"{name}({held})"
  if isinstance(got, BaseException):
    held = ", ".join(written(one) for one in got.args)
    return f"{name}({held})"
  if isinstance(got, str):
    return repr(str(got))
  return repr(got)


def call(name: str, args: tuple, kwargs: dict) -> str:
  """One call of a verb, written out as the word that says it."""
  held = [written(one) for one in args] + [f"{key}={written(one)}" for key, one in kwargs.items()]
  return f"{name}({', '.join(held)})"


class Crossing:
  """One World double of the harness, behind the boundary, on a thread of its own.

  The double is a generator that calls back into the engine while it answers, and nothing may call into a life
  that stands waiting for it. So it runs here: it blocks where it asks, the boundary is told the question, and
  the answer wakes it. Every fact it hears is kept, which is what the probe of the harness was given.
  """

  def __init__(self, held: object, probe: object) -> None:
    self.held = held
    self.probe = probe
    self.to_world: queue.Queue = queue.Queue()
    self.from_world: queue.Queue = queue.Queue()
    self.answers: queue.Queue = queue.Queue()
    self.thread = threading.Thread(target=self.run, daemon=True)
    self.thread.crossing = self
    self.started = False

  def run(self) -> None:
    """The double, run: one fact in, every fact it says out, and a question of its own where it asks one.

    The generator is the one the harness made, and it was started where the loop runs, because a World of the
    suite binds that loop at its first step. It is only driven here, where it may block.
    """
    while True:
      fact = self.to_world.get()
      if fact is None:
        return
      out = []
      try:
        back = self.held.send(fact)
        while back is not None:
          out.append(back)
          back = self.held.send(None)
      except StopIteration:
        pass
      except BaseException as no:
        self.from_world.put(("raised", no))
        continue
      self.from_world.put(("say", out))

  def asks(self, kind: str, on: str, words: tuple) -> object:
    """One question of the engine, put from the World thread, and the answer, which wakes it."""
    self.from_world.put(("ask", kind, on, list(words)))
    return self.answers.get()

  def reads(self, word: str) -> object:
    """One word of the engine, read from the World thread, and the value, which wakes it.

    A World of python reads the engine where it answers, since it is a generator beside it. This is that reach
    for a World the engine cannot see, and it is what `acts[one]` and `scope(one)` go through.
    """
    self.from_world.put(("reads", word))
    return self.answers.get()

  def hears(self, fact: object) -> object:
    """One fact, heard by the double, and what it says of it or the question it must ask first."""
    held = tuple(made(one) for one in (fact.kind, fact.about, fact.by, *fact.words))
    if self.probe is not None:
      self.probe.send(held)
    if not self.started:
      self.started = True
      self.thread.start()
    self.to_world.put(held)
    return self.reply()

  def answered(self, got: object) -> object:
    """The answer to the question the double last asked, handed back to the thread that waits for it."""
    self.answers.put(made(got))
    return self.reply()

  def reply(self) -> object:
    """What the thread said: the facts to say, or the question it stopped at."""
    said = self.from_world.get()
    if said[0] == "raised":
      raise said[1]
    if said[0] == "reads":
      return furb_sand.Reads(said[1])
    if said[0] == "ask":
      _, kind, on, words = said
      return furb_sand.Ask(kind, str(on), *[plainly(one) for one in words])
    return [furb_sand.Fact(one[0], str(one[1]), "", *[plainly(w) for w in one[2:]]) for one in said[1]]


LIVES: list = []
"""LIVES are the lives this rig has opened, newest last, which is what a verb with no chain is run on."""


class Act(str):
  """One act of the engine, as this side holds it: its name, and a wait for what it came to.

  An act is a text, which is what the engine makes of one, so it stands in a word and in an f-string as itself.
  The act runs in the sandbox and this side holds only its name, so awaiting it is saying what the host owes and
  asking what the act came to, with the loop of this interpreter given room between the two.
  """

  held: object

  def __await__(self):  # noqa: ANN204
    return self.came().__await__()

  async def came(self) -> object:
    """What the act came to, once the host has said everything it owes."""
    for _ in range(4000):
      self.held.turn()
      got = self.held.life.came(str(self))
      if got is not None:
        return raised(made(got[0]))
      await asyncio.sleep(0)
    why = f"{self} never came to anything"
    raise AssertionError(why)


class Held:
  """One life of the engine in the sandbox, and the World of this interpreter it reaches."""

  def __init__(self, double: object, probe: object, record: str | None, gate: object) -> None:
    self.voice = furb_sand.Voice()
    self.world = Crossing(double, probe)
    self.life = furb_sand.Life(self.world, gate=gate, record=record, voice=self.voice)
    LIVES.append(self)

  def pump(self) -> int:
    """Everything the host has said into its Voice, done in the life."""
    return self.life.heard()

  def turn(self) -> None:
    """What the host owes, said, and room for the loop of the engine, which turns in the sandbox alone.

    Nothing of a life moves between two calls of it, so a wait for an act is a call: the word says nothing and
    is there to be entered.
    """
    self.pump()
    self.life.word("None")

  def word(self, word: str) -> object:
    """One verb, said as a word, and what it gave, made again as this interpreter holds it."""
    self.pump()
    got = made(self.life.word(word))
    if isinstance(got, str) and got.startswith(("bash://", "wait://", "prompt://", "rung://")):
      held = Act(got)
      held.held = self
      return held
    return got


def raised(got: object) -> object:
  """What an act came to, or the exception it came to, raised in whoever awaited it."""
  if isinstance(got, BaseException):
    raise got
  return got


def living() -> object:
  """The life a verb with no life of its own is run on, which is the newest one."""
  if not LIVES:
    why = "no life is open"
    raise AssertionError(why)
  return LIVES[-1]


SHOWS = ("span", "grep", "differs", "take", "HEAD", "TAIL", "HIDDEN")
"""SHOWS are the verbs that make a show or a filter, which never cross: they are written into the word."""
VOICED = {"send": None, "close": None}
"""VOICED are the verbs a host says with when nothing asked it, which go through the Voice."""


def on_world() -> object:
  """The World double this thread is, when a verb is called from inside one, and nothing otherwise."""
  return getattr(threading.current_thread(), "crossing", None)


class Mapping:
  """One map of the life, read where it stands, which is the sandbox.

  `acts`, `asked` and `outcomes` are the life's own, and a test reads them and so does a World while it answers.
  Neither holds a copy: every read is a word.
  """

  def __init__(self, name: str) -> None:
    self.name = name

  def __contains__(self, key: object) -> bool:
    return bool(reading(f"{written(key)} in {self.name}"))

  def __getitem__(self, key: object) -> object:
    if key not in self:
      raise KeyError(key)
    return reading(f"{self.name}[{written(key)}]")

  def get(self, key: object, default: object = None) -> object:
    """What the map holds under a key, or this instead, as a map of python answers."""
    held = reading(f"({written(key)} in {self.name}, {self.name}.get({written(key)}))")
    return held[1] if held[0] else default

  def __iter__(self):  # noqa: ANN204
    return iter(reading(f"list({self.name})"))

  def __len__(self) -> int:
    return int(reading(f"len({self.name})"))


class Shim:
  """What the suite reads as `furb.engine`: the names of the engine, over a life in the sandbox.

  A name that reads no life is the engine's own, since it says the same thing in either interpreter. A name that
  makes a show is written into the word that carries it. Everything else is a verb, said as a word in the
  sandbox, and what it gives is made again as this interpreter holds it.
  """

  def __getattr__(self, name: str):  # noqa: ANN204
    if name in HELD:
      return Mapping(name)
    if name in PURE or name not in NAMES or not callable(NAMES[name]) or isinstance(NAMES[name], type):
      return NAMES[name]
    if name in SHOWS:
      return lambda *args, **kwargs: Source(call(name, args, kwargs))
    if name == "boot":
      return booted
    return lambda *args, **kwargs: verbed(name, args, kwargs)


def held_of(gen: object) -> object:
  """The object a generator of a method was made from, which is its `self` before the first send.

  The harness hands `boot` the generators its doubles make, not the doubles, and what crosses the boundary is the
  double itself: the World it asks and the gate it reads a word by. Both stand in the frame of the generator.
  """
  frame = getattr(gen, "gi_frame", None)
  return None if frame is None else frame.f_locals.get("self")


def booted(record: object = (), **outside: object) -> str:
  """One life of the suite, opened in the sandbox, where the harness opens one in this interpreter."""
  world = outside.get("world")
  gate = held_of(outside.get("kernel"))
  probe = outside.get("probe")
  if probe is not None:
    probe.send(None)
  # Started here, where the loop runs: a World of the suite reads the running loop at its first step.
  world.send(None)
  held = Held(world, probe, text_of(record), gate)
  return held.life.root


def verbed(name: str, args: tuple, kwargs: dict) -> object:
  """One verb of the suite, said where it belongs: into the Voice, back to the engine, or as a word."""
  crossing = on_world()
  if name == "send":
    held = living()
    held.voice.fact(
      furb_sand.Fact(str(args[0]), str(args[1]), str(kwargs.get("by", "")), *[plainly(o) for o in args[2:]])
    )
    return None
  if name == "close":
    living().voice.close(str(args[1]), plainly(args[0]))
    return None
  return reading(call(name, args, kwargs), crossing)


def reading(word: str, crossing: object = None) -> object:
  """One word of the engine, read where the caller stands: in the life, or through the World that is waiting."""
  held = crossing if crossing is not None else on_world()
  if held is not None:
    return made(held.reads(word))
  return living().word(word)


def text_of(record: object) -> str | None:
  """The record a life is opened on, as the text a World keeps."""
  if not record:
    return None
  return "\n".join(furb_sand.line([plainly(one) for one in entry]) for entry in record)


async def settled(n: int = 80) -> None:
  """Room for the loop and for the sandbox both, which is what `conftest.settle` is here."""
  for _ in range(n):
    for one in LIVES:
      one.pump()
    await asyncio.sleep(0)


def pytest_sessionstart(session: object) -> None:  # noqa: ARG001
  """The engine of the suite, replaced by a life in the sandbox, wherever the suite loaded it."""
  shim = Shim()
  for mod in list(sys.modules.values()):
    if getattr(mod, "engine", None) is real:
      mod.engine = shim
    if getattr(mod, "life", None) is not None and getattr(mod, "__name__", "") == "conftest":
      mod.settle = settled


def main() -> None:
  """The suite with the engine in the sandbox, run as pytest with this file as its plugin."""
  args = sys.argv[1:] or ["test", "--ignore=test/outside", "--ignore=test/test_hygiene.py"]
  said = subprocess.run(  # noqa: S603
    [sys.executable, "-m", "pytest", "--no-cov", "-p", "sanded", *args],
    cwd=ROOT,
    env={**os.environ, "PYTHONPATH": f"{HERE}:{SRC}:{BOUND}"},
    check=False,
  )
  raise SystemExit(said.returncode)


if __name__ == "__main__":
  main()

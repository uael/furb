"""The engine in monty, with every name of engine.pyi.

The engine of this interpreter is `furb.python`, and this module gives the same names over one life of that engine
running in the sandbox of monty with the Kernel of the crate. A name that reads no life is the engine's own: a
constant, a class, a show or a filter, and the pure functions. `acts`, `asked`, `outcomes` and `modules` read the
maps of the life where they stand, and `site` reads who is speaking there. `boot` opens the life, `Act` is the
name of an act, awaited here for what the act comes to in there, and every other name is a verb, which runs its
word in the sandbox and gives back what the word gave, made again as the engine of this interpreter holds it.

What crosses is plain, in both directions. A tuple, a text, an exit, an exception and the name of an act each
stand under a mark, and a value of this interpreter that cannot cross as data crosses as a name: a show, a filter
or an ear a caller hands a verb is called back from the sandbox, and a generator, which a World is, is heard from
the sandbox the way an ear is. A generator is heard on a thread of its own, since a World reads the engine while it
answers, and nothing may call into a life that stands waiting for it: what the thread reads is a word the sandbox
runs in the names of the engine, whose value wakes the thread. A callable that is no generator runs on the thread
of the engine and reads nothing of it, which is the one thing a host callable may not do.
"""

import ast
import asyncio
import builtins
import inspect
import math
import queue
import threading
import types
from collections.abc import Callable, Generator, Iterator, Mapping
from dataclasses import fields, is_dataclass
from pathlib import Path
from string.templatelib import Template

import furb
from furb_monty import _monty

python = furb.python
"""python is the engine of this interpreter, whose names that read no life this module gives as they are."""
NAMES = vars(python)
"""NAMES are the names of the engine of this interpreter, by which a value that crosses back is made again."""
PURE = frozenset({
  "span", "grep", "differs", "take", "HEAD", "TAIL", "HIDDEN",
  "lineage", "under", "question", "showing", "shown", "turns_of", "offered", "ended", "idle",
})  # fmt: skip
"""PURE are the callables of the engine that read no life, so the engine of this interpreter answers them."""
HELD = ("modules", "acts", "asked", "outcomes")
"""HELD are the maps of the life, which stand in the sandbox and are read there."""
SHOWS = ("HEAD", "TAIL", "HIDDEN")
"""SHOWS are the shows the engine names, which cross by their name so that the sandbox reads them as its own."""
TUPLE, SHOW, NAME, ACT, CALL, GEN, TEMPLATE, MADE = "()", "", "name", "Act", "call", "gen", "Template", "made"
"""The marks of the plain form, as `src/preamble.py` says them."""
END = object()
"""END is what a thread of a generator is given when the life it was heard in is over."""
LOCAL = threading.local()
"""LOCAL holds, on the thread of a generator, the crossing that generator is heard through."""
LIFE: Living | None = None
"""LIFE is the life this process holds, and a second boot ends it."""


def named(shape: object) -> str:
  """The name of a shape, as the engine names one: a type by its name, and anything else as python shows it."""
  return shape.__name__ if isinstance(shape, type) else repr(shape).replace(python.Text.__module__ + ".", "")


def wire(x: object) -> object:
  """The plain form of a value of this interpreter: plain data is plain, an act its name, a tuple its entries
  under its own mark, an exception its name and what it was made with, a shape its name beside its fields, a
  template string its interpolations, a type the name of the shape it is, a show of the engine its name, a
  generator the mark of the crossing it is heard through, and any other callable the mark it is called back by.
  """
  match x:
    case Act():
      return {"is": ACT, "args": [str(x)]}
    case None | bool() | int() | float() | str():
      return x
    case Template():
      return {"is": TEMPLATE, "interpolations": [[wire(i.value), i.expression] for i in x.interpolations]}
    case BaseException():
      return {"is": type(x).__name__, "args": [wire(i) for i in x.args]}
    case tuple():
      return {"is": TUPLE, "args": [wire(i) for i in x]}
    case list():
      return [wire(i) for i in x]
    case dict():
      return {str(k): wire(v) for k, v in x.items()}
    case type() | types.GenericAlias() | types.UnionType():
      return named(x)
  for name in SHOWS:
    if x is NAMES[name]:
      return {"is": NAME, "name": name}
  if is_dataclass(x):
    return {"is": type(x).__name__} | {f.name: wire(getattr(x, f.name)) for f in fields(x)}
  if isinstance(x, Generator):
    n = living().crosses(x)
    return {"is": GEN, "id": n, "started": living().crossed[n].started}
  if callable(x):
    return {"is": CALL, "id": living().calls(x)}
  why = f"{x!r} cannot cross to the engine"
  raise TypeError(why)


def unwire(x: object) -> object:
  """The value again from the plain form, made by what its name is known by: a name of the engine of this
  interpreter, or of the interpreter when the engine holds none, and an act as the name it is."""
  match x:
    case list():
      return [unwire(i) for i in x]
    case {"is": str(mark), "args": list(held)} if mark == TUPLE:
      return tuple(unwire(i) for i in held)
    case {"is": str(mark), "args": [str(name)]} if mark == ACT:
      return Act(name)
    case {"is": str(mark), "name": str(name)} if mark == NAME:
      return globals()[name]
    case {"is": str(mark), "id": int(n)} if mark == MADE:
      return calling(n)
    case {"is": str(mark)} if mark == SHOW:
      return None
    case {"is": str(name), "args": list(held)}:
      # An exception is one object in the life, however many facts hold it, so one crosses once and is kept.
      return living().fault(name, held)
    case {"is": str(name), **rest}:
      return known(name)(**{str(k): unwire(v) for k, v in rest.items()})
    case dict():
      return {k: unwire(v) for k, v in x.items()}
  return x


def calling(n: int) -> Callable[..., object]:
  """One callable the engine made, such as a show, called where it stands by a word, with plain arguments."""

  def called(*args: object) -> object:
    return run(f"made[{n}]({', '.join(written(one) for one in args)})")

  return called


def known(name: str) -> Callable[..., object]:
  """What a name of a shape or an exception makes: the engine's own, then the interpreter's."""
  held = NAMES.get(name, getattr(builtins, name, None))
  if not callable(held):
    why = f"{name} is no shape and no exception this interpreter knows"
    raise TypeError(why)
  return held


def raised(name: str, args: object) -> BaseException:
  """The exception the sandbox raised, made again by its name and what it was made with."""
  assert isinstance(args, list)
  made = known(name)(*args)
  assert isinstance(made, BaseException)
  return made


def written(x: object) -> str:
  """One value of this interpreter as the expression a word carries: plain data as itself, and anything else as
  its plain form, made again where the word runs."""
  if isinstance(x, float) and not math.isfinite(x):
    return f"float({x!r})"
  if x is None or isinstance(x, (bool, int, float, str)):
    return repr(x)
  return f"unwire({wire(x)!r})"


def call(name: str, args: tuple, kwargs: dict[str, object]) -> str:
  """One call of a verb, as the word that says it."""
  held = [written(one) for one in args] + [f"{key}={written(one)}" for key, one in kwargs.items()]
  return f"{name}({', '.join(held)})"


def run(word: str) -> object:
  """One word, run in the names of the engine, and what it gave, made again.

  From the thread of a generator the word is read by the sandbox where the generator is heard, since the life
  stands waiting for that generator. From anywhere else it is a word of the operator.
  """
  crossing = getattr(LOCAL, "crossing", None)
  if crossing is not None:
    return crossing.reads(word)
  held = living().held
  if held is None:  # pragma: no cover: a verb runs in a life, so held is set before any word
    why = "no life"
    raise RuntimeError(why)
  try:
    got = held.word(word)
  except _monty.Raised as no:
    name, args = no.args
    raise raised(name, unwire(args)) from None
  return unwire(got)


def living() -> Living:
  """The life this process holds, which every word runs in, and which is opening or open."""
  if LIFE is None:  # pragma: no cover: a verb runs in a life; boot binds LIFE before any word
    why = "no life"
    raise RuntimeError(why)
  return LIFE


class Crossing:
  """One generator of this interpreter, heard from the sandbox on a thread of its own.

  The generator is primed where it is made, on the thread of the loop, since a World reads the running loop at its
  first step, and what that step gave stands for its birth in the sandbox. From then, every fact the sandbox gives
  it goes through its inbox to the thread, and what the generator did with it comes back through its outbox: a
  saying, nothing, its end, or what it raised. A word the generator reads of the engine goes the other way through
  the same two doors, and its value wakes the thread.
  """

  def __init__(self, gen: Generator[tuple | None, tuple]) -> None:
    self.gen = gen
    self.inbox: queue.Queue[object] = queue.Queue()
    self.outbox: queue.Queue[object] = queue.Queue()
    # A generator that was started already stands at a yield, and its stand-in in the sandbox is started too.
    self.started = inspect.getgeneratorstate(gen) != inspect.GEN_CREATED
    self.first = None if self.started else self.stepped(None)
    self.born = False
    self.thread = threading.Thread(target=self.serve, daemon=True)
    self.thread.start()

  def stepped(self, sent: object) -> object:
    """One step of the generator with what it was given, and what came of it, plain."""
    try:
      out = self.gen.send(sent) if isinstance(sent, tuple) else next(self.gen)
    except StopIteration:
      return {"over": True}
    except BaseException as no:
      return {"raised": wire(no)}
    return None if out is None else {"say": wire(list(out))}

  def serve(self) -> None:
    """The thread of the generator: every fact from the inbox stepped, and what came of it put in the outbox."""
    LOCAL.crossing = self
    while (got := self.inbox.get()) is not END:
      self.outbox.put(self.stepped(unwire(got)))

  def channel(self, said: object) -> object:
    """What the sandbox gave this generator, and what the generator did with it.

    At its birth the sandbox gives it nothing, and what the priming step gave stands for that.
    """
    if not self.born:
      self.born = True
      return self.first
    self.inbox.put(said)
    return self.outbox.get()

  def reads(self, word: str) -> object:
    """One word of the engine, read from the thread of the generator: the sandbox is told, and its value wakes
    the thread."""
    self.outbox.put({"reads": word})
    got = self.inbox.get()
    if got is END:  # pragma: no cover: the generator's life is over and its thread is torn down
      why = "the life this generator was heard in is over"
      raise RuntimeError(why)
    assert isinstance(got, dict)
    assert "answered" in got
    return unwire(got["answered"])

  def end(self) -> None:
    """The end of the life the generator was heard in, which ends its thread."""
    self.inbox.put(END)


class Living:
  """One life of the engine in the sandbox: the callables and the generators of this interpreter it reaches, by
  the ids they cross under, and the ears it was opened on, by name."""

  def __init__(self) -> None:
    self.called: list[Callable[..., object]] = []
    self.crossed: list[Crossing] = []
    self.ears: dict[str, Crossing] = {}
    self.faults: dict[str, BaseException] = {}
    self.held: _monty.Life | None = None

  def fault(self, name: str, args: list[object]) -> BaseException:
    """The exception of this name made with these arguments, one object for as long as the life lives."""
    key = repr([name, args])
    if key not in self.faults:
      self.faults[key] = raised(name, [unwire(i) for i in args])
    return self.faults[key]

  def calls(self, one: Callable[..., object]) -> int:
    """One callable of this interpreter, kept under the id the sandbox calls it back by."""
    self.called.append(one)
    return len(self.called) - 1

  def crosses(self, gen: Generator[tuple | None, tuple]) -> int:
    """One generator of this interpreter, heard from now on, under the id the sandbox reaches it by."""
    self.crossed.append(Crossing(gen))
    return len(self.crossed) - 1

  def host(self, name: str, said: object) -> object:
    """Every call of the sandbox, answered: a callable called back, a generator given a fact, or an ear."""
    if name == CALL:
      assert isinstance(said, list)
      n, args = said
      held = unwire(args)
      assert isinstance(held, list)
      try:
        return {"value": wire(self.called[n](*held))}
      except BaseException as no:
        return {"raised": wire(no)}
    if name == GEN:
      assert isinstance(said, list)
      n, a = said
      return self.crossed[n].channel(a)
    return self.ears[name].channel(said)

  def opens(self, record: object, outside: dict[str, Generator[tuple | None, tuple]]) -> None:
    """The life, opened on the record and the ears, each heard under the name it was given."""
    self.ears = {name: Crossing(gen) for name, gen in outside.items()}
    try:
      self.held = _monty.Life(self.host, list(outside), wire(list(record)))  # ty: ignore[invalid-argument-type]
    except _monty.Raised as no:  # pragma: no cover: defensive; the engine's own boot raise comes through .raised
      name, args = no.args
      raise raised(name, unwire(args)) from None

  def end(self) -> None:
    """The end of the life, which ends the thread of every generator it heard."""
    for one in [*self.ears.values(), *self.crossed]:
      one.end()


class Act[T = object](str):
  """The name of an act, which is what a verb gives and what a caller holds of the act: a string, so it names the
  act to close, cancel, pause, peek and get, and awaitable, so it gives what the act comes to."""

  def __await__(self) -> Generator[object, None, T]:
    return self.came().__await__()

  async def came(self) -> T:
    """What the act came to, once the life holds it: the value, or the exception it completed with, raised."""
    while True:
      got = run(f"({str(self)!r} in outcomes, outcomes.get({str(self)!r}))")
      assert isinstance(got, tuple)
      if got[0]:
        if isinstance(got[1], BaseException):
          raise got[1]
        return got[1]
      # A turn of the loop between two looks, and a moment with it: the sandbox counts the words a life says, so a
      # look at an act that never settles is paced, and the timeout of whoever waits comes first.
      await asyncio.sleep(0.001)


class Held(Mapping[str, object]):
  """One map of the life, read where it stands: nothing is copied, and every read is a word."""

  def __init__(self, name: str) -> None:
    self.name = name

  def __getitem__(self, key: str) -> object:
    if key not in self:
      raise KeyError(key)
    return run(f"{self.name}[{key!r}]")

  def __contains__(self, key: object) -> bool:
    return bool(run(f"{key!r} in {self.name}"))

  def __iter__(self) -> Iterator[str]:
    held = run(f"list({self.name})")
    assert isinstance(held, list)
    return iter(held)

  def __len__(self) -> int:
    got = run(f"len({self.name})")
    assert isinstance(got, int)
    return got

  def __repr__(self) -> str:
    return f"{self.name} of the life"


class Modules(Held):
  """The module of every chain of the life, each read where it stands, since a module holds what no host reads."""

  def __getitem__(self, key: str) -> Held:
    if key not in self:
      raise KeyError(key)
    return Held(f"{self.name}[{key!r}]")


class Site:
  """Who is speaking in the life, read and set where it stands.

  A set gives back what stood before it, and a reset puts that back, which is what a token of a context variable
  does within one context.
  """

  def get(self) -> str:
    """Who is speaking."""
    got = run("site.get()")
    assert isinstance(got, str)
    return got

  def set(self, value: str) -> str:
    """Who speaks from now on, and who spoke before, to reset with."""
    previous = self.get()
    run(f"site.set({value!r})")
    return previous

  def reset(self, previous: str) -> None:
    """Who spoke before the set this resets."""
    run(f"site.set({previous!r})")


def boot(record: object = (), **outside: Generator[tuple | None, tuple]) -> Act:
  """A life of the engine in the sandbox, opened from the record and on the ears given, which gives the root.

  The Kernel is the crate's, so a generator under that name is refused as the engine refuses one under a name of
  its own ears. A second boot is a second life, and the first is gone with the threads of its generators.
  """
  asyncio.get_running_loop()
  if "kernel" in outside:
    why = "kernel hears already: the engine of monty holds its Kernel"
    raise python.Refused(why)
  global LIFE  # noqa: PLW0603
  if LIFE is not None:
    LIFE.end()
  LIFE = Living()
  LIFE.opens(record, outside)
  if (no := LIFE.held.raised) is not None:
    assert isinstance(no, dict)
    raise raised(str(no["is"]), unwire(no.get("args", [])))
  return Act(LIFE.held.root)


def worded(name: str) -> Callable[..., object]:
  """One verb of the engine, as the word that calls it in the sandbox."""

  def verb(*args: object, **kwargs: object) -> object:
    return run(call(name, args, kwargs))

  verb.__name__ = verb.__qualname__ = name
  return verb


def defined() -> list[str]:
  """Every name the engine defines at its top, in order, which is the surface this module gives."""
  said: list[str] = []
  for node in ast.parse(Path(python.__file__).read_text(encoding="utf-8")).body:
    match node:
      case ast.FunctionDef() | ast.AsyncFunctionDef() | ast.ClassDef():
        said.append(node.name)
      case ast.TypeAlias(name=ast.Name(id=name)) | ast.AnnAssign(target=ast.Name(id=name)):
        said.append(name)
      case ast.Assign(targets=[ast.Name(id=name)]):
        said.append(name)
      case ast.Assign(targets=[ast.Tuple(elts=elts)]):
        said.extend(one.id for one in elts if isinstance(one, ast.Name))
  return [name for i, name in enumerate(said) if name not in said[:i]]


for _name in defined():
  if _name in ("boot", "Act"):
    continue
  _held = NAMES[_name]
  if _name in HELD:
    globals()[_name] = Modules(_name) if _name == "modules" else Held(_name)
  elif _name == "site":
    globals()[_name] = Site()
  elif _name in PURE or isinstance(_held, type) or not callable(_held):
    globals()[_name] = _held
  else:
    globals()[_name] = worded(_name)

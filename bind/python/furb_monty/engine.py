"""The engine in monty, with every name of engine.pyi.

The engine of this interpreter is `furb.python`, and this module gives the same names over one life of that engine
running in the sandbox of monty with the Kernel of the crate. A name that reads no life is the engine's own: a
constant, a class, a show or a filter, and the pure functions. A verb says itself in the sandbox by its name with
its words, and what it gave comes back as the instance of `furb.python` it is. `acts`, `asked`, `outcomes` and
`modules` are read where they stand, and `site` says who speaks there. An act is awaited for what it comes to,
which the life says once, when it is done.

A generator of this interpreter, which a World is, is heard from the sandbox on a thread of its own, since a World
reads the engine while it answers and nothing may call into a life that stands waiting for it: a verb the thread
says is said by its name to the sandbox, which says it on the thread's behalf and answers. A callable of this
interpreter is called back from the sandbox by name, and a callable the engine made is called back by its handle,
so a show crosses either way as itself. A class a word defined is a type of this interpreter, one per class,
derived from the type its base is here, whose call makes the instance in the sandbox by the handle of the class,
and an instance of one is an object of that type holding its fields, which goes back in made from them.
"""

import ast
import asyncio
import inspect
import queue
import threading
import weakref
from collections.abc import Callable, Generator, Iterable, Iterator, Mapping
from pathlib import Path

import furb
from furb_monty import _monty

python = furb.python
"""python is the engine of this interpreter, whose names that read no life this module gives as they are."""
NAMES = vars(python)
"""NAMES are the names of the engine of this interpreter."""
PURE = frozenset({
  "span", "grep", "differs", "HEAD", "TAIL", "HIDDEN",
  "question", "headed", "commented", "bound", "showing", "shown", "turns_of", "unquoted", "offered", "ended", "idle",
})  # fmt: skip
"""PURE are the callables of the engine that read no life, so the engine of this interpreter answers them."""
HELD = ("modules", "acts", "asked", "outcomes")
"""HELD are the maps of the life, which stand in the sandbox and are read there."""
ACTS = frozenset({"wait", "rung", "prompt", "chain", "grant", "bash", "act"})
"""ACTS are the verbs that give an act, whose name comes back as the act it names."""
END = object()
"""END is what a thread of a generator is given when the life it was heard in is over."""
LOCAL = threading.local()
"""LOCAL holds, on the thread of a generator, the crossing that generator is heard through."""
FORGOTTEN: list[int] = []
"""FORGOTTEN holds the handles of the callables the engine made and of the classes a word defined that this
interpreter dropped since the life last heard of them, which the next verb of the operator says into the sandbox,
so the sandbox drops them too."""
CLASSES: weakref.WeakValueDictionary[int, type] = weakref.WeakValueDictionary()
"""CLASSES holds every class a word defined that crossed to this interpreter, by its handle, as the type this
interpreter holds it as, for as long as this interpreter holds that type."""
LIFE: Living | None = None
"""LIFE is the life this process holds, and a second boot ends it."""


def living() -> Living:
  """The life this process holds, which every verb says in."""
  if LIFE is None:  # pragma: no cover: a verb is said in a life; boot binds LIFE before any verb
    why = "no life"
    raise RuntimeError(why)
  return LIFE


def call(name: str, args: tuple, kwargs: dict[str, object]) -> object:
  """One verb of the engine, said by its name with its words, and what it gave.

  From the thread of a generator the verb is said by the sandbox where the generator is heard, since the life
  stands waiting for that generator. From anywhere else it is a verb of the operator.
  """
  held = getattr(LOCAL, "crossing", None)
  if held is not None:
    return held.calls(name, list(args), dict(kwargs))
  life = living().life
  while FORGOTTEN:
    life.forget(FORGOTTEN.pop())
  return life.verb(name, list(args), dict(kwargs))


def stands(name: str, under: list[str], ask: str) -> object:
  """One reading of a map of the life where it stands, under these keys. From the thread of a generator it is
  asked of the sandbox as a verb is; from anywhere else the life answers it."""
  held = getattr(LOCAL, "crossing", None)
  if held is not None:
    return held.calls("held", [name, under, ask], {})
  return living().life.held(name, under, ask)


def speaks(value: str | None) -> str:
  """Who speaks in the life, and who speaks from now on when a value is given, read as a map of the life is."""
  held = getattr(LOCAL, "crossing", None)
  got = held.calls("spoken", [value], {}) if held is not None else living().life.site(value)
  assert isinstance(got, str)
  return got


class Crossing:
  """One generator of this interpreter, heard from the sandbox on a thread of its own.

  The generator is primed where it is made, on the thread of the loop, since a World reads the running loop at its
  first step, and what that step gave stands for its birth in the sandbox. From then, every fact the sandbox gives
  it goes through its inbox to the thread, and what the generator did with it comes back through its outbox: a
  saying, nothing, its end, or what it raised. A verb the generator says goes the other way through the same two
  doors, and its value wakes the thread.
  """

  def __init__(self, gen: Generator[tuple | None, tuple], *, born: bool = False) -> None:
    self.gen = gen
    self.inbox: queue.Queue[object] = queue.Queue()
    self.outbox: queue.Queue[object] = queue.Queue()
    # A generator that was started already stands at a yield, and so does its stand-in.
    started = inspect.getgeneratorstate(gen) != inspect.GEN_CREATED
    self.first = None if started else self.stepped(None)
    # An ear of a restored life was born in the life that was dumped: what this generator said at its birth was
    # said then, and it hears from the next fact on.
    self.born = born
    threading.Thread(target=self.serve, daemon=True).start()

  def stepped(self, sent: object) -> object:
    """One step of the generator with what it was given, and what came of it, as the sandbox reads a reply."""
    try:
      out = self.gen.send(sent) if isinstance(sent, tuple) else next(self.gen)
    except StopIteration:
      return ("over",)
    except BaseException as no:
      return ("raised", no)
    return None if out is None else ("say", tuple(out))

  def serve(self) -> None:
    """The thread of the generator: every fact from the inbox stepped, and what came of it put in the outbox."""
    LOCAL.crossing = self
    while (got := self.inbox.get()) is not END:
      self.outbox.put(self.stepped(got))

  def channel(self, said: object) -> object:
    """What the sandbox gave this generator, and what the generator did with it: at its birth the sandbox gives
    it nothing, and what the priming step gave stands for that."""
    if not self.born:
      self.born = True
      return self.first
    self.inbox.put(said)
    return self.outbox.get()

  def answered(self, value: object) -> object:
    """The value of the verb the generator said, which wakes its thread, and what the generator did next."""
    self.inbox.put(("answered", value))
    return self.outbox.get()

  def calls(self, name: str, args: list, kwargs: dict[str, object]) -> object:
    """One verb of the engine, said from the thread of the generator: the sandbox is told, and its value wakes
    the thread."""
    self.outbox.put(("calls", name, args, kwargs))
    got = self.inbox.get()
    if got is END:  # pragma: no cover: the generator's life is over and its thread is torn down
      why = "the life this generator was heard in is over"
      raise RuntimeError(why)
    assert isinstance(got, tuple)
    _, (kind, value) = got
    if kind == "raised":
      assert isinstance(value, BaseException)
      raise value
    return value

  def end(self) -> None:
    """The end of the life the generator was heard in, which ends its thread."""
    self.inbox.put(END)


class Living:
  """One life of the engine in the sandbox, and the ears of this interpreter it hears, by name."""

  def __init__(self, outside: dict[str, Generator[tuple | None, tuple]], *, born: bool = False) -> None:
    self.crossings = {name: Crossing(gen, born=born) for name, gen in outside.items()}
    self.callables: dict[str, Callable[..., object]] = {}
    self.life: _monty.Life | None = None

  def ear(self, gen: Generator[tuple | None, tuple]) -> tuple[str, bool]:
    """A generator of this interpreter, heard from now on under a name of its own: its name, and whether it was
    started already, so that its stand-in stands where it stands. The door makes the mark the engine reads."""
    name = f"ear:{len(self.crossings)}"
    started = inspect.getgeneratorstate(gen) != inspect.GEN_CREATED
    self.crossings[name] = Crossing(gen)
    return name, started

  def callable(self, fn: Callable[..., object]) -> str:
    """A callable of this interpreter, called back from now on under a name of its own, which it gives. The door
    makes the mark the engine reads."""
    name = f"callable:{len(self.callables)}"
    self.callables[name] = fn
    return name

  def called(self, name: str, args: tuple, kwargs: dict[str, object]) -> object:
    """One callable of this interpreter, called back by its name with what the sandbox gave it, and what it gave,
    which the door carries in as it carries any value: a generator as an ear, since a callable given to an act
    gives its ear."""
    return self.callables[name](*args, **kwargs)

  def hears(self, name: str, fact: object) -> object:
    """One fact, heard by the ear of this name, or nothing at its birth, and what the ear did with it."""
    return self.crossings[name].channel(fact)

  def answered(self, name: str, value: object) -> object:
    """The value of the verb the ear of this name said, and what the ear did next."""
    return self.crossings[name].answered(value)

  def end(self) -> None:
    """The end of the life, which ends the thread of every generator it heard."""
    for one in self.crossings.values():
      one.end()


def calling(n: int, args: tuple[object, ...], kwargs: dict[str, object]) -> object:
  """One call of what the engine holds under a handle, through the sandbox: from the thread of a generator as a
  verb is said there, and from anywhere else as a call of the operator."""
  held = getattr(LOCAL, "crossing", None)
  if held is not None:
    return held.calls("made", [n, list(args), dict(kwargs)], {})
  return living().life.made(n, list(args), dict(kwargs))


def made(n: int) -> Callable[..., object]:
  """One callable the engine made, as this interpreter calls it: by its handle, which it carries as `__monty__`, so
  that it goes back in as the callable it is and never as a callable of this interpreter."""

  def back(*args: object, **kwargs: object) -> object:
    return calling(n, args, kwargs)

  vars(back)["__monty__"] = n
  # When this interpreter drops the last reference, the handle is forgotten at the next verb of the operator.
  weakref.finalize(back, FORGOTTEN.append, n)
  return back


def classed(n: int, name: str, base: type) -> type:
  """One class a word defined, as this interpreter holds it: a type of that name derived from the type its base is
  here, a builtin, a class of the engine or the type of another class a word defined, one for the class for as
  long as this interpreter holds it, whose call makes the instance in the sandbox by the handle of the class, and
  which goes back in by that handle, which it carries as `__monty__`. A class that is an exception in the sandbox
  is an exception here, so what a word raised raises, and is caught by the builtin it derives from."""
  if (held := CLASSES.get(n)) is not None:
    return held

  def new(cls: type, *args: object, **kwargs: object) -> object:
    handle = cls.__monty__
    assert isinstance(handle, int)
    return calling(handle, args, kwargs)

  cls = type(name, (base,), {"__new__": new, "__monty__": n})
  weakref.finalize(cls, FORGOTTEN.append, n)
  CLASSES[n] = cls
  return cls


def instanced(cls: type, fields: dict[str, object]) -> object:
  """One instance of a class a word defined, as this interpreter holds it: an object of the type that class is
  here, holding the fields, with no `__init__` run; an exception is made with what it was made with."""
  held = dict(fields)
  args = held.pop("args", ())
  assert isinstance(args, tuple)
  if issubclass(cls, BaseException):
    one: object = BaseException.__new__(cls, *args)
  else:
    one = object.__new__(cls)
  vars(one).update(held)
  return one


class Act[T = object](str):
  """The name of an act, which is what a verb gives and what a caller holds of the act: a string, so it names the
  act to close, cancel, pause, peek and get, and awaitable, so it gives what the act comes to."""

  def __await__(self) -> Generator[object, None, T]:
    return self.came().__await__()

  async def came(self) -> T:
    """What the act came to, once the life holds it: the value, or the exception it completed with, raised."""
    done = asyncio.get_running_loop().create_future()

    def told(value: object) -> None:
      if not done.done():
        done.set_result(value)

    living().life.watch(str(self), told)
    got = await done
    if isinstance(got, BaseException):
      raise got
    return got


class Held(Mapping[str, object]):
  """One map of the life, read where it stands: nothing is copied, and every read is one reading."""

  def __init__(self, name: str, under: tuple[str, ...] = ()) -> None:
    self.name = name
    self.under = under

  def __getitem__(self, key: str) -> object:
    if key not in self:
      raise KeyError(key)
    return stands(self.name, [*self.under, key], "at")

  def __contains__(self, key: object) -> bool:
    return bool(stands(self.name, [*self.under, str(key)], "in"))

  def __iter__(self) -> Iterator[str]:
    got = stands(self.name, list(self.under), "keys")
    assert isinstance(got, list)
    return iter(got)

  def __len__(self) -> int:
    got = stands(self.name, list(self.under), "len")
    assert isinstance(got, int)
    return got

  def __repr__(self) -> str:
    return f"{self.name} of the life"


class Modules(Held):
  """The module of every chain of the life, each read where it stands, since a module holds what no host reads."""

  def __getitem__(self, key: str) -> Held:
    if key not in self:
      raise KeyError(key)
    return Held(self.name, (*self.under, key))


class Site:
  """Who is speaking in the life, read and set where it stands: a set gives back what stood before it, and a reset
  puts that back, which is what a token of a context variable does within one context."""

  def get(self) -> str:
    return speaks(None)

  def set(self, value: str) -> str:
    return speaks(value)

  def reset(self, previous: str) -> None:
    speaks(previous)


def boot(record: Iterable[object] = (), **outside: Generator[tuple | None, tuple]) -> Act:
  """A life of the engine in the sandbox, opened from the record and on the ears given, which gives the root.

  The Kernel and the gate are the crate's, so a generator under either name is refused as the engine refuses one
  under a name of its own ears. A second boot is a second life, and the first is gone with the threads of its
  generators.
  """
  living = opened(outside)
  living.life = _monty.Life(living, list(outside), list(record))
  if (no := living.life.raised) is not None:
    raise no
  return Act(living.life.root)


def restore(dump: bytes, **outside: Generator[tuple | None, tuple]) -> Act:
  """A life of the engine in the sandbox, restored from a dump of one that stood still, on ears under the names
  it was dumped with, which gives the root. An ear hears from the next fact on: what it says at its birth was
  said in the life the dump came from."""
  living = opened(outside, born=True)
  living.life = _monty.Life.restored(living, list(outside), dump)
  return Act(living.life.root)


def dump() -> bytes:
  """The life as bytes, where it stands still, for `restore` to go on from without the record replayed."""
  return living().life.dump()


def opened(outside: dict[str, Generator[tuple | None, tuple]], *, born: bool = False) -> Living:
  """The life this process holds from now on, on these ears: the one before it is gone with the threads of its
  generators, and a Kernel among the ears is refused, since the engine of monty holds its own."""
  asyncio.get_running_loop()
  for name in ("kernel", "gate"):
    if name in outside:
      why = f"{name} hears: the engine of monty holds its Kernel and its gate"
      raise python.Refused(why)
  global LIFE  # noqa: PLW0603
  if LIFE is not None:
    LIFE.end()
  LIFE = Living(outside, born=born)
  return LIFE


def worded(name: str) -> Callable[..., object]:
  """One verb of the engine, said by its name in the sandbox with the words it was given."""

  def verb(*args: object, **kwargs: object) -> object:
    got = call(name, args, kwargs)
    return Act(got) if name in ACTS and isinstance(got, str) else got

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

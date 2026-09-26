"""The engine in monty, with every name of engine.pyi.

The engine of this interpreter is `furb.python`, and this module gives the same names over one life of that engine
running in the sandbox of monty with the Kernel of the crate. A name that reads no life is the engine's own: a
constant, a class, a show or a filter, and the pure functions. A verb is the method of the engine of the crate that
says it, and any other name of the engine says itself in the sandbox by its name with its words, and what either
gave comes back as the instance of `furb.python` it is, which is how `get`, `peek` and `transcript` read the life
too, and `site` says who speaks there. An act is awaited for what it comes to, which the life says once, when it is
done.

A generator of this interpreter, which an ear of the World is, is heard from the sandbox on a thread of its own,
since such an ear reads the engine while it hears and nothing may call into a life that stands waiting for it: a
verb the thread says is a call the ear yields to the sandbox, which says it on the ear's behalf and answers. An ear
of the crate, which `furb_monty._monty` gives, is heard as itself. A callable of this interpreter is called back
from the sandbox, and a callable the engine made is called back by its handle, so a show crosses either way as
itself. A class a word defined is a type of this interpreter, one per class, derived from the type its base is here,
whose call makes the instance in the sandbox by the handle of the class, and an instance of one is an object of that
type holding its fields, which goes back in made from them.
"""

import ast
import asyncio
import contextvars
import inspect
import queue
import threading
import weakref
from collections.abc import Callable, Generator, Iterable
from pathlib import Path

import furb
from furb_monty import _monty

python = furb.python
"""python is the engine of this interpreter, whose names that read no life this module gives as they are."""
NAMES = vars(python)
"""NAMES are the names of the engine of this interpreter."""
PURE = frozenset({
  "span", "grep", "differs", "HEAD", "TAIL", "HIDDEN",
  "question", "headed", "commented", "bound", "showing", "shown", "unquoted", "offered", "ended", "idle",
})  # fmt: skip
"""PURE are the callables of the engine that read no life, so the engine of this interpreter answers them."""
ACTS = frozenset({"wait", "rung", "prompt", "chain", "grant", "bash", "act"})
"""ACTS are the verbs that give an act, whose name comes back as the act it names."""
VERBS = frozenset(NAMES) & frozenset(vars(_monty.Engine)) - {"boot", "site", "raised"}
"""VERBS are the verbs of the contract, each of which the engine of the crate says by a method of its own. Boot opens
an engine, and the site and what boot raised are read where they stand, which no word says."""
END = object()
"""END is what a thread of a generator is given when the life it was heard in is over."""
LOCAL = threading.local()
"""LOCAL holds, on the thread of a generator, the crossing that generator is heard through."""
FORGOTTEN: list[int] = []
"""FORGOTTEN holds the handles of the callables the engine made and of the classes a word defined that this
interpreter dropped since the life last heard of them, which the next verb of the operator says into the sandbox,
so the sandbox drops them too."""
MADE: weakref.WeakValueDictionary[int, Callable[..., object]] = weakref.WeakValueDictionary()
"""MADE holds every callable the engine made that crossed to this interpreter, by its handle, as the one function
this interpreter calls it by, for as long as this interpreter holds that function, so a callable read twice is one."""
CLASSES: weakref.WeakValueDictionary[int, type] = weakref.WeakValueDictionary()
"""CLASSES holds every class a word defined that crossed to this interpreter, by its handle, as the type this
interpreter holds it as, for as long as this interpreter holds that type."""
LIFE: Living | None = None
"""LIFE is the life this process holds, and a second boot ends it."""
SPEAKER: contextvars.ContextVar[str | None] = contextvars.ContextVar("speaker", default=None)
"""SPEAKER is the ear of this interpreter whose work speaks, on the thread of that ear and in the work it began there,
which keeps the context it was begun in, so a verb that work says later is said by that ear."""


def living() -> Living:
  """The life this process holds, which every verb says in."""
  if LIFE is None:  # pragma: no cover: a verb is said in a life; boot binds LIFE before any verb
    why = "no life"
    raise RuntimeError(why)
  return LIFE


def held_engine() -> _monty.Engine:
  """The engine of the crate that the life this process holds lives in."""
  engine = living().engine
  assert engine is not None
  return engine


def call(name: str, args: tuple, kwargs: dict[str, object]) -> object:
  """One verb of the engine, said by its name with its words, and what it gave.

  From the thread of a generator the verb is said by the sandbox where the generator is heard, since the life
  stands waiting for that generator. From anywhere else it is a verb of the operator.
  """
  held = getattr(LOCAL, "crossing", None)
  if held is not None:
    return held.calls(name, list(args), dict(kwargs))
  engine = held_engine()
  while FORGOTTEN:
    engine.forget(FORGOTTEN.pop())

  def said() -> object:
    if name in VERBS:
      return getattr(engine, name)(*args, **kwargs)
    return engine.verb(name, args, kwargs)

  if (who := SPEAKER.get()) is None:
    return said()
  before = engine.site(who)
  try:
    return said()
  finally:
    engine.site(before)


def speaks(value: str | None) -> str:
  """Who speaks in the life, and who speaks from now on when a value is given."""
  held = getattr(LOCAL, "crossing", None)
  got = held.calls("spoken", [value], {}) if held is not None else held_engine().site(value)
  assert isinstance(got, str)
  return got


class Crossing:
  """One generator of this interpreter, heard from the sandbox on a thread of its own through a generator that the
  engine of the crate steps as it steps any: sent what the generator hears, and thrown in what a verb it said raised.

  The generator is primed where it is made, on the thread of the loop, since an ear of the World reads the running
  loop at its first step, and what that step gave stands for its birth in the sandbox. From then, every fact the
  sandbox gives it goes through its inbox to the thread, and what the generator did with it comes back through its
  outbox: a saying, nothing, its end, or what it raised. A verb the generator says goes the other way through the
  same two doors, as a call it yields, and the value of the verb, or what it raised, wakes the thread.
  """

  def __init__(self, name: str | None, gen: Generator[tuple | None, tuple]) -> None:
    self.name = name
    self.gen = gen
    self.inbox: queue.Queue[object] = queue.Queue()
    self.outbox: queue.Queue[object] = queue.Queue()
    started = inspect.getgeneratorstate(gen) != inspect.GEN_CREATED
    self.first = ("step", None) if started else self.stepped(None)
    threading.Thread(target=self.serve, daemon=True).start()
    self.ear = self.stepping()
    # A generator that was started already stands at a yield, and so does the generator that crosses for it.
    if started:
      next(self.ear)

  def stepped(self, sent: object) -> tuple:
    """One step of the generator with what it was given, as the ear it hears by, and what came of it."""
    token = SPEAKER.set(self.name)
    try:
      out = self.gen.send(sent) if isinstance(sent, tuple) else next(self.gen)
    except StopIteration:
      return ("over",)
    except BaseException as no:
      return ("raised", no)
    finally:
      SPEAKER.reset(token)
    return ("step", None if out is None else tuple(out))

  def serve(self) -> None:
    """The thread of the generator: every fact from the inbox stepped, and what came of it put in the outbox."""
    LOCAL.crossing = self
    while (got := self.inbox.get()) is not END:
      self.outbox.put(self.stepped(got))

  def stepping(self) -> Generator[object, object]:
    """The generator that crosses for this one: it yields what this one did, a saying, nothing, or a call of a
    verb, and ends or raises as it did. At its birth it yields what the priming step gave. It is sent a fact, or the
    value of the verb this one said, and it is thrown in what that verb raised."""
    got = self.first
    while True:
      match got:
        case ("over",):
          return
        case ("raised", BaseException() as no):
          raise no
        case ("calls", str(name), list(args), dict(kwargs)):
          calling, step = True, {"verb": name, "args": args, "kwargs": kwargs}
        case _:
          assert isinstance(got, tuple)
          calling, step = False, got[1]
      try:
        sent = yield step
      except GeneratorExit:
        raise
      except BaseException as no:
        self.inbox.put(("raised", no))
      else:
        self.inbox.put(("value", sent) if calling else sent)
      got = self.outbox.get()

  def calls(self, name: str, args: list, kwargs: dict[str, object]) -> object:
    """One verb of the engine, said from the thread of the generator: the sandbox is told, and its value wakes
    the thread."""
    self.outbox.put(("calls", name, args, kwargs))
    got = self.inbox.get()
    if got is END:  # pragma: no cover: the generator's life is over and its thread is torn down
      why = "the life this generator was heard in is over"
      raise RuntimeError(why)
    assert isinstance(got, tuple)
    kind, value = got
    if kind == "raised":
      assert isinstance(value, BaseException)
      raise value
    return value

  def end(self) -> None:
    """The end of the life the generator was heard in, which ends its thread."""
    self.inbox.put(END)


class Living:
  """One life of the engine in the sandbox, and the generators of this interpreter it hears on threads of their
  own."""

  def __init__(self) -> None:
    self.crossings: list[Crossing] = []
    self.engine: _monty.Engine | None = None

  def crossed(self, name: str | None, ear: object) -> object:
    """An ear given under this name, as the engine of the crate hears it: a generator on a thread of its own, whose
    work speaks by that name, and an ear of the crate as itself."""
    if not isinstance(ear, Generator):
      return ear
    self.crossings.append(one := Crossing(name, ear))
    return one.ear

  def end(self) -> None:
    """The end of the life, with its ears: the thread of every generator it heard ends, and so does every ear of the
    crate."""
    if self.engine is not None:
      self.engine.dispose()
    for one in self.crossings:
      one.end()


def calling(n: int, args: tuple[object, ...], kwargs: dict[str, object]) -> object:
  """One call of what the engine holds under a handle, through the sandbox: from the thread of a generator as a
  verb is said there, and from anywhere else as a call of the operator."""
  held = getattr(LOCAL, "crossing", None)
  if held is not None:
    return held.calls("made", [n, list(args), dict(kwargs)], {})
  return held_engine().made(n, list(args), dict(kwargs))


def made(n: int) -> Callable[..., object]:
  """One callable the engine made, as this interpreter calls it: by its handle, which it carries as `__monty__`, so
  that it goes back in as the callable it is and never as a callable of this interpreter."""
  if (held := MADE.get(n)) is not None:
    return held

  def back(*args: object, **kwargs: object) -> object:
    return calling(n, args, kwargs)

  vars(back)["__monty__"] = n
  # When this interpreter drops the last reference, the handle is forgotten at the next verb of the operator.
  weakref.finalize(back, FORGOTTEN.append, n)
  MADE[n] = back
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

    held_engine().watch(str(self), told)
    got = await done
    if isinstance(got, BaseException):
      raise got
    return got


class Site:
  """Who is speaking in the life, read and set where it stands: a set gives back what stood before it, and a reset
  puts that back, which is what a token of a context variable does within one context."""

  def get(self) -> str:
    return speaks(None)

  def set(self, value: str) -> str:
    return speaks(value)

  def reset(self, previous: str) -> None:
    speaks(previous)


def boot(record: Iterable[object] = (), **outside: object) -> Act:
  """A life of the engine in the sandbox, opened from the record and on the ears given, which gives the root.

  An ear is a generator of this interpreter or an ear of the crate. The Kernel and the gate are the crate's, so an
  ear under either name is refused as the engine refuses one under a name of its own ears. A second boot is a
  second life, and the first is gone with its ears.
  """
  asyncio.get_running_loop()
  for name in ("kernel", "gate"):
    if name in outside:
      why = f"{name} hears: the engine of monty holds its Kernel and its gate"
      raise python.Refused(why)
  global LIFE  # noqa: PLW0603
  if LIFE is not None:
    LIFE.end()
  LIFE = living = Living()
  ears = [(name, living.crossed(name, ear)) for name, ear in outside.items()]
  living.engine = _monty.Engine.boot(list(record), ears)
  if (no := living.engine.raised) is not None:
    raise no
  return Act(living.engine.root)


def ours(making: object) -> bool:
  """Whether a callable is one of this interpreter: no name of the engine, and no callable the engine made, which go
  back in as themselves."""
  return callable(making) and not hasattr(making, "__monty__") and all(making is not one for one in NAMES.values())


def eared(name: str, args: tuple) -> tuple:
  """The words of a name of the engine that takes an ear, with each ear as the engine of the crate hears it.

  A generator is heard on a thread of its own, under the name drive gives it. A function that makes an ear is given
  the name of that ear, which is how an act brings its ear to life, so the ear it makes is heard on a thread of its
  own under that name. Either way the work of the ear speaks by its name.
  """
  match name, args:
    case ("drive" | "lives", (Generator() as g, *rest)):
      return (living().crossed(str(rest[0]) if name == "drive" else None, g), *rest)
    case ("act", (kind, on, making, *rest)) if ours(making):
      return (kind, on, lambda act: living().crossed(act, making(act)), *rest)
    case ("pausing" | "ending", (making, *rest)) if ours(making):
      return (lambda act: living().crossed(act, making(act)), *rest)
  return args


def worded(name: str) -> Callable[..., object]:
  """One verb of the engine, said by its name in the sandbox with the words it was given."""

  def verb(*args: object, **kwargs: object) -> object:
    got = call(name, eared(name, args), kwargs)
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
  if _name == "site":
    globals()[_name] = Site()
  elif _name in PURE or isinstance(_held, type) or not callable(_held):
    globals()[_name] = _held
  else:
    globals()[_name] = worded(_name)

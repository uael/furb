"""The engine in monty, with every name of engine.pyi.

This module gives the names of `furb.python` over one life of that engine in the sandbox of monty. A name that reads
no life is the engine's own. A verb is the method of the engine of the crate that says it, and any other name says
itself in the sandbox, and what it gave comes back as the instance of `furb.python` it is. An act is awaited for what
it comes to. A generator of this interpreter is heard where the engine runs, and a verb it says while it hears goes
to the life that hears it.
"""

import ast
import asyncio
import contextvars
import inspect
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
  "question", "headed", "bound", "shown", "unquoted", "offered", "ended", "idle",
})  # fmt: skip
"""PURE are the callables of the engine that read no life, so the engine of this interpreter answers them."""
ACTS = frozenset({"wait", "rung", "prompt", "chain", "grant", "bash", "act"})
"""ACTS are the verbs that give an act, whose name comes back as the act it names."""
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
"""SPEAKER is the ear of this interpreter whose work speaks, while that ear hears and in the work it began then, which
keeps the context it was begun in, so a verb that work says later is said by that ear."""


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

  While the life hears a generator of this interpreter, the verb is that generator's, and the life that waits on it
  answers it. Anywhere else it is a verb of the operator.
  """
  if _monty.hearing():
    return _monty.call(name, list(args), dict(kwargs))
  engine = held_engine()
  while FORGOTTEN:
    engine.forget(FORGOTTEN.pop())

  if (who := SPEAKER.get()) is None:
    return engine.verb(name, args, kwargs)
  before = engine.site(who)
  try:
    return engine.verb(name, args, kwargs)
  finally:
    engine.site(before)


def speaks(value: str | None) -> str:
  """Who speaks in the life, and who speaks from now on when a value is given."""
  got = _monty.call("spoken", [value], {}) if _monty.hearing() else held_engine().site(value)
  assert isinstance(got, str)
  return got


def heard(name: str | None, gen: Generator[tuple | None, tuple | None]) -> Generator[tuple | None, tuple | None]:
  """One generator of this interpreter, as the engine of the crate hears it: each step of it is taken with the name
  it is heard under as the one who speaks, so the work it begins then speaks by that name. A generator that was
  started already stands at a yield, and so does the generator that is heard for it."""
  sent = (yield None) if inspect.getgeneratorstate(gen) != inspect.GEN_CREATED else None
  while True:
    token = SPEAKER.set(name)
    try:
      said = gen.send(sent)
    except StopIteration:
      return
    finally:
      SPEAKER.reset(token)
    sent = yield said


class Living:
  """One life of the engine in the sandbox."""

  def __init__(self) -> None:
    self.engine: _monty.Engine | None = None

  def crossed(self, name: str | None, ear: object) -> object:
    """An ear given under this name, as the engine of the crate hears it: a generator whose work speaks by that
    name, and an ear of the crate as itself."""
    if not isinstance(ear, Generator):
      return ear
    started = inspect.getgeneratorstate(ear) != inspect.GEN_CREATED
    one = heard(name, ear)
    if started:
      next(one)
    return one

  def end(self) -> None:
    """The end of the life, with its ears: every ear of the crate ends."""
    if self.engine is not None:
      self.engine.dispose()


def calling(n: int, args: tuple[object, ...], kwargs: dict[str, object]) -> object:
  """One call of what the engine holds under a handle, through the sandbox: by the generator the life hears, while
  it hears one, and as a call of the operator anywhere else."""
  if _monty.hearing():
    return _monty.call("made", [n, list(args), dict(kwargs)], {})
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
    """What the act came to, once the life holds it: the value, or the exception it completed with, raised."""
    return held_engine().result(str(self)).__await__()


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


def pending() -> list[tuple[str, str]]:
  """The work that an earlier life left, which waits for a wake that this life says, each act by its name and its
  kind, as the engine of the crate finds it."""
  return held_engine().pending()


def ours(making: object) -> bool:
  """Whether a callable is one of this interpreter: no name of the engine, and no callable the engine made, which go
  back in as themselves."""
  return callable(making) and not hasattr(making, "__monty__") and all(making is not one for one in NAMES.values())


def eared(name: str, args: tuple) -> tuple:
  """The words of a name of the engine that takes an ear, with each ear as the engine of the crate hears it.

  A generator is heard under the name drive gives it. A function that makes an ear is given the name of that ear,
  which is how an act brings its ear to life, so the ear it makes is heard under that name. Either way the work of
  the ear speaks by its name.
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

"""The stand-in: what stands inside the sandbox for a host that is not python.

The engine takes any generator of the outside as an ear, under the name it hears by. An ear of a host written in
another language is no generator, so a generator here stands in its place: every fact it is given goes to the ears
of the host by that name, and what comes back is what the ear did with it. The ears of the World are ears of the
host like any other.

The Kernel is here too, since the word of a rung runs where the engine runs, in the module of its chain, and so
is the gate, an ear of its own, which reads a word with the checker of the host on the sheet of the engine.

What crosses, crosses as the interpreter carries it, but for the callables, which it carries no way back. An
instance of a class of the engine comes in as a map that names its class under `is`, with its fields, since the
interpreter makes no instance of a class of the sandbox on a host's behalf, and so the instance is made here. A
name of the engine crosses as its name, both ways, so a show of the engine is the same show on both sides. A
callable the engine made goes out as a handle the host calls it back by, and a function of the host comes in as a
function the interpreter calls back on the host. A class a word defined goes out by a handle too, with the class it
was made with, which the host holds as a type of its own derived from the type that base is there, and the class
comes back in by; an instance of one goes out with its fields under its class and comes back in made here from
them, with no `__init__` run. An instance of a class that inherits `str` is a string, and crosses as one. A map
that holds the key `is` crosses as its pairs under `is` with the name `dict`, both ways, so no side reads the map
of a word or of a host as a mark.
"""

from ast import PyCF_ALLOW_TOP_LEVEL_AWAIT
from collections.abc import Callable, Coroutine, Generator
from contextvars import ContextVar
from typing import TYPE_CHECKING

from monty import instance

type Names = dict[str, object]
"""The names of the engine: its module, in which every word of the operator and every stand-in runs."""
type Ear = Generator[tuple | None, tuple | None]
"""An ear, as the engine hears one."""


# The two objects of the host, as the stand-in reaches them. The sandbox makes no class from a class of the
# interpreter, so they are read here and never run.
if TYPE_CHECKING:
  from typing import Protocol

  class Ears(Protocol):
    """The ears of the host, by the name the engine hears each by."""

    def born(self, name: str, by: str) -> object: ...
    def hears(self, name: str, fact: object) -> object: ...
    def answered(self, name: str, got: object) -> object: ...

  class Gate(Protocol):
    """The gate of the host, which reads a sheet and answers each finding by its line."""

    def checked(self, sheet: str) -> object: ...


IS = "is"
"""IS marks a map that is an instance of a class of the engine, by the name of that class."""
MODULE: dict[str, object] = {
  "__debug__": True,
  "__doc__": None,
  "__package__": "furb",
  "__spec__": None,
  "__loader__": None,
}
"""MODULE is what the module of the engine holds before the engine runs: the names python gives that module, so a
word reads them in a chain as it reads them in python, and as the gate reads them. No loader made the module here, so
it has no spec and no loader. Python binds `__debug__` among its builtins, and the sandbox binds it in its main module
alone."""
MADE: dict[int, object] = {}
"""MADE holds every callable the engine made and every class a word defined that crossed to the host, by its
handle, which is its identity, for as long as the host holds the handle: the host says when it forgot one, and it
is dropped then."""


def verb(names: Names, which: str) -> Callable[..., object]:
  """One name of the engine that is a verb."""
  got = names[which]
  assert callable(got), which
  return got


def named(x: object, names: Names) -> str | None:
  """The name of the engine a value is bound to, when it is one, by identity."""
  return next((name for name, held in names.items() if held is x and not name.startswith("_")), None)


def worded(cls: type, names: Names) -> bool:
  """Whether a class is one a word defined: a class of the session, which the interpreter says is written in
  `__main__` as it says of the engine's, and no name of the engine. A builtin type has no module."""
  try:
    cls.__module__  # noqa: B018  # the read is the test: a builtin type raises
  except AttributeError:
    return False
  return named(cls, names) is None


def handled(x: object) -> int:
  """The handle a value goes out by, which is its identity, held here for as long as the host holds it."""
  MADE[id(x)] = x
  return id(x)


def outward(x: object, names: Names) -> object:
  """A value as it goes out to the host: a callable or a class of the engine as its name, a callable the engine
  made as the handle the host holds it by, a class a word defined as that handle with the class it was made with
  as it goes out, an instance of such a class with its fields under its class, a map that holds the key IS as its
  pairs, and the entries of a container each as they go out. Anything else the interpreter carries out as it is,
  a string of any class among it."""
  match x:
    case dict() if IS in x:
      return {IS: "dict", "args": [[(k, outward(v, names)) for k, v in x.items()]]}
    case dict():
      return {k: outward(v, names) for k, v in x.items()}
    case list():
      return [outward(v, names) for v in x]
    case tuple():
      return tuple(outward(v, names) for v in x)
    case str():
      return x
  if (name := named(x, names)) is not None and callable(x):
    return {IS: "name", "name": name}
  if isinstance(x, type):
    if not worded(x, names):
      return x
    return {IS: "class", "id": handled(x), "name": x.__name__, "base": outward(x.__bases__[0], names)}
  if worded(type(x), names):
    return {IS: "instance", "class": outward(type(x), names), "value": x}
  if not callable(x):
    return x
  return {IS: "made", "id": handled(x)}


def known(names: Names, name: str) -> object:
  """What a name of a class is bound to: in the engine, or in the builtins its module reads, and nothing otherwise."""
  try:
    return eval(name, names)  # noqa: S307  # a bare name, resolved as the module resolves one
  except NameError:
    return None


def again(x: object, names: Names, ears: Ears) -> object:
  """A value of the host, as the engine holds it: an instance of a class of the engine made from its name and its
  fields, an ear of the host as the generator that stands in for it, and everything else as it came, its entries
  made again."""
  match x:
    # A name of the engine crosses as its name, so a show of the engine is the engine's own here.
    case {"is": "name", "name": str(name)} if name in names:
      return names[name]
    # A callable the engine made, or a class a word defined, back from the host by the handle it crossed under.
    case {"is": "made", "id": int(n)}:
      return MADE[n]
    # An instance of a class a word defined, back from the host by the handle of its class and its fields, made
    # here from them as the interpreter makes one, with no `__init__` run.
    case {"is": "instance", "class": int(n), "fields": dict(fields)}:
      cls = MADE[n]
      assert isinstance(cls, type)
      return instance(cls, {str(k): again(v, names, ears) for k, v in fields.items()})
    case {"is": "ear", "name": str(name), "started": bool(started)}:
      ear = crossing(name, ears, names, started=started)
      # A generator that was started already stands at a yield, and so must its stand-in.
      if started:
        ear.send(None)
      return ear
    case {"is": "Templated", "interpolations": list(held)}:
      return Templated([Interpolated(again(value, names, ears), str(expression)) for value, expression in held])
    case {"is": str(name), **fields} if callable(maker := known(names, name)):
      held = {str(k): again(v, names, ears) for k, v in fields.items()}
      # An exception takes its arguments by position, and by position alone: the interpreter makes one from at most
      # one string and refuses a call that carries keywords, empty or not. A dataclass takes its fields by name.
      args = held.pop("args", None)
      if isinstance(args, list | tuple):
        return maker(*args) if not held else maker(*args, **held)
      return maker(**held)
    case dict():
      return {k: again(v, names, ears) for k, v in x.items()}
    case list():
      return [again(v, names, ears) for v in x]
    case tuple():
      return tuple(again(v, names, ears) for v in x)
  # What else is callable is a function of the host, the one callable that crosses in as itself.
  if callable(x) and not isinstance(x, type):
    return hosted(x, names, ears)
  return x


def hosted(f: Callable[..., object], names: Names, ears: Ears) -> Callable[..., object]:
  """A function of the host, as the engine calls it: what it is given goes out as any value goes out, and what it
  gives comes in as any value comes in, so a function that gives an ear gives the generator that stands in for it."""

  def called(*args: object, **kwargs: object) -> object:
    got = f(*[outward(v, names) for v in args], **{k: outward(v, names) for k, v in kwargs.items()})
    return again(got, names, ears)

  return called


def crossing(name: str, ears: Ears, names: Names, *, started: bool = False) -> Ear:
  """One ear of the host, heard under its name.

  The ear is on the other side, so this stands in for it. At its birth the ear is told the name it speaks by, and
  then every fact it is given goes to the ears of the host, and what comes back is what the ear did with it. It said
  something, which this yields, and the bus hands back the fact as it was said, which goes to the host next. It said
  nothing, so this waits for the next fact. It said a verb, which is said here by its name and its value handed
  back. It is over, so this returns, and it raised, so this raises. A generator that the host started before it
  crossed was born there, so its stand-in begins where it waits.
  """
  speaking = names["site"]
  assert isinstance(speaking, ContextVar)
  reply = None if started else ears.born(name, speaking.get())
  while True:
    match reply:
      case ("calls", str(which), list(args), dict(kwargs)):
        # What the ear asked raised: the ear is answered with the raise, and never left waiting.
        try:
          got = ("value", outward(asked(names, ears, which, args, kwargs), names))
        except BaseException as no:
          got = ("raised", no)
        reply = ears.answered(name, got)
        continue
      case ("raised", no):
        raise fault(no, names, ears)
      case ("say", tuple(saying)):
        made = again(saying, names, ears)
        assert isinstance(made, tuple)
        a = yield made
      case ("over",):
        return
      case _:
        a = yield
    reply = ears.hears(name, outward(a, names))


def fault(no: object, names: Names, ears: Ears) -> BaseException:
  """What the host raised, made again as the exception it is."""
  made = again(no, names, ears)
  assert isinstance(made, BaseException)
  return made


def ran_in(word: str, rung: str, module: dict[str, object]) -> object:
  """The word of a rung, run in the globals of its chain.

  A rung may await at its top level, so the word is compiled as a body of a module that takes one: a word that
  awaits gives back what carries it forward, and a word that does not is over where it began.
  """
  return eval(compile(word, rung, "exec", flags=PyCF_ALLOW_TOP_LEVEL_AWAIT), module)  # noqa: S307


class Running:
  """The runs of one life: the word of each rung that stands.

  The Kernel takes a run as that run, and begins its word when it hears that it took it, in the globals of its chain
  and as its rung, which is what makes a fact of the word the rung's own. When the word waits for an act that is not
  done, it makes a wants as the run, and carries the word forward at the done of that wants. It says the run done,
  as the run, with what the word gave.
  """

  def __init__(self, names: Names) -> None:
    site = names["site"]
    assert isinstance(site, ContextVar)
    self.site: ContextVar[str] = site
    # A name of the engine is read at each use and never held: boot binds the bus into the engine itself, and a
    # rebound name is used from the next use on, so a verb kept here would be the one that stood before it.
    self.names = names
    self.frames: dict[str, Coroutine[object, object, object]] = {}
    self.taken: set[str] = set()
    self.waits: dict[str, str] = {}

  def of(self, run: str) -> tuple:
    """The run, as the life holds it: its chain, its rung and its word, among its words."""
    got = verb(self.names, "get")(run)
    assert isinstance(got, tuple)
    return got

  def took(self, run: str) -> None:
    """The run, taken as that run, which is what the Kernel says of it first."""
    with self.site.set(run):
      verb(self.names, "say")("started", run)
    self.taken.add(run)

  def ended(self, run: str, got: BaseException | None) -> None:
    """The word is over, and the run is done with what the word gave, as that run."""
    self.frames.pop(run, None)
    with self.site.set(run):
      verb(self.names, "say")("done", run, got)

  def carry(self, run: str, given: object) -> None:
    """The word stepped as its rung with what it waited for, and stepped again while what it waits for is done."""
    peek = verb(self.names, "peek")
    with self.site.set(str(self.of(run)[4])):
      while True:
        try:
          frame = self.frames[run]
          got = frame.throw(given) if isinstance(given, BaseException) else frame.send(given)
          # A rung awaits an act and nothing else, so anything else is refused where the word waited.
          while not isinstance(got, str):
            got = frame.throw(self.refusal(got))
        except StopIteration:
          return self.ended(run, None)
        except BaseException as raised:
          return self.ended(run, raised)
        if peek(got, ...) is ...:
          with self.site.set(run):
            wants = verb(self.names, "act")("wants", "", None, got)
          assert isinstance(wants, str)
          self.waits[wants] = run
          return None
        given = peek(got)

  def refusal(self, got: object) -> BaseException:
    made = verb(self.names, "Refused")(f"a rung awaits an act, and {got!r} is none")
    assert isinstance(made, BaseException)
    return made

  def begin(self, run: str) -> None:
    """A word begun as its rung: it runs in the globals of its chain, and one that awaits nothing is over where it
    began. A rung that is done already begins no word."""
    _, _, _, chain, rung, word, *_ = self.of(run)
    self.taken.discard(run)
    if verb(self.names, "peek")(rung, ...) is not ...:
      return None
    module = verb(self.names, "module")(chain)
    assert isinstance(module, dict)
    try:
      with self.site.set(str(rung)):
        ran = ran_in(str(word), str(rung), module)
    except BaseException as raised:
      return self.ended(run, raised)
    if not isinstance(ran, Coroutine):
      return self.ended(run, None)
    self.frames[run] = ran
    return self.carry(run, None)

  def dropped(self, about: str) -> None:
    """Every run whose rung a control is over, dropped: the frame of a word that is mid step is never closed."""
    under = verb(self.names, "under")
    for one in [x for x in self.frames if under(self.of(x)[4], about)]:
      if not getattr(self.frames[one], "cr_running", False):
        self.frames[one].close()
        got = verb(self.names, "CancelledError")()
        assert isinstance(got, BaseException)
        self.ended(one, got)


def gating(gate: Gate, sheet: Names, engine: Names) -> Ear:
  """The gate as the ear of a life: it reads the word of a rung on its sheet, after the program the gate says.

  The sheet is `furb.sheet`'s, written here as the python package writes it, from the names of the module of the
  engine, and the reading of it is the gate of the host, given the sheet, which answers each finding by its line.
  """

  def checked(text: str) -> list[tuple[int, str]]:
    found = gate.checked(text)
    assert isinstance(found, list)
    return [(line, why) for line, why in found]

  while True:
    match (yield):
      case ("gate", qid, _, on, word):
        program = verb(engine, "program")(on)
        assert isinstance(program, dict)
        yield "done", qid, verb(sheet, "gate")(engine, [*program.values()], word, checked)


def kernel(names: Names) -> Ear:
  """The Kernel: it takes each run, begins its word when it hears that it took it, and carries the word at the done
  of each wants it made."""
  held = Running(names)
  while True:
    match (yield):
      case ("run", run, *_):
        # This Kernel runs every word, retold or not, so it reads no donor off the run.
        held.took(run)
      case ("started", run, *_) if run in held.taken:
        held.begin(run)
      case ("done", wants, _, value) if wants in held.waits:
        held.carry(held.waits.pop(wants), value)
      case ("cancel" | "close", about, *_):
        held.dropped(about)


def loaded(source: str, held: dict[str, object]) -> dict[str, object]:
  """One module of its own, from its source, run in what it holds before: a namespace nothing else shares, under a
  name the engine binds to nothing, since a word of the host reads the names of the preamble beside the engine's."""
  exec(source, held)  # noqa: S102
  return held


def opened(engine: Names, sheet: Names, record: object, gate: Gate, ears: Ears, names: list[str]) -> tuple:
  """A life of that engine, opened from what the store kept of the life before it, on the ears of these names, in
  this order: the root it opened on, and what boot raised, if it raised.

  The Kernel and the gate are given first, so that the gate answers before any ear hears it. The record is the
  entries as the store hands them: each the fact as a tuple.
  """
  MADE.clear()
  kept = again(record, engine, ears)
  assert isinstance(kept, list)
  entries = [(tuple(e[0]), *e[1:]) for e in kept]
  outside = {name: crossing(name, ears, engine) for name in names}
  try:
    root = verb(engine, "boot")(entries, kernel=kernel(engine), gate=gating(gate, sheet, engine), **outside)
  except BaseException as no:
    # What boot raised comes out of the entry the operator went in by, and the life goes on: a drift breaks the
    # journal and keeps nothing more, so the root stands when the record held it.
    return ("chain1" if verb(engine, "get")("chain1") is not None else "", no)
  return (str(root), None)


def called(engine: Names, ears: Ears, name: str, args: list, kwargs: dict) -> object:
  """One verb of the engine, called by the operator with values of the host, and what it gave, as it goes out."""
  words, held = again(args, engine, ears), again(kwargs, engine, ears)
  assert isinstance(words, list)
  assert isinstance(held, dict)
  return outward(verb(engine, name)(*words, **held), engine)


def made_called(engine: Names, ears: Ears, n: int, args: list, kwargs: dict) -> object:
  """One callable the engine made, called back by the host by its handle, and what it gave, as it goes out."""
  words, held = again(args, engine, ears), again(kwargs, engine, ears)
  assert isinstance(words, list)
  assert isinstance(held, dict)
  made = MADE[n]
  assert callable(made)
  return outward(made(*words, **held), engine)


def forgotten(n: int) -> None:
  """One handle the host holds no more, so the callable or the class it named is the sandbox's to drop."""
  MADE.pop(n, None)


def asked(engine: Names, ears: Ears, which: str, args: list, kwargs: dict) -> object:
  """What an ear of the host asks of the life from a thread of its own, answered here on its behalf: who speaks,
  or a verb of the engine."""
  match which:
    case "spoken":
      return spoken(engine, *args)
    case "made":
      return made_called(engine, ears, *args)
  return called(engine, ears, which, args, kwargs)


def outcomes_of(engine: Names, ids: list) -> list:
  """What each of these acts came to, in one reading: what it came to, alone in a tuple, or nothing for an act
  that lives, so that an act that came to nothing is told apart from one that is not done."""
  peek = verb(engine, "peek")
  return [None if (got := peek(one, ...)) is ... else (outward(got, engine),) for one in ids]


def spoken(engine: Names, value: object) -> str:
  """Who speaks in the life, and who speaks from now on when a value is given: `site`, read and set where it stands."""
  site = engine["site"]
  assert isinstance(site, ContextVar)
  previous = site.get()
  if value is not None:
    site.set(str(value))
  return previous


class Interpolated:
  """One interpolation of a template string of the host: its value and its expression, which a tell of it reads."""

  def __init__(self, value: object, expression: str) -> None:
    self.value = value
    self.expression = expression


class Templated:
  """A template string of the host, by its interpolations, since this interpreter makes one from a literal alone."""

  def __init__(self, interpolations: list[Interpolated]) -> None:
    self.interpolations = interpolations


def said(engine: Names, ears: Ears, by: str, saying: object) -> None:
  """One saying of the work of an ear, said into the life once the hearing that began that work is over, under the
  name of that ear."""
  site = engine["site"]
  assert isinstance(site, ContextVar)
  made = again(saying, engine, ears)
  assert isinstance(made, tuple)
  with site.set(by):
    verb(engine, "say")(*made)

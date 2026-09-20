"""The boundary of a host that is not python: the ears of the host, the Kernel, and the plain form of every value.

The engine takes the World, and any other generator of the outside, as an ear under the name it hears by. A host
written in another language is no generator, so this stands in its place: it hears every fact, makes it plain,
hands it to the host under the name of the ear, and says back what the host answers. Nothing of the engine is
bound here, and nothing here is bound in the engine: the crate runs this in a module of its own and hands the
generators to boot, so the globals of a chain hold what the file defines and nothing more.

The host answers a fact one of four ways: with nothing, with one saying, which the ear yields and the bus hands back
whole as the next fact the ear hears; with a word to read in the names of the engine, whose value is handed back
and the host asked again; or with what the ear raised, which is raised in the ear. A host that reads the engine
never calls into a life that stands waiting for it.

A callable of the host crosses as a name: a show, a filter or an ear a host hands a verb is called back through
the host with plain arguments, and a generator of the host is heard the way an ear is. A template string cannot be
made in this interpreter, so one of the host crosses as the interpolations it holds, which is all a tell of it reads.
"""

from ast import PyCF_ALLOW_TOP_LEVEL_AWAIT
from collections.abc import Callable, Coroutine, Generator
from contextvars import ContextVar

type Host = Callable[[str, object], object]
"""A host hears one plain value under the name of the ear it is for, and answers plain."""
type Names = dict[str, object]
"""The names of the engine, by which a plain value is made again, and which the Kernel reaches the bus through."""
type Channel = Callable[[object], object]
"""One way to the host for one ear: what the ear was given, plain, and what the host answers, plain."""

TUPLE = "()"
"""TUPLE is how the plain form says a tuple, since a host that says a fact back must say it as the fact it was."""
SHOW = ""
"""SHOW is how the plain form says a callable of the engine that no host reads: a show, a filter, an ear."""
NAME = "name"
"""NAME is how the plain form says a name of the engine, such as a verb, which a host of python reads as its own."""
ACT = "Act"
"""ACT is how the plain form says the name of an act, which is a string to a host and an act to python."""
CALL = "call"
"""CALL is how the plain form says a callable of the host, which the engine calls back through the host by id."""
GEN = "gen"
"""GEN is how the plain form says a generator of the host, which the engine hears the way it hears an ear."""
TEMPLATE = "Template"
"""TEMPLATE is how the plain form says a template string of the host, by its interpolations."""
MADE = "made"
"""MADE is how the plain form says a callable of the engine that has no name, such as a show a word made, which a
host of python calls back through a word by its id."""

made: list[object] = []
"""made holds every callable of the engine that crossed to the host by its id, for as long as the life lives."""


def verb(names: Names, which: str) -> Callable[..., object]:
  """One name of the engine that is a verb, which is what a name the boundary calls must be."""
  got = names[which]
  assert callable(got), which
  return got


def known(names: Names, x: object) -> str | None:
  """The name of the engine a callable is bound under, and nothing for a callable of no name."""
  return next((k for k, v in names.items() if v is x and not k.startswith("_")), None)


def wire(x: object, names: Names) -> object:
  """The plain form of a value: an act its name, an exception its name and what it was made with, a shape its name
  beside its fields, a tuple its entries under its own mark, a list its entries, a map its entries, a name of the
  engine its name, and plain data is plain.

  What is none of these is a callable, which no host reads and no record holds, and it crosses as a mark of what
  it was and nothing else.
  """
  if type(x).__name__ == ACT:
    return {"is": ACT, "args": [str(x)]}
  match x:
    case None | bool() | int() | float() | str():
      return x
    case BaseException():
      return {"is": type(x).__name__, "args": [wire(i, names) for i in x.args]}
    case dict():
      return {k: wire(v, names) for k, v in x.items()}
    case tuple():
      return {"is": TUPLE, "args": [wire(i, names) for i in x]}
    case list():
      return [wire(i, names) for i in x]
  fields = getattr(x, "__dataclass_fields__", None)
  if fields is not None and not isinstance(x, type):
    return {"is": type(x).__name__} | {name: wire(getattr(x, name), names) for name in fields}
  name = known(names, x)
  if name is not None:
    return {"is": NAME, "name": name}
  if not callable(x):
    return {"is": SHOW, "of": type(x).__name__}
  made.append(x)
  return {"is": MADE, "id": len(made) - 1}


def unwire(x: object, names: Names, host: Host) -> object:
  """The value again from the plain form, made by what its name is known by: a name of the engine, or of the
  interpreter when the engine holds none."""
  match x:
    case list():
      return [unwire(i, names, host) for i in x]
    case {"is": str(mark), "args": list(held)} if mark == TUPLE:
      return tuple(unwire(i, names, host) for i in held)
    case {"is": str(mark), "id": int(n)} if mark == CALL:
      return calling(n, names, host)
    case {"is": str(mark), "id": int(n), "started": bool(started)} if mark == GEN:
      g = crossing(lambda a, n=n: host(GEN, [n, a]), names, host)
      if started:
        g.send(None)
      return g
    case {"is": str(mark), "interpolations": list(held)} if mark == TEMPLATE:
      return Templated([Interpolated(unwire(v, names, host), e) for v, e in held])
    case {"is": str(mark), "name": str(name)} if mark == NAME:
      return names[name]
    case {"is": str(mark)} if mark == SHOW:
      return None
    case {"is": str(name), **rest}:
      held = rest.pop("args", [])
      args = [unwire(i, names, host) for i in held] if isinstance(held, list) else []
      maker = names[name] if name in names else eval(name)  # noqa: S307
      assert callable(maker), name
      # An exception of this interpreter takes its arguments by position and no keyword at all.
      return maker(*args) if not rest else maker(*args, **{str(k): unwire(v, names, host) for k, v in rest.items()})
    case dict():
      return {k: unwire(v, names, host) for k, v in x.items()}
  return x


class Interpolated:
  """One interpolation of a template string of the host: its value and its expression, which a tell of it reads."""

  def __init__(self, value: object, expression: str) -> None:
    self.value = value
    self.expression = expression


class Templated:
  """A template string of the host, by its interpolations, since this interpreter makes one from a literal alone."""

  def __init__(self, interpolations: list[Interpolated]) -> None:
    self.interpolations = interpolations


def calling(n: int, names: Names, host: Host) -> Callable[..., object]:
  """One callable of the host, called back through the host with plain arguments, and what it gave, made again."""

  def called(*args: object) -> object:
    got = host(CALL, [n, [wire(a, names) for a in args]])
    return answered(got, names, host)

  return called


def answered(got: object, names: Names, host: Host) -> object:
  """What a call of the host came to: its value, or what it raised, which is raised here."""
  match got:
    case {"raised": no}:
      raise raised(no, names, host)
    case {"value": value}:
      return unwire(value, names, host)
  return None


def raised(no: object, names: Names, host: Host) -> BaseException:
  """What the host raised, made again as the exception it is."""
  made = unwire(no, names, host)
  assert isinstance(made, BaseException)
  return made


def unwiring(engine: dict[str, object], host: Host) -> Callable[[object], object]:
  """The plain form read back for one life, which a word of the operator reaches under the name `unwire`.

  It is made once for the life, since the sandbox counts the functions a session defines and a word of the
  operator is said many times over.
  """
  return lambda x: unwire(x, engine, host)


def crossing(channel: Channel, names: Names, host: Host) -> Generator[tuple | None, tuple | None]:
  """One generator of the host, heard through a channel.

  The generator is on the other side, so this stands in for it: every fact it is given goes through the channel,
  and what comes back is what the generator did with it. It yielded a saying, which this yields, and the bus hands
  back the fact as it was said, which goes through the channel next. It yielded nothing, so this waits for the
  next fact. It read the engine, which is a word run here in the names of the engine and handed back. It returned,
  so this returns, and it raised, so this raises.
  """
  a, unwire_ = None, unwiring(names, host)
  while True:
    reply = channel(wire(a, names))
    while True:
      match reply:
        case {"reads": str(word)}:
          reply = channel({"answered": wire(eval(word, names, {"unwire": unwire_, "made": made}), names)})  # noqa: S307
        case {"raised": no}:
          raise raised(no, names, host)
        case {"say": list(saying)}:
          said = unwire(saying, names, host)
          assert isinstance(said, list)
          a = yield tuple(said)
          break
        case {"over": _}:
          return
        case _:
          a = yield
          break


def ran_in(word: str, rung: str, module: dict[str, object]) -> object:
  """The word of a rung, run in the globals of its chain.

  A rung may await at its top level, so the word is compiled as a body of a module that takes one: a word that
  awaits gives back what carries it forward, and a word that does not is over where it began.
  """
  return eval(compile(word, rung, "exec", flags=PyCF_ALLOW_TOP_LEVEL_AWAIT), module)  # noqa: S307


class Running:
  """The runs of one life: the word of each rung that stands, and the ladder of each chain.

  A run is begun in the globals of its chain and carried forward at the done of every act its word waits for, and
  what it says it says under the name of its rung, which is what makes a fact of the word the rung's own.
  """

  def __init__(self, names: Names) -> None:
    """The runs of the life whose engine these names are."""
    site, outcomes, modules = names["site"], names["outcomes"], names["modules"]
    assert isinstance(site, ContextVar)
    assert isinstance(outcomes, dict)
    assert isinstance(modules, dict)
    self.site: ContextVar[str] = site
    self.outcomes: dict[str, object] = outcomes
    self.modules: dict[str, dict[str, object]] = modules
    # A name of the engine is read at each use and never held: boot binds the bus into the engine itself, and a
    # rebound name is used from the next use on, so a verb kept here would be the one that stood before it.
    self.names = names
    self.frames: dict[str, Coroutine[object, object, object]] = {}
    self.ladders: dict[str, list[str]] = {}

  def ended(self, rung: str, got: BaseException | None) -> None:
    """The run is over, and what it came to goes to the chain that had it run."""
    self.frames.pop(rung, None)
    verb(self.names, "send")("ran", rung, got, by=rung)

  def carry(self, rung: str, sent: object) -> None:
    """The run stepped with what it waited for, and stepped again while what it waits for is over already."""
    token = self.site.set(rung)
    try:
      while True:
        try:
          frame = self.frames[rung]
          got = frame.throw(sent) if isinstance(sent, BaseException) else frame.send(sent)
          # A rung awaits an act and nothing else, so anything else is refused where the word waited.
          while not isinstance(got, str):
            got = frame.throw(self.refusal(got))
        except StopIteration:
          return self.ended(rung, None)
        except BaseException as raised:
          return self.ended(rung, raised)
        if got not in self.outcomes:
          verb(self.names, "send")("wants", rung, got, by=rung)
          return None
        sent = self.outcomes[got]
    finally:
      self.site.reset(token)

  def refusal(self, got: object) -> BaseException:
    """What a word that awaited something that is no act is given where it waited."""
    made = verb(self.names, "Refused")(f"a rung awaits an act, and {got!r} is none")
    assert isinstance(made, BaseException)
    return made

  def begin(self, rung: str, chain: str, word: str) -> None:
    """A run begun: the word runs in the globals of its chain, and one that awaits nothing is over where it began."""
    token = self.site.set(rung)
    try:
      ran = ran_in(word, rung, self.modules[chain])
    except BaseException as raised:
      return self.ended(rung, raised)
    finally:
      self.site.reset(token)
    if not isinstance(ran, Coroutine):
      return self.ended(rung, None)
    self.frames[rung] = ran
    return self.carry(rung, None)

  def dropped(self, about: str) -> None:
    """Every run a control is over, dropped: the frame of a word that is mid step is never closed."""
    under = verb(self.names, "under")
    for one in [x for x in self.frames if under(x, about) and not getattr(self.frames[x], "cr_running", False)]:
      self.frames[one].close()
      got = verb(self.names, "CancelledError")()
      assert isinstance(got, BaseException)
      self.ended(one, got)


def kernel(name: str, host: Host, names: Names, sheet: Names, bound: list[str]) -> Generator[tuple | None, tuple]:
  """The Kernel: it gates the word of a rung on its sheet, and it runs that word in the module of its chain.

  The sheet is `furb.sheet`'s, written here as the python package writes it, and the reading of it is the crate's:
  ty is asked of the host under the name of the Kernel, given the sheet, and answers each finding by its line. The
  run is here: the word runs where the engine runs, in the globals of its chain, so nothing of it crosses and
  nothing calls back into a life that stands waiting.
  """
  held = Running(names)

  def checked(text: str) -> list[tuple[int, str]]:
    found = unwire(host(name, text), names, host)
    assert isinstance(found, list)
    return [(line, why) for line, why in found]

  while True:
    match (yield):
      case ("gate", qid, _, chain, word):
        # The ladder is the accepted words of the chain, so an accepted word joins it here, before it runs.
        found = verb(sheet, "gate")(bound, held.ladders.setdefault(chain, []), word, checked)
        if not found:
          held.ladders[chain].append(word)
        yield "done", qid, found
      case ("run", rung, _, chain, word):
        held.begin(rung, chain, word)
      case ("sent", rung, _, value) if rung in held.frames:
        held.carry(rung, value)
      case ("cancel" | "close", about, *_):
        held.dropped(about)


def module(source: str) -> dict[str, object]:
  """One module of its own, from its source: a namespace nothing else shares.

  The engine runs in one of these, so the globals of a chain hold what the engine defines and nothing of the
  boundary, and what a chain copies of them is the engine's alone.
  """
  held: dict[str, object] = {}
  exec(source, held)  # noqa: S102
  return held


def opened(
  engine: dict[str, object], sheet: dict[str, object], bound: list[str], record: object, host: Host, ears: list[str]
) -> object:
  """A life of that engine, opened from what a World kept of the life before it, on the ears of these names: the
  root it opened on, and what boot raised, if it raised.

  The Kernel is the crate's and is given first, so that it answers the gate before any ear of the host hears it;
  it writes the sheet of a word with the names the globals of a chain hold, which the crate read off the engine.
  Each ear of the host is a generator of the host, heard through the host under its name. The record is the
  entries as the World hands them: each the act made last before its fact, the fact as a tuple, and for a query
  of a run what it was answered.
  """
  made.clear()
  kept = unwire(record, engine, host)
  assert isinstance(kept, list)
  entries = [(e[0], tuple(e[1]), *e[2:]) for e in kept]
  outside = {name: crossing(lambda a, name=name: host(name, a), engine, host) for name in ears}
  try:
    root = verb(engine, "boot")(entries, kernel=kernel("kernel", host, engine, sheet, bound), **outside)
  except BaseException as no:
    # What boot raised comes out of the entry the operator went in by, and the life goes on: a drift breaks the
    # journal and keeps nothing more, so the root stands when the record held it.
    acts = engine["acts"]
    assert isinstance(acts, dict)
    root = "chain://operator.1" if "chain://operator.1" in acts else ""
    return {"root": root, "raised": wire(no, engine)}
  return {"root": str(root), "raised": None}


def asked(engine: dict[str, object], word: str, unwire_: Callable[[object], object]) -> object:
  """One word of the operator, run in the globals of the engine, and what it gave, plain.

  It runs in the engine's own globals and not in a copy of them, so what it binds stays bound, as a word of the
  operator does when the operator is python. The word reaches the plain form under one name of its own, so a
  value of the host stands in it as the mark that says it.
  """
  return wire(eval(word, engine, {"unwire": unwire_, "made": made}), engine)  # noqa: S307

"""The boundary of a host that is not python: one generator of the outside, and the plain form of every value.

The engine takes the World and the Kernel as generators, each under the name it hears by. A host written in another
language is no generator, so this stands in its place: it hears every fact, makes it plain, hands it to the host,
and says back what the host answers. Nothing of the engine is bound here, and nothing here is bound in the engine:
the crate runs this in a module of its own and hands the two generators to boot, so the globals of a chain hold
what the file defines and nothing more.

A host that must ask the engine something while it answers says so and is asked, so it never calls into a life
that stands waiting for it.
"""

from ast import PyCF_ALLOW_TOP_LEVEL_AWAIT
from collections.abc import Callable, Coroutine, Generator, Mapping
from contextvars import ContextVar

type Host = Callable[[str, object], object]
"""A host hears one fact, plain, under the name of the generator that carries it, and answers what to do."""
type Names = Mapping[str, object]
"""The names of the engine, by which a plain value is made again, and which the Kernel reaches the bus through."""


def verb(names: Names, which: str) -> Callable[..., object]:
  """One name of the engine that is a verb, which is what a name the boundary calls must be."""
  got = names[which]
  assert callable(got), which
  return got


TUPLE = "()"
"""TUPLE is how the plain form says a tuple, since a host that says a fact back must say it as the fact it was."""
SHOW = ""
"""SHOW is how the plain form says what no host reads: a show or a filter, which no record holds either."""


def wire(x: object) -> object:
  """The plain form of a value: an exception its name and what it was made with, a shape its name beside its
  fields, a tuple its entries under its own mark, a list its entries, a map its entries, and plain data is plain.

  What is none of these is a show or a filter, which no host reads and no record holds, and it crosses as a mark
  of what it was and nothing else.
  """
  match x:
    case None | bool() | int() | float() | str():
      return x
    case BaseException():
      return {"is": type(x).__name__, "args": [wire(i) for i in x.args]}
    case dict():
      return {k: wire(v) for k, v in x.items()}
    case tuple():
      return {"is": TUPLE, "args": [wire(i) for i in x]}
    case list():
      return [wire(i) for i in x]
  fields = getattr(x, "__dataclass_fields__", None)
  return (
    {"is": SHOW, "of": type(x).__name__}
    if fields is None
    else {"is": type(x).__name__} | {name: wire(getattr(x, name)) for name in fields}
  )


def unwire(x: object, names: Names) -> object:
  """The value again from the plain form, made by what its name is known by, and nothing for what cannot be made."""
  match x:
    case list():
      return [unwire(i, names) for i in x]
    case {"is": str(mark), "args": list(held)} if mark == TUPLE:
      return tuple(unwire(i, names) for i in held)
    case {"is": str(mark)} if mark == SHOW:
      return None
    case {"is": str(name), **rest}:
      held = rest.pop("args", [])
      args = [unwire(i, names) for i in held] if isinstance(held, list) else []
      return verb(names, name)(*args, **{str(k): unwire(v, names) for k, v in rest.items()})
    case dict():
      return {k: unwire(v, names) for k, v in x.items()}
  return x


def outside(name: str, host: Host, names: Names) -> Generator[tuple | None, tuple]:
  """One generator of the outside: every fact it hears goes to the host, and what the host says it says.

  The host answers one of three ways: it says facts, which this yields one by one, as any generator of a life does;
  it asks a question of the engine, which this puts and hands back the answer; or it says nothing at all.
  """
  while True:
    a = yield
    if a is None:
      continue
    reply = host(name, wire(a))
    while True:
      match reply:
        case ("ask", str(kind), str(on), list(words)):
          got = verb(names, "ask")(kind, on, *[unwire(w, names) for w in words])
          reply = host(name, wire(("answered", got[1] if isinstance(got, tuple) else got)))
        case ("say", list(facts)):
          for one in facts:
            said = unwire(one, names)
            if isinstance(said, (list, tuple)):
              yield tuple(said)
          break
        case _:
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
    self.ladders.setdefault(chain, []).append(word)
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


def kernel(name: str, host: Host, names: Names) -> Generator[tuple | None, tuple]:
  """The Kernel: it gates the word of a rung with the host, and it runs that word in the module of its chain.

  The gate is machinery of the outside, since what a word is read against is the host's to decide, so it is asked
  of the host and the chain waits for the answer. The run is not: the word runs where the engine runs, in the
  globals of its chain, so nothing of it crosses and nothing calls back into a life that stands waiting.
  """
  held = Running(names)
  while True:
    match (yield):
      case ("gate", qid, _, chain, word, returns):
        yield "done", qid, host(name, wire(("gate", word, held.ladders.get(chain, []), returns)))
      case ("run", rung, _, chain, word):
        held.begin(rung, chain, word)
      case ("sent", rung, _, value) if rung in held.frames:
        held.carry(rung, value)
      case ("cancel" | "close", about, *_):
        held.dropped(about)

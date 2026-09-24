"""The stand-in: what stands inside the sandbox for a host that is not python.

The engine takes the World, and any other generator of the outside, as an ear under the name it hears by. A host
written in another language is no generator, so a generator here stands in its place. The World is the one ear
every life has, and what the engine asks of every World is known, so its stand-in asks the World of the host by
method, `world.clock()`, and says the answer back: it is the World's law, the same for every host. Every other fact
the World hears as an ear does, `world.hears(fact)`, since what an extension asks of it the stand-in does not know,
and the stand-in closes with a refusal the start of an act of a kind the World does not do. Any other ear is heard
by name through the ears of the host, which hear every fact and say what they will.

The stand-in plays the extensions too, as the World: the words of their python parts once on each chain without a
source whose program lacks them, and their life words on each such chain in every life, once boot stands on its
record, and at the birth of each such chain after.

The Kernel is here too, since the word of a rung runs where the engine runs, in the module of its chain, and so
is the gate, an ear of its own, which reads a word with the checker of the host on the sheet of the engine.

What crosses, crosses as the interpreter carries it, but for the callables, which it carries no way back. An
instance of a class of the engine comes in as a map that names its class under `is`, with its fields, since the
interpreter makes no instance of a class of the sandbox on a host's behalf, and so the instance is made here. A
name of the engine crosses as its name, both ways, so a show of the engine is the same show on both sides. A
callable the engine made goes out as a handle the host calls it back by, and a callable of the host comes in as a
name the ears of the host call it back by. A class a word defined goes out by a handle too, with the class it
was made with, which the host holds as a type of its own derived from the type that base is there, and the class
comes back in by; an instance of one goes out with its fields under its class and comes back in made here from
them, with no `__init__` run. An instance of a class that inherits `str` is a string, and crosses as one. A map
that holds the key `is` crosses as its pairs under `is` with the name `dict`, both ways, so no side reads the map
of a word or of a host as a mark.
"""

from ast import PyCF_ALLOW_TOP_LEVEL_AWAIT
from collections.abc import Callable, Coroutine, Generator
from contextvars import ContextVar
from dataclasses import fields, is_dataclass
from typing import TYPE_CHECKING

from monty import instance

type Names = dict[str, object]
"""The names of the engine: its module, in which every word of the operator and every stand-in runs."""
type Ear = Generator[tuple | None, tuple | None]
"""An ear, as the engine hears one."""


# The three objects of the host, as the stand-in reaches them. The sandbox makes no class from a class of the
# interpreter, so they are read here and never run.
if TYPE_CHECKING:
  from typing import Protocol

  class World(Protocol):
    """The World of the host, as its stand-in asks it: what the engine asks of the machine, by method."""

    def kinds(self) -> object: ...
    def stand(self) -> object: ...
    def clock(self) -> float: ...
    def chance(self) -> float: ...
    def ask(self, rung: str, on: str, actor: str, turns: list) -> None: ...
    def wait(self, about: str, seconds: float) -> None: ...
    def prompt(self, about: str, shape: str, message: str) -> None: ...
    def keep(self, entry: object) -> None: ...
    def hears(self, fact: object) -> object: ...
    def answered(self, got: object) -> object: ...

  class Ears(Protocol):
    """The ears of the host, by name: every ear that is no World, and the World too when the host has none."""

    def hears(self, name: str, fact: object) -> object: ...
    def answered(self, name: str, got: object) -> object: ...
    def called(self, name: str, args: object, kwargs: object) -> object: ...

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
CORE: set[str] = set()
"""CORE are the names the module of the engine binds before the words of its extensions: the names every host knows,
which a value of the engine crosses by, and a class of the engine crosses as."""
LIVES: list[str] = []
"""LIVES are the life words of the extensions the host plays, which stand empty while boot says the record again."""


def verb(names: Names, which: str) -> Callable[..., object]:
  """One name of the engine that is a verb."""
  got = names[which]
  assert callable(got), which
  return got


def named(x: object, names: Names) -> str | None:
  """The name of the engine a value is bound to, when it is one, by identity: a name the engine binds before the
  words of its extensions, which every host knows."""
  return next(
    (name for name, held in names.items() if held is x and not name.startswith("_") and (not CORE or name in CORE)),
    None,
  )


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
    return {IS: "instance", "class": outward(type(x), names), "value": fielded(x, names)}
  if not callable(x):
    return x
  return {IS: "made", "id": handled(x)}


def fielded(x: object, names: Names) -> object:
  """The fields of an instance of a class a word defined, as they go out: each field of a dataclass by its name, as it
  goes out, so an instance it holds goes out as one too; and the instance itself for any other class, whose fields
  the interpreter carries out as they are."""
  if not is_dataclass(x):
    return x
  return {one.name: outward(getattr(x, one.name), names) for one in fields(x)}


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
      ear = crossing(name, ears, names)
      # A generator that was started already stands at a yield, and so must its stand-in.
      if started:
        ear.send(None)
      return ear
    # A callable of the host, called back through the ears by its name, with what it is given as it goes out,
    # and what it gives back made again, as anything of the host is.
    case {"is": "callable", "name": str(name)}:
      return lambda *args, **kwargs: again(
        ears.called(name, outward(list(args), names), outward(kwargs, names)), names, ears
      )
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
  return x


def worldly(world: World, names: Names, ears: Ears) -> Ear:
  """The World as the engine hears it, over the World as a host has it.

  It answers what the engine asks of every World: what a chain stands on, the clock and a chance. It starts what
  takes time, a model's turn, a wait and a prompt of the operator, and the host says the result into the life later,
  as the fact the engine waits for. It keeps what the journal says to keep. Every other fact it hands to the World as
  an ear hears it, and says what the World says of it, but the start of an act of a kind that the World does not do,
  which it closes with a refusal, so no word waits for it.
  """
  acts = names["acts"]
  assert isinstance(acts, dict)
  kinds = world.kinds()
  assert isinstance(kinds, list)
  while True:
    a = yield
    said = None
    match a:
      case ("stand", qid, *_):
        said = ("done", qid, again(world.stand(), names, ears))
      case ("clock", qid, *_):
        said = ("done", qid, world.clock())
      case ("chance", qid, *_):
        said = ("done", qid, world.chance())
      case ("ask", rung, _, on, actor, turns):
        world.ask(rung, on, actor, turns)
      case ("start", about, _) if acts[about][0] == "wait":
        world.wait(about, acts[about][4])
      case ("start", about, _) if acts[about][0] == "prompt":
        world.prompt(about, acts[about][4], acts[about][5])
      case ("start", about, _) if acts[about][0] not in kinds:
        refused = verb(names, "Refused")(f"the World does no {acts[about][0]}")
        verb(names, "close")(refused, about)
      case ("keep", _, _, entry):
        world.keep(entry)
      case _:
        # A method of the host is called by the sandbox and never held as a value, so the lambda calls it.
        match settled(world.hears(a), lambda got: world.answered(got), names, ears):  # noqa: PLW0108
          case ("say", tuple(saying)):
            said = again(saying, names, ears)
    if isinstance(said, tuple):
      yield said


def settled(reply: object, answered: Callable[[object], object], names: Names, ears: Ears) -> object:
  """What an ear of the host said of a fact, once every verb it said is answered: each verb is said here by its
  name and its value handed back, and what the ear raised is raised here."""
  while True:
    match reply:
      case ("calls", str(which), list(args), dict(kwargs)):
        # What the ear asked raised: the ear is answered with the raise, on its own thread, and never left waiting.
        try:
          got = ("value", outward(asked(names, ears, which, args, kwargs), names))
        except BaseException as no:
          got = ("raised", no)
        reply = answered(got)
      case ("raised", no):
        raise fault(no, names, ears)
      case _:
        return reply


def crossing(name: str, ears: Ears, names: Names) -> Ear:
  """One ear of the host, heard under its name.

  The ear is on the other side, so this stands in for it: every fact it is given goes to the ears of the host, and
  what comes back is what the ear did with it. It said something, which this yields, and the bus hands back the
  fact as it was said, which goes to the host next. It said nothing, so this waits for the next fact. It said a
  verb, which is said here by its name and its value handed back. It is over, so this returns, and it raised, so
  this raises.
  """
  a = None
  while True:
    match settled(ears.hears(name, outward(a, names)), lambda got: ears.answered(name, got), names, ears):
      case ("say", tuple(saying)):
        made = again(saying, names, ears)
        assert isinstance(made, tuple)
        a = yield made
      case ("over",):
        return
      case _:
        a = yield


def plays(engine: Names, chain: str) -> None:
  """The life words of the extensions, played on a chain as rungs by whoever speaks, in order."""
  for word in LIVES:
    verb(engine, "rung")(word, on=chain)


def playing(world: Ear, engine: Names) -> Ear:
  """The World, which plays the life words on each chain born without a source once the life stands on its record,
  and hands every fact to the World it wraps and every saying of that World to the bus."""
  a = None
  while True:
    match a:
      case ("chain", str(id), _, _, _, "") if LIVES:
        plays(engine, id)
    try:
      said = world.send(a)
    except StopIteration:
      return
    a = yield said


def pinning(engine: Names, root: str, taken: list[str], words: list[str]) -> None:
  """The builtins the life takes and the words it runs, said once as the World about the root, which the record
  keeps, so a later life on the record runs the same system prompt."""
  verb(engine, "send")("extensions", root, taken, words, by=str(engine["WORLD"]))


def played(engine: Names, lives: list[str]) -> None:
  """The life words, played as the World on every chain without a source once boot stands on the record, and on each
  such chain at its birth from then on."""
  LIVES[:] = lives
  site, acts = engine["site"], engine["acts"]
  assert isinstance(site, ContextVar)
  assert isinstance(acts, dict)
  token = site.set(str(engine["WORLD"]))
  try:
    for one in [a[1] for a in list(acts.values()) if a[0] == "chain" and not a[5]]:
      plays(engine, one)
  finally:
    site.reset(token)


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

  A run is begun in the globals of its chain and carried forward at the done of every act its word waits for, and
  what it says it says under the name of its rung, which is what makes a fact of the word the rung's own.
  """

  def __init__(self, names: Names) -> None:
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
      case ("gate", qid, _, _, word, program):
        yield "done", qid, verb(sheet, "gate")(engine, [*program.values()], word, checked)


def kernel(names: Names) -> Ear:
  """The Kernel: it runs the word of a rung in the module of its chain, and carries the run at every done."""
  held = Running(names)
  while True:
    match (yield):
      case ("run", rung, _, chain, word, _):
        # This Kernel runs every word, retold or not, so it reads no donor off the run.
        held.begin(rung, chain, word)
      case ("sent", rung, _, value) if rung in held.frames:
        held.carry(rung, value)
      case ("cancel" | "close", about, *_):
        held.dropped(about)


def module(source: str, held: dict[str, object]) -> dict[str, object]:
  """One module of its own, from its source, run in what it holds before: a namespace nothing else shares."""
  exec(source, held)  # noqa: S102
  return held


def extended(engine: Names, words: list[str]) -> Names:
  """The module of the engine with the words of its extensions run in it after the engine, and the names it bound
  before them held as CORE, since a host knows those and no name a word bound."""
  CORE.clear()
  CORE.update(engine)
  for word in words:
    exec(word, engine)  # noqa: S102
  return engine


def opened(
  engine: Names, sheet: Names, record: object, world: World | None, gate: Gate, ears: Ears, names: list[str]
) -> tuple:
  """A life of that engine, opened from what a World kept of the life before it, on the ears of these names, in
  this order: the root it opened on, and what boot raised, if it raised.

  The World of the host stands under the name `world` when the host has one; otherwise the ears of the host hear
  that name too, as they hear every other. The Kernel and the gate are given first, so that the gate answers
  before any ear hears it. The record is the entries as the World hands them: each the fact as a tuple, and for a query
  of a run what it was answered.
  """
  MADE.clear()
  LIVES.clear()
  kept = again(record, engine, ears)
  assert isinstance(kept, list)
  entries = [(tuple(e[0]), *e[1:]) for e in kept]
  outside = {
    name: worldly(world, engine, ears) if name == "world" and world is not None else crossing(name, ears, engine)
    for name in names
  }
  if "world" in outside:
    outside["world"] = playing(outside["world"], engine)
  try:
    root = verb(engine, "boot")(entries, kernel=kernel(engine), gate=gating(gate, sheet, engine), **outside)
  except BaseException as no:
    # What boot raised comes out of the entry the operator went in by, and the life goes on: a drift breaks the
    # journal and keeps nothing more, so the root stands when the record held it.
    acts = engine["acts"]
    assert isinstance(acts, dict)
    return ("chain1" if "chain1" in acts else "", no)
  return (str(root), None)


def called(engine: Names, ears: Ears, name: str, args: list, kwargs: dict) -> object:
  """One verb, called by the operator with values of the host, and what it gave, as it goes out: the verb of the
  chain it is said on, when that chain binds the name, since an extension binds its verbs there, and the verb of the
  engine otherwise. A verb said with no chain is said on the chain of who speaks, as the engine resolves it."""
  words, held = again(args, engine, ears), again(kwargs, engine, ears)
  assert isinstance(words, list)
  assert isinstance(held, dict)
  modules, scope, site = engine["modules"], engine["scope"], engine["site"]
  assert isinstance(modules, dict)
  assert callable(scope)
  assert isinstance(site, ContextVar)
  on = held.get("on") or scope(site.get())
  chain = modules.get(on, engine)
  if name not in chain and name not in engine:
    why = f"name {name!r} is not defined on {on}" if on else f"name {name!r} is not defined, and no chain was said"
    raise NameError(why)
  return outward(verb(chain if name in chain else engine, name)(*words, **held), engine)


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
  """What an ear of the host asks of the life from a thread of its own, answered here on its behalf: a reading of a
  map, who speaks, or a verb of the engine."""
  match which:
    case "held":
      return held(engine, *args)
    case "spoken":
      return spoken(engine, *args)
    case "made":
      return made_called(engine, ears, *args)
  return called(engine, ears, which, args, kwargs)


def held(engine: Names, name: str, keys: list, ask: str) -> object:
  """One reading of a map of the life where it stands, under these keys: whether a key is held, the keys, how
  many, or the value, which is what a host reads of `acts`, `asked`, `outcomes` and `modules`."""
  table = engine[name]
  for key in keys[:-1] if ask in ("in", "at") else keys:
    assert isinstance(table, dict)
    table = table[key]
  assert isinstance(table, dict)
  match ask:
    case "in":
      return keys[-1] in table
    case "at":
      return outward(table[keys[-1]], engine)
    case "keys":
      return list(table)
    case "len":
      return len(table)
  raise KeyError(ask)


def outcomes_of(engine: Names, ids: list) -> list:
  """What each of these acts came to, in one reading: what it came to, alone in a tuple, or nothing for an act
  that lives, so that an act that came to nothing is told apart from one that is not done."""
  outcomes = engine["outcomes"]
  assert isinstance(outcomes, dict)
  return [(outward(outcomes[one], engine),) if one in outcomes else None for one in ids]


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


def debugged(engine: Names, ears: Ears, pairs: list) -> None:
  """debug, said by the operator with what a host can say: each expression beside its value."""
  verb(engine, "debug")(
    Templated([Interpolated(again(value, engine, ears), str(expression)) for expression, value in pairs])
  )


def said(engine: Names, ears: Ears, one: object) -> None:
  """One thing the World said unasked, said into the life: a fact as the World, a close of an act, or a pause.

  A saying has no slot for who said it: the bus says that, as it does for what an ear yields.
  """
  match again(one, engine, ears):
    case ("fact", (str(kind), str(about), *words)):
      verb(engine, "send")(kind, about, *words, by="world")
    case ("close", str(id), value):
      verb(engine, "close")(value, id)
    case ("pause", str(id)):
      verb(engine, "pause")(id)

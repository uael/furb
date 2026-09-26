"""The Kernel, which runs the word of a rung where the engine runs, in the module of its chain.

One Kernel serves both engines: this interpreter imports this module, and the sandbox of the crate loads it as it
loads the sheet. Each gives the Kernel the names of its engine, which it reads at each use and never holds, since
boot binds the bus into the engine and a name rebound is used from the next use on.

A word answers by a close and never by a return: a body of a module takes no return, so a word that holds one is no
python and the gate says so.
"""

from ast import PyCF_ALLOW_TOP_LEVEL_AWAIT
from collections.abc import Callable, Coroutine, Generator
from contextvars import ContextVar

type Names = dict[str, object]
"""The names of the engine: its module, in which every word runs in the module of its chain."""
type Ear = Generator[tuple | None, tuple | None]
"""An ear, as the engine hears one."""


def verb(names: Names, which: str) -> Callable[..., object]:
  """One name of the engine that is a verb."""
  got = names[which]
  assert callable(got), which
  return got


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
    """The word is over, and the run is done with what the word gave, as that run: its frame and the wants it
    waits on are dropped, so no done that comes later carries a word that is gone."""
    self.frames.pop(run, None)
    self.waits = {wants: one for wants, one in self.waits.items() if one != run}
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
    began. The run of a rung that is done already is done with CancelledError, since its word never begins."""
    _, _, _, chain, rung, word, *_ = self.of(run)
    self.taken.discard(run)
    if verb(self.names, "peek")(rung, ...) is not ...:
      return self.ended(run, self.cancelled())
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

  def cancelled(self) -> BaseException:
    """The CancelledError of the engine, which a run is done with when its word is dropped or never begins."""
    got = verb(self.names, "CancelledError")()
    assert isinstance(got, BaseException)
    return got

  def dropped(self, control: tuple) -> None:
    """Every run whose rung a control is over, dropped: the frame of a word that is mid step is never closed."""
    covers = verb(self.names, "covers")
    for one in [x for x in self.frames if covers(control, self.of(x)[4])]:
      if not getattr(self.frames[one], "cr_running", False):
        self.frames[one].close()
        self.ended(one, self.cancelled())


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
      case ("cancel" | "close", *_) as control:
        held.dropped(control)

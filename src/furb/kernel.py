"""The Kernel, which runs the word of a rung where the engine runs, in the module of its chain.

One Kernel serves both engines: this interpreter imports this module, and the sandbox of the crate loads it as it
loads the sheet. Each gives the Kernel the names of its engine, which it reads at each use and never holds, since
boot binds the bus into the engine and a name rebound is used from the next use on.

A word answers by a close and never by a return: a body of a module takes no return, so a word that holds one is no
python and the gate says so.

A word runs in parts when the World says the statements of it that its model finished, each as a writing of its
rung, before the model wrote the rest. A writing is a fact and no act: the World says it and awaits nothing, so it
stands at a yield while the part runs and takes each reply that the part makes a chain ask. The Kernel runs the parts
of a word in their order, says that each ran, with what it raised, and the run of the rung then runs what the parts
left of the word, so a statement of a word runs once, wherever the word came from.
"""

from ast import PyCF_ALLOW_TOP_LEVEL_AWAIT
from collections.abc import Callable, Coroutine, Generator
from contextvars import ContextVar

type Names = dict[str, object]
"""The names of the engine: its module, in which every word runs in the module of its chain."""
type Ear = Generator[tuple | set[str] | None, tuple | None]
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
  """The runs of one life: the word of each rung that stands, in the parts that the life runs it in.

  The Kernel takes a run as that run, and begins its word when it hears that it took it, in the globals of its chain
  and as its rung, which is what makes a fact of the word the rung's own. When the word waits for an act that is not
  done, it makes a wants as the run, and carries the word forward at the done of that wants. It says the run done,
  as the run, with what the word gave.

  A writing is a fact that holds a part of the word of a rung, which the World says as the model of the rung writes
  it, and the Kernel runs that part from when it hears the writing, as it runs a run. A job is a run, or the part of
  a writing, which is no act and so has a name of the Kernel alone. The jobs of one rung stand in a line, and the
  Kernel runs the job at the head of that line, so the statements of a word run in their order and a statement that
  waits holds the statements after it. It keeps the word of each rung as far as it ran it, so a run runs what the
  writings of its rung left, and what a part raised, which is the raise of the word: the run of the rung is done
  with it. When the part of a writing is over, the Kernel says that it ran, as its rung, with what it raised. A part
  that waits for an act while its model writes makes no wants, since its rung waits for the turn of its model, and
  goes on at the done of that act. Once the run of its rung stands in the line, the part waits as that run, with a
  wants, as the run itself does: the turn of the model is over, and the chain counts a run that makes no wants as a
  word that runs, so it would ask no other prompt while the part waits.
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
    self.ran: dict[str, str] = {}
    self.line: dict[str, list[str]] = {}
    self.broke: dict[str, BaseException] = {}
    self.driving: set[str] = set()
    self.awaits: dict[str, list[str]] = {}
    self.parts: dict[str, tuple[str, str, int]] = {}
    self.heard = 0

  def of(self, job: str) -> tuple[str, str, str, str, int]:
    """The job, as the life holds it: its kind, its chain, its rung, its word, and how many characters of the word
    of its rung stand before that word, which no run counts, since a run carries the word whole."""
    if job in self.parts:
      rung, part, before = self.parts[job]
      return "writing", str(verb(self.names, "scope")(rung)), rung, part, before
    got = verb(self.names, "get")(job)
    assert isinstance(got, tuple)
    kind, _, _, chain, rung, word, *_ = got
    return str(kind), str(chain), str(rung), str(word), 0

  def wrote(self, rung: str, part: str, before: int) -> str:
    """A writing heard, as a job of its rung, under a name of the Kernel that no act takes."""
    self.heard += 1
    job = f"{rung} part {self.heard}"
    self.parts[job] = (rung, part, before)
    return job

  def took(self, run: str) -> None:
    """The run, taken as that run, which is what the Kernel says of it first."""
    with self.site.set(run):
      verb(self.names, "say")("started", run)
    self.taken.add(run)

  def ended(self, run: str, got: BaseException | None) -> None:
    """The word, or the part of it that a writing holds, is over: a run is done with what the word gave, as that
    run, and the rung says that the part of a writing ran, with what it raised. The frame of the job and the wants it
    waits on are dropped, so no done that comes later carries a word that is gone. What a part raised is the raise of
    the word, which the run of its rung is done with, and the job after it in the line of its rung runs."""
    kind, _, rung, _, before = self.of(run)
    self.frames.pop(run, None)
    self.waits = {wants: one for wants, one in self.waits.items() if one != run}
    self.awaits = {made: kept for made, ones in self.awaits.items() if (kept := [x for x in ones if x != run])}
    if isinstance(got, BaseException):
      self.broke[rung] = got
    line = self.line.get(rung) or []
    if run in line:
      line.remove(run)
    if kind == "run":
      with self.site.set(run):
        verb(self.names, "say")("done", run, got)
      self.line.pop(rung, None)
      self.broke.pop(rung, None)
    else:
      del self.parts[run]
      with self.site.set(rung):
        verb(self.names, "say")("ran", rung, before, got)
    self.drives(rung)

  def carry(self, run: str, given: object) -> None:
    """The word stepped as its rung with what it waited for, and stepped again while what it waits for is done."""
    peek = verb(self.names, "peek")
    with self.site.set(self.of(run)[2]):
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
          if not self.wants(run, got):
            # The rung of a part waits for the turn of its model, which a wants would keep it from hearing, so the
            # part waits for its act with no wants, and goes on at the done of that act.
            self.awaits.setdefault(got, []).append(run)
          return None
        given = peek(got)

  def wants(self, job: str, made: str) -> bool:
    """A wants for the act that a job waits for, made as the run of its rung, once that run stands in the line of
    the rung; or nothing, while the model of the rung writes and no run of it stands."""
    run = next((x for x in self.line.get(self.of(job)[2], []) if x not in self.parts), "")
    if not run:
      return False
    with self.site.set(run):
      wants = verb(self.names, "act")("wants", "", None, made)
    assert isinstance(wants, str)
    self.waits[wants] = job
    return True

  def refusal(self, got: object) -> BaseException:
    made = verb(self.names, "Refused")(f"a rung awaits an act, and {got!r} is none")
    assert isinstance(made, BaseException)
    return made

  def refusing(self, why: str) -> BaseException:
    """A refusal of the engine, which a job is done with when its part of a word is no part of the word that ran."""
    made = verb(self.names, "Refused")(why)
    assert isinstance(made, BaseException)
    return made

  def begin(self, run: str) -> None:
    """A job taken, put in the line of its rung: it runs when the jobs of that rung before it are over. The job of a
    rung that is done already is done with CancelledError, since its word never begins. Once a run stands in the
    line, each part of its rung that waits for an act with no wants waits with a wants as that run."""
    rung = self.of(run)[2]
    self.taken.discard(run)
    if verb(self.names, "peek")(rung, ...) is not ...:
      return self.ended(run, self.cancelled())
    self.line.setdefault(rung, []).append(run)
    for made, ones in list(self.awaits.items()):
      for one in [x for x in ones if self.of(x)[2] == rung and self.wants(x, made)]:
        ones.remove(one)
      if not ones:
        del self.awaits[made]
    return self.drives(rung)

  def drives(self, rung: str) -> None:
    """Each job at the head of the line of a rung, run, while the job at that head waits for nothing."""
    if rung in self.driving:
      return
    self.driving.add(rung)
    try:
      while (line := self.line.get(rung)) and line[0] not in self.frames:
        self.starts(line[0])
    finally:
      self.driving.discard(rung)

  def starts(self, run: str) -> None:
    """The job at the head of the line of its rung, run: the statements of the word that are new to the life run in
    the globals of its chain, and a job that holds no such statement is over at once. A job of a word that a part of
    it raised in is done with that raise, since a word is over where it raises. A job whose part follows another
    word than the one that ran is refused, since the Kernel runs no statement twice."""
    _, chain, rung, _, _ = self.of(run)
    held = self.ran.get(rung, "")
    part = self.broke.get(rung) or self.part(run, held)
    if isinstance(part, BaseException):
      return self.ended(run, part)
    self.ran[rung] = held + part
    if not part.strip():
      return self.ended(run, None)
    module = verb(self.names, "module")(chain)
    assert isinstance(module, dict)
    try:
      with self.site.set(rung):
        # The part keeps the line it stands on in the word, so a traceback of it names the line its model wrote.
        ran = ran_in("\n" * held.count("\n") + part, rung, module)
    except BaseException as raised:
      return self.ended(run, raised)
    if not isinstance(ran, Coroutine):
      return self.ended(run, None)
    self.frames[run] = ran
    return self.carry(run, None)

  def part(self, run: str, held: str) -> str | BaseException:
    """What a job runs of the word of its rung, given what ran of it: what a run holds past what ran, and the part of
    a writing that follows what ran; or the refusal of a job that does not follow it."""
    kind, _, rung, whole, before = self.of(run)
    if kind != "run":
      ok = before == len(held)
      return whole if ok else self.refusing(f"a part of {rung} follows {before} characters of it, and {len(held)} ran")
    if whole.startswith(held):
      return whole[len(held) :]
    # A turn ends with no blank at its end, so the word may hold what ran less the blank lines after it.
    if held.startswith(whole) and not held[len(whole) :].strip():
      return ""
    return self.refusing(f"{held!r:.100} of the word of {rung} ran, and its run holds another")

  def cancelled(self) -> BaseException:
    """The CancelledError of the engine, which a run is done with when its word is dropped or never begins."""
    got = verb(self.names, "CancelledError")()
    assert isinstance(got, BaseException)
    return got

  def dropped(self, control: tuple) -> None:
    """Every run whose rung a control is over, dropped: the frame of a word that is mid step is never closed."""
    covers = verb(self.names, "covers")
    for one in [x for x in self.frames if covers(control, self.of(x)[2])]:
      if not getattr(self.frames[one], "cr_running", False):
        self.frames[one].close()
        self.ended(one, self.cancelled())


def kernel(names: Names) -> Ear:
  """The Kernel: it takes each run and begins its word when it hears that it took it, runs the part of each writing
  of a rung that the life holds when it hears the writing, and carries the word at the done of each wants it made."""
  held, heard = Running(names), {"run", "writing", "wants", "cancel", "close"}
  while True:
    match (yield heard | set(held.awaits)):
      case ("run", run, *_):
        # This Kernel runs every word, retold or not, so it reads no donor off the run.
        held.took(run)
      case ("writing", str(rung), _, str(part), int(before)) if verb(names, "get")(rung):
        held.begin(held.wrote(rung, part, before))
      case ("started", run, *_) if run in held.taken:
        held.begin(run)
      case ("done", made, _, value) if made in held.awaits:
        for one in held.awaits.pop(made):
          held.carry(one, value)
      case ("done", wants, _, value) if wants in held.waits:
        held.carry(held.waits.pop(wants), value)
      case ("cancel" | "close", *_) as control:
        held.dropped(control)

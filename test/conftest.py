"""The harness: a World in memory, a Kernel that is python, and what a test reads of the turns of a chain.

The World and the Kernel are generators of the test, which it hands boot by keyword, each under the name it is to
hear by, so no name of them stands in the globals of the engine. Every fact is a tuple, as the contract says:
(kind, about, by, *words), and a question (kind, id, by, on, *words), deconstructed only by match.
"""

import ast
import asyncio
import builtins
import json
import sys
from asyncio import CancelledError
from collections.abc import Coroutine, Generator, Sequence
from dataclasses import dataclass, field, fields, is_dataclass
from functools import partial
from pathlib import Path

import pytest

import furb
import furb_monty.engine
from furb import engine, sheet
from furb.engine import OPERATOR, WORLD, Act, Refused, Text, modules, outcomes, site, under
from furb.kernel import NAMES
from furb_monty import _monty

HERE = Path(__file__).resolve().parent
"""HERE is the directory of the suite, whose modules bind the names of the engine under test."""
ENGINES = {"python": furb.python, "monty": furb_monty.engine}
"""ENGINES are the two engines every test runs on: the one of this interpreter, and the one in the sandbox of monty."""
SURFACE = frozenset(furb_monty.engine.defined())
"""SURFACE is every name the engine defines, which is what a module of the suite may have bound of it."""
MONTY_SKIPS: dict[str, str] = {
  "test_every_name_that_the_file_defines_is_in_the_globals_of_a_chain": (
    "the test reads the file of the engine through the module under test, and on monty that module is another file"
  ),
  "test_a_chain_with_a_source_holds_the_classes_its_origin_defined_before_that_source": (
    "a class a word defined is the sandbox's own, and crosses to this interpreter as a callable and no type"
  ),
  "test_an_ear_is_any_generator_of_that_shape": (
    "the test boots on a Kernel of this interpreter, and the engine of monty holds its own"
  ),
  "test_boot_gives_the_root_as_an_act_of_never_and_the_root_never_completes": (
    "the test boots on a Kernel of this interpreter, and the engine of monty holds its own"
  ),
}
"""MONTY_SKIPS names the tests the engine of monty does not run, each with why: what the test reads is a fact of one
interpreter, which the boundary does not carry."""

type World = Generator[tuple | None, tuple]
"""The World, as engine.pyi declares it: engine.py binds no such name, so the suite says the type itself."""
type Kernel = Generator[tuple | None, tuple]
"""The Kernel, as engine.pyi declares it: engine.py binds no such name, so the suite says the type itself."""

STANDS = (((OPERATOR, (), 200000), ("m", ("low", "high"), 400000), ("n", ("low",), 200000)), "/w", "m/low")
"""A standing of three actors, a directory and a default actor, which a test takes when it needs a roster."""

WORD = "t = read('a.txt')\nx = bash('echo hi')\nk = len(t.lines)\nclose((await x).code)"
"""A word of a rung that reads a file, starts a command and gives back what the command came to."""

MANY = "".join(f"line {i}\n" for i in range(1, 301))
"""A text of three hundred lines, which is longer than what a tell of a text shows of it."""

DOOR = (
  "def note(id):\n"
  "  while True:\n"
  "    match (yield):\n"
  "      case ('read', qid, _, _, path) if path.startswith('note://'):\n"
  "        yield 'done', qid, Text(path, 'kept')\n"
  "\n"
  "act('note', '', note)\n"
  "close(1)\n"
)
"""A word of a rung that opens an act of an extension, which answers a read of a door of its own."""


def wire(x: object) -> object:
  """The plain form of a value, as the World of this machine makes it: the same shape world.py writes."""
  match x:
    case BaseException():
      return {"is": type(x).__name__, "args": wire(x.args)}
    case Text():
      return {"is": "Text", "path": x.path, "content": x.content}
    case dict():
      return {k: wire(v) for k, v in x.items()}
    case list() | tuple():
      return [wire(i) for i in x]
  if is_dataclass(x) and not isinstance(x, type):
    return {"is": type(x).__name__} | {f.name: wire(getattr(x, f.name)) for f in fields(x)}
  return x


def unwire(x: object) -> object:
  """The value again from its plain form, as the World of this machine reads it back."""
  match x:
    case list():
      return [unwire(i) for i in x]
    case {"is": str(name), **rest}:
      held = rest.pop("args", [])
      args = [unwire(i) for i in held] if isinstance(held, list) else []
      return (vars(builtins) | vars(engine))[name](*args, **{str(k): unwire(v) for k, v in rest.items()})
    case dict():
      return {k: unwire(v) for k, v in x.items()}
  return x


def plain(record: Sequence[object]) -> list:
  """A record as a later life is given it: through the wire and back, so every tuple is a list and every text a Text."""
  got = unwire(json.loads(json.dumps(wire(list(record)))))
  assert isinstance(got, list)
  return [(e[0], tuple(e[1]), *e[2:]) for e in got]


@dataclass
class Sand:
  """A World in memory, outside the engine.

  `files` is its disk by path, `script` what its models answer by the id of the chain the ask names, one word per
  ask, `record` what it keeps of the keep facts of the journal, `fed` what was fed to its commands, and `calls`
  every fact it answered or performed, in order. `stands` is what a chain stands on, `cost` the usage of one
  answer, and `auto` says whether a command tells a line and exits at once. `tick` counts the readings of its
  clock and the chances it drew, so a later life reads what the life before it read.
  """

  files: dict[str, str] = field(default_factory=dict)
  script: dict[str, list[str]] = field(default_factory=dict)
  record: list[tuple] = field(default_factory=list)
  fed: list[str | None] = field(default_factory=list)
  calls: list[tuple] = field(default_factory=list)
  auto: bool = True
  stands: tuple | None = None
  cost: tuple | None = None
  tick: int = 0

  def hears(self) -> World:  # noqa: PLR0915
    """The World as one generator for one life: it does the act a start names, answers the questions that are its
    own, feeds and ends its commands, answers an ask with the next word of its script, and keeps what it is told.
    """
    running: dict[str, str] = {}
    acts: dict[str, tuple] = {}
    loop = asyncio.get_running_loop()

    def ends(about: str) -> None:
      """The command ended with no code, when the World still runs it at its timeout."""
      if about in running:
        engine.send("exited", about, None, by=WORLD)

    def resolved(here: str, path: str) -> str:
      """A path against a working directory: a path of its own stands as it is, and any other hangs off it."""
      return path if path.startswith("/") or "://" in path else f"{here}/{path}"

    while True:
      a = yield
      if a[0] in ("start", "stand", "read", "write", "ask", "feed", "clock", "chance"):
        self.calls.append(a)
      match a:
        case (_, id, *_) if engine.question(a) and id in engine.acts:
          acts[id] = a
        case ("start", about, _):
          match acts[about]:
            case ("bash", _, _, on, command, _, timeout):
              running[about] = on
              engine.ask("merged", on, about)
              loop.call_later(timeout, ends, about)
              if self.auto:
                loop.call_soon(partial(engine.send, "out", about, f"ran {command}\n", "stdout", by=WORLD))
                loop.call_soon(partial(engine.send, "exited", about, 0, by=WORLD))
            case ("wait", _, _, _, seconds):
              loop.call_later(seconds, partial(engine.send, "done", about, None, by=WORLD))
            case ("prompt", _, _, _, shape, _, _) if shape not in ("None", "bool", "int", "float", "str"):
              engine.close(Refused(f"the operator answers no {shape}"), about)
        case ("stand", qid, *_):
          yield "done", qid, self.stands or ((), "", "")
        case ("read", qid, _, on, path) if (full := resolved(engine.cwd(on=on), path)) in self.files:
          yield "done", qid, Text(full, self.files[full])
        case ("write", qid, _, on, Text(path=path, content=content)) if "://" not in path:
          self.files[full := resolved(engine.cwd(on=on), path)] = content
          yield "done", qid, Text(full, content)
        case ("ask", rung, _, on, _, _):
          running[rung] = on
          if self.script.get(on):
            word = self.script[on].pop(0)
            turn = ("assistant", [word], self.cost or (0, 0, 0, 0, 0.0), [f"signed {len(word)}"])
            loop.call_soon(partial(engine.send, "answer", rung, turn, by=WORLD))
        case ("cancel" | "close", *_):
          for one in list(running):
            if engine.covers(a, one):
              running.pop(one)
              if one.startswith("bash"):
                yield "exited", one, None
        case ("exited", about, *_):
          running.pop(about, None)
        case ("feed", _, _, text):
          self.fed.append(text)
        case ("keep", _, _, entry):
          self.record.append(entry)
        case ("clock", qid, *_):
          self.tick += 1
          yield "done", qid, 1000.0 + self.tick
        case ("chance", qid, *_):
          self.tick += 1
          yield "done", qid, (self.tick % 7) / 7


class Dead(Sand):
  """A World that refuses everything: every question it hears is answered with a refusal.

  What a chain stands on is the one question it answers, since every chain asks it as it opens and no record holds
  it. It does no act, it answers no ask and it keeps nothing, so a life that reaches it for anything else fails
  where it reaches it.
  """

  def hears(self) -> World:
    """The World that answers a standing and refuses every other question."""
    while True:
      a = yield
      match a:
        case ("stand", qid, *_):
          self.calls.append(a)
          yield "done", qid, self.stands or ((), "", "")
        case (kind, qid, *_) if engine.question(a) and qid in engine.asked:
          self.calls.append(a)
          yield "done", qid, Refused(f"a dead World answers no {kind}")


@dataclass
class Where(Sand):
  """A World that asks the chain where it stands at every path it is given and at every command it starts."""

  where: list[str] = field(default_factory=list)

  def hears(self) -> World:
    """The World that reads, writes and starts a command, each against the directory it asks the chain for."""
    while True:
      a = yield
      match a:
        case ("stand", qid, *_):
          yield "done", qid, self.stands or ((), "", "")
        case ("read", qid, _, on, path):
          full = f"{engine.cwd(on=on)}/{path}"
          yield "done", qid, Text(full, self.files.get(full, ""))
        case ("write", qid, _, on, Text(path=path, content=content)):
          self.files[full := f"{engine.cwd(on=on)}/{path}"] = content
          yield "done", qid, Text(full, content)
        case ("start", about, _):
          self.where.append(engine.cwd(on=about and engine.scope(about)))
          yield "exited", about, 0


def seen(held: list[tuple]):  # noqa: ANN201
  """A filter of the suite that keeps every act and holds the acts it was given in the list it is made with."""

  def keeps(acts: list[tuple]) -> list[tuple]:
    held.extend(acts)
    return acts

  return keeps


def gated(log: Sequence[tuple]) -> list[str]:
  """Every word the gate was given in the life, in order, which the ready of each rung says."""
  return [a[3] for a in said(log, "ready")]


def ran(log: Sequence[tuple]) -> list[str]:
  """Every word the Kernel ran in the life, in order, which the run of each rung says."""
  return [a[4] for a in said(log, "run")]


def findings(log: Sequence[tuple]) -> list[list[str]]:
  """What the gate found against each word it was given, in order, which the done of each gate query says."""
  return [a[3] for a in said(log, "done") if a[1].startswith("gate://")]


def refusals(log: Sequence[tuple]) -> list[str]:
  """The findings that refused a word, each as the body the refused tag tells them as."""
  return ["\n".join(found) for found in findings(log) if found]


class Py:
  """A Kernel that is python, outside the engine like the World.

  It answers a gate with what the gate of the crate finds on the sheet of the word, begins a run by compiling the
  word with a top level await and running it in the module of its chain, says wants for the act the run waits for,
  carries the run forward at each sent, says ran with what the word gave, and drops the frame of a run a cancel is
  over.
  """

  def gate(self, word: str, program: list[str]) -> list[str]:
    """What the gate finds against a word: the sheet of the engine, read by the gate of the crate."""
    return sheet.gate(NAMES, program, word, _monty.gate)

  def kernel(self) -> Kernel:
    """The Kernel as one generator for one life, which speaks from the run it steps."""
    frames: dict[str, Coroutine[object, object, object]] = {}

    def ended(name: str, got: BaseException | None) -> None:
      """The run is over, and what it came to goes to the chain that had it run."""
      frames.pop(name, None)
      engine.send("ran", name, got, by=name)

    def carry(name: str, sent: object) -> None:
      """The run stepped with what it waited for, and stepped again while what it waits for is already ended."""
      token = site.set(name)
      try:
        while True:
          try:
            got = frames[name].throw(sent) if isinstance(sent, BaseException) else frames[name].send(sent)
          except StopIteration:
            return ended(name, None)
          except BaseException as raised:
            return ended(name, raised)
          assert isinstance(got, Act), got
          if got not in outcomes:
            engine.send("wants", name, got, by=name)
            return None
          sent = outcomes[got]
      finally:
        site.reset(token)

    def begin(name: str, word: str, held: dict[str, object]) -> None:
      """A run begun: the word of it is python, and a word that awaits nothing is over where it is begun."""
      code = compile(word, "<rung>", "exec", flags=ast.PyCF_ALLOW_TOP_LEVEL_AWAIT)
      token = site.set(name)
      try:
        ran = eval(code, held)  # noqa: S307
      except BaseException as raised:
        return ended(name, raised)
      finally:
        site.reset(token)
      if not asyncio.iscoroutine(ran):
        return ended(name, None)
      frames[name] = ran
      return carry(name, None)

    while True:
      match (yield):
        case ("run", rung, _, chain, word, _):
          begin(rung, word, modules[chain])
        case ("sent", rung, _, value) if rung in frames:
          carry(rung, value)
        case ("cancel" | "close", about, *_):
          for one in [x for x in frames if under(x, about) and not getattr(frames[x], "cr_running", False)]:
            frames[one].close()
            ended(one, CancelledError())


def kept(inner: Kernel) -> Kernel:
  """A Kernel that keeps what each run left in the module of its chain, and answers a run that retells one it kept
  with that module again, so the word of a retold rung never runs a second time.
  """
  held: dict[str, dict[str, object]] = {}
  where: dict[str, str] = {}
  engine.lives(inner, None)
  while True:
    match a := (yield):
      case ("run", rung, _, chain, _, whose):
        where[rung] = chain
        if whose in held:
          modules[chain].update(held[whose])
          engine.send("ran", rung, None, by=rung)
        else:
          engine.lives(inner, a)
      case ("ran", rung, *_) if rung in where:
        held[rung] = dict(modules[where[rung]])
        engine.lives(inner, a)
      case _:
        engine.lives(inner, a)


def watched(log: list[tuple]) -> Kernel:
  """A generator of the suite that says nothing and keeps every fact it hears."""
  while True:
    if (a := (yield)) is not None:
      log.append(a)


def kernel() -> dict[str, Kernel]:
  """The Kernel a life of the suite is given: the one that is python for the engine of this interpreter, and none
  for the engine of monty, which holds its own."""
  return {} if engine is not furb.python else {"kernel": Py().kernel()}


def life(world: Sand, record: Sequence[tuple] = ()) -> tuple[list[tuple], str]:
  """A life: the engine opened from a record, with the Kernel it takes, a World in memory and a generator that keeps
  every fact said in it; it gives what was said and the id of the root.
  """
  log: list[tuple] = []
  return log, engine.boot(record, **kernel(), probe=watched(log), world=world.hears())


async def settle(n: int = 80) -> None:
  """Room for the loop to do what it still owes, so that finding nothing done means something."""
  for _ in range(n):
    await asyncio.sleep(0)


def sown() -> Sand:
  """A World with one file and the roster of the suite."""
  return Sand(files={"/w/a.txt": "one\ntwo\n"}, stands=STANDS)


async def lived(sand: Sand) -> tuple[list[tuple], str]:
  """A life that reads a file, runs a command and returns what it came to."""
  log, root = life(sand)
  sand.script[root] = [WORD, "close(None)"]
  assert await engine.prompt(int, "read and run", on=root) == 0
  await settle()
  return log, root


async def relived(sand: Sand, record: Sequence[tuple]) -> tuple[list[tuple], str]:
  """A later life on a kept record, with the World it is given."""
  log, root = life(sand, record)
  await settle(300)
  return log, root


def keeping(heard: list[tuple], say: tuple | None = None) -> Generator[tuple | None, tuple]:
  """A generator that says one fact of its own at its birth when it is given one, and keeps every fact it hears."""
  if say is not None:
    yield say
  while True:
    if (word := (yield)) is not None:
      heard.append(word)


def pair() -> Generator[tuple | None, tuple]:
  """A generator that says two facts of its own at its birth and nothing after."""
  yield "done", "none://one", None
  yield "done", "none://two", None
  while True:
    yield


def said(log: Sequence[tuple], kind: str) -> list[tuple]:
  """Every fact of that kind that was said in the life, in order."""
  return [a for a in log if a[0] == kind]


def tags(got: Sequence[tuple], name: str = "") -> list[tuple]:
  """Every tag a fold of turns holds, of that name when a name is given."""
  return [g for _, content, _, _ in got for g in content if isinstance(g, tuple) and name in ("", g[0])]


def shown(tag: tuple) -> list[tuple]:
  """The tags that a tag holds as its body, which is what a tell of a text shows of it."""
  body = tag[2]
  return (
    [one for one in body if isinstance(one, tuple) and len(one) == 3 and isinstance(one[0], str)]
    if isinstance(body, list)
    else []
  )


def attr(tag: tuple, name: str) -> object:
  """The value of the named attribute of a tag; a tag that holds no such attribute is a failed test."""
  found = [value for key, value in tag[1] if key == name]
  assert len(found) == 1, (tag[0], name, tag[1])
  return found[0]


def text_of(turn: tuple) -> str:
  """The text of a turn, which is everything it holds that is text."""
  return "\n".join(x for x in turn[1] if isinstance(x, str))


def swapped(to: object) -> None:
  """Every name of the engine that a module of the suite bound at import, rebound to the engine under test.

  A test reads the engine through the module it imported and through the names it took from it, so both are
  rebound, by identity: a name that is the same object in both engines stays as it is.
  """
  fro = furb_monty.engine if to is furb.python else furb.python
  seen: set[int] = set()
  for mod in list(sys.modules.values()):
    file = getattr(mod, "__file__", None)
    if not file or id(mod) in seen or not str(file).startswith(str(HERE)):
      continue
    seen.add(id(mod))
    for key, value in list(vars(mod).items()):
      if value is fro:
        setattr(mod, key, to)
      elif key in SURFACE and value is getattr(fro, key):
        setattr(mod, key, getattr(to, key))


def pytest_generate_tests(metafunc: pytest.Metafunc) -> None:
  """Every test of the suite runs once on each engine; the hygiene laws read the file and run once."""
  path = metafunc.definition.path
  if "engine_of" in metafunc.fixturenames and path.parent == HERE and path.name != "test_hygiene.py":
    metafunc.parametrize("engine_of", list(ENGINES), indirect=True)


@pytest.fixture(autouse=True)
def engine_of(request: pytest.FixtureRequest) -> Generator[str]:
  """The engine a test runs on, bound under every name the suite reads it by for the length of the test."""
  which = getattr(request, "param", "python")
  if which == "monty" and request.node.originalname in MONTY_SKIPS:
    pytest.skip(MONTY_SKIPS[request.node.originalname])
  swapped(ENGINES[which])
  yield which
  swapped(furb.python)


@pytest.fixture
def sand() -> Sand:
  """A World in memory of its own for one test."""
  return Sand()


@pytest.fixture
def py() -> Py:
  """A Kernel that is python of its own for one test."""
  return Py()


# pytest loads this file as its plugin under a name of its own; the tests and the probe import it as conftest. One
# module object under both names keeps isinstance true against the classes above.
sys.modules.setdefault("conftest", sys.modules[__name__])

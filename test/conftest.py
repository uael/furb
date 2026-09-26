"""The harness: a World in memory, a Kernel that is python, and what a test reads of the turns of a chain.

The World and the Kernel are generators of the test, which it hands boot by keyword, each under the name it is to
hear by, so no name of them stands in the globals of the engine. Every fact is a tuple, as the contract says:
(kind, about, by, *words), and a question (kind, id, by, on, *words), deconstructed only by match.
"""

import ast
import asyncio
import builtins
import json
import re
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
from furb.engine import OPERATOR, Act, Exit, Refused, Text, site
from furb.kernel import ENGINE

HERE = Path(__file__).resolve().parent
"""HERE is the directory of the suite, whose modules bind the names of the engine under test."""
ENGINES = {"python": furb.python, "monty": furb_monty.engine}
"""ENGINES are the two engines every test runs on: the one of this interpreter, and the one in the sandbox of monty."""
SURFACE = frozenset(furb_monty.engine.defined())
"""SURFACE is every name the engine defines, which is what a module of the suite may have bound of it."""
MONTY_SKIPS: dict[str, str] = {
  "test_an_ear_is_any_generator_of_that_shape": (
    "the test boots on a Kernel of this interpreter, and the engine of monty holds its own"
  ),
  "test_boot_gives_the_root_as_an_act_of_never_and_the_root_never_completes": (
    "the test boots on a Kernel of this interpreter, and the engine of monty holds its own"
  ),
  "test_a_run_names_the_rung_that_the_word_retells": (
    "the test boots on a Kernel of this interpreter, and the engine of monty holds its own"
  ),
  "test_a_rung_that_retells_is_done_with_nothing": (
    "the test boots on a Kernel of this interpreter, and the engine of monty holds its own"
  ),
}
"""MONTY_SKIPS names the tests the engine of monty does not run, each with why: what the test reads is a fact of one
interpreter, which the boundary does not carry."""

type World = Generator[tuple | None, tuple]
"""The World, an Ear of engine.pyi: engine.py binds no such name, so the suite says the type itself."""
type Kernel = Generator[tuple | None, tuple]
"""The Kernel, an Ear of engine.pyi: engine.py binds no such name, so the suite says the type itself."""

WORLD = "world"
"""WORLD is the name that the World of the suite is heard by, and that it says its facts by."""

STANDS: list = [[[OPERATOR, [], 200000], ["m", ["low", "high"], 400000], ["n", ["low"], 200000]], "/w", "m/low"]
"""A standing of three actors, a directory and a default actor, which a test takes when it needs a roster."""

WORD = "t = read('a.txt')\nx = bash('echo hi')\nk = len(t.lines)\nclose((await x).code)"
"""A word of a rung that reads a file, starts a command and gives back what the command came to."""

BAD = "line 1: error[unresolved-reference] Name `BAD` used when not defined"
"""BAD is what the gate finds against a word whose first line names BAD, which nothing binds."""

MANY = "".join(f"line {i}\n" for i in range(1, 301))
"""A text of three hundred lines, which is longer than what a tell of a text shows of it."""

DOOR = (
  "def note(id):\n"
  "  yield 'started', id\n"
  "  while True:\n"
  "    match (yield):\n"
  "      case ('read', qid, _, _, path) if path.startswith('note://'):\n"
  "        yield 'done', qid, Text(path, 'kept')\n"
  "\n"
  "act('note', '', note)\n"
  "close(1)\n"
)
"""A word of a rung that opens an act of an extension, which takes it and answers a read of a door of its own."""


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
  return [(tuple(e[0]), *e[1:]) for e in got]


@dataclass
class Sand:
  """A World in memory, outside the engine.

  `files` is its disk by path, `script` what its models answer by the id of the chain a reply is on, one word per
  reply, `record` what it keeps of the keep facts of the record, `fed` what was fed to its commands, `calls` every
  fact it answered or performed, in order, and `turns` the turns it read for each reply it took, by its name.
  `stands` is what a chain stands on, or the refusal it answers a stand with, `cost` the usage of one answer, and `auto` says whether a command tells a line
  and exits at once. `tick` counts the readings of its clock and the chances it drew, so a later life reads what the
  life before it read. `outs` holds each command it runs: whether its stderr flows into its stdout, and its two
  streams as they came.
  """

  files: dict[str, str] = field(default_factory=dict)
  script: dict[str, list[str]] = field(default_factory=dict)
  record: list[tuple] = field(default_factory=list)
  fed: list[str | None] = field(default_factory=list)
  calls: list[tuple] = field(default_factory=list)
  auto: bool = True
  stands: list | Refused | None = None
  cost: tuple | None = None
  tick: int = 0
  turns: dict[str, list] = field(default_factory=dict)
  outs: dict[str, list] = field(default_factory=dict)

  def exits(self, about: str, code: int | None) -> None:
    """The command ends with its code, and the World answers it with what it came to: its code and its streams as
    they came, from its own loop."""
    if about not in self.outs:
      return
    _, out, err = self.outs.pop(about)
    world_says("done", about, Exit(code, Text(f"{about}/stdout", out), Text(f"{about}/stderr", err)))

  def hears(self) -> World:  # noqa: PLR0915
    """The World as one generator for one life: it takes a command, a wait, a prompt to the operator and a reply,
    answers the questions that are its own, feeds and ends its commands, answers a reply with the next word of its
    script, and keeps what it is told.
    """
    loop = asyncio.get_running_loop()

    def ends(about: str) -> None:
      """The command ended with no code, when the World still runs it at its timeout."""
      if about in self.outs:
        self.exits(about, None)

    def resolved(here: str, path: str) -> str:
      """A path against a working directory: a path of its own stands as it is, and any other hangs off it."""
      return path if path.startswith("/") or "://" in path else f"{here}/{path}"

    while True:
      a = yield
      if a[0] in ("bash", "wait", "prompt", "reply", "stand", "read", "write", "feed", "clock", "chance"):
        self.calls.append(a)
      match a:
        case ("bash", about, _, on, command, _, timeout):
          yield "started", about
          self.outs[about] = [engine.ask("merged", on, about), "", ""]
          loop.call_later(timeout, ends, about)
          if self.auto:
            loop.call_soon(partial(engine.say, "out", about, f"ran {command}\n", "stdout"))
            loop.call_soon(partial(self.exits, about, 0))
        case ("out", about, _, text, stream) if about in self.outs:
          self.outs[about][1 if self.outs[about][0] or stream == "stdout" else 2] += text
        case ("wait", about, _, _, seconds):
          yield "started", about
          loop.call_later(seconds, partial(engine.say, "done", about, None))
        case ("prompt", about, _, _, shape, _, _):
          yield "started", about
          if shape not in ("None", "bool", "int", "float", "str"):
            engine.close(Refused(f"the operator answers no {shape}"), about)
        case ("stand", qid, *_):
          yield "done", qid, self.stands or [[], "", ""]
        case ("read", qid, _, on, path) if (full := resolved(engine.cwd(on=on), path)) in self.files:
          yield "done", qid, Text(full, self.files[full])
        case ("write", qid, _, on, Text(path=path, content=content)) if (
          "://" not in path and engine.get(path.split("/")[0]) is None
        ):
          self.files[full := resolved(engine.cwd(on=on), path)] = content
          yield "done", qid, Text(full, content)
        case ("reply", about, _, on, _):
          yield "started", about
          self.turns[about] = engine.turns(on=on)
          if self.script.get(on):
            word = self.script[on].pop(0)
            turn = ("assistant", word, self.cost or (0, 0, 0, 0, 0.0), [f"signed {len(word)}"])
            loop.call_soon(partial(engine.say, "done", about, turn))
        case ("cancel" | "close", *_):
          for one in [x for x in self.outs if engine.covers(a, x)]:
            self.outs.pop(one)
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
          yield "done", qid, self.stands or [[], "", ""]
        case (kind, qid, *_) if engine.get(qid) == a:
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
          yield "done", qid, self.stands or [[], "", ""]
        case ("read", qid, _, on, path):
          full = f"{engine.cwd(on=on)}/{path}"
          yield "done", qid, Text(full, self.files.get(full, ""))
        case ("write", qid, _, on, Text(path=path, content=content)):
          self.files[full := f"{engine.cwd(on=on)}/{path}"] = content
          yield "done", qid, Text(full, content)
        case ("bash", about, _, on, *_):
          yield "started", about
          self.where.append(engine.cwd(on=on))
          yield "done", about, Exit(0, Text(f"{about}/stdout"), Text(f"{about}/stderr"))


def seen(held: list[tuple]):  # noqa: ANN201
  """A filter of the suite that keeps every act and holds the acts it was given in the list it is made with."""

  def keeps(acts: list[tuple]) -> list[tuple]:
    held.extend(acts)
    return acts

  return keeps


def gated(log: Sequence[tuple]) -> list[str]:
  """Every word the gate was given in the life, in order, which the ready of each rung says that no chain made: a
  chain gates no word it wrote itself, and a rung that retells stands with the gate where its donor stood."""
  return [a[3] for a in said(log, "ready") if (engine.get(engine.get(a[1])[2]) or ("",))[0] != "chain"]


def ran(log: Sequence[tuple]) -> list[str]:
  """Every word the Kernel ran in the life, in order, which the run of each rung says."""
  return [a[5] for a in said(log, "run")]


def findings(log: Sequence[tuple]) -> list[list[str]]:
  """What the gate found against each word it was given, in order, which the done of each gate says."""
  return [a[3] for a in said(log, "done") if a[1].startswith("gate")]


class Py:
  """A Kernel that is python, outside the engine like the World.

  Its Kernel takes a run, begins it by compiling the word with a top level await and running it in the module of
  its chain, makes a wants for the act the run waits for, carries the run forward at the done of each wants, says
  the run done with what the word gave, and drops the frame of a run a cancel is over. Its gate is an ear of its own, which reads a word on the
  sheet of the engine with the gate of the crate.
  """

  def gate(self, word: str, program: list[str]) -> list[str]:
    """What the gate finds against a word: the sheet of the engine, read by the gate of the crate."""
    return sheet.gate(ENGINE, program, word, furb_monty.gate)

  def gating(self) -> Kernel:
    """The gate as the ear of a life, which answers each gate with what it finds against the word."""
    while True:
      match (yield):
        case ("gate", qid, _, on, word):
          yield "done", qid, self.gate(word, [*engine.program(on).values()])

  def kernel(self) -> Kernel:  # noqa: PLR0915
    """The Kernel as one generator for one life. It takes each run as that run, begins the word of it when it hears
    that it took it, which is after the step of the chain, and runs the word as its rung. When the word waits for an
    act that is not done, it makes a wants as the run, and carries the word forward at the done of that wants. It is
    done with the run with what the word gave, and drops the frame of a word that a cancel is over."""
    frames: dict[str, Coroutine[object, object, object]] = {}
    waits: dict[str, str] = {}
    taken: set[str] = set()

    def ended(run: str, got: BaseException | None) -> None:
      """The word is over, and the Kernel is done with its run with what the word gave, as that run: its frame and
      the wants it waits on are dropped."""
      frames.pop(run, None)
      for wants in [x for x, one in waits.items() if one == run]:
        waits.pop(wants)
      with site.set(run):
        engine.say("done", run, got)

    def carry(run: str, given: object) -> None:
      """The word stepped as its rung with what it waited for, and stepped again while what it waits for is done."""
      with site.set(str(engine.get(run)[4])):
        while True:
          try:
            got = frames[run].throw(given) if isinstance(given, BaseException) else frames[run].send(given)
          except StopIteration:
            return ended(run, None)
          except BaseException as raised:
            return ended(run, raised)
          assert isinstance(got, Act), got
          if engine.peek(got, ...) is ...:
            with site.set(run):
              waits[engine.act("wants", "", None, got)] = run
            return None
          given = engine.peek(got)

    def begin(run: str) -> None:
      """A word begun as its rung: it is python, and a word that awaits nothing is over where it is begun."""
      _, _, _, chain, rung, word, _ = (str(x) for x in engine.get(run))
      code = compile(word, "<rung>", "exec", flags=ast.PyCF_ALLOW_TOP_LEVEL_AWAIT)
      try:
        with site.set(rung):
          ran = eval(code, engine.module(chain))  # noqa: S307
      except BaseException as raised:
        return ended(run, raised)
      if not asyncio.iscoroutine(ran):
        return ended(run, None)
      frames[run] = ran
      return carry(run, None)

    while True:
      match (yield):
        case ("run", run, *_):
          with site.set(run):
            engine.say("started", run)
          taken.add(run)
        case ("started", run, *_) if run in taken:
          taken.discard(run)
          if engine.peek(str(engine.get(run)[4]), ...) is ...:
            begin(run)
          else:
            ended(run, CancelledError())
        case ("done", wants, _, value) if wants in waits:
          carry(waits.pop(wants), value)
        case ("cancel" | "close", *_) as control:
          for one in [x for x in frames if engine.covers(control, str(engine.get(x)[4]))]:
            if not getattr(frames[one], "cr_running", False):
              frames[one].close()
              ended(one, CancelledError())


def kept(inner: Kernel) -> Kernel:
  """A Kernel that keeps what each word left in the module of its chain, and answers a run that retells a rung it
  kept with that module again, so the word of a retold rung never runs a second time.
  """
  held: dict[str, dict[str, object]] = {}
  engine.lives(inner, None)
  while True:
    match a := (yield):
      case ("run", run, _, chain, _, _, whose) if whose in held:
        engine.module(chain).update(held[whose])
        with site.set(run):
          engine.say("started", run)
          engine.say("done", run, None)
      case ("done", run, *_) if engine.question(("run", run)):
        _, _, _, chain, rung, *_ = (str(x) for x in engine.get(run))
        held[rung] = dict(engine.module(chain))
        engine.lives(inner, a)
      case _:
        engine.lives(inner, a)


def watched(log: list[tuple]) -> Kernel:
  """A generator of the suite that says nothing and keeps every fact it hears, each act among them: an ear of the
  outside hears only an act that no ear before it took, so an act it did not hear it keeps before the first fact it
  hears about that act."""
  heard: set[str] = set()
  while True:
    if (a := (yield)) is not None:
      if a[1] not in heard and (made := engine.get(a[1])) is not None:
        heard.add(a[1])
        if made != a:
          log.append(made)
      log.append(a)


def kernel() -> dict[str, Kernel]:
  """The Kernel and the gate a life of the suite is given: the ones that are python for the engine of this
  interpreter, and none for the engine of monty, which holds its own."""
  return {} if engine is not furb.python else {"kernel": Py().kernel(), "gate": Py().gating()}


def life(world: Sand, record: Sequence[tuple] = ()) -> tuple[list[tuple], str]:
  """A life: the engine opened from a record, with the Kernel it takes, a World in memory and a generator that keeps
  every fact said in it; it gives what was said and the id of the root.
  """
  log: list[tuple] = []
  return log, engine.boot(record, **kernel(), probe=watched(log), world=world.hears())


def world_says(kind: str, about: str, *words: object) -> tuple:
  """A fact the World says from its own loop, under its own name, as a command, a wait or a model answers later."""
  token = site.set(WORLD)
  try:
    return engine.say(kind, about, *words)
  finally:
    site.reset(token)


def outside(label: str = "outside") -> str:
  """A chain the outside makes under a site of its own, which neither the operator nor an act is."""
  token = site.set("outside")
  try:
    return engine.chain(label)
  finally:
    site.reset(token)


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


def dones(log: Sequence[tuple], kind: str) -> list[tuple]:
  """Every done said in the life of an act of that kind, in order."""
  return [a for a in said(log, "done") if engine.question((kind, a[1]))]


def tip(name: str, root: str = "chain1", standing: list = STANDS) -> list[tuple]:
  """What a life enters in the record at its tip when nothing else happens in it: the stand that boot makes, and the
  answer of the World."""
  return [(("stand", name, OPERATOR, root),), (("done", name, WORLD, standing),)]


def lasting(facts: Sequence[tuple]) -> list[tuple]:
  """The facts as they compare wherever they are read: a module by its kind, the chain it is about and who said it,
  since a module is the living dict of a chain, which every rung fills, and the engine of monty gives a copy of it."""
  return [a[:3] if a[0] == "module" else a for a in facts]


def said(log: Sequence[tuple], kind: str) -> list[tuple]:
  """Every fact of that kind that was said in the life, in order."""
  return [a for a in log if a[0] == kind]


def acts(log: Sequence[tuple]) -> dict[str, tuple]:
  """Every act that was said in the life, under its name, as the life holds it."""
  return {a[1]: a for a in log if engine.question(a)}


def paragraphs(got: Sequence[tuple]) -> list[str]:
  """Every paragraph the user turns of a fold hold, in order: what one fact that tells stands as. A blank line that a
  header follows is where one paragraph ends, since a word its caller wrote may hold a blank line of its own."""
  return [one for role, py, _, _ in got if role == "user" and py for one in re.split(r"\n\n(?=#\w)", py)]


def heads(got: Sequence[tuple]) -> list[str]:
  """The header of every paragraph the user turns of a fold hold, in order, which is the first line of each."""
  return [one.split("\n", 1)[0] for one in paragraphs(got)]


def named(got: Sequence[tuple]) -> list[str]:
  """What each header of the user turns of a fold names, in order: the id of an act, or the kind of a query."""
  return [one[1:].split(" ", 1)[0] for one in heads(got)]


def rows(id: str, standing: list = STANDS) -> list[str]:
  """The three headers a chain tells a standing it takes under, in one paragraph: the roster, the directory and the
  actor."""
  roster, directory, actor = standing
  return [f"#{id} roster {roster!r}", f"#{id} cwd {directory}", f"#{id} actor {actor}"]


def takes(id: str, standing: list = STANDS) -> str:
  """The paragraph a chain tells of a standing it takes, whose header is the row of its roster."""
  return "\n".join(rows(id, standing))


def of(got: Sequence[tuple], name: str) -> list[str]:
  """Every paragraph of the user turns of a fold whose header names that act, or that kind of query, in order."""
  return [one for one in paragraphs(got) if one.split("\n", 1)[0].split(" ", 1)[0] == f"#{name}"]


def bindings(root: str, act: str, shape: str) -> str:
  """The word of the rung a fresh root writes at its first ask for a prompt: the bindings of the root and of that
  prompt."""
  return f"{engine.bound(root)}\n{engine.bound(act, shape)}"


def gatings(log: Sequence[tuple]) -> list[tuple[str, list[str]]]:
  """Every word the gate was asked of in the life, in order, with the words of the program it read that word after,
  which is the program of its chain where the gate stands in the transcript of that chain."""
  got = []
  for a in said(log, "gate"):
    held = engine.transcript(a[3])
    at = next(i for i, x in enumerate(held) if x[:2] == a[:2])
    since = max([i for i, x in enumerate(held[:at]) if x[0] == "module"], default=0)
    words = {x[4]: str(x[5]) for x in held[since:at] if x[0] == "run"}
    got.append((str(a[4]), [*words.values()]))
  return got


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

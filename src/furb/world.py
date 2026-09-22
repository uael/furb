"""The World of this machine: the disk, the shell, the models of the provider, the operator, a clock and chance.

The engine holds the record as entries and this holds it as lines, one json array to an entry, made plain by wire
and read back by unwire. Everything the World does runs on the loop the operator booted the life on: a command and
an ask are tasks of that loop, and what they come to reaches the life through send, under the name of the World.
"""

import asyncio
import builtins
import codecs
import json
import os
import random
import re
import signal
import subprocess
import sys
import time
from asyncio import Task
from asyncio.subprocess import Process
from collections.abc import Callable, Coroutine, Generator, Sequence
from contextlib import suppress
from dataclasses import dataclass, field, fields, is_dataclass
from functools import partial
from pathlib import Path

from pydantic import TypeAdapter
from pydantic_ai.direct import model_request
from pydantic_ai.messages import (
  ModelMessage,
  ModelRequest,
  ModelResponse,
  ModelResponsePart,
  SystemPromptPart,
  TextPart,
  UserPromptPart,
)
from pydantic_ai.models import Model
from python_minifier import minify

from furb import engine
from furb.engine import WORLD, Drift, Refused, Text, under
from furb.provider.claude import ACTOR, Claude, Settings, actors

type World = Generator[tuple | None, tuple]
"""The World, as engine.pyi declares it: engine.py binds no such name, so this module says the type itself."""

CAP = 524288
"""CAP is the most bytes the World reads of one file, since a text a model cannot hold is no answer."""
PIPE = 65536
"""PIPE is the bytes the World reads of a stream at a time, which is one Out word of the command."""
MUTE = "{} answered nothing"
"""MUTE is how the World says an actor gave no turn, which it reads back to tell a fault of the moment from one that stands."""


SYSTEM = minify(
  Path(engine.__file__).read_text(encoding="utf-8"),
  remove_annotations=False,
  remove_pass=False,
  combine_imports=False,
  hoist_literals=False,
  rename_locals=False,
  rename_globals=False,
  remove_object_base=False,
  convert_posargs_to_args=False,
  remove_explicit_return_none=False,
  remove_builtin_exception_brackets=False,
  constant_folding=False,
)
"""SYSTEM is the system prompt of every model: the engine, minified in layout alone, and nothing else."""


PARTS = TypeAdapter(list[ModelResponsePart])
"""PARTS reads the parts of an answer back into the shapes the provider gave, whether from a record or from the answer."""
FENCE = re.compile(r"```(?:python|py)?\n(.*?)```", re.DOTALL)
"""FENCE finds a block of code in what a model wrote, since the word of a rung is the code and nothing around it."""


def wire(x: object) -> object:
  """The plain form of a value, which is how a record leaves a life: an exception its name and what it was made
  with, a text its path and its content, a shape its name beside its fields, a list and a tuple their entries, a map
  its entries, and plain data is plain.
  """
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
  """The value again from the plain form wire gave, made by what its name is known by: a name of the engine, or of
  the interpreter when the engine holds none."""
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


def shown(tag: tuple) -> str:
  """One tag as the model reads it: its name, its short attributes beside the name, and everything else inside it.

  A value that holds a line break or a quotation mark stands in the body and not beside the name, and nothing is
  ever escaped, so a text crosses to the model byte for byte.
  """
  name, held, body = tag
  attrs, parts = "", []
  for key, value in held:
    said = value if isinstance(value, str) else repr(value)
    if "\n" in said or '"' in said:
      parts.append(f"<{key}>\n{said}\n</{key}>")
    else:
      attrs += f' {key}="{said}"'
  if isinstance(body, str):
    parts.append(body)
  elif isinstance(body, list):
    # A tag says its attributes as a list, which nothing else a body holds does, a showing among it.
    parts.extend(shown(one) if isinstance(one, tuple) and isinstance(one[1], list) else repr(one) for one in body)
  elif body is not None:
    # A body is any value a tag was told with, so one of a kind this World does not know stands as python shows it.
    parts.append(repr(body))
  inner = "\n".join(one for one in parts if one)
  return f"<{name}{attrs}/>" if not inner else f"<{name}{attrs}>\n{inner}\n</{name}>"


def rendered(content: Sequence[tuple | str]) -> str:
  """What one turn holds, as one text: a tag as its block, and a text as itself."""
  return "\n".join(shown(one) if isinstance(one, tuple) else one for one in content)


def worded(got: ModelResponse) -> str:
  """The word of the rung, which is what the model wrote: the one block of code it holds, or the whole of it."""
  text = "".join(one.content for one in got.parts if isinstance(one, TextPart)).strip()
  found = FENCE.findall(text)
  return str(found[0]).strip() if len(found) == 1 else text


def truth(line: str) -> bool:
  """A line of the operator as a truth: yes or no, and nothing else."""
  if line.lower() in ("y", "yes", "true", "1"):
    return True
  if line.lower() in ("n", "no", "false", "0"):
    return False
  why = f"{line!r} is neither yes nor no"
  raise ValueError(why)


LINES: dict[str, Callable[[str], object]] = {
  "None": lambda _: None,
  "str": str,
  "int": int,
  "float": float,
  "bool": truth,
}
"""LINES is every shape the operator answers, by name, and how a line of the operator becomes a value of that shape."""


def kept(record: Path) -> list[tuple]:
  """The record of an earlier life, as the entries it holds, which is what boot is given.

  A crash tears the last line alone, which is cut away; a line anywhere else that is no entry is a drift.
  """
  said: list[tuple] = []
  lines = [line for line in record.read_text(encoding="utf-8").split("\n") if line.strip()]
  for n, line in enumerate(lines, 1):
    try:
      got = unwire(json.loads(line))
    except ValueError:
      if n < len(lines):
        raise
      break
    if not (isinstance(got, list) and len(got) in (2, 3) and isinstance(got[1], list) and got[1]):
      why = f"line {n} of {record} is no entry of the record"
      raise Drift(why)
    said.append((got[0], tuple(got[1]), *got[2:]))
  return said


@dataclass
class Command:
  """One command of the World: its act, its process once the process stands, and what waits to be fed to it.

  A rung writes the stdin of a command as soon as it has made the command, which is before the World has the
  process up, so what is fed before then waits here and goes in the order it was said once the process stands.
  """

  id: str
  on: str
  command: str
  fed: bool
  timeout: float
  merged: bool
  proc: Process | None = None
  waiting: list[str | None] = field(default_factory=list)
  over: bool = False

  def feed(self, text: str | None) -> None:
    """The text into the stdin of the command, and a text of nothing closes that stdin."""
    if self.proc is None:
      self.waiting.append(text)
    elif self.proc.stdin is None:
      return
    elif text is None:
      self.proc.stdin.close()
    else:
      self.proc.stdin.write(text.encode())

  def stands(self, proc: Process) -> None:
    """The process of the command, up, and everything that waited to be fed to it, fed."""
    self.proc, waiting = proc, self.waiting
    self.waiting = []
    for text in waiting:
      self.feed(text)

  def slay(self) -> None:
    """The whole group of the command dies, and not its shell alone, since a command grows a tree of its own."""
    if self.proc is None:
      return
    try:
      os.killpg(self.proc.pid, signal.SIGKILL)
    except OSError, AttributeError:
      with suppress(ProcessLookupError):
        self.proc.kill()


@dataclass
class Live:
  """The World of one record on this machine, which one life holds.

  `directory` is where the chains of the life start, `record` the file it keeps the record in and reads it back
  from, `actor` the actor a prompt goes to when it names none, and `roster` the actors it offers. `calls` holds
  every fact it answered or performed, in order, and `model` is the one model it asks, when it is given one.
  `reader` reads the terminal and `reading` keeps one read of it at a time, since there is one operator.
  """

  directory: str
  record: Path | None = None
  actor: str = ACTOR
  roster: tuple[tuple[str, tuple[str, ...], int], ...] = field(default_factory=actors)
  calls: list[tuple] = field(default_factory=list)
  model: Model[object] | None = None
  bought: dict[str, Model[object]] = field(default_factory=dict)
  reader: asyncio.StreamReader | None = None
  reading: asyncio.Lock = field(default_factory=asyncio.Lock)

  def buys(self, name: str) -> Model[object]:
    """The model a name asks for, bought once, or the one model the World was given for every name it hears."""
    if self.model is not None:
      return self.model
    if name not in self.bought:
      self.bought[name] = Claude(name)
    return self.bought[name]

  async def answer(self, actor: str, on: str, turns: Sequence[tuple]) -> tuple:
    """One turn of a model for one ask: the turns of the chain as messages, and what comes back as the turn it is.

    The system prompt stands first, then each turn of the chain: a user turn as the text of its tags, an assistant
    turn as the parts the provider gave, so that the provider reads its own answer whole and its cache holds the
    conversation from one end. A user turn that holds nothing goes not at all.
    """
    who, effort = actor.partition("/")[::2]
    messages: list[ModelMessage] = [ModelRequest(parts=[SystemPromptPart(content=SYSTEM)])]
    for role, content, _, blocks in turns:
      if role == "assistant":
        held = blocks if isinstance(blocks, list) else []
        text = "\n".join(x for x in content if isinstance(x, str))
        messages.append(ModelResponse(parts=PARTS.validate_python(held) if held else [TextPart(text)]))
      elif said := rendered(content):
        messages.append(ModelRequest(parts=[UserPromptPart(content=said)]))
    settings = Settings(claude_session_id=on, claude_effort=effort)
    got = await model_request(self.buys(who), messages, model_settings=settings)
    spent = got.usage
    usage = (
      spent.input_tokens,
      spent.output_tokens,
      spent.cache_read_tokens,
      spent.cache_write_tokens,
      float(spent.cost or 0),
    )
    return ("assistant", [worded(got)], usage, PARTS.dump_python(list(got.parts), mode="json"))

  def at(self, here: str, path: str = "") -> Path:
    """One path of the disk: the directory of the life, where the chain stands, and then the path.

    A chain holds the path a cd was given, which may name no directory of its own, and the World has one place to
    stand such a path against: the directory every chain of the life started in.
    """
    return Path(self.directory, here, path)

  async def line(self) -> str:
    """One line of the operator, read on the loop and never on a thread, so no read outlives the life.

    The reader of the terminal is made once, at the first read, and it is the one reader there is.
    """
    if self.reader is None:
      # The loop is asked for the number of the file first: a terminal it cannot read fails here, where nothing
      # has been built yet, rather than half way through a reader whose own end then fails again.
      sys.stdin.fileno()
      self.reader = reader = asyncio.StreamReader()
      made = asyncio.StreamReaderProtocol(reader)
      await asyncio.get_running_loop().connect_read_pipe(lambda: made, sys.stdin)
    return (await self.reader.readline()).decode(errors="replace").strip()

  def serves(self, path: str) -> bool:
    """Whether the World answers for a path: a path of the disk, and a door of an act of the life, which it refuses
    once nothing lives behind it. A door of no act of the life is another ear's to answer, so the World says nothing
    of it, whatever the order the ears were given in."""
    return "://" not in path or path.rsplit("/", 1)[0] in engine.acts

  def read(self, here: str, path: str) -> Text | Refused:
    """The text at a path: the file on the disk, and a refusal for the door of nothing that lives."""
    if "://" in path:
      return Refused(f"{path} is the door of nothing that lives")
    at = self.at(here, path)
    if not at.is_file():
      return Refused(f"no file at {at}")
    raw = at.read_bytes()
    if len(raw) > CAP:
      return Refused(f"{at} holds {len(raw)} bytes, over the {CAP} the World reads")
    try:
      return Text(str(at), raw.decode())
    except UnicodeDecodeError:
      return Refused(f"{at} is no text")

  def write(self, here: str, path: str, content: str) -> Text | Refused:
    """The content onto the file at a path, and the text of that file as it stands on the disk after the write."""
    if "://" in path:
      return Refused(f"{path} is the door of nothing that takes a word")
    at = self.at(here, path)
    at.parent.mkdir(parents=True, exist_ok=True)
    at.write_text(content, encoding="utf-8")
    return Text(str(at), at.read_text(encoding="utf-8"))

  def keep(self, entry: tuple) -> None:
    """One entry of the record onto its file, plain, as json, and on the disk before this gives back."""
    if self.record is None:
      return
    line = json.dumps(wire(entry), separators=(",", ":")) + "\n"
    self.record.parent.mkdir(parents=True, exist_ok=True)
    with self.record.open("a", encoding="utf-8") as file:
      file.write(line)
      file.flush()
      os.fsync(file.fileno())

  async def ran(self, one: Command, here: str) -> None:
    """The command in a session of its own: what it says as it says it, and its code when it is over.

    When the command is merged, its stderr is its stdout, so the two stand in the order the command wrote them. The
    World ends the command at its timeout, and the code of it is nothing then.
    """
    hiss = subprocess.STDOUT if one.merged else subprocess.PIPE
    mouth = subprocess.PIPE if one.fed else subprocess.DEVNULL
    try:
      proc = await asyncio.create_subprocess_shell(
        one.command, stdin=mouth, stdout=subprocess.PIPE, stderr=hiss, cwd=self.at(here), start_new_session=True
      )
    except OSError as no:
      # The machine would not start it, so the command never runs and whoever waits for it hears why instead.
      engine.close(Refused(f"{one.command!r} did not start: {no}"), one.id)
      return
    one.stands(proc)
    if one.over:
      one.slay()

    async def drained() -> None:
      """Both streams to their end, and then the code of the command."""
      await asyncio.gather(self.told(proc.stdout, one.id, "stdout"), self.told(proc.stderr, one.id, "stderr"))
      await proc.wait()

    # Every way out reads both streams to their end and reaps the process, the one ended before it stood and the
    # one the life leaves up too, since a pipe of a command that outlives the loop is a pipe nobody closes.
    job = asyncio.ensure_future(drained())
    late = False
    try:
      async with asyncio.timeout(one.timeout):
        await asyncio.shield(job)
    except TimeoutError:
      late = True
      one.slay()
      await job
    except asyncio.CancelledError:
      one.slay()
      await job
      raise
    if not one.over:
      engine.send("exited", one.id, None if late else proc.returncode, by=WORLD)

  async def told(self, reader: asyncio.StreamReader | None, about: str, stream: str) -> None:
    """One stream of a command, said as it comes, one out fact of the engine for each part that arrives."""
    if reader is None:
      return
    decoder = codecs.getincrementaldecoder("utf-8")(errors="replace")
    while raw := await reader.read(PIPE):
      if text := decoder.decode(raw):
        engine.send("out", about, text, stream, by=WORLD)
    if text := decoder.decode(b"", final=True):
      engine.send("out", about, text, stream, by=WORLD)

  def twice(self, actor: str, turns: Sequence[tuple]) -> bool:
    """Whether this actor was mute already at the turn this ask was handed, so the fault stands.

    The World tells a fault of the moment from one that stands by its own last refusal: the turns of an ask end
    with what the chain was told since the answer before it, so a refusal of the same actor there is the second
    in a row, where one of an older turn was answered after.
    """
    last = turns[-1] if turns else None
    return last is not None and any(isinstance(one, tuple) and MUTE.format(actor) in str(one[2]) for one in last[1])

  async def asked(self, rung: str, on: str, actor: str, turns: Sequence[tuple]) -> None:
    """One turn of a model for one ask, and the refusal for an ask the World cannot answer, with a pause when the
    fault of it stands.

    A fault of the moment is no pause: the rung is closed with the refusal, the prompt asks again, and the model
    reads what was dropped. A second nothing of the same actor answers the same way twice, so the chain goes quiet
    until the operator wakes it, and the operator is told here why it went quiet.
    """
    try:
      turn = await self.answer(actor, on, turns)
    except Exception as no:
      why = Refused(f"{MUTE.format(actor)}: {type(no).__name__}: {no}")
      if self.twice(actor, turns):
        sys.stderr.write(f"{on} is paused: {why}\n")
        engine.pause(on)
      engine.close(why, rung)
      return
    engine.send("answer", rung, turn, by=WORLD)

  async def show(self, about: str, shape: str, message: str) -> None:
    """A prompt of the operator: the message on the terminal, and one line back as the shape the prompt wants.

    There is one terminal and one operator, so prompts of the operator are shown and answered one at a time, in
    the order they asked, and never two at once on one stream.
    """
    if shape not in LINES:
      engine.close(Refused(f"the operator answers no {shape}"), about)
      return
    try:
      async with self.reading:
        sys.stdout.write(f"{about} wants a {shape}: {message}\n> ")
        sys.stdout.flush()
        line = await self.line()
    except (OSError, ValueError) as no:
      engine.close(Refused(f"the operator cannot be read: {no}"), about)
      return
    try:
      engine.close(LINES[shape](line), about)
    except ValueError as no:
      engine.close(Refused(f"{line!r} is no {shape}: {no}"), about)

  def hears(self) -> World:  # noqa: PLR0912
    """The World as one generator for one life: it does the act a start names, answers the questions that are its
    own, feeds and ends its commands, answers an ask with the turn of a model, and keeps what it is told.
    """
    running: dict[str, Command] = {}
    acts: dict[str, tuple] = {}
    jobs: set[Task[None]] = set()
    loop = asyncio.get_running_loop()

    def start(work: Coroutine[object, object, None]) -> None:
      """One task of the World, held while it runs, so that nothing collects it before it is done."""
      job = loop.create_task(work)
      jobs.add(job)
      job.add_done_callback(jobs.discard)

    while True:
      a = yield
      # Every fact the World answered or performed, and none that it only heard.
      if a[0] in ("start", "stand", "read", "write", "ask", "feed", "clock", "chance"):
        self.calls.append(a)
      match a:
        case (_, id, *_) if engine.question(a) and id in engine.acts:
          acts[id] = a
        case ("start", about, _):
          match acts[about]:
            case ("bash", _, _, on, command, fed, timeout):
              merged = engine.ask("merged", on, about)[1]
              running[about] = held = Command(about, on, command, fed, timeout, bool(merged))
              start(self.ran(held, engine.cwd(on=on)))
            case ("wait", _, _, _, seconds):
              loop.call_later(seconds, partial(engine.send, "done", about, None, by=WORLD))
            case ("prompt", _, _, _, shape, message, _):
              start(self.show(about, shape, message))
        case ("stand", qid, *_):
          yield "done", qid, (self.roster, self.directory, self.actor)
        case ("read", qid, _, on, path) if self.serves(path):
          yield "done", qid, self.read(engine.cwd(on=on), path)
        case ("write", qid, _, on, Text(path=path, content=content)) if self.serves(path):
          yield "done", qid, self.write(engine.cwd(on=on), path, content)
        case ("ask", rung, _, on, actor, turns):
          start(self.asked(rung, on, actor, turns))
        case ("feed", about, _, text) if about in running:
          running[about].feed(text)
        case ("cancel" | "close", about, *_):
          for one in [x for x in running.values() if under(x.id, about) or x.on == about]:
            one.over = True
            one.slay()
            yield "exited", one.id, None
        case ("exited", about, *_):
          running.pop(about, None)
        case ("keep", _, _, entry):
          self.keep(entry)
        case ("clock", qid, *_):
          yield "done", qid, time.time()
        case ("chance", qid, *_):
          yield "done", qid, random.random()  # noqa: S311

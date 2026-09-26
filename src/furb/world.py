"""The World of this machine: the models of the provider and the operator, beside the ears of the crate.

The ears of the crate serve the rest of the World: the files, the commands, time and the store of the record, which
`furb_monty` gives. This World answers what they do not: the standing, a reply, which a model of the provider
answers, and a prompt to the operator, which the terminal answers. A reply and a prompt are tasks of the loop the
operator booted the life on, begun while the World speaks, so what they come to reaches the life through say under
the name of the World.
"""

import asyncio
import sys
from asyncio import Task
from collections.abc import Callable, Coroutine, Generator, Sequence
from dataclasses import dataclass, field
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

from furb import engine, python
from furb.engine import Refused
from furb.provider.claude import ACTOR, Claude, Settings, actors
from furb_monty import _monty

type World = Generator[tuple | None, tuple]
"""The World, an Ear of engine.pyi: engine.py binds no such name, so this module says the type itself."""

MUTE = "{} answered nothing"
"""MUTE is how the World says an actor gave no turn."""


SYSTEM = minify(
  Path(python.__file__).read_text(encoding="utf-8"),
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


def worded(got: ModelResponse) -> str:
  """The word of the rung, which is all the text the model wrote: a model speaks python and nothing else, so a
  fence or a line of prose around the code is part of the word, which the gate refuses."""
  return "".join(one.content for one in got.parts if isinstance(one, TextPart)).strip()


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
  """The record of an earlier life, as the entries it holds, which is what boot is given: what the store of the crate
  kept at the path, read with no lease."""
  return entries(_monty.kept(str(record)))


def entries(record: list[list[list[object]]]) -> list[tuple]:
  """The entries of a record as the store of the crate gives them, each one fact, as the engine takes them."""
  return [(tuple(one),) for (one,) in record]


def answered(entries: Sequence[tuple]) -> list[tuple]:
  """Every answer of a model that the entries of a record hold: the done of each reply that came to a turn, which a
  record gives back as a list, and none of a reply that came to a refusal."""
  return [
    one for one, *_ in entries if one[0] == "done" and engine.question(("reply", one[1])) and isinstance(one[3], list)
  ]


@dataclass
class Live:
  """The World of one life on this machine: its models and its operator, and the ears of the crate it stands beside.

  `directory` is where the chains of the life start, `actor` the actor a prompt goes to when it names none, and
  `roster` the actors it offers. `calls` holds every fact it answered or performed, in order, and `model` is the one
  model it asks, when it is given one. `ears` are the ears of the crate the life is booted on, which the World lets
  go at its end.
  `mute` holds, for each chain, the actor whose last reply on that chain answered nothing, so a second such reply in
  a row pauses the chain, and an answer between the two ends the row.
  `reader` reads the terminal and `reading` keeps one read of it at a time, since there is one operator.
  """

  directory: str
  actor: str = ACTOR
  roster: list[list[str | list[str] | int]] = field(default_factory=actors)
  calls: list[tuple] = field(default_factory=list)
  model: Model[object] | None = None
  bought: dict[str, Model[object]] = field(default_factory=dict)
  mute: dict[str, str] = field(default_factory=dict)
  reader: asyncio.StreamReader | None = None
  reading: asyncio.Lock = field(default_factory=asyncio.Lock)
  ears: dict[str, _monty.NativeEar] = field(default_factory=dict)

  def end(self) -> None:
    """The ears of the crate let go: a command ends, a wait ends, and the store lets its record go."""
    for one in self.ears.values():
      one.dispose()

  def buys(self, name: str) -> Model[object]:
    """The model a name asks for, bought once, or the one model the World was given for every name it hears."""
    if self.model is not None:
      return self.model
    if name not in self.bought:
      self.bought[name] = Claude(name)
    return self.bought[name]

  async def answer(self, actor: str, on: str, turns: Sequence[tuple]) -> tuple:
    """One turn of a model for one reply: the turns of the chain as messages, and what comes back as the turn it is.

    The system prompt stands first, then each turn of the chain: a user turn as the python the engine wrote, an
    assistant turn as the parts the provider gave, so that the provider reads its own answer whole and its cache
    holds the conversation from one end. A user turn that holds nothing goes not at all.
    """
    who, effort = actor.partition("/")[::2]
    messages: list[ModelMessage] = [ModelRequest(parts=[SystemPromptPart(content=SYSTEM)])]
    for role, py, _, blocks in turns:
      if role == "assistant":
        held = blocks if isinstance(blocks, list) else []
        messages.append(ModelResponse(parts=PARTS.validate_python(held) if held else [TextPart(py)]))
      elif py:
        messages.append(ModelRequest(parts=[UserPromptPart(content=py)]))
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
    return ("assistant", worded(got), usage, PARTS.dump_python(list(got.parts), mode="json"))

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

  async def asked(self, about: str, on: str, actor: str, turns: Sequence[tuple]) -> None:
    """One turn of a model for one reply, and the refusal for a reply the World cannot answer, with a pause when the
    fault of it stands.

    A fault of the moment is no pause: the reply is done with the refusal, which its rung comes to, and the prompt
    asks again. A second nothing of the same actor in a row on the chain is a fault that stands, so the
    chain goes quiet until the operator wakes it, and the operator is told here why it went quiet. The World counts
    the row by what it was answered, and never by a text a chain was told.
    """
    try:
      turn = await self.answer(actor, on, turns)
    except Exception as no:
      why = Refused(f"{MUTE.format(actor)}: {type(no).__name__}: {no}")
      if self.mute.get(on) == actor:
        sys.stderr.write(f"{on} is paused: {why}\n")
        engine.pause(on)
      self.mute[on] = actor
      engine.say("done", about, why)
      return
    self.mute.pop(on, None)
    engine.say("done", about, turn)

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

  def hears(self) -> World:
    """The World as one generator for one life: it answers the stand, and takes a prompt to the operator and a reply,
    whose done it says when the operator or the model answered."""
    replies: dict[str, Task[None]] = {}
    jobs: set[Task[None]] = set()
    loop = asyncio.get_running_loop()

    def start(work: Coroutine[object, object, None]) -> Task[None]:
      """One task of the World, held while it runs, so that nothing collects it before it is done."""
      job = loop.create_task(work)
      jobs.add(job)
      job.add_done_callback(jobs.discard)
      return job

    while True:
      a = yield
      # Every fact the World answered or performed, and none that it only heard.
      if a[0] in ("prompt", "reply", "stand"):
        self.calls.append(a)
      match a:
        case ("prompt", about, _, _, shape, message, _):
          yield "started", about
          start(self.show(about, shape, message))
        case ("reply", about, _, on, actor):
          yield "started", about
          replies[about] = start(self.asked(about, on, actor, engine.turns(on=on)))
        case ("stand", qid, *_):
          yield "done", qid, [self.roster, self.directory, self.actor]
        case ("done", about, *_):
          # A reply that another ear ended wants no turn, so the model is asked for nothing more. The World says the
          # done of its own reply from the task of that reply.
          if (job := replies.pop(about, None)) is not None and job is not asyncio.current_task():
            job.cancel()

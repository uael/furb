"""The World of this machine: the ears of the crate, and the operator at its terminal.

The ears of the crate serve the World: the provider of models, whose models the catalog of the crate knows, the files,
the commands, time and the store of the record, which `furb_monty` opens a life on as every host of the crate does.
This World answers what they do not: a prompt to the operator, which the terminal answers, as a task of the loop the
operator booted the life on, begun while the World speaks, so what it comes to reaches the life through close under
the name of the World. It tells the operator on stderr why the provider paused a chain.
"""

import asyncio
import os
import stat
import sys
import threading
from collections.abc import Callable, Coroutine, Generator, Sequence
from contextlib import suppress
from dataclasses import dataclass, field
from pathlib import Path
from typing import TextIO

from furb import engine
from furb.engine import Refused
from furb_monty import _monty

type World = Generator[tuple | None, tuple]
"""The World, an Ear of engine.pyi: engine.py binds no such name, so this module says the type itself."""

type Answer = Callable[[dict, Callable[..., None]], tuple]
"""A function that answers each request of the provider with a turn in place of a model, and that may tell what it
writes as it writes it, through `write(text="", thinking="")`."""


def kept(record: Path) -> list[tuple]:
  """The record of an earlier life, as the entries it holds, which is what boot is given: what the store of the crate
  kept at the path, read with no lease."""
  return entries(_monty.kept(str(record)))


def entries(record: list[list[list[object]]]) -> list[tuple]:
  """The entries of a record as the store of the crate gives them, each one fact, as the engine takes them."""
  return [(tuple(one),) for (one,) in record]


def answered(entries: Sequence[tuple]) -> list[tuple]:
  """Every answer of a model that the entries of a record hold: the done of each reply that came to a turn, which a
  record gives back as the tuple it is, or as a list when the store kept it before it kept tuples, and none of a
  reply that came to a refusal."""
  return [
    one
    for one, *_ in entries
    if one[0] == "done" and engine.question(("reply", one[1])) and isinstance(one[3], tuple | list)
  ]


def handed(loop: asyncio.AbstractEventLoop, reader: asyncio.StreamReader, stream: TextIO) -> None:
  """The lines of the operator, read on a thread of their own and each handed to the loop with the end of the terminal
  after them: the thread reads the terminal and runs nothing else, and it stops when the loop is closed."""
  with suppress(RuntimeError):
    for line in iter(stream.buffer.readline, b""):
      loop.call_soon_threadsafe(reader.feed_data, line)
    loop.call_soon_threadsafe(reader.feed_eof)


@dataclass
class Live:
  """The World of one life on this machine: its operator, and the ears of the crate, the provider of its models among
  them.

  `directory` is where the chains of the life start. `actor` is the actor a prompt goes to when it names none, and
  `roster` names the models the life offers beside its model, and the life offers every model that the catalog of the
  crate offers when it is unsaid. `answer` answers each request in place of the models, when it is given, and
  `stream` is told what a model writes as it writes it, on a thread of the models. `images` is the directory of the
  images that a turn names. `calls` holds every question the World heard that it or the provider answered, in order.
  `ears` are the ears of the crate the life is booted on, which the World lets go at its end. `reader` reads the
  terminal and `reading` keeps one read of it at a time, since there is one operator.
  """

  directory: str
  actor: str | None = None
  roster: list[str] | None = None
  answer: Answer | None = None
  stream: Callable[[str, str, str, str], None] | None = None
  images: str | None = None
  calls: list[tuple] = field(default_factory=list)
  reader: asyncio.StreamReader | None = None
  reading: asyncio.Lock = field(default_factory=asyncio.Lock)
  ears: dict[str, _monty.NativeEar] = field(default_factory=dict)

  def opened(self, record: Path | None = None, *, keeps: bool = True, extensions: bool = True) -> list[tuple]:
    """The record a life opens on, and the ears of the crate held for it, as every host of the crate opens a life:
    the provider, which stands on what this World names, then the extensions, the files, the commands, time, and
    the store of the record when the life keeps, which holds its lease until the World ends. The life enables at its
    tip the extensions that the configs turn on, unless `extensions` is false."""
    stored, ears = _monty.opened(
      directory=self.directory,
      record=None if record is None else str(record),
      keeps=keeps,
      extensions=extensions,
      actor=self.actor,
      roster=self.roster,
      answer=self.answer,
      stream=self.stream,
      images=self.images,
    )
    self.ears.update(ears)
    return entries(stored)

  def end(self) -> None:
    """The ears of the crate let go: a command ends, a wait ends, a model is asked nothing more, and the store lets
    its record go."""
    for one in self.ears.values():
      one.dispose()

  async def line(self) -> str:
    """One line of the operator, which the loop reads off the terminal, or, when the loop reads no pipe of it, as the
    loop of Windows reads none, a thread that reads the lines of the operator alone and hands each to the loop, so
    nothing of the life or of a model runs on it.

    The reader of the terminal is made once, at the first read, and it is the one reader there is.
    """
    if self.reader is None:
      # The loop is asked for the number of the file first: a terminal it cannot read fails here, where nothing
      # has been built yet, rather than half way through a reader whose own end then fails again.
      mode = os.fstat(sys.stdin.fileno()).st_mode
      self.reader = reader = asyncio.StreamReader()
      made = asyncio.StreamReaderProtocol(reader)
      loop = asyncio.get_running_loop()
      # A selector loop reads a pipe, a socket or a device, and no file. The proactor of Windows takes a pipe that it
      # cannot read and fails only at its first read, so it reads none.
      piped = (
        isinstance(loop, asyncio.SelectorEventLoop)
        and (stat.S_ISFIFO(mode) or stat.S_ISSOCK(mode) or stat.S_ISCHR(mode))
        and await loop.connect_read_pipe(lambda: made, sys.stdin)
      )
      if not piped:
        threading.Thread(target=handed, args=(loop, reader, sys.stdin), name="furb-operator", daemon=True).start()
    return (await self.reader.readline()).decode(errors="replace").rstrip("\r\n")

  async def show(self, about: str, shape: str, message: str) -> None:
    """A prompt of the operator: the message on the terminal, and one line back as the shape the prompt wants, by the
    rules of the crate; and no line for a shape the operator answers not, which that no line refuses. A prompt of
    None asks nothing: the World closed it as it took it, and the terminal shows its message as a notice.

    There is one terminal and one operator, so prompts of the operator are shown and answered one at a time, in
    the order they asked, and never two at once on one stream.
    """
    if shape == "None":
      async with self.reading:
        sys.stdout.write(f"{about} tells: {message}\n")
        sys.stdout.flush()
      return
    try:
      line = ""
      if shape in _monty.shapes():
        async with self.reading:
          sys.stdout.write(f"{about} wants a {shape}: {message}\n> ")
          sys.stdout.flush()
          line = await self.line()
      value = _monty.answered(shape, line)
    except (OSError, ValueError) as no:
      value = Refused(f"the operator cannot be read: {no}")
    except Refused as no:
      value = no
    engine.close(value, about)

  def hears(self) -> World:
    """The World as one generator for one life, which comes before the provider: it takes a prompt to the operator,
    whose done it says when the operator answered, and tells the operator why the provider paused a chain."""
    jobs: set[asyncio.Task[None]] = set()
    loop = asyncio.get_running_loop()
    # The chains the provider paused, until it says why with the refusal of the reply it paused them at.
    paused: set[str] = set()

    def start(work: Coroutine[object, object, None]) -> None:
      """One task of the World, held while it runs, so that nothing collects it before it is done."""
      job = loop.create_task(work)
      jobs.add(job)
      job.add_done_callback(jobs.discard)

    while True:
      a = yield
      # Every question the World or the provider answered, and none that the World only heard.
      if a[0] in ("prompt", "reply", "stand"):
        self.calls.append(a)
      match a:
        case ("prompt", about, _, _, shape, message, _):
          yield "started", about
          if shape == "None":
            engine.close(None, about)
          start(self.show(about, shape, message))
        case ("pause", on, "provider", *_):
          paused.add(on)
        case ("done", about, "provider", Refused() as why) if engine.scope(about) in paused:
          paused.discard(engine.scope(about))
          sys.stderr.write(f"{engine.scope(about)} is paused: {why}\n")

"""The client of the process of a kit, in Python, as `@uael/furb/skill` is in TypeScript.

A life starts the process of a kit at the first act of a kind that the manifest of the kit names under `furb-acts`,
with `FURB_LIFE` and `FURB_SKILL`, and speaks to it on its stdin and its stdout, one frame of JSON for each line.
`Skill.attach` says its hello and attaches the life. The kit then answers the acts of its kinds, hears the facts of
the kinds that it asks, and says verbs that the life takes, as a fact that it learned of its own accord. The welcome of
the life names the folders of the life and each verb past those of the first protocol of a kit. A kit draws nothing:
its word draws its views.

    skill = await Skill.attach(kinds=[])
    skill.answers("note_add", lambda fact: f"added {fact[4]}")
    await skill.closed

The types of the protocol stand in `furb.protocol`, which the schema of the daemon makes.
"""

import asyncio
import contextlib
import inspect
import json
import os
import sys
import threading
from collections.abc import Callable, Collection, Mapping
from typing import Protocol

from furb.protocol import PROTOCOL, Answer, Data, Frame, Kind, Peer

LIFE = "FURB_LIFE"
SKILL = "FURB_SKILL"
"""The variables that the life gives the process of a kit: the record of the life that it serves, and the name of its
kit."""

LIMIT = 1 << 30
"""The longest line that a connection reads: a frame of the life is one line, and a fact can hold a large text."""

type Fact = list[object]
"""A fact of the life as the life tells it: its kind, the act that it is about, who said it, and its words."""


class Refused(Exception):
  """A refusal of the life, named by its kind, as `closed` or `role`, and `refused` when it says no more."""

  def __init__(self, why: str, kind: str = "refused") -> None:
    super().__init__(why)
    self.kind = kind


class Closed(Exception):
  """A request that the life left with no answer when its stream closed, as when it ends."""


class Writer(Protocol):
  """Where a connection writes its lines: a stream of asyncio, or the stdout of this process."""

  def write(self, data: bytes) -> object: ...

  def close(self) -> object: ...


class Stdout:
  """The stdout of this process, written whole for each line, which the life reads."""

  def write(self, data: bytes) -> None:
    sys.stdout.buffer.write(data)
    sys.stdout.buffer.flush()

  def close(self) -> None:
    sys.stdout.close()


def stdin() -> asyncio.StreamReader:
  """The stdin of this process as a stream, fed by a thread, since asyncio reads no pipe of stdin on Windows."""
  loop = asyncio.get_running_loop()
  reader = asyncio.StreamReader(LIMIT)

  def fed() -> None:
    for line in sys.stdin.buffer:
      loop.call_soon_threadsafe(reader.feed_data, line)
    loop.call_soon_threadsafe(reader.feed_eof)

  threading.Thread(target=fed, daemon=True).start()
  return reader


class Connection:
  """One connection to the life, after its hello. Each verb takes a number and its answer, in any order, and each
  frame that the life pushes goes to each listener of `pushed`. `closed` is done when the stream ends, from either
  side, and each verb that waits then raises `Closed`."""

  peer: Peer
  welcome: Data
  closed: asyncio.Future[None]

  def __init__(self, reader: asyncio.StreamReader, writer: Writer) -> None:
    self.reader, self.writer = reader, writer
    self.next = 0
    self.waiting: dict[int, asyncio.Future[Answer]] = {}
    self.listeners: list[Callable[[Frame], object]] = []
    self.closed = asyncio.get_running_loop().create_future()
    self.reading = asyncio.ensure_future(self.read())

  async def hello(self, kind: Kind, name: str | None = None) -> None:
    """The hello of this connection, as a peer of a kind, with the name of its kit, and the peer that the life
    welcomes."""
    hello: dict[str, object] = {"verb": "hello", "protocol": PROTOCOL, "kind": kind, "name": name}
    welcome = await self.ask(hello)
    if welcome["answer"] != "welcome":
      self.close()
      why = f"The life gave {welcome['answer']}, and no welcome."
      raise Refused(why)
    self.welcome = welcome["data"]
    self.peer = self.welcome["peer"]

  async def ask(self, verb: Mapping[str, object]) -> Answer:
    """The answer of the life to a verb, or its refusal as a `Refused`. A field that is None is not said."""
    if self.closed.done():
      why = "This connection holds no stream to the life."
      raise Closed(why)
    number, self.next = self.next, self.next + 1
    waits = self.waiting[number] = asyncio.get_running_loop().create_future()
    said = {"id": number, **{key: value for key, value in verb.items() if value is not None}}
    self.writer.write(json.dumps(said).encode() + b"\n")
    return await waits

  def pushed(self, listener: Callable[[Frame], object]) -> Callable[[], None]:
    """Each frame that the life pushes from now on, heard until the call that this gives."""
    self.listeners.append(listener)
    return lambda: self.listeners.remove(listener)

  def close(self) -> None:
    self.writer.close()

  async def read(self) -> None:
    try:
      # A life that resets the stream ends it as one that closes it.
      with contextlib.suppress(ConnectionError):
        while line := await self.reader.readline():
          if line.strip():
            self.heard(json.loads(line))
    finally:
      for waits in self.waiting.values():
        waits.done() or waits.set_exception(Closed("The life closed the connection."))
      self.waiting.clear()
      self.writer.close()
      self.closed.set_result(None)

  def heard(self, frame: Frame) -> None:
    if frame["frame"] in ("reply", "refused"):
      waits = self.waiting.pop(frame.get("id") or 0, None)
      if waits is None or waits.done():
        return
      if frame["frame"] == "reply":
        waits.set_result(frame["answer"])
      else:
        waits.set_exception(Refused(frame["why"], frame["error"]["error"]))  # ty: ignore[invalid-key]
      return
    for listener in list(self.listeners):
      listener(frame)


class Skill:
  """The process of a kit as a peer of its life: it attaches the life under the name of its kit, hears the facts of the
  kinds that it asked, answers the acts of its kinds, and says verbs to the life, which the life takes."""

  def __init__(self, connection: Connection, name: str, life: str, kinds: Collection[str] | None) -> None:
    self.connection, self.name, self.life, self.kinds = connection, name, life, kinds
    self.facts: list[Callable[[Fact, int], object]] = []
    self.answering: dict[str, Callable[[Fact], object]] = {}
    self.running: dict[str, asyncio.Task[None]] = {}
    connection.pushed(self.pushed)

  @classmethod
  async def attach(
    cls,
    kinds: Collection[str] | None = None,
    start: int = 0,
    env: Mapping[str, str] = os.environ,
    streams: tuple[asyncio.StreamReader, Writer] | None = None,
  ) -> Skill:
    """The kit of this process attached to its life, on the stdin and the stdout of this process unless `streams`
    names others, from the variables that the life gave it. It hears the facts of the kinds that it names, each kind
    when it names none, from the place `start` of the life."""
    life, name = env.get(LIFE), env.get(SKILL)
    if life is None or not name:
      why = f"A process of a kit needs {LIFE} and {SKILL}, which its life gives it."
      raise LookupError(why)
    reader, writer = streams or (stdin(), Stdout())
    connection = Connection(reader, writer)
    await connection.hello("skill", name)
    skill = cls(connection, name, life, kinds)
    await connection.ask({"verb": "attach", "life": life, "from": start})
    return skill

  @property
  def closed(self) -> asyncio.Future[None]:
    """Done when the stream of the life ends, as at the end of the life."""
    return self.connection.closed

  def heard(self, listener: Callable[[Fact, int], object]) -> Callable[[], None]:
    """Each fact of a kind that the skill asked, with its place in the log, in the order of the log, from now on."""
    self.facts.append(listener)
    return lambda: self.facts.remove(listener)

  def answers(self, kind: str, answer: Callable[[Fact], object]) -> Callable[[], None]:
    """Each act of a kind that the life gives the kit, and that is not done, answered from now on, whatever kinds the
    kit hears: the kit closes it with what `answer` gives for its fact, awaited when it is awaitable, or with a refusal
    that names what it raised. An act that is done already gets no answer, and the done of an act that a control
    ended cancels its answer."""
    self.answering[kind] = answer

    def unanswered() -> None:
      self.answering.pop(kind, None)

    return unanswered

  async def answered(self, fact: Fact, answer: Callable[[Fact], object]) -> None:
    act = str(fact[1])
    try:
      await self.say({"verb": "peek", "act": act})
    except Refused:
      try:
        got = answer(fact)
        value = json.dumps(await got if inspect.isawaitable(got) else got)
      except Exception as no:
        value = json.dumps({"is": "Refused", "args": [str(no) or type(no).__name__]})
      await self.say({"verb": "close", "act": act, "value": value})

  async def say(self, verb: Mapping[str, object]) -> Answer:
    """The answer of the life to a verb that the kit says."""
    return await self.connection.ask({**verb, "life": self.life})

  async def fact(self, kind: str, about: str, *words: object) -> None:
    """A fact said to the life of its own accord, of a kind, about a chain or an act, with words of JSON, which a life
    that names the capability `fact` in its welcome takes."""
    if "fact" not in self.connection.welcome.get("capabilities", []):
      why = "This life takes no fact from a kit: it is older than the capability fact."
      raise Refused(why)
    said = [json.dumps(one) for one in words]
    await self.say({"verb": "fact", "kind": kind, "about": about, "words": said})

  async def settings(self) -> dict[str, object]:
    """The value of each key of the kit, as the configs give it, else the default of its file of keys, which a life
    that names the capability `config` in its welcome gives. The life says a fact `configured` at each change of a
    key through the configs of its window, with the key and the value that wins for it, which a kit hears."""
    if "config" not in self.connection.welcome.get("capabilities", []):
      why = "This life gives no config to a kit: it is older than the capability config."
      raise Refused(why)
    answer = await self.say({"verb": "config", "config": {"action": "get"}})
    if answer["answer"] != "values":
      why = f"The life gave {answer['answer']}, and no values."
      raise Refused(why)
    keys = [one for one in answer["data"] if isinstance(one, dict) and one.get("skill") == self.name]
    return {one["key"][1]: one["value"] for one in keys}

  async def run(self, word: str, on: str | None = None) -> str:
    """A word run on a chain, the root unless it names one: the name of its rung."""
    return named(await self.say({"verb": "run", "word": word, "on": on}))

  async def prompt(self, shape: str, message: str, to: str | None = None, on: str | None = None) -> str:
    """A prompt on a chain, of a shape, with a message, to an actor: the name of the prompt."""
    return named(await self.say({"verb": "prompt", "shape": shape, "message": message, "to": to, "on": on}))

  def close(self) -> None:
    self.connection.close()

  def pushed(self, frame: Frame) -> None:
    match frame:
      case {"frame": "event", "life": self.life, "seq": int(seq), "event": {"event": "fact", "data": list(fact)}}:
        if (answer := self.answering.get(str(fact[0]))) is not None:
          task = self.running[str(fact[1])] = asyncio.ensure_future(self.answered(fact, answer))
          task.add_done_callback(lambda _: self.running.pop(str(fact[1]), None))
        elif fact[0] == "done" and (task := self.running.get(str(fact[1]))) is not None:
          task.cancel()
        if self.kinds is None or fact[0] in self.kinds:
          for listener in list(self.facts):
            listener(fact, seq)


def named(answer: Answer) -> str:
  """The act that a verb made, as the answer of the daemon names it."""
  if answer["answer"] != "act":
    why = f"The life gave {answer['answer']}, and no act."
    raise Refused(why)
  return answer["data"]


__all__ = ["LIFE", "SKILL", "Closed", "Connection", "Fact", "Refused", "Skill", "Stdout", "Writer", "stdin"]

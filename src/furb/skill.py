"""The client of the process of a skill, in Python, as `@uael/furb/skill` is in TypeScript.

The daemon of the machine starts the process of a skill with `FURB_HOST`, `FURB_LIFE` and `FURB_SKILL`. `Skill.attach`
dials the daemon from them and attaches the life as a peer of the kind skill. The skill then hears the facts of the
kinds that it asks, says verbs that the life takes under its name, and serves a tree to each view that a window mounts.

    skill = await Skill.attach(kinds=["prompt"])
    skill.heard(lambda fact, seq: fact[5] == "ping" and asyncio.ensure_future(skill.run("pong = 1")))
    skill.served(lambda view: asyncio.ensure_future(view.tree({"id": "said", "node": "text", "runs": [{"text": "hi"}]})))
    await skill.closed

The types of the protocol stand in `furb.protocol`, which the schema of the daemon makes.
"""

import asyncio
import contextlib
import json
import os
import re
import sys
from collections.abc import Callable, Collection, Mapping
from pathlib import Path

from furb.protocol import PROTOCOL, Answer, Effect, Frame, Kind, Mount, Node, Peer, Slot, ViewEvent

HOST = "FURB_HOST"
LIFE = "FURB_LIFE"
SKILL = "FURB_SKILL"
"""The variables that the daemon gives the process of a skill: the folder where it listens, the record of the life
that the process serves, and the name of its skill."""

LIMIT = 1 << 30
"""The longest line that a connection reads: a frame of the daemon is one line, and a fact can hold a large text."""

type Fact = list[object]
"""A fact of the life as the daemon tells it: its kind, the act that it is about, who said it, and its words."""


def daemon_place(folder: str) -> str:
  """Where a peer dials the daemon that listens in a folder: its socket, or its named pipe on Windows."""
  return (
    "\\\\.\\pipe\\furb-" + re.sub(r"[\\/:]", "-", folder) if sys.platform == "win32" else str(Path(folder, "host.sock"))
  )


class Refused(Exception):
  """A refusal of the daemon, named by its kind, as `closed`, `role` or `no_skill`, and `refused` when it says no more."""

  def __init__(self, why: str, kind: str = "refused") -> None:
    super().__init__(why)
    self.kind = kind


class Closed(Exception):
  """A request that the daemon left with no answer when its connection closed, as when it ends or hands over."""


async def piped(place: str) -> tuple[asyncio.StreamReader, asyncio.StreamWriter]:
  """The streams of a named pipe of Windows, where asyncio opens no pipe by a stream function of its own."""
  loop = asyncio.get_running_loop()
  reader = asyncio.StreamReader(LIMIT)
  protocol = asyncio.StreamReaderProtocol(reader)
  transport, _ = await loop.create_pipe_connection(lambda: protocol, place)  # ty: ignore[unresolved-attribute]
  return reader, asyncio.StreamWriter(transport, protocol, reader, loop)


class Connection:
  """One connection to the daemon, after its hello. Each verb takes a number and its answer, in any order, and each
  frame that the daemon pushes goes to each listener of `pushed`. `closed` is done when the stream ends, from either
  side, and each verb that waits then raises `Closed`."""

  peer: Peer
  closed: asyncio.Future[None]

  def __init__(self, reader: asyncio.StreamReader, writer: asyncio.StreamWriter) -> None:
    self.reader, self.writer = reader, writer
    self.next = 0
    self.waiting: dict[int, asyncio.Future[Answer]] = {}
    self.listeners: list[Callable[[Frame], object]] = []
    self.closed = asyncio.get_running_loop().create_future()
    self.reading = asyncio.ensure_future(self.read())

  @classmethod
  async def dial(cls, place: str, kind: Kind, name: str | None = None) -> Connection:
    """A connection to the daemon at a place, as a peer of a kind, with the name of a skill for a skill."""
    win = sys.platform == "win32"
    reader, writer = await (piped(place) if win else asyncio.open_unix_connection(place, limit=LIMIT))
    connection = cls(reader, writer)
    hello: dict[str, object] = {"verb": "hello", "protocol": PROTOCOL, "kind": kind, "name": name}
    welcome = await connection.ask(hello)
    if welcome["answer"] != "welcome":
      connection.close()
      why = f"The daemon gave {welcome['answer']}, and no welcome."
      raise Refused(why)
    connection.peer = welcome["data"]["peer"]
    return connection

  async def ask(self, verb: Mapping[str, object]) -> Answer:
    """The answer of the daemon to a verb, or its refusal as a `Refused`. A field that is None is not said."""
    if self.closed.done():
      why = "This connection holds no stream to the daemon."
      raise Closed(why)
    number, self.next = self.next, self.next + 1
    waits = self.waiting[number] = asyncio.get_running_loop().create_future()
    said = {"id": number, **{key: value for key, value in verb.items() if value is not None}}
    self.writer.write(json.dumps(said).encode() + b"\n")
    return await waits

  def pushed(self, listener: Callable[[Frame], object]) -> Callable[[], None]:
    """Each frame that the daemon pushes from now on, heard until the call that this gives."""
    self.listeners.append(listener)
    return lambda: self.listeners.remove(listener)

  def close(self) -> None:
    self.writer.close()

  async def read(self) -> None:
    try:
      # A daemon that resets the stream ends it as one that closes it.
      with contextlib.suppress(ConnectionError):
        while line := await self.reader.readline():
          if line.strip():
            self.heard(json.loads(line))
    finally:
      for waits in self.waiting.values():
        waits.done() or waits.set_exception(Closed("The daemon closed the connection."))
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


class View:
  """A view of the skill that a window mounted: its viewer and its slot. `tree` serves its tree, each in the place of
  the one before, and `effect` asks its one window to do something. What a person does to its nodes goes to each
  listener of `events`, and `disposed` is done when the window or the daemon ends it."""

  def __init__(self, connection: Connection, mount: Mount) -> None:
    self.connection = connection
    self.viewer: str = mount["viewer"]
    self.slot: Slot = mount["slot"]
    self.version = 0
    self.listeners: list[Callable[[ViewEvent], object]] = []
    self.disposed: asyncio.Future[None] = asyncio.get_running_loop().create_future()

  async def tree(self, tree: Node) -> None:
    self.version += 1
    verb = {"verb": "view.tree", "viewer": self.viewer, "slot": self.slot, "version": self.version, "tree": tree}
    await self.connection.ask(verb)

  async def effect(self, effect: Effect) -> None:
    await self.connection.ask({"verb": "view.effect", "viewer": self.viewer, "effect": effect})

  def events(self, listener: Callable[[ViewEvent], object]) -> Callable[[], None]:
    self.listeners.append(listener)
    return lambda: self.listeners.remove(listener)


class Skill:
  """The process of a skill as a peer of the daemon: it attaches the life that the daemon started it for, under the
  name of its skill, hears the facts of the kinds that it asked, says verbs to the life, which the life takes under
  that name, and serves a view to each window that mounts one."""

  def __init__(self, connection: Connection, name: str, life: str, kinds: Collection[str] | None) -> None:
    self.connection, self.name, self.life, self.kinds = connection, name, life, kinds
    self.views: dict[str, View] = {}
    self.facts: list[Callable[[Fact, int], object]] = []
    self.mounts: list[Callable[[View], object]] = []
    connection.pushed(self.pushed)

  @classmethod
  async def attach(
    cls, kinds: Collection[str] | None = None, start: int = 0, env: Mapping[str, str] = os.environ
  ) -> Skill:
    """The skill of this process attached to its life, from the variables that the daemon gave it. It hears the
    facts of the kinds that it names, each kind when it names none, from the place `start` of the log of the life."""
    host, life, name = env.get(HOST), env.get(LIFE), env.get(SKILL)
    if not (host and life and name):
      why = f"A process of a skill needs {HOST}, {LIFE} and {SKILL}, which the daemon gives it."
      raise LookupError(why)
    connection = await Connection.dial(daemon_place(host), "skill", name)
    skill = cls(connection, name, life, kinds)
    await connection.ask({"verb": "attach", "life": life, "from": start})
    return skill

  @property
  def closed(self) -> asyncio.Future[None]:
    """Done when the connection to the daemon ends, as at the end of the life or of the daemon."""
    return self.connection.closed

  def heard(self, listener: Callable[[Fact, int], object]) -> Callable[[], None]:
    """Each fact of a kind that the skill asked, with its place in the log, in the order of the log, from now on."""
    self.facts.append(listener)
    return lambda: self.facts.remove(listener)

  def served(self, listener: Callable[[View], object]) -> Callable[[], None]:
    """Each view that a window mounts from now on."""
    self.mounts.append(listener)
    return lambda: self.mounts.remove(listener)

  async def say(self, verb: Mapping[str, object]) -> Answer:
    """The answer of the life to a verb that the skill says, under its name."""
    return await self.connection.ask({**verb, "life": self.life})

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
        if self.kinds is None or fact[0] in self.kinds:
          for listener in list(self.facts):
            listener(fact, seq)
      case {"frame": "view.mount", "life": self.life}:
        view = self.views[frame["viewer"]] = View(self.connection, frame)
        for listener in list(self.mounts):
          listener(view)
      case {"frame": "view.event", "viewer": str(viewer)} if viewer in self.views:
        for listener in list(self.views[viewer].listeners):
          listener(frame)
      case {"frame": "view.dispose", "viewer": str(viewer)} if viewer in self.views:
        self.views.pop(viewer).disposed.set_result(None)


def named(answer: Answer) -> str:
  """The act that a verb made, as the answer of the daemon names it."""
  if answer["answer"] != "act":
    why = f"The daemon gave {answer['answer']}, and no act."
    raise Refused(why)
  return answer["data"]


__all__ = ["HOST", "LIFE", "SKILL", "Closed", "Connection", "Fact", "Refused", "Skill", "View", "daemon_place"]

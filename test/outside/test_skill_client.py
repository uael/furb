"""The client of the process of a skill in Python, `furb.skill`, against a double of the daemon on a real socket.

The double answers each verb and pushes the frames that a test gives it. The lines that the client says are held to
`host/tests/kits/44.jsonl`, which the suite of furb-host says to each later daemon: a client that says those lines is
welcome there.
"""

import asyncio
import json
import shutil
import sys
import tempfile
from collections.abc import AsyncIterator, Callable, Mapping
from pathlib import Path

import pytest

from furb import skill as client
from furb.protocol import PROTOCOL

GOLDEN = Path(__file__).resolve().parents[2] / "host" / "tests" / "kits" / "44.jsonl"
PEER = {"operator": "o", "role": "write", "kind": "skill", "name": "k"}


class Daemon:
  """A double of the daemon in a folder: it keeps each verb that it hears, answers it with `answers`, and pushes the
  frames of `push` to the peer."""

  def __init__(self, folder: str) -> None:
    self.folder = folder
    self.said: list[dict[str, object]] = []
    self.writer: asyncio.StreamWriter | None = None
    self.answers: Callable[[dict[str, object]], dict[str, object] | None] = answered

  async def serve(self, reader: asyncio.StreamReader, writer: asyncio.StreamWriter) -> None:
    self.writer = writer
    while line := await reader.readline():
      verb = json.loads(line)
      self.said.append(verb)
      frame = self.answers(verb)
      if frame is not None:
        self.push({"id": verb["id"], **frame})
    writer.close()

  def push(self, frame: Mapping[str, object]) -> None:
    assert self.writer is not None, "a peer dialed"
    self.writer.write(json.dumps(frame).encode() + b"\n")


def answered(verb: dict[str, object]) -> dict[str, object]:
  """What the daemon answers a verb: a welcome, the act that it made, or done."""
  match verb["verb"]:
    case "hello":
      return {"frame": "reply", "answer": {"answer": "welcome", "data": {"peer": PEER}}}
    case "run" | "prompt":
      return {"frame": "reply", "answer": {"answer": "act", "data": f"{verb['verb']}{verb['id']}"}}
  return {"frame": "reply", "answer": {"answer": "done"}}


@pytest.fixture
async def daemon() -> AsyncIterator[Daemon]:
  # A socket path has a short bound, which the folder of a test passes on macOS.
  folder = tempfile.mkdtemp(prefix="furb-skill-", dir=None if sys.platform == "win32" else "/tmp")  # noqa: S108
  double = Daemon(folder)
  place = client.daemon_place(folder)
  if sys.platform == "win32":
    loop = asyncio.get_running_loop()
    pipes = await loop.start_serving_pipe(
      lambda: asyncio.StreamReaderProtocol(asyncio.StreamReader(), double.serve), place
    )
    servers = [pipe.close for pipe in pipes]
  else:
    servers = [(await asyncio.start_unix_server(double.serve, place)).close]
  try:
    yield double
  finally:
    for close in servers:
      close()
    shutil.rmtree(folder)


def env(daemon: Daemon) -> dict[str, str]:
  return {client.HOST: daemon.folder, client.LIFE: "a.jsonl", client.SKILL: "k"}


async def until(check: Callable[[], object]) -> None:
  async with asyncio.timeout(5):
    while not check():
      await asyncio.sleep(0.01)


async def test_a_skill_says_the_lines_of_a_kit_that_each_later_daemon_takes(daemon: Daemon) -> None:
  skill = await client.Skill.attach(env=env(daemon))
  assert skill.connection.peer == PEER
  assert await skill.run("x = 1") == "run2"
  await skill.run("y = 2", on="chain2")
  assert await skill.prompt("str", "hi") == "prompt4"
  await skill.prompt("str", "hi", to="claude-cli:opus/high", on="chain2")
  views: list[client.View] = []
  skill.served(views.append)
  daemon.push(
    {"frame": "view.mount", "viewer": "w", "life": "a.jsonl", "slot": {"slot": "dock", "on": "chain1"}, "skill": "k"}
  )
  await until(lambda: views)
  golden = [json.loads(line) for line in GOLDEN.read_text().splitlines()]
  await views[0].tree(golden[6]["tree"])
  for line in golden[8:]:
    await views[0].effect(line["effect"])
  # The second tree of the golden is served on another slot, which a view of the client keeps from its mount, and the
  # number of a verb is the order in which a peer says it.
  said = [{**golden[0], "protocol": PROTOCOL}, *golden[1:7], *golden[8:]]
  assert daemon.said == [{**line, "id": number} for number, line in enumerate(said)]


async def test_a_skill_hears_the_facts_of_its_kinds_and_the_gestures_and_end_of_each_view(daemon: Daemon) -> None:
  skill = await client.Skill.attach(kinds=["prompt"], start=3, env=env(daemon))
  assert daemon.said[1] == {"id": 1, "verb": "attach", "life": "a.jsonl", "from": 3}
  facts: list[tuple[list[object], int]] = []
  skill.heard(lambda fact, seq: facts.append((fact, seq)))
  ignored = skill.heard(lambda _fact, _seq: pytest.fail("a listener that left hears nothing"))
  ignored()
  for seq, kind in enumerate(["run", "prompt"]):
    fact = [kind, f"{kind}1", "operator/o", "root"]
    daemon.push({"frame": "event", "life": "a.jsonl", "seq": seq, "event": {"event": "fact", "data": fact}})
  daemon.push({"frame": "event", "life": "b.jsonl", "seq": 2, "event": {"event": "fact", "data": ["prompt"]}})
  views: list[client.View] = []
  skill.served(views.append)()
  skill.served(views.append)
  daemon.push({"frame": "view.mount", "viewer": "w", "life": "a.jsonl", "slot": {"slot": "sidebar"}, "skill": "k"})
  await until(lambda: views)
  gestures: list[object] = []
  views[0].events(gestures.append)
  views[0].events(gestures.append)()
  pressed = {"frame": "view.event", "viewer": "w", "node": "go", "kind": "press"}
  daemon.push(pressed)
  daemon.push({**pressed, "viewer": "x"})
  daemon.push({"frame": "view.dispose", "viewer": "w"})
  await asyncio.wait_for(views[0].disposed, 5)
  assert facts == [(["prompt", "prompt1", "operator/o", "root"], 1)]
  assert gestures == [pressed]
  assert (views[0].viewer, views[0].slot, skill.views) == ("w", {"slot": "sidebar"}, {})


async def test_a_refusal_names_its_kind_and_a_close_ends_each_verb_that_waits(daemon: Daemon) -> None:
  skill = await client.Skill.attach(env=env(daemon))
  daemon.answers = lambda _verb: {"frame": "refused", "error": {"error": "role"}, "why": "No role."}
  with pytest.raises(client.Refused, match=r"No role\.") as refused:
    await skill.run("x = 1")
  assert refused.value.kind == "role"
  daemon.answers = lambda _verb: {"frame": "reply", "answer": {"answer": "done"}}
  with pytest.raises(client.Refused, match=r"The daemon gave done, and no act\."):
    await skill.prompt("str", "hi")
  daemon.answers = lambda _verb: None
  waits = asyncio.ensure_future(skill.run("x = 1"))
  await until(lambda: len(daemon.said) == 5)
  daemon.push({"frame": "reply", "id": 99, "answer": {"answer": "done"}})
  assert daemon.writer is not None
  daemon.writer.close()
  with pytest.raises(client.Closed, match=r"The daemon closed the connection\."):
    await waits
  await asyncio.wait_for(skill.closed, 5)
  with pytest.raises(client.Closed, match="no stream"):
    await skill.run("x = 1")


async def test_a_skill_needs_the_variables_of_the_daemon_and_its_welcome(daemon: Daemon) -> None:
  with pytest.raises(LookupError, match="FURB_HOST, FURB_LIFE and FURB_SKILL"):
    await client.Skill.attach(env={client.HOST: daemon.folder})
  daemon.answers = lambda _verb: {"frame": "reply", "answer": {"answer": "done"}}
  with pytest.raises(client.Refused, match=r"The daemon gave done, and no welcome\."):
    await client.Skill.attach(env=env(daemon))
  daemon.answers = answered
  skill = await client.Skill.attach(env=env(daemon))
  skill.close()
  await asyncio.wait_for(skill.closed, 5)


@pytest.mark.skipif(sys.platform == "win32", reason="the suite on Windows dials a real pipe")
async def test_a_pipe_of_windows_gives_the_streams_that_a_socket_gives(
  daemon: Daemon, monkeypatch: pytest.MonkeyPatch
) -> None:
  # The pipe and the socket take the same arguments and give a transport and a protocol, so the socket stands in for
  # the pipe here, where no pipe of Windows is.
  loop = asyncio.get_running_loop()
  monkeypatch.setattr(loop, "create_pipe_connection", loop.create_unix_connection, raising=False)
  reader, writer = await client.piped(client.daemon_place(daemon.folder))
  writer.write(b'{"id": 0, "verb": "hello"}\n')
  assert json.loads(await reader.readline())["answer"]["answer"] == "welcome"
  writer.close()

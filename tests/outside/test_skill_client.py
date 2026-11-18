"""The client of the process of a kit in Python, `furb.skill`, against a double of its life on two streams.

The double answers each verb and pushes the frames that a test gives it. The lines that the client says are held to
`crates/host/tests/kits/44.jsonl`, the lines that a kit of the protocol 44 says, which each later life takes.
"""

import asyncio
import json
import os
import sys
from asyncio.subprocess import PIPE
from collections.abc import Callable, Collection, Mapping
from pathlib import Path

import pytest

from furb import skill as client
from furb.protocol import PROTOCOL

GOLDEN = Path(__file__).resolve().parents[2] / "crates" / "host" / "tests" / "kits" / "44.jsonl"
HEARD = GOLDEN.with_suffix(".heard.jsonl")
PEER = {"operator": "", "role": "write", "kind": "skill", "name": "k"}
ENV = {client.LIFE: "a.jsonl", client.SKILL: "k"}


class Life:
  """A double of the life of a kit: it keeps each verb that the kit writes, answers it with `answers`, and pushes
  the frames of `push` to the kit, on the stream that the kit reads."""

  def __init__(self) -> None:
    self.said: list[dict[str, object]] = []
    self.reader = asyncio.StreamReader()
    self.answers: Callable[[dict[str, object]], dict[str, object] | None] = answered

  def write(self, data: bytes) -> None:
    for line in data.splitlines():
      verb = json.loads(line)
      self.said.append(verb)
      frame = self.answers(verb)
      if frame is not None:
        self.push({"id": verb["id"], **frame})

  def close(self) -> None:
    self.reader.feed_eof()

  def push(self, frame: Mapping[str, object]) -> None:
    self.reader.feed_data(json.dumps(frame).encode() + b"\n")

  async def attach(self, kinds: Collection[str] | None = None, start: int = 0) -> client.Skill:
    return await client.Skill.attach(kinds, start, env=ENV, streams=(self.reader, self))


def answered(verb: dict[str, object]) -> dict[str, object]:
  """What the daemon answers a verb: a welcome, the act that it made, or done."""
  match verb["verb"]:
    case "hello":
      return {"frame": "reply", "answer": {"answer": "welcome", "data": {"peer": PEER}}}
    case "run" | "prompt":
      return {"frame": "reply", "answer": {"answer": "act", "data": f"{verb['verb']}{verb['id']}"}}
  return {"frame": "reply", "answer": {"answer": "done"}}


@pytest.fixture
async def life() -> Life:
  return Life()


async def until(check: Callable[[], object]) -> None:
  async with asyncio.timeout(5):
    while not check():
      await asyncio.sleep(0.01)


async def test_a_skill_says_the_lines_of_a_kit_that_each_later_life_takes(life: Life) -> None:
  skill = await life.attach()
  assert skill.connection.peer == PEER
  assert await skill.run("x = 1") == "run2"
  await skill.run("y = 2", on="chain2")
  assert await skill.prompt("str", "hi") == "prompt4"
  await skill.prompt("str", "hi", to="claude-cli:opus/high", on="chain2")
  golden = [json.loads(line) for line in GOLDEN.read_text().splitlines()]
  said = [{**golden[0], "protocol": PROTOCOL}, *golden[1:6]]
  assert life.said == said


async def test_a_skill_hears_the_facts_of_its_kinds_of_its_life_alone(life: Life) -> None:
  skill = await life.attach(kinds=["prompt"], start=3)
  assert life.said[1] == {"id": 1, "verb": "attach", "life": "a.jsonl", "from": 3}
  facts: list[tuple[list[object], int]] = []
  skill.heard(lambda fact, seq: facts.append((fact, seq)))
  ignored = skill.heard(lambda _fact, _seq: pytest.fail("a listener that left hears nothing"))
  ignored()
  for seq, kind in enumerate(["run", "prompt"]):
    fact = [kind, f"{kind}1", "operator/o", "root"]
    life.push({"frame": "event", "life": "a.jsonl", "seq": seq, "event": {"event": "fact", "data": fact}})
  life.push({"frame": "event", "life": "b.jsonl", "seq": 2, "event": {"event": "fact", "data": ["prompt"]}})
  await until(lambda: facts)
  await asyncio.sleep(0.05)
  assert facts == [(["prompt", "prompt1", "operator/o", "root"], 1)]


async def test_a_skill_answers_each_open_act_of_its_kind_with_a_close_and_skips_one_that_is_done(life: Life) -> None:
  skill = await life.attach(kinds=["prompt"])
  done = {"k.add1"}

  def answering(verb: dict[str, object]) -> dict[str, object]:
    if verb["verb"] == "peek" and verb["act"] not in done:
      return {"frame": "refused", "error": {"error": "refused"}, "why": f"No act named {verb['act']} is done."}
    return answered(verb)

  life.answers = answering

  async def added(fact: list[object]) -> str:
    await asyncio.sleep(0)
    return f"added {fact[4]}"

  def failed(_fact: list[object]) -> object:
    raise ValueError("no room")

  skill.answers("k.add", added)
  skill.answers("k.drop", failed)
  skill.answers("k.count", lambda _fact: 2)
  skill.answers("k.gone", added)()
  for seq, (kind, act) in enumerate(
    [("k.add", "k.add1"), ("k.add", "k.add2"), ("k.drop", "k.drop1"), ("k.count", "k.count1"), ("k.gone", "x")]
  ):
    fact = [kind, act, "rung4", "chain1", "milk"]
    life.push({"frame": "event", "life": "a.jsonl", "seq": seq, "event": {"event": "fact", "data": fact}})
  await until(lambda: sum(verb["verb"] == "close" for verb in life.said) == 3)
  closes = sorted((str(verb["act"]), str(verb["value"])) for verb in life.said if verb["verb"] == "close")
  refusal = json.dumps({"is": "Refused", "args": ["no room"]})
  assert closes == [("k.add2", json.dumps("added milk")), ("k.count1", "2"), ("k.drop1", refusal)]
  assert sorted(str(verb["act"]) for verb in life.said if verb["verb"] == "peek") == [
    "k.add1",
    "k.add2",
    "k.count1",
    "k.drop1",
  ]


async def test_a_kit_answers_an_act_that_waits_for_it_as_it_attaches_with_the_answers_that_its_welcome_made(
  life: Life,
) -> None:
  waiting = ["k.add", "k.add1", "rung4", "chain1", "milk"]

  def answering(verb: dict[str, object]) -> dict[str, object] | None:
    match verb["verb"]:
      case "hello":
        welcome = {"peer": PEER, "folders": {"project": "/p"}}
        return {"frame": "reply", "answer": {"answer": "welcome", "data": welcome}}
      case "attach":
        # The life gives the act that waits for the kit in the instant after its reply to the attach.
        life.push({"id": verb["id"], "frame": "reply", "answer": {"answer": "done"}})
        life.push({"frame": "event", "life": "a.jsonl", "seq": 0, "event": {"event": "fact", "data": waiting}})
        return None
      case "peek":
        return {"frame": "refused", "error": {"error": "refused"}, "why": "not done"}
    return answered(verb)

  life.answers = answering

  def made(welcome: Mapping[str, object]) -> dict[str, Callable[[list[object]], object]]:
    folders = welcome["folders"]
    assert isinstance(folders, dict)
    return {"k.add": lambda fact: f"added {fact[4]} in {folders['project']}"}

  await client.Skill.attach([], env=ENV, streams=(life.reader, life), answering=made)
  await until(lambda: any(verb["verb"] == "close" for verb in life.said))
  assert [(verb["act"], verb["value"]) for verb in life.said if verb["verb"] == "close"] == [
    ("k.add1", json.dumps("added milk in /p"))
  ]


async def test_the_done_of_an_act_that_a_kit_answers_cancels_its_answer(life: Life) -> None:
  skill = await life.attach(kinds=[])
  life.answers = lambda verb: (
    {"frame": "refused", "error": {"error": "refused"}, "why": "not done"} if verb["verb"] == "peek" else answered(verb)
  )
  started, cancelled = asyncio.Event(), asyncio.Event()

  async def slow(_fact: list[object]) -> str:
    started.set()
    try:
      await asyncio.sleep(60)
    except asyncio.CancelledError:
      cancelled.set()
      raise
    return "late"

  skill.answers("k.slow", slow)
  life.push({"frame": "event", "life": "a.jsonl", "seq": 0, "event": {"event": "fact", "data": ["k.slow", "k.slow1"]}})
  await asyncio.wait_for(started.wait(), 5)
  done = ["done", "k.slow1", "operator", "chain1", None]
  life.push({"frame": "event", "life": "a.jsonl", "seq": 1, "event": {"event": "fact", "data": done}})
  await asyncio.wait_for(cancelled.wait(), 5)
  assert not [verb for verb in life.said if verb["verb"] == "close"]


async def test_a_skill_reads_each_frame_that_a_kit_of_its_version_reads(life: Life) -> None:
  skill = await life.attach()
  facts: list[list[object]] = []
  skill.heard(lambda fact, _seq: facts.append(fact))
  heard = [json.loads(line) for line in HEARD.read_text().splitlines()]
  for frame in heard:
    life.push(frame)
  await until(lambda: facts)
  assert facts == [heard[0]["event"]["data"]]


async def test_a_refusal_names_its_kind_and_a_close_ends_each_verb_that_waits(life: Life) -> None:
  skill = await life.attach()
  life.answers = lambda _verb: {"frame": "refused", "error": {"error": "role"}, "why": "No role."}
  with pytest.raises(client.Refused, match=r"No role\.") as refused:
    await skill.run("x = 1")
  assert refused.value.kind == "role"
  life.answers = lambda _verb: {"frame": "reply", "answer": {"answer": "done"}}
  with pytest.raises(client.Refused, match=r"The life gave done, and no act\."):
    await skill.prompt("str", "hi")
  life.answers = lambda _verb: None
  waits = asyncio.ensure_future(skill.run("x = 1"))
  await until(lambda: len(life.said) == 5)
  life.push({"frame": "reply", "id": 99, "answer": {"answer": "done"}})
  life.close()
  with pytest.raises(client.Closed, match=r"The life closed the connection\."):
    await waits
  await asyncio.wait_for(skill.closed, 5)
  with pytest.raises(client.Closed, match="no stream"):
    await skill.run("x = 1")


async def test_a_skill_needs_the_variables_of_its_life_and_its_welcome(life: Life) -> None:
  with pytest.raises(LookupError, match="FURB_LIFE and FURB_SKILL"):
    await client.Skill.attach(env={client.LIFE: "a.jsonl"}, streams=(life.reader, life))
  life.answers = lambda _verb: {"frame": "reply", "answer": {"answer": "done"}}
  with pytest.raises(client.Refused, match=r"The life gave done, and no welcome\."):
    await life.attach()


async def test_a_fact_goes_to_a_life_that_names_its_capability_and_an_older_life_refuses_it(life: Life) -> None:
  skill = await life.attach()
  with pytest.raises(client.Refused, match=r"older than the capability fact"):
    await skill.fact("k.seen", "chain1", "milk")
  skill.connection.welcome["capabilities"] = ["fact"]
  await skill.fact("k.seen", "chain1", "milk", 2)
  assert life.said[-1] == {
    "id": 2,
    "verb": "fact",
    "kind": "k.seen",
    "about": "chain1",
    "words": ['"milk"', "2"],
    "life": "a.jsonl",
  }


KIT = """
import asyncio
from furb.skill import Skill


async def main():
  skill = await Skill.attach(kinds=[])
  skill.answers("k.add", lambda fact: f"added {fact[4]}")
  await skill.run("ready")
  await skill.closed
  skill.close()


asyncio.run(main())
"""
"""KIT is a kit whose process answers the acts `k.add`, and says a word once it answers them."""


async def test_a_kit_speaks_to_its_life_on_the_stdin_and_the_stdout_of_its_process(tmp_path: Path) -> None:
  program = tmp_path / "kit.py"
  program.write_text(KIT, encoding="utf-8")
  kit = await asyncio.create_subprocess_exec(
    sys.executable, str(program), env={**os.environ, **ENV}, stdin=PIPE, stdout=PIPE
  )
  assert kit.stdin is not None and kit.stdout is not None

  async def said() -> dict[str, object]:
    return json.loads(await asyncio.wait_for(kit.stdout.readline(), 10))  # ty: ignore[unresolved-attribute]

  def told(frame: Mapping[str, object]) -> None:
    kit.stdin.write(json.dumps(frame).encode() + b"\n")  # ty: ignore[unresolved-attribute]

  for _ in range(3):
    verb = await said()
    told({"id": verb["id"], **answered(verb)})
  fact = ["k.add", "k.add1", "rung4", "chain1", "milk"]
  told({"frame": "event", "life": "a.jsonl", "seq": 0, "event": {"event": "fact", "data": fact}})
  peek = await said()
  told({"frame": "refused", "id": peek["id"], "error": {"error": "refused"}, "why": "not done"})
  close = await said()
  assert (close["verb"], close["act"], close["value"]) == ("close", "k.add1", json.dumps("added milk"))
  kit.stdin.close()
  assert await asyncio.wait_for(kit.wait(), 10) == 0


async def test_a_kit_reads_its_own_keys_from_a_life_that_gives_config(life: Life) -> None:
  settings = [
    {"key": ["k", "folder"], "value": "journal", "skill": "k"},
    {"key": ["other", "folder"], "value": "x", "skill": "other"},
    {"key": ["automatic"], "value": []},
  ]

  def answers(verb: dict[str, object]) -> dict[str, object]:
    match verb["verb"]:
      case "hello":
        welcome = {"peer": PEER, "capabilities": ["config"]}
        return {"frame": "reply", "answer": {"answer": "welcome", "data": welcome}}
      case "config":
        return {"frame": "reply", "answer": {"answer": "values", "data": settings}}
    return answered(verb)

  life.answers = answers
  skill = await life.attach()
  assert await skill.settings() == {"folder": "journal"}
  assert life.said[-1] == {"id": 2, "verb": "config", "config": {"action": "get"}, "life": "a.jsonl"}
  life.answers = lambda verb: (
    {"frame": "reply", "answer": {"answer": "done"}} if verb["verb"] == "config" else answers(verb)
  )
  with pytest.raises(client.Refused, match="The life gave done, and no values"):
    await skill.settings()
  life.answers = answered
  older = await Life().attach()
  with pytest.raises(client.Refused, match="older than the capability config"):
    await older.settings()

"""Standing, what a chain stands on."""

import pytest

from conftest import STANDS, Sand, born, life, paragraphs, plain, said, settle, stood
from furb import engine
from furb.engine import Drift

ROSTER = STANDS[0]


async def test_what_a_chain_stands_on() -> None:
  """What a chain stands on: the actors the World offers, the directory the chain starts in, the actor a prompt goes to when it names none, and the path of the doctrine, which the World reads in the first turn of the setup of the root, and which a World that gives no doctrine leaves unsaid."""
  _, log, root = born("close(1)", files={"/w/b.txt": "here\n"})
  assert await engine.prompt(int, "count", on=root) == 1
  assert engine.standing() == STANDS and engine.standing()[0] == ROSTER
  assert engine.cwd(on=root) == "" and engine.read("b.txt", on=root).path == "/w/b.txt"
  assert [a[4] for a in said(log, "reply")] == ["m/low"]
  assert engine.module(root)["actor"] == "m/low"
  doctrine = "/w/.furb/DOCTRINE.md"
  born(stands=[*STANDS, doctrine], files={doctrine: "Look first.\n"})
  assert engine.standing() == [*STANDS, doctrine]


async def test_the_roster_the_directory_and_the_actor_that_a_model_reads_are_in_the_transcript() -> None:
  """The roster, the directory and the actor that a model reads are in the transcript of its chain."""
  sand, log, root = born("close(1)")
  step = engine.rung("stand()\ncd('/' + 'w')", on=root)
  assert await step is None
  assert await engine.prompt(int, "count", on=root) == 1
  told = paragraphs(sand.turns[said(log, "reply")[0][1]])
  assert told[1:4] == ["stand()\ncd('/' + 'w')", stood("stand2"), "#cd1\ncd1_path = '/w'"]
  assert [engine.module(root)[name] for name in ("stand2_roster", "cd1_path", "stand2_actor")] == STANDS


async def test_a_standing_holds_no_source() -> None:
  """A standing holds no source: the engine is one file the model imports, and a record replays only on an engine that makes each of its acts again, and else drifts."""
  sand, _, root = born("x = bash('echo hi')\nclose(1)")
  assert await engine.prompt(int, "run it", on=root) == 1
  await settle()
  kept = [
    ((*fact[:4], "echo other", *fact[5:]), *rest) if fact[0] == "bash" else (fact, *rest)
    for fact, *rest in plain(sand.record)
  ]
  with pytest.raises(Drift, match=r"^bash1 drifts: "):
    life(Sand(), kept)

"""Standing, what a chain stands on."""

import pytest

from conftest import STANDS, Sand, born, life, paragraphs, plain, said, settle, takes
from furb import engine
from furb.engine import Drift

ROSTER = STANDS[0]


async def test_what_a_chain_stands_on() -> None:
  """What a chain stands on: the actors the World offers, the directory the chain starts in, and the actor a thread goes to when it names none."""
  _, log, root = born("close(1)")
  assert await engine.thread(int, "count", on=root) == 1
  assert engine.module(root)[f"{root}_roster"] == ROSTER
  assert engine.cwd(on=root) == "/w"
  assert [a[4] for a in said(log, "reply")] == ["m/low"]
  assert engine.module(root)["actor"] == "m/low"


async def test_the_roster_the_directory_and_the_actor_that_a_model_reads_are_in_the_transcript() -> None:
  """The roster, the directory and the actor that a model reads are in the transcript of its chain."""
  sand, log, root = born("close(1)")
  assert await engine.thread(int, "count", on=root) == 1
  told = takes(root)
  assert told == (
    "#chain1 standing\n"
    "chain1_roster = [['operator', [], 200000], ['m', ['low', 'high'], 400000], ['n', ['low'], 200000]]\n"
    "chain1_cwd = '/w'\n"
    "chain1_actor = 'm/low'"
  )
  assert [paragraphs(sand.turns[a[1]])[1] for a in said(log, "reply")] == [told]
  held = engine.transcript(root)
  assert [a for a in said(held, "tell") if a[3][0].startswith(f"#{root} standing")] == [
    ("tell", root, root, [takes(root)])
  ]


async def test_a_standing_holds_no_source() -> None:
  """A standing holds no source: the engine is one file the model imports, and a record made by another engine is a drift."""
  sand, _, root = born("x = bash('echo hi')\nclose(1)")
  assert await engine.thread(int, "run it", on=root) == 1
  await settle()
  kept = [
    ((*fact[:4], "echo other", *fact[5:]), *rest) if fact[0] == "bash" else (fact, *rest)
    for fact, *rest in plain(sand.record)
  ]
  with pytest.raises(Drift, match=r"^bash1 drifts$"):
    life(Sand(), kept)

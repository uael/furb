"""Standing, what a chain stands on."""

import pytest

from conftest import STANDS, Sand, life, paragraphs, plain, said, settle
from furb import engine
from furb.engine import Drift

ROSTER, WHERE, WHO = STANDS


async def test_what_a_chain_stands_on() -> None:
  """What a chain stands on: the actors the World offers, the directory the chain starts in, and the actor a prompt goes to when it names none."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  assert (WHERE, WHO) == ("/w", "m/low")
  assert [name for name, *_ in ROSTER] == ["operator", "m", "n"]
  assert engine.cwd(on=root) == "/w"
  assert [a[4] for a in said(log, "ask")] == ["m/low"]
  assert engine.modules[root]["actor"] == "m/low"


async def test_the_roster_the_directory_and_the_actor_that_a_model_reads_are_in_the_transcript() -> None:
  """The roster, the directory and the actor that a model reads are in the transcript of its chain."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  told = f"#{root} stands {STANDS!r}"
  assert told == (
    "#chain1 stands ((('operator', (), 200000), ('m', ('low', 'high'), 400000), ('n', ('low',), 200000)), '/w', 'm/low')"
  )
  assert [paragraphs(a[5])[1] for a in said(log, "ask")] == [told]
  _, held = engine.ask("transcript", root, root)
  assert isinstance(held, list)
  assert [a for a in held if a[0] == "tell" and a[3][0].startswith(f"#{root} stands ")] == [
    ("tell", root, root, [told])
  ]


async def test_a_standing_holds_no_source() -> None:
  """A standing holds no source: the engine is one file the model imports, and a record made by another engine is a drift."""
  assert len(STANDS) == 3
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose(1)"]
  assert await engine.prompt(int, "run it", on=root) == 1
  await settle()
  kept = [
    (after, (*fact[:4], "echo other", *fact[5:]), *rest) if fact[0] == "bash" else (after, fact, *rest)
    for after, fact, *rest in plain(sand.record)
  ]
  with pytest.raises(Drift, match=r"^bash1 drifts$"):
    life(Sand(stands=STANDS), kept)

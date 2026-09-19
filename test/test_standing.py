"""Standing, what a chain stands on."""

import pytest

from conftest import STANDS, Sand, life, plain, said, settle, tags
from furb import engine
from furb.engine import Drift

ROSTER, WHERE, WHO = STANDS


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
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


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_the_roster_the_directory_and_the_actor_that_a_model_reads_are_in_the_transcript() -> None:
  """The roster, the directory and the actor that a model reads are in the transcript of its chain."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  told = tags(engine.turns(on=root), "opened")[1]
  assert told == ("opened", [("id", root), ("roster", ROSTER), ("directory", "/w"), ("actor", "m/low")], None)
  _, held = engine.ask("transcript", root, root)
  assert isinstance(held, list)
  assert [tag for a in held if a[0] == "tell" for tag in a[3] if ("roster", ROSTER) in tag[1]] == [told]


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
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
  with pytest.raises(Drift, match="is not the bash://"):
    life(Sand(stands=STANDS), kept)

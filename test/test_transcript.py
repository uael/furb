"""Transcript, the question of the transcript of a chain up to an act."""

import pytest

from conftest import life, said, settle, sown
from furb import engine
from furb.engine import OPERATOR


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_transcript_is_the_question_of_the_transcript_of_a_chain_up_to_an_act() -> None:
  """A transcript is the question of the transcript of a chain up to an act, which the chain answers, and which a chain with a source and a grant ask."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["a = 1\nclose(1)"]
  assert await engine.prompt(int, "work", on=root) == 1
  await settle()
  asking, held = engine.ask("transcript", root, root)
  assert isinstance(held, list)
  assert asking == ("transcript", asking[1], OPERATOR, root, root)
  assert engine.outcomes[asking[1]] is held
  _, first, *_ = said(log, "prompt")[0]
  assert [a[1] for a in held if a[0] == "prompt"] == [first]
  ceiling = engine.grant(usd=10.0, on=root)
  twin = engine.chain("twin", source=root)
  await settle(300)
  _, now = engine.ask("transcript", root, root)
  assert isinstance(now, list)
  assert [(a[2], a[4]) for a in said(now, "transcript") if a[2] in (ceiling, twin)] == [(ceiling, root), (twin, root)]
  cut, upto = engine.ask("transcript", root, first)
  assert isinstance(upto, list)
  assert cut == ("transcript", cut[1], OPERATOR, root, first) and len(upto) < len(now)

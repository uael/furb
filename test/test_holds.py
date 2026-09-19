"""Holds, the question of what the record kept of an act."""

import pytest

from conftest import STANDS, Sand, life, relived, said, settle
from furb import engine


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_holds_is_the_question_of_what_the_record_kept_of_an_act() -> None:
  """A holds is the question of what the record kept of an act."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  await engine.bash("echo hi", on=root)
  await settle()
  command = said(log, "bash")[0][1]
  assert engine.ask("holds", root, command)[1] == []
  _, over = await relived(Sand(stands=STANDS), list(sand.record))
  held = engine.ask("holds", over, command)[1]
  assert isinstance(held, list)
  assert [one[0] for one in held] == ["bash", "out", "exited"]
  assert engine.ask("holds", over, "bash://nobody")[1] == []


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_the_chain_holds_its_holds_in_the_transcript() -> None:
  """The chain holds its holds in the transcript, where the ask stands in a life that asks, so the fold cuts a user turn there in every life."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  sand.script[root] = ["x = bash('slow')", "y = 2", "close(3)"]
  assert await engine.prompt(int, "count", on=root) == 3
  await settle()
  was = engine.turns(on=root)
  assert [turn[0] for turn in was] == ["user", "assistant", "user", "assistant", "user", "assistant", "user"]
  _, over = await relived(Sand(stands=STANDS), list(sand.record))
  _, held = engine.ask("transcript", over, over)
  assert isinstance(held, list)
  asked = [one for one in held if one[0] == "holds" and one[2] == over]
  assert [one[4] for one in asked] == [f"rung://operator.2.{n}" for n in (1, 2, 3)]
  assert engine.turns(on=over) == was

"""Holds, the question of what the record kept of an act."""

from conftest import STANDS, Sand, life, relived, said, settle
from furb import engine


async def test_a_holds_is_the_question_of_what_the_record_kept_of_an_act() -> None:
  """A holds is the question of what the record kept of an act."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  await engine.wait(0, on=root)
  await settle()
  waited = said(log, "wait")[0][1]
  assert engine.ask("holds", root, waited)[1] == []
  _, over = await relived(Sand(stands=STANDS), list(sand.record))
  held = engine.ask("holds", over, waited)[1]
  assert isinstance(held, list)
  assert [one[0] for one in held] == ["wait", "done"]
  assert engine.ask("holds", over, "wait9")[1] == []


async def test_the_chain_holds_its_holds_in_the_transcript() -> None:
  """The chain holds its holds in the transcript, where the ask stands in a life that asks, so the fold cuts a user turn there in every life."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = wait(100)", "y = 2", "close(3)"]
  act = engine.prompt(int, "count", on=root)
  assert await act == 3
  await settle()
  steps = [a[1] for a in said(log, "rung") if a[2] == act]
  assert [a[1] for a in said(log, "ask")] == steps
  was = engine.turns(on=root)
  assert [turn[0] for turn in was] == ["user", "assistant", "user", "assistant", "user", "assistant", "user"]
  _, over = await relived(Sand(stands=STANDS), list(sand.record))
  _, held = engine.ask("transcript", over, over)
  assert isinstance(held, list)
  asked = [one for one in held if one[0] == "holds" and one[2] == over]
  assert [one[4] for one in asked] == steps
  assert engine.turns(on=over) == was

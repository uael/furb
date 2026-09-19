"""outcomes, what every question of the life came to."""

import pytest

from conftest import STANDS, Sand, life, relived, said, settle
from furb import engine
from furb.engine import WORLD, Exit, Text


async def test_outcomes_holds_what_every_question_of_the_life_came_to_under_its_name() -> None:
  """outcomes holds what every question of the life came to, under its name, once the done that names it has landed: the value it was answered or completed with, or the exception it completed with, and no entry while it waits."""
  sand = Sand(files={"/w/a.txt": "one\n"}, stands=STANDS, auto=False)
  log, root = life(sand)
  act = engine.bash("slow", on=root)
  await settle()
  assert act not in engine.outcomes
  engine.send("exited", act, 0, by=WORLD)
  await settle()
  assert isinstance(engine.outcomes[act], Exit)
  got = engine.read("a.txt", on=root)
  word = next(one for one in sand.calls if one[0] == "read")
  assert engine.outcomes[word[1]] == got == Text("/w/a.txt", "one\n")
  with pytest.raises(ValueError, match="boom"):
    await engine.rung("raise ValueError('boom')", on=root)
  step = said(log, "rung")[0][1]
  assert isinstance(engine.outcomes[step], ValueError)


async def test_the_first_done_of_a_question_is_what_it_came_to() -> None:
  """The first done of a question is what it came to, and the life fills it in when that done is said, and a later done of the same question fills nothing."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.rung("close(21)", on=root)
  await act
  assert engine.outcomes[act] == 21
  mark = len(log)
  engine.send("done", act, 99, by=WORLD)
  await settle()
  assert [one[3] for one in said(log[mark:], "done")] == [99]
  assert engine.outcomes[act] == 21


async def test_the_outcome_is_no_slot_of_the_question() -> None:
  """The outcome is no slot of the question, so the plain form of a question holds none of it, and a question made again from the record waits as it did."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  act = engine.bash("slow", on=root)
  await settle()
  assert act not in engine.outcomes and len(said(log, "bash")[0]) == 7
  kept = list(sand.record)
  assert [len(e[1]) for e in kept if e[1][0] == "bash"] == [7]
  later = Sand(stands=STANDS, auto=False)
  _, over = await relived(later, kept)
  assert over == root and act in engine.acts and act not in engine.outcomes


async def test_boot_empties_it() -> None:
  """boot empties it."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = engine.rung("close(1)", on=root)
  await act
  assert engine.outcomes[act] == 1
  _, over = life(Sand(stands=STANDS))
  await settle()
  assert over == root and act not in engine.outcomes

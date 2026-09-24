"""Done, what an act came to."""

from asyncio import CancelledError
from collections.abc import Generator

from conftest import STANDS, Sand, life, said, settle
from furb import engine
from furb.engine import OPERATOR, WORLD


def lasting(id: str) -> Generator[tuple | None, tuple]:
  """The ear of an act of an extension that is done with one at its birth and lives on to answer a read of a door of
  its own, and says nothing else."""
  yield "done", id, 1
  while True:
    match (yield):
      case ("read", qid, _, _, path) if path == f"{id}/kept":
        yield "done", qid, "kept"


async def test_what_an_act_came_to_a_done_settles_the_act_it_names() -> None:
  """What an act came to: a done settles the act it names."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.rung("close(21)", on=root)
  await act
  ends = [one for one in said(log, "done") if one[1] == act]
  assert [one[3] for one in ends] == [21]
  assert act in engine.outcomes and (await act) == 21


async def test_a_result_enters_the_transcript_whether_or_not_anyone_awaits_it() -> None:
  """A result enters the transcript whether or not anyone awaits it."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = engine.wait(0, on=root)
  await settle()
  _, held = engine.ask("transcript", root, root)
  assert isinstance(held, list)
  ends = [one for one in held if one[0] == "done" and one[1] == act]
  assert [(one[2], one[3]) for one in ends] == [(WORLD, None)]


async def test_an_act_that_is_over_says_nothing_of_its_own_and_an_ear_that_lives_past_its_done_still_answers() -> None:
  """An act that is over says nothing of its own, and an ear that lives past its done still answers what it is asked."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  step = engine.rung("k = 1", on=root)
  await step
  kept = engine.act("note", root, lasting)
  assert await kept == 1
  await settle()
  mark = len(log)
  engine.send("tell", kept, [f"#{kept} late"], by=WORLD)
  engine.send("tell", step, [f"#{step} late"], by=WORLD)
  await settle()
  assert [one for one in log[mark:] if one[2] in (step, kept)] == []
  assert engine.ask("read", root, f"{kept}/kept")[1] == "kept"


async def test_a_done_that_an_act_said_itself_is_the_result_of_the_act() -> None:
  """A done that an act said itself is the result of the act."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.rung("close(21)", on=root)
  await act
  ends = [one for one in said(log, "done") if one[1] == act]
  assert [one[2] for one in ends] == [act]
  assert engine.peek(act, on=root) == 21


async def test_a_done_that_names_a_question_is_the_answer_to_the_question() -> None:
  """A done that names a question is the answer to the question."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  got = engine.clock(on=root)
  word = next(one for one in sand.calls if one[0] == "clock")
  ends = [one for one in said(log, "done") if one[1] == word[1]]
  assert [(one[2], one[3]) for one in ends] == [(WORLD, got)]


async def test_a_kind_that_ends_when_it_is_told_to() -> None:
  """A kind that ends when it is told to: it starts its ear, and then a done that names it is what it came to; a cancel over it ends it with a CancelledError, and a close of it with the value that close carries."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  assert await engine.wait(0, on=root) is None
  shut = engine.prompt(int, "how many?", to=OPERATOR, on=root)
  await settle()
  engine.close(21, shut)
  await settle()
  assert (await shut) == 21
  gone = engine.prompt(int, "how many?", to=OPERATOR, on=root)
  await settle()
  engine.cancel(gone)
  await settle()
  assert isinstance(engine.outcomes[gone], CancelledError)
  assert isinstance(engine.peek(gone, on=root), CancelledError)
  assert [one[1] for one in said(log, "done") if one[2] == one[1]] == [shut, gone]

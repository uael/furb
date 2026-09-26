"""Done, what an act came to."""

from asyncio import CancelledError

from conftest import WORLD, born, said, settle, world_says
from furb import engine
from furb.engine import OPERATOR, Exit


async def test_what_an_act_came_to_a_done_settles_the_act_it_names() -> None:
  """What an act came to: a done settles the act it names."""
  _, log, root = born()
  act = engine.rung("close(21)", on=root)
  await act
  ends = [one for one in said(log, "done") if one[1] == act]
  assert [one[3] for one in ends] == [21]
  assert engine.peek(act, ...) is not ... and (await act) == 21


async def test_a_result_enters_the_transcript_whether_or_not_anyone_awaits_it() -> None:
  """A result enters the transcript whether or not anyone awaits it."""
  _, _, root = born()
  act = engine.bash("echo hi", on=root)
  await settle()
  held = engine.transcript(root)
  ends = [one for one in held if one[0] == "done" and one[1] == act]
  assert [isinstance(one[3], Exit) for one in ends] == [True]


async def test_an_act_that_is_over_says_nothing_and_a_command_lives_on_to_answer_its_doors() -> None:
  """An act that is over says nothing, and a command lives on to answer its doors."""
  _, log, root = born()
  step = engine.rung("k = 1", on=root)
  await step
  command = engine.bash("echo hi", on=root)
  await command
  await settle()
  mark = len(log)
  world_says("out", command, "late\n", "stdout")
  await settle()
  assert [one for one in log[mark:] if one[2] in (step, command)] == []
  assert engine.read(f"{command}/stdout", on=root).content == "ran echo hi\n"


async def test_a_done_that_an_act_said_itself_is_the_result_of_the_act() -> None:
  """A done that an act said itself is the result of the act."""
  _, log, root = born()
  act = engine.rung("close(21)", on=root)
  await act
  ends = [one for one in said(log, "done") if one[1] == act]
  assert [one[2] for one in ends] == [act]
  assert engine.peek(act) == 21


async def test_a_done_that_an_ear_says_while_the_act_is_put_to_it_is_the_answer_to_the_act() -> None:
  """A done that an ear says while the act is put to it is the answer to the act, which takes it now."""
  sand, log, root = born(files={"/w/a.txt": "one\n"})
  got = engine.read("a.txt", on=root)
  word = next(one for one in sand.calls if one[0] == "read")
  ends = [one for one in said(log, "done") if one[1] == word[1]]
  assert [(one[2], one[3]) for one in ends] == [(WORLD, got)]
  command = engine.bash("echo hi", on=root)
  await command
  door = engine.read(f"{command}/stdout", on=root)
  assert [one[2] for one in said(log, "done") if one[1] == "read2"] == [command] and door.content == "ran echo hi\n"


async def test_a_kind_that_ends_when_it_is_told_to() -> None:
  """A kind that ends when it is told to: it starts its ear, and then a done that names it is what it came to; a cancel over it ends it with a CancelledError, and a close of it with the value that close carries."""
  _, log, root = born()
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
  assert isinstance(engine.peek(gone), CancelledError)
  assert said(log, "bash") == []

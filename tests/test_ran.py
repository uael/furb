"""Ran, the fact that the Kernel says once the part of a writing is over."""

from conftest import Sand, born, said, settle, world_says
from furb import engine
from furb.engine import Act


async def waiting() -> tuple[Sand, list[tuple], str, Act[int], str]:
  """A life whose prompt waits on the reply of its rung, which no model answers: the World, what was said, the root,
  the prompt and the rung of the prompt."""
  sand, log, root = born()
  act = engine.prompt(int, "count", on=root)
  await settle()
  return sand, log, root, act, said(log, "rung")[0][1]


def wrote(rung: str, part: str, before: int) -> tuple:
  """A writing of a part of the word of a rung, said as the World says one."""
  return world_says("writing", rung, part, before)


async def test_a_ran_is_the_fact_that_the_kernel_says_as_a_rung_once_the_part_of_a_writing_of_that_rung_is_over() -> (
  None
):
  """A ran is the fact that the Kernel says as a rung once the part of a writing of that rung is over, with how many characters of the word stand before that part, and with what a statement of the part raised, or nothing when the part ran to its end."""
  _, log, _, _, rung = await waiting()
  wrote(rung, "a = 1\n", 0)
  wrote(rung, "1 / 0\n", 6)
  await settle()
  fine, broke = said(log, "ran")
  assert fine == ("ran", rung, rung, 0, None)
  assert broke[:4] == ("ran", rung, rung, 6) and type(broke[4]).__name__ == "ZeroDivisionError"


async def test_the_ran_of_a_writing_whose_part_follows_another_word_than_the_word_that_ran_holds_a_refusal() -> None:
  """The ran of a writing whose part follows another word than the word that ran holds a refusal, and its part never begins, since the Kernel runs no statement of a word twice."""
  _, log, root, _, rung = await waiting()
  wrote(rung, "a = 1\n", 0)
  wrote(rung, "b = 2\n", 3)
  await settle()
  _, stray = said(log, "ran")
  assert stray[3] == 3 and type(stray[4]).__name__ == "Refused" and "b" not in engine.module(root)


async def test_the_ran_of_a_writing_of_a_rung_that_is_done_holds_cancelled_error() -> None:
  """The ran of a writing of a rung that is done holds CancelledError, since its part never begins."""
  _, log, root = born("close(1)")
  assert await engine.prompt(int, "count", on=root) == 1
  await settle()
  rung = said(log, "rung")[0][1]
  wrote(rung, "q = 1\n", 0)
  await settle()
  (late,) = said(log, "ran")
  assert type(late[4]).__name__ == "CancelledError" and "q" not in engine.module(root)

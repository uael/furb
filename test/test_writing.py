"""Writing, the fact of a part of the word of a rung that its model finished while it writes the rest."""

from conftest import WORLD, Sand, born, said, settle, world_says
from furb import engine
from furb.engine import Act


async def waiting(**world: object) -> tuple[Sand, list[tuple], str, Act[int], str, str]:
  """A life whose prompt waits on the reply of its rung, which no model answers until the test does: the World, what
  was said, the root, the prompt, the rung of the prompt and its reply."""
  sand, log, root = born(**world)
  act = engine.prompt(int, "count", on=root)
  await settle()
  return sand, log, root, act, said(log, "rung")[0][1], said(log, "reply")[0][1]


def wrote(rung: str, part: str, before: int) -> tuple:
  """A writing of a part of the word of a rung, said as the World says one."""
  return world_says("writing", rung, part, before)


def answered(reply: str, word: str) -> None:
  """The reply answered with a turn whose word is the whole word, as the World answers it."""
  world_says("done", reply, ("assistant", word, (0, 0, 0, 0, 0.0), []))


async def test_a_writing_is_the_fact_that_holds_a_part_of_the_word_of_a_rung() -> None:
  """A writing is the fact that holds a part of the word of a rung, which the World says while the model of the rung writes the word, and which the Kernel hears and runs."""
  _, log, root, _, rung, _ = await waiting()
  fact = wrote(rung, "k = 1\n", 0)
  await settle()
  assert fact == ("writing", rung, WORLD, "k = 1\n", 0)
  assert said(log, "writing") == [fact] and engine.get(rung) is not None
  assert engine.module(root)["k"] == 1


async def test_a_writing_holds_the_part_of_the_word_that_its_model_finished_since_the_writing_before_it() -> None:
  """A writing holds the part of the word that its model finished since the writing before it, and how many characters of the word stand before that part."""
  _, log, root, _, rung, _ = await waiting()
  wrote(rung, "a = 1\n", 0)
  wrote(rung, "b = a + 1\n", 6)
  await settle()
  assert [a[3:] for a in said(log, "writing")] == [("a = 1\n", 0), ("b = a + 1\n", 6)]
  assert engine.module(root)["b"] == 2


async def test_the_kernel_runs_the_part_of_a_writing_in_the_globals_of_the_chain_of_its_rung_and_as_that_rung() -> None:
  """The Kernel runs the part of a writing in the globals of the chain of its rung and as that rung, so what the part binds and what it makes are the rung's own."""
  _, log, root, _, rung, _ = await waiting()
  wrote(rung, "x = bash('one')\n", 0)
  await settle()
  (made,) = said(log, "bash")
  assert engine.module(root)["x"] == made[1] and made[2] == rung


async def test_the_kernel_runs_the_writings_of_one_rung_in_the_order_it_heard_them() -> None:
  """The Kernel runs the writings of one rung in the order it heard them, and a part that waits for an act holds the parts after it."""
  sand, log, root, _, rung, _ = await waiting(auto=False)
  first = "x = await bash('slow')\n"
  wrote(rung, first, 0)
  wrote(rung, "y = 2\n", len(first))
  await settle()
  assert "y" not in engine.module(root) and said(log, "ran") == []
  sand.exits(said(log, "bash")[0][1], 0)
  await settle()
  assert engine.module(root)["y"] == 2
  assert [a[3] for a in said(log, "ran")] == [0, len(first)]


async def test_a_part_that_waits_for_an_act_makes_no_wants() -> None:
  """A part that waits for an act makes no wants, so its rung hears the turn of its model while the part waits."""
  sand, log, _, act, rung, reply = await waiting(auto=False)
  first = "x = await bash('slow')\n"
  wrote(rung, first, 0)
  await settle()
  answered(reply, first + "close(3)")
  await settle()
  assert said(log, "wants") == [] and said(log, "ran") == []
  assert rung in [a[4] for a in said(log, "run")]
  sand.exits(said(log, "bash")[0][1], 1)
  assert await act == 3


async def test_the_run_of_a_rung_runs_what_the_writings_of_that_rung_left_of_the_word() -> None:
  """The run of a rung runs what the writings of that rung left of the word, and nothing when they ran the word whole."""
  _, log, root, act, rung, reply = await waiting()
  wrote(rung, "x = bash('one')\n", 0)
  await settle()
  answered(reply, "x = bash('one')\nclose(1)")
  assert await act == 1
  later = engine.prompt(int, "again", on=root)
  await settle()
  step, asked = said(log, "rung")[-1][1], said(log, "reply")[-1][1]
  wrote(step, "y = bash('two')\n", 0)
  await settle()
  answered(asked, "y = bash('two')")
  await settle()
  (run,) = [a[1] for a in said(log, "run") if a[4] == step]
  assert engine.over(run) and engine.peek(run) is None and not engine.over(later)
  assert [a[4] for a in said(log, "bash")] == ["one", "two"]


async def test_the_run_of_a_rung_whose_writing_raised_is_done_with_that_raise() -> None:
  """The run of a rung whose writing raised is done with that raise and runs no more of the word, since a word is over where it raises."""
  _, log, _, act, rung, reply = await waiting()
  wrote(rung, "1 / 0\n", 0)
  await settle()
  answered(reply, "1 / 0\nclose(5)")
  await settle()
  (run,) = [a[1] for a in said(log, "run") if a[4] == rung]
  assert type(engine.peek(run)).__name__ == "ZeroDivisionError" and not engine.over(act)


async def test_a_run_whose_word_is_not_the_word_that_ran_is_refused() -> None:
  """A run whose word is not the word that ran is refused, since the Kernel runs no statement of a word twice."""
  _, log, root, _, rung, reply = await waiting()
  wrote(rung, "a = 1\n", 0)
  await settle()
  answered(reply, "b = 2")
  await settle()
  (run,) = [a[1] for a in said(log, "run") if a[4] == rung]
  assert type(engine.peek(run)).__name__ == "Refused" and "b" not in engine.module(root)

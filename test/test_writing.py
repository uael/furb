"""Writing, the act of running a part of the word of a rung while its model writes the word."""

from conftest import Sand, born, said, settle, world_says
from furb import engine
from furb.engine import Act, site


async def waiting(**world: object) -> tuple[Sand, list[tuple], str, Act[int], str, str]:
  """A life whose prompt waits on the reply of its rung, which no model answers until the test does: the World, what
  was said, the root, the prompt, the rung of the prompt and its reply."""
  sand, log, root = born(**world)
  act = engine.prompt(int, "count", on=root)
  await settle()
  return sand, log, root, act, said(log, "rung")[0][1], said(log, "reply")[0][1]


def wrote(rung: str, part: str, before: int, on: str) -> str:
  """A writing of a part of the word of a rung, made as that rung, as the World makes one."""
  token = site.set(rung)
  try:
    return engine.act("writing", on, None, rung, part, before)
  finally:
    site.reset(token)


def answered(reply: str, word: str) -> None:
  """The reply answered with a turn whose word is the whole word, as the World answers it."""
  world_says("done", reply, ("assistant", word, (0, 0, 0, 0, 0.0), []))


async def test_a_writing_is_the_act_of_running_a_part_of_the_word_of_a_rung() -> None:
  """A writing is the act of running a part of the word of a rung, which the World makes as that rung while the model of the rung writes the word, and the Kernel takes."""
  _, log, root, _, rung, _ = await waiting()
  made = wrote(rung, "k = 1\n", 0, root)
  await settle()
  assert engine.get(made) == ("writing", made, rung, root, rung, "k = 1\n", 0)
  owned = [a[0] for a in log if a[1] == made and a[0] in ("started", "done") and a[2] == made]
  assert owned == ["started", "done"]
  assert engine.module(root)["k"] == 1


async def test_a_writing_holds_the_part_of_the_word_that_its_model_finished_since_the_writing_before_it() -> None:
  """A writing holds the part of the word that its model finished since the writing before it, and how many characters of the word stand before that part."""
  _, log, root, _, rung, _ = await waiting()
  wrote(rung, "a = 1\n", 0, root)
  wrote(rung, "b = a + 1\n", 6, root)
  await settle()
  assert [a[5:] for a in said(log, "writing")] == [("a = 1\n", 0), ("b = a + 1\n", 6)]
  assert engine.module(root)["b"] == 2


async def test_the_kernel_runs_the_part_of_a_writing_in_the_globals_of_the_chain_of_its_rung_and_as_that_rung() -> None:
  """The Kernel runs the part of a writing in the globals of the chain of its rung and as that rung, so what the part binds and what it makes are the rung's own."""
  _, log, root, _, rung, _ = await waiting()
  wrote(rung, "x = bash('one')\n", 0, root)
  await settle()
  (made,) = said(log, "bash")
  assert engine.module(root)["x"] == made[1] and made[2] == rung


async def test_a_writing_is_done_with_what_a_statement_of_its_part_raised() -> None:
  """A writing is done with what a statement of its part raised, and with nothing when the part ran to its end."""
  _, _, root, _, rung, _ = await waiting()
  fine = wrote(rung, "a = 1\n", 0, root)
  broke = wrote(rung, "1 / 0\n", 6, root)
  await settle()
  assert engine.peek(fine) is None and engine.over(fine)
  assert type(engine.peek(broke)).__name__ == "ZeroDivisionError"


async def test_the_kernel_runs_the_writings_of_one_rung_in_the_order_it_took_them() -> None:
  """The Kernel runs the writings of one rung in the order it took them, and a part that waits for an act holds the parts after it."""
  sand, log, root, _, rung, _ = await waiting(auto=False)
  first = "x = await bash('slow')\n"
  held = wrote(rung, first, 0, root)
  after = wrote(rung, "y = 2\n", len(first), root)
  await settle()
  assert "y" not in engine.module(root) and not engine.over(held) and not engine.over(after)
  sand.exits(said(log, "bash")[0][1], 0)
  await settle()
  assert engine.module(root)["y"] == 2 and engine.over(held) and engine.over(after)


async def test_a_part_that_waits_for_an_act_makes_no_wants() -> None:
  """A part that waits for an act makes no wants, so its rung hears the turn of its model while the part waits."""
  sand, log, root, act, rung, reply = await waiting(auto=False)
  first = "x = await bash('slow')\n"
  held = wrote(rung, first, 0, root)
  await settle()
  answered(reply, first + "close(3)")
  await settle()
  assert said(log, "wants") == [] and not engine.over(held)
  assert rung in [a[4] for a in said(log, "run")]
  sand.exits(said(log, "bash")[0][1], 1)
  assert await act == 3


async def test_the_run_of_a_rung_runs_what_the_writings_of_that_rung_left_of_the_word() -> None:
  """The run of a rung runs what the writings of that rung left of the word, and nothing when they ran the word whole."""
  _, log, root, act, rung, reply = await waiting()
  wrote(rung, "x = bash('one')\n", 0, root)
  await settle()
  answered(reply, "x = bash('one')\nclose(1)")
  assert await act == 1
  later = engine.prompt(int, "again", on=root)
  await settle()
  step, asked = said(log, "rung")[-1][1], said(log, "reply")[-1][1]
  wrote(step, "y = bash('two')\n", 0, root)
  await settle()
  answered(asked, "y = bash('two')")
  await settle()
  (run,) = [a[1] for a in said(log, "run") if a[4] == step]
  assert engine.over(run) and engine.peek(run) is None and not engine.over(later)
  assert [a[4] for a in said(log, "bash")] == ["one", "two"]


async def test_the_run_of_a_rung_whose_writing_raised_is_done_with_that_raise() -> None:
  """The run of a rung whose writing raised is done with that raise and runs no more of the word, since a word is over where it raises."""
  _, log, root, act, rung, reply = await waiting()
  wrote(rung, "1 / 0\n", 0, root)
  await settle()
  answered(reply, "1 / 0\nclose(5)")
  await settle()
  (run,) = [a[1] for a in said(log, "run") if a[4] == rung]
  assert type(engine.peek(run)).__name__ == "ZeroDivisionError" and not engine.over(act)


async def test_a_writing_whose_part_follows_another_word_than_the_word_that_ran_is_refused() -> None:
  """A writing whose part follows another word than the word that ran is refused, and so is a run whose word is not the word that ran, since the Kernel runs no statement of a word twice."""
  _, _, root, _, rung, _ = await waiting()
  wrote(rung, "a = 1\n", 0, root)
  stray = wrote(rung, "b = 2\n", 3, root)
  await settle()
  assert type(engine.peek(stray)).__name__ == "Refused" and "b" not in engine.module(root)
  _, log, root, _, rung, reply = await waiting()
  wrote(rung, "a = 1\n", 0, root)
  await settle()
  answered(reply, "b = 2")
  await settle()
  (run,) = [a[1] for a in said(log, "run") if a[4] == rung]
  assert type(engine.peek(run)).__name__ == "Refused" and "b" not in engine.module(root)


async def test_a_writing_of_a_rung_that_is_done_never_begins_its_part() -> None:
  """A writing of a rung that is done never begins its part, and is done with CancelledError."""
  _, log, root = born("close(1)")
  assert await engine.prompt(int, "count", on=root) == 1
  await settle()
  rung = said(log, "rung")[0][1]
  late = wrote(rung, "q = 1\n", 0, root)
  await settle()
  assert type(engine.peek(late)).__name__ == "CancelledError" and "q" not in engine.module(root)

"""The defects that the owner met and that w/converge still holds, each a test of the rule that its fix keeps, marked
as an expected failure that names the row of the register of defects and its carrier, so the fix turns it green and
xfail_strict turns the suite red until the mark goes."""

import time

import pytest

from conftest import born, life, said, settle, sown
from furb import engine
from furb.engine import OPERATOR


async def test_a_message_of_a_model_on_its_own_thread_asks_that_model_no_step_again() -> None:
  """A message that a word of a thread says on that thread is a fact of its own maker, so it asks the model of the
  thread no step again: the word awaits a command, and no second rung of the thread comes."""
  _, log, root = born(
    "x = bash('slow')\nprompt(None, 'progress', on='prompt1')\nclose((await x).code)", "close(2)", auto=False
  )
  task = engine.prompt(int, "work", on=root)
  assert task == "prompt1"
  await settle()
  await settle()
  assert [a[5] for a in said(log, "prompt") if a[3] == task] == ["progress"]
  assert len([a for a in said(log, "rung") if a[2] == task]) == 1


async def test_a_word_and_its_replay_take_a_time_that_grows_with_their_length_alone() -> None:
  """A long word and the replay of its record take a time that grows with their length alone: a fact costs the same
  whatever count of facts waits in the log, as the facts that a run says while it runs do."""

  async def spent(asks: int) -> tuple[float, float]:
    sand, _, root = born()
    start = time.process_time()
    await engine.rung(f"for i in range({asks}):\n  chance()", on=root)
    ran = time.process_time() - start
    record = list(sand.record)
    start = time.process_time()
    life(sown(), record)
    return ran, time.process_time() - start

  small = [await spent(2000) for _ in range(3)]
  large = [await spent(8000) for _ in range(3)]
  # The large word is 4 times longer: linear gives a ratio near 4 and quadratic near 16, as the walks of the log gave.
  # The bound lies between them with room for a loaded machine.
  for part in (0, 1):
    assert min(one[part] for one in large) < 8 * min(one[part] for one in small)


@pytest.mark.xfail(
  raises=AssertionError,
  reason="H3 of the register of defects: the reminder of an owed message is said at each quiet step; carrier engine:advance-one-rule-r3, then engine:threads-question",
)
async def test_the_reminder_of_a_message_owed_to_the_operator_is_said_once() -> None:
  """The reminder of a message of the operator that a thread owes is said once, however many quiet steps follow."""
  _, log, root = born("await wait(60, __name__)", "x = 1", "y = 2", "z = 3", "close(1)", auto=False)
  first = engine.prompt(int, "work", on=root)
  await settle()
  engine.prompt(None, "use this", on=first)
  await settle()
  await settle()
  reminders = [a for a in said(log, "prompt") if a[3] == first and a[2] == first]
  assert len(reminders) == 1, [a[1] for a in reminders]


@pytest.mark.xfail(
  raises=AssertionError,
  reason="H4b of the register of defects: a close of its own prompt drops the rest of the word in silence; carrier engine:close-rules",
)
async def test_a_word_that_closes_its_own_prompt_runs_its_next_statement_or_tells_that_it_skipped_it() -> None:
  """A word that closes its own prompt loses no statement after the close in silence: the statement runs, or a tell
  on the rung of the word names what the close skipped."""
  _, log, root = born("close(1)\nlater = 2")
  task = engine.prompt(int, "count", on=root)
  await settle()
  (rung,) = [a[1] for a in said(log, "rung") if a[2] == task]
  skipped = [a for a in log if a[0] == "tell" and a[1] == rung and "later" in "\n".join(a[3])]
  assert engine.module(on=root).get("later") == 2 or skipped


@pytest.mark.xfail(
  raises=AssertionError,
  reason="H5 of the register of defects (#1273): a second close of a thread is dropped untold; carrier engine:close-rules",
)
async def test_a_message_on_a_thread_whose_word_awaits_reaches_the_value_of_the_thread_or_is_told_lost() -> None:
  """A message on a thread whose word awaits a command gets a step whose close the thread takes, or that is refused
  and told: no close is dropped untold after the first word closed the thread."""
  sand, log, root = born(
    "run = bash('slow')\nclose('first:' + str((await run).code), 'prompt1')",
    "await Act('bash1')\nclose('second', 'prompt1')",
    auto=False,
  )
  task = engine.prompt(str, "run", on=root)
  assert task == "prompt1"
  await settle()
  (command,) = [a[1] for a in said(log, "bash")]
  engine.prompt(None, "add more", on=task)
  await settle()
  sand.exits(command, 0)
  await settle()
  closes = [a for a in log if a[0] == "close" and a[1] == task]
  second = [a[1] for a in said(log, "rung") if a[2] == task][1]
  told = [a for a in log if a[0] == "tell" and a[1] in (task, second) and "second" in "\n".join(a[3])]
  assert len(closes) == 1 or told, [(a[2], a[3]) for a in closes]


async def test_the_operator_closes_a_thread_whose_word_awaits_a_question_to_the_operator() -> None:
  """The operator closes a thread whose word awaits a question to the operator: H6 of the register of defects (#1262),
  fixed on the path of the engine, which this test guards."""
  _, log, root = born("fruit = await prompt(str, 'which fruit?', to=OPERATOR)\nclose(fruit)")
  task = engine.prompt(str, "ask", on=root)
  await settle()
  assert [a for a in said(log, "prompt") if a[6] == OPERATOR]
  engine.close("resolved", task)
  await settle()
  assert (await task) == "resolved"

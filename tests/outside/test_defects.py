"""The defects that the owner met and that w/converge still holds, each a test of the rule that its fix keeps, marked
as an expected failure that names the row of the register of defects and its carrier, so the fix turns it green and
xfail_strict turns the suite red until the mark goes."""

from pathlib import Path
from sys import monitoring
from types import CodeType

import pytest

import furb
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
  # The work is the count of lines that the package furb runs, which no load of the machine changes, where a time does.
  package = str(Path(furb.__file__).parent)
  ran = 0

  def line(code: CodeType, _: int) -> object:
    nonlocal ran
    if not code.co_filename.startswith(package):
      return monitoring.DISABLE
    ran += 1
    return None

  async def spent(asks: int) -> tuple[int, int]:
    nonlocal ran
    sand, _, root = born()
    ran = 0
    await engine.rung(f"for i in range({asks}):\n  chance()", on=root)
    word = ran
    ran = 0
    life(sown(), list(sand.record))
    return word, ran

  monitoring.use_tool_id(monitoring.PROFILER_ID, "lines of furb")
  monitoring.register_callback(monitoring.PROFILER_ID, monitoring.events.LINE, line)
  monitoring.set_events(monitoring.PROFILER_ID, monitoring.events.LINE)
  try:
    small, large = await spent(2000), await spent(8000)
  finally:
    monitoring.set_events(monitoring.PROFILER_ID, 0)
    monitoring.free_tool_id(monitoring.PROFILER_ID)
  # The large word is 4 times longer: a cost that grows with the length alone, plus what each word costs once, makes it
  # cost at most 4 times more, and each walk of the log of the facts that wait costs more.
  for part in (0, 1):
    assert large[part] <= 4 * small[part], (small, large)


async def test_the_reminder_of_a_message_owed_to_the_operator_is_said_once() -> None:
  """The reminder of a message of the operator that a thread owes is said once, however many quiet steps follow: the
  message asks the chain one step, and a step that answers it with no act asks none more."""
  _, log, root = born("await wait(60, __name__)", "x = 1", "y = 2", "z = 3", "close(1)", auto=False)
  first = engine.prompt(int, "work", on=root)
  await settle()
  engine.prompt(None, "use this", on=first)
  for _ in range(4):
    await settle()
  told = [turn for turn in engine.turns(on=root) if turn[0] == "user" and "'use this'" in turn[1]]
  assert len(told) == 1, told
  assert [a[1] for a in said(log, "rung")] == ["rung1", "rung2"]


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

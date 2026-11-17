"""cancel, which ends an act and everything it made."""

from asyncio import CancelledError

import pytest

from conftest import born, chained, paragraphs, posed, said, settle, slow, stalled
from furb import engine
from furb.engine import OPERATOR, Refused


async def test_a_cancel_of_that_prompt_reaches_the_acts_that_its_rungs_made_on_the_chain_with_a_source() -> None:
  """A cancel of that prompt reaches the acts that its rungs made on the chain with a source."""
  sand, log, root = born(auto=False)
  twin = await chained("twin", root)
  sand.script[root] = [f"y = bash('there', on={twin!r})\nclose((await y).code)"]
  one = engine.prompt(int, "run it there", on=root)
  await settle()
  command = said(log, "bash")[0]
  assert command[3] == twin
  engine.cancel(one)
  await settle()
  assert isinstance(engine.peek(one), CancelledError)
  assert isinstance(engine.peek(command[1]), CancelledError)


async def test_a_cancel_is_over_the_act_it_names_and_everything_that_act_made() -> None:
  """A cancel is over the act it names and everything that act made, and each of them is done with CancelledError."""
  _, log, root, one, step, command = await stalled()
  engine.cancel(one)
  await settle()
  assert isinstance(engine.peek(one), CancelledError)
  assert isinstance(engine.peek(step), CancelledError)
  assert isinstance(engine.peek(command), CancelledError)
  assert [a[1] for a in said(log, "cancel")] == [one]
  waiting = engine.rung("await wait(30, __name__)\nclose(1)", on=root)
  await settle()
  (run,) = [a[1] for a in said(log, "run") if a[4] == waiting]
  engine.cancel(root)
  await settle()
  assert isinstance(engine.peek(waiting), CancelledError) and isinstance(engine.peek(run), CancelledError)
  _, log, root = born(auto=False)
  word = "import asyncio\nasync def watch():\n  await bash('slow')\nt = asyncio.create_task(watch())"
  left = engine.rung(word, on=root)
  await settle()
  (run,) = [a[1] for a in said(log, "run") if a[4] == left]
  engine.cancel(run)
  await settle()
  ended = [engine.peek(x, ...) for x in (run, left, said(log, "wants")[0][1])]
  assert [type(x) for x in ended] == [CancelledError] * 3
  word = (
    "import asyncio\n"
    "seen, later = [], asyncio.get_running_loop().create_future()\n"
    "async def hold():\n"
    "  seen.append(await later)\n"
    "t = asyncio.create_task(hold())\n"
    "await bash('slow')"
  )
  held = engine.rung(word, on=root)
  await settle()
  engine.cancel(held)
  await engine.rung("later.set_result(1)", on=root)
  await settle()
  assert (type(engine.peek(held, ...)), await engine.rung("close(seen)", on=root)) == (CancelledError, [])


async def test_cancel_is_given_the_id_of_an_act_and_says_a_cancel_over_it() -> None:
  """cancel is given the id of an act and a cause, None when none is given, and says a cancel over it that carries that cause."""
  _, log, _, one = slow()
  engine.cancel(one)
  engine.cancel(one, "no need")
  assert said(log, "cancel") == [("cancel", one, OPERATOR, None), ("cancel", one, OPERATOR, "no need")]


async def test_a_cancelled_act_completes_with_cancellederror() -> None:
  """A cancelled act completes with CancelledError, which carries the cause of the cancel when it has one."""
  _, _, root, one = slow()
  two = engine.bash("two", on=root)
  engine.cancel(one)
  engine.cancel(two, "no need")
  await settle()
  assert [(type(got := engine.peek(x)), isinstance(got, CancelledError) and got.args) for x in (one, two)] == [
    (CancelledError, ()),
    (CancelledError, ("no need",)),
  ]


async def test_a_cancelled_prompt_raises_cancellederror_to_whoever_awaits_it() -> None:
  """A cancelled prompt raises CancelledError to whoever awaits it."""
  _, _, _, one = await posed()
  engine.cancel(one)
  with pytest.raises(CancelledError):
    await one


async def test_a_cancel_touches_nothing_else_on_the_chain() -> None:
  """A cancel touches nothing else on the chain."""
  _, _, root = born()
  one = engine.prompt(int, "one", to=OPERATOR, on=root)
  two = engine.prompt(int, "two", to=OPERATOR, on=root)
  await settle()
  engine.cancel(one)
  await settle()
  assert isinstance(engine.peek(one), CancelledError) and engine.peek(two) is None
  engine.close(21, two)
  await settle()
  assert (await two) == 21


async def test_the_awaiter_of_a_cancelled_command_raises_cancellederror_in_its_step() -> None:
  """The awaiter of a cancelled command raises CancelledError in its step."""
  _, _, root, one, step, command = await stalled()
  engine.cancel(command)
  await settle()
  assert isinstance(engine.peek(step), CancelledError)
  ends = [one for one in paragraphs(engine.turns(on=root)) if one.split()[1:2] in (["raised"], ["cancelled"])]
  assert ends == [f"#{command} cancelled by operator", f"#{step} cancelled"]
  assert engine.peek(one) is None
  engine.cancel(one)


async def test_a_cancel_of_a_name_of_no_act_raises_refused_that_names_it() -> None:
  """A cancel of a name of no act of the life raises Refused, which names it, and says nothing."""
  _, log, _ = born()
  await settle()
  before = len(log)
  with pytest.raises(Refused, match="no act 'nothing9'"):
    engine.cancel("nothing9")
  await settle()
  assert log[before:] == []


async def test_a_word_cannot_cancel_a_question_of_a_person_that_is_open() -> None:
  """A word cannot cancel a question of a person that is open, or an act whose cancel is over one, since a question of a person ends with its answer or by a person: the call raises Refused, which names the question and says that it waits for the answer, and says nothing."""
  first = "[x[1] for x in transcript(of='prompt') if x[5] == 'count'][0]"
  asked = "[x[1] for x in transcript(of='prompt') if x[5] == 'why?'][0]"
  words = ("await wait(600, __name__)", f"cancel({asked})", f"cancel({first})", "close('because')", "close(2)")
  _, log, root = born(*words, auto=False)
  one = engine.prompt(int, "count", on=root)
  await settle()
  why = engine.prompt(str, "why?", on=one)
  assert (await why, await one) == ("because", 2)
  refused = [str(engine.peek(a[1])) for a in said(log, "rung") if isinstance(engine.peek(a[1]), Refused)]
  assert refused == [f"{why} waits for your answer", f"{one} waits for your answer"]
  assert said(log, "cancel") == []

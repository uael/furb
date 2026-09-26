"""close, which ends an act from outside with the value it is done with."""

from asyncio import CancelledError

import pytest

from conftest import born, heads, said, settle
from furb import engine
from furb.engine import OPERATOR, Refused


async def test_an_act_ended_from_outside_by_its_name_with_a_value() -> None:
  """An act ended from outside, by its name, with a value: it is done with it, and it ends what it made, since a close is a cancel that carries what the act it names is done with."""
  _, log, root = born("x = bash('slow')\nclose((await x).code)", auto=False)
  act = engine.prompt(int, "go", on=root)
  await settle()
  step, command = said(log, "rung")[0][1], said(log, "bash")[0][1]
  engine.close(21, act)
  await settle()
  assert (await act) == 21
  assert isinstance(engine.peek(step), CancelledError)
  assert engine.peek(command) is None


async def test_the_close_of_the_operator_enters_the_record_as_a_fact_of_its_own() -> None:
  """The close of the operator enters the record as a fact of its own."""
  sand, _, root = born()
  act = engine.prompt(int, "how many?", to=OPERATOR, on=root)
  await settle()
  engine.close(21, act)
  await settle()
  kept = [fact for fact, *_ in sand.record if fact[0] == "close"]
  assert [(one[1], one[2], one[3]) for one in kept] == [(act, OPERATOR, 21)]


async def test_a_close_ends_the_rung_of_a_prompt_at_its_next_await() -> None:
  """A close ends the rung of a prompt at its next await."""
  _, log, root = born("x = bash('slow')\nclose((await x).code)", auto=False)
  act = engine.prompt(int, "go", on=root)
  await settle()
  step = said(log, "rung")[0][1]
  engine.close(21, act)
  await settle()
  assert (await act) == 21 and isinstance(engine.peek(step), CancelledError)


async def test_the_operator_closes_a_prompt_of_shape_none_with_none() -> None:
  """The operator closes a prompt of shape None with None."""
  _, _, root = born()
  act = engine.prompt(None, "look at this", to=OPERATOR, on=root)
  await settle()
  engine.close(None, act)
  await settle()
  assert engine.peek(act, ...) is not ... and (await act) is None


async def test_the_close_of_the_operator_delivers_to_the_act_of_the_rung_whenever_the_close_comes() -> None:
  """The close of the operator delivers to the act of the rung whenever the close comes."""
  _, log, root = born("p = prompt(int, 'how many?', to='operator')\nclose(await p)")
  act = engine.prompt(int, "ask them", on=root)
  await settle()
  theirs = said(log, "prompt")[-1][1]
  assert engine.peek(act, ...) is ...
  engine.close(21, theirs)
  await settle()
  assert (await act) == 21


async def test_the_operator_closes_a_pending_prompt_of_any_actor() -> None:
  """The operator closes a pending prompt of any actor."""
  _, log, root = born()
  act = engine.prompt(int, "count", to="m/low", on=root)
  await settle()
  assert engine.peek(act, ...) is ... and said(log, "reply")
  engine.close(21, act)
  await settle()
  assert (await act) == 21


async def test_a_rung_closes_a_pending_prompt_of_any_actor() -> None:
  """A rung closes a pending prompt of any actor."""
  sand, _, root = born()
  two = engine.chain("two")
  waiting = engine.prompt(int, "count", to="n/low", on=two)
  await settle()
  sand.script[root] = [f"close(21, {waiting!r})\nclose(1)"]
  act = engine.prompt(int, "close it", on=root)
  await settle()
  assert ((await waiting), (await act)) == (21, 1)


async def test_close_is_given_the_result_of_a_pending_act_and_the_id_of_that_act() -> None:
  """close is given the result of a pending act, and the id of that act when it is not the prompt of the running word."""
  _, _, root = born(auto=False)
  act = engine.bash("slow", on=root)
  await settle()
  engine.close("done with it", act)
  await settle()
  assert (await act) == "done with it"


async def test_an_exception_closes_a_prompt_with_that_exception() -> None:
  """An exception closes a prompt with that exception."""
  _, _, root = born()
  act = engine.prompt(int, "how many?", to=OPERATOR, on=root)
  await settle()
  engine.close(ValueError("boom"), act)
  await settle()
  got = engine.peek(act)
  assert isinstance(got, ValueError) and str(got) == "boom"


async def test_the_close_of_the_operator_stands_in_the_transcript_with_the_name_of_the_operator() -> None:
  """The close of the operator stands in the transcript with the name of the operator."""
  _, _, root = born()
  act = engine.prompt(int, "how many?", to=OPERATOR, on=root)
  await settle()
  engine.close(21, act)
  await settle()
  held = engine.transcript(root)
  assert [one[2] for one in held if one[0] == "close"] == [OPERATOR]
  assert heads(engine.turns(on=root))[2:] == [f"#{act} how many?", f"#{act} closed 21"]


async def test_a_prompt_completes_with_the_exception_that_the_word_of_the_prompt_gave_to_close() -> None:
  """A prompt completes with the exception that the word of the prompt gave to close."""
  sand, _, root = born()
  waiting = engine.prompt(int, "count", to=OPERATOR, on=root)
  await settle()
  sand.script[root] = [f"close(ValueError('boom'), {waiting!r})\nclose(1)"]
  act = engine.prompt(int, "close it", on=root)
  await settle()
  assert (await act) == 1 and isinstance(engine.peek(waiting), ValueError)


async def test_close_is_given_the_value_first() -> None:
  """close is given the value first, since a word that answers its own prompt names no act at all."""
  _, log, root = born(auto=False)
  act = engine.bash("slow", on=root)
  engine.close(21, act)
  word = said(log, "close")[0]
  assert (word[0], word[1], word[2], word[3]) == ("close", act, OPERATOR, 21)
  assert word[4] == [f"#{act} closed 21"]


async def test_a_value_closes_an_act_with_that_value_and_a_prompt_with_a_value_that_has_its_shape() -> None:
  """A value closes an act with that value, and a prompt with a value that has its shape."""
  _, _, root = born()
  waiting = engine.wait(30.0, on=root)
  engine.close("twenty one", waiting)
  assert (await waiting) == "twenty one"
  act = engine.prompt(int, "how many?", to=OPERATOR, on=root)
  await settle()
  with pytest.raises(Refused, match="'twenty one' not int"):
    engine.close("twenty one", act)
  engine.close(21, act)
  assert (await act) == 21


async def test_a_close_that_answers_a_prompt_with_a_value_that_does_not_have_the_shape_of_the_prompt_raises() -> None:
  """A close that answers a prompt with a value that does not have the shape of the prompt raises Refused in the word that said it, so the prompt asks again."""
  _, log, root = born("close('nope')", "close(1)")
  assert await engine.prompt(int, "count", on=root) == 1
  await settle()
  assert len(said(log, "reply")) == 2
  step = said(log, "reply")[0][2]
  raised = [line for line in heads(engine.turns(on=root)) if " raised " in line]
  assert raised == [f"#{step} raised Refused(\"'nope' not int\")"]


async def test_a_close_on_an_act_that_is_over_reaches_nothing() -> None:
  """A close on an act that is over reaches nothing."""
  sand, log, root = born(auto=False)
  act = engine.bash("echo hi", on=root)
  sand.exits(act, 0)
  sand.script[root] = ["y = bash('slow')\nclose(7)"]
  answered = engine.prompt(int, "go", on=root)
  assert await answered == 7
  await settle()
  running = said(log, "bash")[1][1]
  closes, was = said(log, "close"), engine.turns(on=root)
  engine.close(21, act)
  engine.close(9, answered)
  await settle()
  assert (await act).code == 0 and await answered == 7
  assert engine.peek(running) is None
  assert said(log, "close") == closes
  assert engine.turns(on=root) == was


async def test_a_close_said_from_a_word_that_names_no_act_is_over_the_prompt_that_made_the_rung_of_the_word() -> None:
  """A close said from a word that names no act is over the prompt that made the rung of the word, and over the rung itself for a word its caller wrote, which answers no prompt."""
  sand, log, root = born("close(21)")
  act = engine.prompt(int, "count", on=root)
  assert await act == 21
  await settle()
  assert [one[1] for one in said(log, "close")] == [act]
  mine = engine.rung("close(9)", on=root)
  assert await mine == 9
  assert [one[1] for one in said(log, "close")] == [act, mine]
  sand.script[root] = ["close(5, 'rung4')", "close(6)"]
  named = engine.prompt(int, "count", on=root)
  assert await named == 6
  assert [one[1] for one in said(log, "close")][2:] == ["rung4", named] and engine.peek("rung4") == 5


async def test_a_close_said_from_a_word_that_retells_reaches_nothing_and_says_nothing() -> None:
  """A close said from a word that retells reaches nothing and says nothing: it stops the word where it stands, so the rung is done with nothing and answers no prompt."""
  _, log, root = born("x = bash('slow')\nk = 1\nclose(7, x)\nclose(21)\nj = 2", auto=False)
  act = engine.prompt(int, "count", on=root)
  assert await act == 21
  await settle()
  command = said(log, "bash")[0][1]
  twin = engine.chain("twin", source=root)
  await settle(300)
  copy = next(a[1] for a in said(log, "rung") if a[3] == twin)
  assert engine.module(twin)["k"] == 1 and "j" not in engine.module(twin)
  assert engine.peek(copy) is None
  assert [(one[1], one[3]) for one in said(log, "close")] == [(command, 7), (act, 21)]


async def test_a_close_of_the_prompt_of_the_running_word_stops_that_word_where_it_stands() -> None:
  """A close of the prompt of the running word stops that word where it stands, as a raise does, and nothing after the call runs."""
  _, _, root = born("close(21)\nk = 1")
  assert await engine.prompt(int, "count", on=root) == 21
  await settle()
  assert "k" not in engine.module(root)
  await engine.rung("try:\n  close(5)\nexcept Exception:\n  after = 1", on=root)
  assert "after" not in engine.module(root)

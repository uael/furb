"""wake, which ends a pause and gives what waited."""

import pytest

from conftest import (
  Sand,
  born,
  fresh,
  gated,
  heads,
  hushed,
  paragraphs,
  plain,
  prompted,
  ran,
  relived,
  said,
  settle,
  slow,
)
from furb import engine
from furb.engine import OPERATOR, Act, Exit, Refused

TAKEN = ("bash", "wait", "prompt", "reply")
"""The kinds of act that the World takes."""


async def withheld(first: int = 0, second: int = 0) -> tuple[str, Act[Exit], Act[Exit]]:
  """A life whose two commands end, with these codes, while its root is paused, so their exits wait for a wake: the
  root and the two commands."""
  sand, _, root = born(auto=False)
  one, two = engine.bash("one", on=root), engine.bash("two", on=root)
  await settle()
  engine.pause(root)
  sand.exits(one, first)
  sand.exits(two, second)
  await settle()
  return root, one, two


async def test_a_wake_it_ends_the_pause_over_the_same_act_and_what_waited_is_heard() -> None:
  """A wake: it ends the pause over the same act, and what waited is heard."""
  sand, _, root, act = slow()
  await settle()
  engine.pause(act)
  sand.exits(act, 3)
  await settle()
  assert paragraphs(engine.turns(on=root))[-1] == f"#{act} paused by operator"
  engine.wake(act)
  await settle()
  assert (await act).code == 3
  assert paragraphs(engine.turns(on=root))[-2:] == [f"#{act} paused by operator", f"#{act} exited 3"]


async def test_delivery_carries_on_the_rungs_that_await_the_result_on_whatever_chain() -> None:
  """Delivery carries on the rungs that await the result, on whatever chain."""
  sand, _, root = born("x = bash('slow')\nclose(x)", auto=False)
  which = await engine.prompt(str, "start one", on=root)
  two = engine.chain("two")
  sand.script[two] = [f"out = await Act({which!r})\nassert isinstance(out, Exit)\nclose(out.code)"]
  act = engine.prompt(int, "await it", on=two)
  await settle()
  engine.pause(two)
  sand.exits(which, 0)
  await settle()
  assert engine.peek(act, ...) is ...
  engine.wake(two)
  await settle()
  assert (await act) == 0


async def test_a_wake_on_one_act_lifts_a_pause_of_its_chain_for_that_act_alone() -> None:
  """A wake on one act lifts a pause of its chain for that act alone."""
  root, one, _ = await withheld()
  engine.wake(one)
  await settle()
  assert heads(engine.turns(on=root))[-1] == f"#{one} exited 0"
  asked, log, root = born()
  engine.pause(root)
  counting, other = engine.prompt(int, "count", on=root), engine.prompt(int, "wait", on=root)
  await settle()
  assert said(log, "rung") == [] and said(log, "reply") == [] and asked.turns == {}
  engine.wake(counting)
  await settle()
  (step,) = [a[1] for a in said(log, "rung") if not a[4]]
  assert engine.get(step)[2] == counting != other
  assert [a[2] for a in said(log, "reply")] == [step] and [*asked.turns] == ["reply1"]


async def test_wake_is_given_the_id_of_an_act_or_the_id_of_a_chain() -> None:
  """wake is given the id of an act or the id of a chain."""
  root, one, two = await withheld()
  for woken, exited in ((one, one), (root, two)):
    engine.wake(woken)
    await settle()
    assert heads(engine.turns(on=root))[-1] == f"#{exited} exited 0"
    assert (f"#{woken} woke by operator" in heads(engine.turns(on=root))) == (woken == root)


async def test_a_wake_lifts_the_pause_and_delivers_every_held_result() -> None:
  """A wake lifts the pause and delivers every held result."""
  root, one, two = await withheld(1, 2)
  assert heads(engine.turns(on=root))[-1] == f"#{root} paused by operator"
  engine.wake(root)
  await settle()
  assert heads(engine.turns(on=root))[-3:] == [f"#{root} woke by operator", f"#{one} exited 1", f"#{two} exited 2"]


async def test_a_wake_gates_and_runs_a_held_response() -> None:
  """A wake gates and runs a held response."""
  _, log, root, act = await hushed("close(7)")
  assert gated(log) == [] and ran(log) == []
  engine.wake(root)
  await settle()
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  bound = fresh(
    root,
    prompted(act, "int", "count"),
    f"#{step} advance on {act}",
    f"#{root} paused by operator",
    f"#{root} woke by operator",
  )
  assert gated(log) == ["close(7)"] and ran(log) == [bound, "close(7)"] and (await act) == 7


async def test_a_wake_makes_a_prompt_ask_its_model_with_the_transcript_as_it_grew() -> None:
  """A wake makes a prompt ask its model with the transcript as it grew."""
  sand, log, root = born("a = 1", "close(a + 1)")
  act = engine.prompt(int, "count", on=root)
  engine.pause(act)
  await settle()
  engine.say("tell", root, [f"#{root} noted"])
  engine.wake(act)
  await settle()
  asks = [sand.turns[a[1]] for a in said(log, "reply")]
  assert len(asks) == 2 and (await act) == 2
  assert f"#{root} noted" in paragraphs(asks[1])
  assert f"#{root} noted" not in paragraphs(asks[0])


async def test_a_wake_makes_no_reply_twice_and_loses_none() -> None:
  """A wake makes no reply twice and loses none."""
  sand, log, root, act = await hushed("a = 1", "b = a + 1", "close(b + 1)")
  engine.wake(root)
  await settle()
  asks = [a[1] for a in said(log, "reply")]
  assert (await act) == 3 and asks == ["reply1", "reply2", "reply3"]
  assert [a[1] for a in said(sand.calls, "reply")] == asks


async def test_a_wake_that_this_life_says_puts_every_pending_act_it_is_over_on_to_the_outside() -> None:
  """A wake that this life says past the tip, and not one that the journal says again, puts every pending act it is over on to the outside, so the World takes each wait, prompt to the operator and reply of them and closes each command of them as interrupted, and a model reads the transcript as it grew."""
  sand, log, root = born(auto=False)
  command = engine.bash("sleep 9", on=root)
  waited = engine.wait(100.0, on=root)
  shown = engine.prompt(str, "why?", to=OPERATOR, on=root)
  engine.prompt(int, "count", on=root)
  await settle()
  engine.pause(root)
  engine.wake(root)
  await settle()
  (pending,) = [a[1] for a in said(log, "reply")]
  later = Sand()
  again, over = await relived(later, plain(sand.record))
  assert [a[1] for a in said(again, "wake") if a[2] == "journal"] == [root]
  assert [a for a in later.calls if a[0] in TAKEN] == []
  await engine.rung("k = 1", on=over)
  engine.wake(over)
  await settle()
  assert [a[1] for a in later.calls if a[0] in TAKEN] == [command, waited, shown, pending]
  assert isinstance(engine.peek(command), Refused) and "interrupted" in str(engine.peek(command))
  assert "k = 1" in later.turns[pending][-1][1]
  first, _, root = born(auto=False)
  engine.bash("sleep 9", on=root)
  await engine.rung(f"wake({root!r})", on=root)
  engine.clock(on=root)
  then = Sand()
  await relived(then, plain(first.record))
  assert [a for a in then.calls if a[0] in TAKEN] == []


async def test_a_wake_of_a_name_of_no_act_raises_refused_that_names_it() -> None:
  """A wake of a name of no act of the life raises Refused, which names it, and says nothing."""
  _, log, _ = born()
  await settle()
  before = len(log)
  with pytest.raises(Refused, match="no act 'nothing9'"):
    engine.wake("nothing9")
  await settle()
  assert log[before:] == []


async def test_a_wake_that_the_journal_says_again_says_at_once_the_answers_of_the_outside_to_it() -> None:
  """A wake that the journal says again says at once each next entry of the record that answers a pending act it is over, as the outside said them while it heard that wake, before any other fact of the life."""
  sand, _, root = born(auto=False)
  word = "try:\n  await bash('one')\nexcept Refused:\n  pass\nclock() if all(over(a[1]) for a in transcript(of='bash')) else chance()"
  engine.rung(word, on=root)
  await settle()
  two = engine.bash("two", on=root)
  await settle()
  later = Sand()
  _, over = await relived(later, plain(sand.record))
  engine.wake(over)
  await settle()
  assert isinstance(engine.peek(two), Refused) and engine.peek("clock1", None) is not None
  again, _ = await relived(Sand(), plain([*sand.record, *later.record]))
  assert [a[1] for a in said(again, "clock")] == ["clock1"]

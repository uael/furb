"""wake, which ends a pause and gives what waited."""

from conftest import STANDS, Sand, gated, life, paragraphs, plain, ran, relived, said, settle
from furb import engine
from furb.engine import OPERATOR, WORLD


async def test_a_wake_it_ends_the_pause_over_the_same_act_and_what_waited_is_heard() -> None:
  """A wake: it ends the pause over the same act, and what waited is heard."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  act = engine.bash("slow", on=root)
  await settle()
  engine.pause(act)
  engine.send("exited", act, 3, by=WORLD)
  await settle()
  assert act not in engine.outcomes
  engine.wake(act)
  await settle()
  assert (await act).code == 3
  assert paragraphs(engine.turns(on=root))[-3:] == [
    f"#{act} paused",
    f"#{act} woke",
    f"#{act} exited 3\n# {act}/stdout, 0 known",
  ]


async def test_delivery_carries_on_the_rungs_that_await_the_result_on_whatever_chain() -> None:
  """Delivery carries on the rungs that await the result, on whatever chain."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  sand.script[root] = ["x = bash('slow')\nclose(x)"]
  which = await engine.prompt(str, "start one", on=root)
  two = engine.chain("two")
  sand.script[two] = [f"out = await Act({which!r})\nassert isinstance(out, Exit)\nclose(out.code)"]
  act = engine.prompt(int, "await it", on=two)
  await settle()
  engine.pause(two)
  engine.send("exited", which, 0, by=WORLD)
  await settle()
  assert act not in engine.outcomes
  engine.wake(two)
  await settle()
  assert (await act) == 0


async def test_a_wake_on_one_act_lifts_a_pause_of_its_chain_for_that_act_alone() -> None:
  """A wake on one act lifts a pause of its chain for that act alone."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  one, two = engine.bash("one", on=root), engine.bash("two", on=root)
  await settle()
  engine.pause(root)
  engine.send("exited", one, 0, by=WORLD)
  engine.send("exited", two, 0, by=WORLD)
  await settle()
  engine.wake(one)
  await settle()
  assert one in engine.outcomes and two not in engine.outcomes
  asked = Sand(stands=STANDS)
  log, root = life(asked)
  engine.pause(root)
  engine.prompt(int, "count", on=root)
  await settle()
  (step,) = [a[1] for a in said(log, "rung") if not a[4]]
  assert said(log, "ask") == []
  engine.wake(step)
  await settle()
  assert [a[1] for a in said(log, "ask")] == [step]


async def test_wake_is_given_the_id_of_an_act_or_the_id_of_a_chain() -> None:
  """wake is given the id of an act or the id of a chain."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  one, two = engine.bash("one", on=root), engine.bash("two", on=root)
  await settle()
  engine.pause(root)
  engine.send("exited", one, 0, by=WORLD)
  engine.send("exited", two, 0, by=WORLD)
  await settle()
  engine.wake(one)
  await settle()
  assert one in engine.outcomes and two not in engine.outcomes
  engine.wake(root)
  await settle()
  assert two in engine.outcomes


async def test_a_wake_lifts_the_pause_and_delivers_every_held_result() -> None:
  """A wake lifts the pause and delivers every held result."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  one, two = engine.bash("one", on=root), engine.bash("two", on=root)
  await settle()
  engine.pause(root)
  engine.send("exited", one, 1, by=WORLD)
  engine.send("exited", two, 2, by=WORLD)
  await settle()
  assert one not in engine.outcomes and two not in engine.outcomes
  engine.wake(root)
  await settle()
  assert ((await one).code, (await two).code) == (1, 2)


async def test_a_wake_gates_and_runs_a_held_response() -> None:
  """A wake gates and runs a held response."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(7)"]
  act = engine.prompt(int, "count", on=root)
  engine.pause(root)
  await settle()
  bound = f"{root}: Act[object] = Act({root!r})\n{act}: Act[int] = Act({act!r})"
  assert gated(log) == [] and ran(log) == [bound]
  engine.wake(root)
  await settle()
  assert gated(log) == ["close(7)"] and ran(log) == [bound, "close(7)"] and (await act) == 7


async def test_a_wake_makes_a_prompt_ask_with_the_transcript_as_it_grew() -> None:
  """A wake makes a prompt ask with the transcript as it grew."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["a = 1", "close(a + 1)"]
  act = engine.prompt(int, "count", on=root)
  engine.pause(act)
  await settle()
  engine.send("tell", root, [f"#{root} noted"])
  engine.wake(act)
  await settle()
  asks = said(log, "ask")
  assert len(asks) == 2 and (await act) == 2
  assert f"#{root} noted" in paragraphs(asks[1][5])
  assert f"#{root} noted" not in paragraphs(asks[0][5])


async def test_a_wake_makes_no_ask_twice_and_loses_none() -> None:
  """A wake makes no ask twice and loses none."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["a = 1", "b = a + 1", "close(b + 1)"]
  act = engine.prompt(int, "count", on=root)
  engine.pause(root)
  await settle()
  engine.wake(root)
  await settle()
  asks = said(log, "ask")
  assert (await act) == 3 and len(asks) == 3
  assert len({a[1] for a in asks}) == 3


async def test_a_wake_that_this_life_says_starts_the_pending_acts_it_is_over() -> None:
  """A wake that this life says, and not one that the record says again, starts the pending acts it is over: the World starts each command, wait and prompt to the operator of them, and the chain asks for its pending rung with the transcript as it grew."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  command = engine.bash("sleep 9", on=root)
  waited = engine.wait(100.0, on=root)
  shown = engine.prompt(str, "why?", to=OPERATOR, on=root)
  engine.prompt(int, "count", on=root)
  await settle()
  engine.pause(root)
  engine.wake(root)
  await settle()
  pending = said(log, "ask")[0][1]
  later = Sand(stands=STANDS)
  again, over = await relived(later, plain(sand.record))
  assert [a[1] for a in said(again, "wake") if a[2] == "record"] == [root]
  assert said(later.calls, "start") == [] and said(later.calls, "ask") == []
  await engine.rung("k = 1", on=over)
  engine.wake(over)
  await settle()
  assert [a[1] for a in said(later.calls, "start")] == [command, waited, shown]
  asks = said(later.calls, "ask")
  assert [a[1] for a in asks] == [pending] and "k = 1" in asks[0][5][-1][1]

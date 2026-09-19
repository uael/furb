"""wake, which ends a pause and gives what waited."""

from conftest import STANDS, Py, Sand, life, said, settle, tags
from furb import engine
from furb.engine import WORLD


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
  assert tags(engine.turns(on=root), "woke") == [("woke", [("over", act)], None)]


async def test_delivery_carries_on_the_rungs_that_await_the_result_on_whatever_chain() -> None:
  """Delivery carries on the rungs that await the result, on whatever chain."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  sand.script[root] = ["x = bash('slow')\nclose(x)"]
  which = await engine.prompt(str, "start one", on=root)
  two = engine.chain("two")
  sand.script[two] = [f"close((await Act({which!r})).code)"]
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
  sand, py = Sand(stands=STANDS), Py()
  log, root = life(sand, kernel=py)
  sand.script[root] = ["close(7)"]
  act = engine.prompt(int, "count", on=root)
  engine.pause(root)
  await settle()
  assert py.gated == [] and said(log, "run") == []
  engine.wake(root)
  await settle()
  assert py.gated == ["close(7)"] and py.ran == ["close(7)"] and (await act) == 7


async def test_a_wake_makes_a_prompt_ask_with_the_transcript_as_it_grew() -> None:
  """A wake makes a prompt ask with the transcript as it grew."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["a = 1", "close(a + 1)"]
  act = engine.prompt(int, "count", on=root)
  engine.pause(act)
  await settle()
  engine.send("tell", root, [("noted", [], None)])
  engine.wake(act)
  await settle()
  asks = said(log, "ask")
  assert len(asks) == 2 and (await act) == 2
  assert "noted" in [name for name, *_ in tags(asks[1][5])]
  assert "noted" not in [name for name, *_ in tags(asks[0][5])]


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

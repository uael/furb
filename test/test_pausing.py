"""pausing, the kind that holds what an act hears while a pause over it stands."""

from asyncio import CancelledError

from conftest import STANDS, Sand, life, said, settle
from furb import engine
from furb.engine import WORLD


async def test_an_act_is_paused_while_the_last_control_in_record_order_that_is_over_it_is_a_pause() -> None:
  """An act is paused while the last control in record order that is over it is a pause."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  sand.script[root] = ["x = bash('slow')\nclose((await x).code)"]
  act = engine.prompt(int, "go", on=root)
  await settle()
  command = said(log, "bash")[0][1]
  engine.pause(root)
  engine.wake(root)
  engine.pause(root)
  engine.send("exited", command, 0, by=WORLD)
  await settle()
  assert act not in engine.outcomes
  engine.wake(root)
  await settle()
  assert (await act) == 0


async def test_a_pause_holds_delivery_a_result_that_arrives_enters_the_record_and_waits() -> None:
  """A pause holds delivery: a result that arrives enters the record and waits."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  sand.script[root] = ["x = bash('slow')\nclose((await x).code)"]
  act = engine.prompt(int, "go", on=root)
  await settle()
  command = said(log, "bash")[0][1]
  engine.pause(root)
  engine.send("exited", command, 0, by=WORLD)
  await settle()
  kept = [fact for _, fact, *_ in sand.record if fact[0] == "exited"]
  assert [one[1] for one in kept] == [command] and act not in engine.outcomes
  engine.wake(root)
  await settle()
  assert (await act) == 0


async def test_a_paused_prompt_stops_at_its_next_boundary_with_its_loop_where_it_stood() -> None:
  """A paused prompt stops at its next boundary, with its loop where it stood."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["a = 1", "close(a + 1)"]
  act = engine.prompt(int, "count", on=root)
  engine.pause(act)
  await settle()
  assert len(said(log, "ask")) == 1 and said(log, "run") == [] and act not in engine.outcomes
  engine.wake(act)
  await settle()
  assert (await act) == 2 and len(said(log, "ask")) == 2 and len(said(log, "run")) == 2


async def test_the_engine_holds_the_response_of_an_ask_that_returns_on_a_paused_chain() -> None:
  """The engine holds the response of an ask that returns on a paused chain."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(7)"]
  act = engine.prompt(int, "count", on=root)
  engine.pause(root)
  await settle()
  assert len(said(log, "answer")) == 1 and said(log, "run") == [] and act not in engine.outcomes
  engine.wake(root)
  await settle()
  assert (await act) == 7 and len(said(log, "run")) == 1


async def test_a_rung_carries_on_only_while_its_own_chain_is_not_paused() -> None:
  """A rung carries on only while its own chain is not paused."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  sand.script[root] = ["x = bash('slow')\nclose((await x).code)"]
  act = engine.prompt(int, "go", on=root)
  await settle()
  command = said(log, "bash")[0]
  engine.pause(root)
  engine.send("exited", command[1], 0, by=WORLD)
  await settle()
  assert said(log, "sent") == []
  engine.wake(root)
  await settle()
  assert [one[1] for one in said(log, "sent")] == [command[2]] and (await act) == 0


async def test_a_control_from_outside_reaches_a_paused_act_at_once() -> None:
  """A control from outside reaches a paused act at once, where what the words of the act say waits for the wake."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  act = engine.bash("slow", on=root)
  await settle()
  engine.pause(act)
  engine.send("exited", act, 0, by=WORLD)
  await settle()
  assert act not in engine.outcomes
  engine.cancel(act)
  await settle()
  assert isinstance(engine.outcomes[act], CancelledError)

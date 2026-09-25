"""pausing, the kind that holds what an act hears while a pause over it stands."""

from asyncio import CancelledError
from collections.abc import Generator

from conftest import STANDS, Sand, dones, life, said, settle, sown
from furb import engine


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
  sand.exits(command, 0)
  await settle()
  assert engine.peek(act, ...) is ...
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
  sand.exits(command, 0)
  await settle()
  kept = [fact for fact, *_ in sand.record if fact[:2] == ("done", command)]
  assert [one[1] for one in kept] == [command] and engine.peek(act, ...) is ...
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
  (first,) = [a[1] for a in said(log, "rung") if a[2] == act]
  replies = [a[1] for a in said(log, "done") if a[1].startswith("reply")]
  assert [a[2] for a in said(log, "reply")] == [first] and replies == ["reply1"]
  assert [a for a in said(log, "ready") if a[1] == first] == [] and engine.peek(act, ...) is ...
  engine.wake(act)
  await settle()
  assert (await act) == 2
  steps = [a[1] for a in said(log, "rung") if a[2] == act]
  assert [a[2] for a in said(log, "reply")] == steps == [first, steps[1]]
  assert [(a[4], a[5]) for a in said(log, "run") if a[4] in steps] == [(first, "a = 1"), (steps[1], "close(a + 1)")]


async def test_the_engine_holds_the_response_of_a_reply_that_returns_on_a_paused_chain() -> None:
  """The engine holds the response of a reply that returns on a paused chain."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(7)"]
  act = engine.prompt(int, "count", on=root)
  engine.pause(root)
  await settle()
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  assert [a[1] for a in said(log, "done") if a[1].startswith("reply")] == ["reply1"] and engine.peek(act, ...) is ...
  assert [a for a in said(log, "ready") if a[1] == step] == [] and [a for a in said(log, "run") if a[4] == step] == []
  engine.wake(root)
  await settle()
  assert (await act) == 7 and [a[5] for a in said(log, "run") if a[4] == step] == ["close(7)"]


async def test_a_rung_carries_on_only_while_its_own_chain_is_not_paused() -> None:
  """A rung carries on only while its own chain is not paused."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  sand.script[root] = ["x = bash('slow')\nclose((await x).code)"]
  act = engine.prompt(int, "go", on=root)
  await settle()
  command = said(log, "bash")[0]
  engine.pause(root)
  sand.exits(command[1], 0)
  await settle()
  assert dones(log, "wants") == []
  engine.wake(root)
  await settle()
  assert [one[2] for one in dones(log, "wants")] == [command[2]] and (await act) == 0


async def test_a_control_from_outside_reaches_a_paused_act_at_once() -> None:
  """A control from outside reaches a paused act at once, where what the words of the act say waits for the wake."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  act = engine.bash("slow", on=root)
  await settle()
  engine.pause(act)
  engine.cancel(act)
  await settle()
  assert isinstance(engine.peek(act), CancelledError)
  heard: list[tuple] = []

  def listening(id: str) -> Generator[None, tuple]:
    engine.say("started", id)
    while True:
      heard.append((yield))

  probe = engine.act("probe", root, engine.pausing(listening))
  engine.pause(probe)
  await settle()
  before = len(heard)
  other = engine.bash("other", on=root)
  engine.close(0, other)
  await settle()
  assert [a for a in heard[before:] if not engine.question(a)] == []
  engine.wake(probe)
  await settle()
  assert [a[:2] for a in heard[before:] if a[0] in ("bash", "close")] == [("bash", other), ("close", other)]


async def test_a_paused_ear_hears_at_once_an_act_put_to_it() -> None:
  """A paused ear is offered a question at once, since an offer is no delivery, and hears every other fact at the wake."""
  sand = sown()
  _, root = life(sand)
  heard: list[tuple] = []

  def listening(id: str) -> Generator[None, tuple]:
    engine.say("started", id)
    while True:
      heard.append((yield))

  probe = engine.act("probe", root, engine.pausing(listening))
  engine.pause(probe)
  before = len(heard)
  read = engine.act("read", root, None, "a.txt")
  assert heard[before:] == [engine.get(read)]
  engine.wake(probe)
  await settle()
  assert [a for a in heard[before:] if a[1] == read] == [engine.get(read), ("done", read, "world", engine.peek(read))]

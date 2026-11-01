"""controlled, the ear of an act that the controls over that act reach."""

from asyncio import CancelledError
from collections.abc import Generator

from conftest import born, dones, hushed, said, settle, slow, stalled
from furb import engine
from furb.engine import Text


async def test_an_act_is_paused_while_the_last_control_in_record_order_that_is_over_it_is_a_pause() -> None:
  """An act is paused while the last control in record order that is over it is a pause."""
  sand, _, root, act, _, command = await stalled()
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
  sand, _, root, act, _, command = await stalled()
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
  _, log, root = born("a = 1", "close(a + 1)")
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
  _, log, root, act = await hushed("close(7)")
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  assert [a[1] for a in said(log, "done") if a[1].startswith("reply")] == ["reply1"] and engine.peek(act, ...) is ...
  assert [a for a in said(log, "ready") if a[1] == step] == [] and [a for a in said(log, "run") if a[4] == step] == []
  engine.wake(root)
  await settle()
  assert (await act) == 7 and [a[5] for a in said(log, "run") if a[4] == step] == ["close(7)"]


async def test_a_rung_carries_on_only_while_its_own_chain_is_not_paused() -> None:
  """A rung carries on only while its own chain is not paused."""
  sand, log, root, act, _, _ = await stalled()
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
  _, _, root, act = slow()
  await settle()
  engine.pause(act)
  engine.cancel(act)
  await settle()
  assert isinstance(engine.peek(act), CancelledError)
  heard: list[tuple] = []

  def listening(id: str) -> Generator[tuple | None, tuple]:
    yield "started", id
    while True:
      heard.append((yield))

  probe = engine.act("probe", root, engine.controlled(listening))
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
  """A paused ear hears at once an act put to it, and every other fact at the wake."""
  _, _, root = born()
  heard: list[tuple] = []

  def listening(id: str) -> Generator[tuple | None, tuple]:
    yield "started", id
    while True:
      heard.append((yield))

  probe = engine.act("probe", root, engine.controlled(listening))
  engine.pause(probe)
  before = len(heard)
  read = engine.act("read", root, None, "a.txt")
  assert heard[before:] == [engine.get(read)]
  engine.wake(probe)
  await settle()
  assert [a for a in heard[before:] if a[1] == read] == [engine.get(read), ("done", read, "world", engine.peek(read))]


async def test_a_close_from_outside_still_ends_what_a_pause_is_over() -> None:
  """A close from outside still ends what a pause is over."""
  _, _, root = born()
  act = engine.prompt(int, "how many?", on=root)
  await settle()
  engine.pause(act)
  engine.close(21, act)
  await settle()
  assert engine.peek(act, ...) is not ... and (await act) == 21


async def test_an_ear_that_is_not_lasting_is_over_as_it_says_a_done_of_its_own() -> None:
  """An ear that is not lasting is over as it says a done of its own, so it never hears that done and says nothing after it."""
  _, log, root = born()
  act = engine.rung("close(21)", on=root)
  await act
  ends = [one for one in said(log, "done") if one[1] == act]
  assert [(one[2], one[3]) for one in ends] == [(act, 21)]
  mark = len(log)
  engine.cancel(act)
  await settle()
  assert [one for one in log[mark:] if one[2] == act] == []
  assert engine.peek(act) == 21


async def test_a_lasting_ear_lives_past_the_done_of_its_act() -> None:
  """A lasting ear lives past the done of its act until it returns, as the ear of a command does to hear that done, whoever said it, and to tell its exit, and a cancel or a close over its act ends the act all the same."""
  _, _, root = born()

  def serving(id: str) -> Generator[tuple | None, tuple]:
    yield "started", id
    while True:
      match (yield):
        case ("read", qid, *_, path) if path == f"{id}/door":
          yield "done", qid, Text(path, "open")
        case ("read", qid, *_, path) if path == f"{id}/end":
          yield "done", qid, Text(path, "ended")
          return

  door = engine.act("door", root, engine.controlled(serving, True))
  assert engine.read(f"{door}/door", on=root).content == "open"
  engine.cancel(door)
  await settle()
  assert isinstance(engine.peek(door), CancelledError)
  assert engine.read(f"{door}/door", on=root).content == "open"
  assert engine.read(f"{door}/end", on=root).content == "ended"
  engine.drive(engine.idle(door), door)


async def test_what_an_act_a_control_is_over_is_done_with() -> None:
  """What an act a control is over is done with: the value a close carries for the act it names, and a CancelledError for every other."""
  _, _, _, act, step, _ = await stalled()
  engine.close(21, act)
  await settle()
  assert engine.peek(act) == 21 and isinstance(engine.peek(step), CancelledError)


async def test_a_controlled_ear_waits_for_what_the_ear_that_it_carries_waits_for() -> None:
  """A controlled ear waits for what the ear that it carries waits for."""
  _, _, root = born()
  heard: list[tuple] = []

  def asking(id: str) -> Generator[tuple | set[str], tuple]:
    yield "started", id
    while True:
      heard.append((yield {id, "clock"}))

  probe = engine.act("probe", root, engine.controlled(asking))
  engine.read("a.txt", on=root)
  engine.clock(on=root)
  await settle()
  assert {engine.get(a[1])[0] for a in heard} == {"probe", "clock"}
  assert probe == "probe1"

"""Started, what an ear says to take an act whose done comes later."""

from conftest import STANDS, Sand, life, plain, relived, said, settle
from furb import engine
from furb.engine import WORLD, Act, Exit


async def test_what_an_ear_says_to_take_an_act_whose_done_comes_later() -> None:
  """What an ear says to take an act whose done comes later, and which no ear after it hears."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  act = engine.bash("slow", on=root)
  assert said(log, "started")[-1] == ("started", act, WORLD) and engine.peek(act, ...) is ...
  step = engine.rung("k = 1", on=root)
  assert ("started", step, step) in log and said(sand.calls, "rung") == []


async def test_the_started_names_the_act_and_says_no_more_of_it() -> None:
  """The started names the act and says no more of it, since the ear heard the act itself and nothing is told twice."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.bash("echo hi", on=root)
  assert (await act).code == 0
  assert [a for a in said(log, "started") if a[1] == act] == [("started", act, WORLD)]


async def test_the_journal_keeps_a_started_of_the_outside() -> None:
  """The journal keeps a started of the outside, so a later life holds the act from the outside and says no started for it, since only the outside runs it: the act is done where the record holds its done, and pending when the record holds none."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  step = engine.rung("x = bash('sleep 9')", on=root)
  await settle()
  command = said(log, "bash")[0][1]
  assert (("started", command, WORLD),) in sand.record
  later = Sand(stands=STANDS)
  heard, over = await relived(later, plain(sand.record))
  assert [a for a in said(heard, "started") if a[1] == command] == []
  assert said(later.calls, "bash") == [] and engine.peek(command, ...) is ... and engine.peek(step) is None
  engine.wake(over)
  await settle()
  assert [a[1] for a in said(later.calls, "bash")] == [command] and (await Act[Exit](command)).code == 0
  assert [a for a in said(heard, "started") if a[1] == command] == [("started", command, WORLD)]
  ended = Sand(stands=STANDS)
  _, root = life(ended)
  one = engine.bash("echo hi", on=root)
  assert (await one).code == 0
  again, _ = await relived(Sand(stands=STANDS), plain(ended.record))
  assert [(a[0], a[2]) for a in again if a[1] == one and a[0] in ("started", "done")] == [("done", "journal")]

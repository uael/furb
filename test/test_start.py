"""Start, the fact that has the World do an act."""

from conftest import STANDS, Sand, life, plain, relived, said, settle
from furb import engine


async def test_an_act_the_world_does() -> None:
  """An act the World does: it asks the record what it holds of the act, says so at its birth, and the World does it then and not at its open, which everything hears."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.bash("echo hi", on=root)
  assert (await act).code == 0
  await settle()
  _, command, *_ = said(log, "bash")[0]
  _, held = engine.ask("transcript", root, root)
  assert isinstance(held, list)
  assert [a[4] for a in said(held, "holds")] == [command]
  names = [a[0] for a in held]
  assert names.index("bash") < names.index("start")
  assert [a[1] for a in said(sand.calls, "start")] == [command]
  assert [a[1] for a in said(log, "start")] == [command]


async def test_the_start_names_the_act_and_says_no_more_of_it() -> None:
  """The start names the act and says no more of it, since the World heard the act itself and nothing is told twice."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.bash("echo hi", on=root)
  assert (await act).code == 0
  made = said(log, "bash")[0]
  assert made[3] == root
  assert said(log, "start") == [("start", act, act)]


async def test_it_says_nothing_when_the_record_boot_was_given_holds_the_act() -> None:
  """It says nothing when the record boot was given holds the act, since what the World did once it does no more."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose((await x).code)"]
  assert await engine.prompt(int, "run it", on=root) == 0
  await settle()
  later = Sand(stands=STANDS)
  again, over = await relived(later, plain(sand.record))
  assert over == root and said(again, "bash") != []
  assert said(again, "start") == [] and said(later.calls, "start") == []

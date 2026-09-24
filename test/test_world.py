"""WORLD, the name that boot takes the World under."""

from conftest import STANDS, Sand, life, said, settle
from furb import engine
from furb.engine import WORLD


async def test_world_is_the_name_that_boot_takes_the_world_under() -> None:
  """WORLD is the name that boot takes the World under, and that the World says its facts by."""
  assert WORLD == "world"
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose((await x).code)"]
  assert await engine.prompt(int, "run it", on=root) == 0
  await settle()
  _, command, *_ = said(log, "bash")[0]
  assert said(log, "answer")[0][2] == WORLD
  assert [a[2] for a in log if a[0] in ("out", "exited")] == [WORLD, WORLD]
  _, standing, *_ = said(log, "stand")[0]
  assert [a[2] for a in said(log, "done") if a[1] == standing] == [WORLD]
  assert [a[1] for a in said(sand.calls, "start")] == [command]

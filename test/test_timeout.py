"""TIMEOUT, the timeout of a command that says none."""

from conftest import STANDS, Sand, life, said, settle
from furb import engine
from furb.engine import TIMEOUT


async def test_timeout_is_the_timeout_in_seconds_of_a_command_that_does_not_say_one() -> None:
  """TIMEOUT is the timeout, in seconds, of a command that does not say one."""
  assert TIMEOUT == 600.0
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  quiet = engine.bash("slow", on=root)
  short = engine.bash("shorter", timeout=0.05, on=root)
  assert [timeout for *_, timeout in said(log, "bash")] == [600.0, 0.05]
  await settle()
  assert quiet not in engine.outcomes and short not in engine.outcomes
  assert (await short).code is None
  engine.cancel(quiet)

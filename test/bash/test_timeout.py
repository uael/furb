"""TIMEOUT, the timeout of a command that says none."""

from conftest import BASH, STANDS, Sand, exited, life, said, settle, verb
from furb import engine
from furb.engine import Act


async def test_timeout_is_the_timeout_in_seconds_of_a_command_that_does_not_say_one() -> None:
  """TIMEOUT is the timeout, in seconds, of a command that does not say one."""
  sand = Sand(stands=STANDS, auto=False, words=BASH)
  log, root = life(sand)
  assert await engine.rung("close(TIMEOUT)", on=root) == 600.0
  quiet = verb("bash", root)("slow")
  short = verb("bash", root)("shorter", timeout=0.05)
  assert isinstance(quiet, Act) and isinstance(short, Act)
  assert [timeout for *_, timeout in said(log, "bash")] == [600.0, 0.05]
  await settle()
  assert quiet not in engine.outcomes and short not in engine.outcomes
  assert exited(await short)[0] is None
  engine.cancel(quiet)

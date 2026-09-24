"""Bash, the act the World hears as a command."""

from conftest import BASH, STANDS, Bound, Sand, exited, life, made, paragraphs, said, verb
from furb import engine
from furb.engine import OPERATOR, Act


async def test_a_bash_carries_the_command_the_fed_flag_and_the_timeout() -> None:
  """A bash carries the command, the fed flag and the timeout, and no show and no working directory."""
  sand = Sand(stands=STANDS, words=BASH)
  log, root = life(sand)
  verb("cd", root)("/deep")
  one = verb("bash", root)("echo hi", fed=True, show=Bound(root).HEAD, show_err=made(root, "span", -9, -1), timeout=5.0)
  assert isinstance(one, Act)
  got = exited(await one)
  assert said(log, "bash") == [("bash", one, OPERATOR, root, "echo hi", True, 5.0)]
  assert got[1][1] == "ran echo hi\n"
  assert paragraphs(engine.turns(on=root))[-1] == (
    f"#{one} exited 0\n# {one}/stdout, 0 known\n# 1 ran echo hi\n# {one}/stderr, 0 known"
  )

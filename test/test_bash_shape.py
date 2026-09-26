"""Bash, the act the World hears as a command."""

from conftest import Sand, life, paragraphs, said
from furb import engine
from furb.engine import HEAD, OPERATOR, TAIL


async def test_a_bash_carries_the_command_the_fed_flag_and_the_timeout() -> None:
  """A bash carries the command, the fed flag and the timeout, and no show and no working directory."""
  sand = Sand()
  log, root = life(sand)
  engine.cd("/deep", on=root)
  one = engine.bash("echo hi", fed=True, show=HEAD, show_err=TAIL, timeout=5.0, on=root)
  got = await one
  assert said(log, "bash") == [("bash", one, OPERATOR, root, "echo hi", True, 5.0)]
  assert got.stdout.content == "ran echo hi\n"
  assert paragraphs(engine.turns(on=root))[-1] == (
    f"#{one} exited 0\n# {one}/stdout, 0 known\n# 1 ran echo hi\n# {one}/stderr, 0 known"
  )

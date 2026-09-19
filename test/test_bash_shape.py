"""Bash, the act the World hears as a command."""

import pytest

from conftest import STANDS, Sand, attr, life, said, shown, tags
from furb import engine
from furb.engine import HEAD, OPERATOR, TAIL


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_bash_carries_the_command_the_fed_flag_and_the_timeout() -> None:
  """A bash carries the command, the fed flag and the timeout, and no show and no working directory."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  engine.cd("/deep", on=root)
  got = await engine.bash("echo hi", fed=True, show=HEAD, show_err=TAIL, timeout=5.0, on=root)
  word = said(log, "bash")[0]
  assert word == ("bash", word[1], OPERATOR, root, "echo hi", True, 5.0)
  assert got.stdout.content == "ran echo hi\n"
  closed = tags(engine.turns(on=root), "closed")[-1]
  assert [attr(one, "path") for one in shown(closed)] == [f"{word[1]}/stdout", f"{word[1]}/stderr"]

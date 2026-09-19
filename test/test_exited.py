"""Exited, the fact that says the code of a command."""

from conftest import STANDS, Sand, life, said
from furb import engine
from furb.engine import TAIL, WORLD, Exit, Text


async def test_exited_says_the_code_and_the_command_is_done_with_its_exit() -> None:
  """exited says the code, and the command is done with its Exit of that code and the streams it kept."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  act = engine.bash("run", show_err=TAIL, on=root)
  engine.send("out", act, "out\n", "stdout", by=WORLD)
  engine.send("out", act, "err\n", "stderr", by=WORLD)
  engine.send("exited", act, 7, by=WORLD)
  word = said(log, "exited")[0]
  assert (word[1], word[3], word[2]) == (act, 7, WORLD)
  assert await act == Exit(7, Text(f"{act}/stdout", "out\n"), Text(f"{act}/stderr", "err\n"))

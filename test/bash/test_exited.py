"""Exited, the fact that says the code of a command."""

from conftest import BASH, STANDS, Sand, exited, life, made, said, verb
from furb import engine
from furb.engine import WORLD, Act


async def test_exited_says_the_code_and_the_command_is_done_with_its_exit() -> None:
  """exited says the code, and the command is done with its Exit of that code and the streams it kept."""
  sand = Sand(stands=STANDS, auto=False, words=BASH)
  log, root = life(sand)
  act = verb("bash", root)("run", show_err=made(root, "span", -9, -1))
  assert isinstance(act, Act)
  engine.send("out", act, "out\n", "stdout", by=WORLD)
  engine.send("out", act, "err\n", "stderr", by=WORLD)
  engine.send("exited", act, 7, by=WORLD)
  word = said(log, "exited")[0]
  assert (word[1], word[3], word[2]) == (act, 7, WORLD)
  assert exited(await act) == (7, (f"{act}/stdout", "out\n"), (f"{act}/stderr", "err\n"))

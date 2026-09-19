"""Exit, what a command came to."""

import pytest

from conftest import STANDS, Sand, life, said
from furb import engine
from furb.engine import TAIL, WORLD, Exit, Text


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_what_a_command_came_to_its_code_and_each_of_its_streams_as_a_text() -> None:
  """What a command came to: its code, and each of its streams as a text, in the order the command keeps them."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  act = engine.bash("run", show_err=TAIL, on=root)
  engine.send("out", act, "out\n", "stdout", by=WORLD)
  engine.send("out", act, "err\n", "stderr", by=WORLD)
  engine.send("exited", act, 3, by=WORLD)
  got = await act
  assert list(vars(got)) == ["code", "stdout", "stderr"]
  assert got == Exit(3, Text(f"{act}/stdout", "out\n"), Text(f"{act}/stderr", "err\n"))


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_an_exit_is_the_value_that_a_command_completes_with() -> None:
  """An Exit is the value that a command completes with."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  got = await engine.bash("echo hi", on=root)
  command = said(log, "bash")[0][1]
  assert isinstance(got, Exit) and got.code == 0
  assert engine.peek(command, on=root) == got


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_the_streams_of_an_exit_are_its_stdout_and_its_stderr_each_a_text() -> None:
  """The streams of an Exit are its stdout and its stderr, each a Text."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  got = await engine.bash("echo hi", on=root)
  command = said(log, "bash")[0][1]
  assert isinstance(got.stdout, Text) and isinstance(got.stderr, Text)
  assert (got.stdout.path, got.stderr.path) == (f"{command}/stdout", f"{command}/stderr")
  assert (got.stdout.content, got.stderr.content) == ("ran echo hi\n", "")

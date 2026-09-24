"""Exit, what a command came to."""

from conftest import BASH, STANDS, Sand, exited, life, made, said, verb, worded
from furb import engine
from furb.engine import WORLD, Act


async def test_what_a_command_came_to_its_code_and_each_of_its_streams_as_a_text() -> None:
  """What a command came to: its code, and each of its streams as a text, in the order the command keeps them."""
  word = (
    "e = Exit(3, Text('x/stdout', 'out'), Text('x/stderr', 'err'))\nclose([e.code, e.stdout.path, e.stderr.content])"
  )
  assert await worded(word, BASH) == [3, "x/stdout", "err"]
  sand = Sand(stands=STANDS, auto=False, words=BASH)
  _, root = life(sand)
  act = verb("bash", root)("run", show_err=made(root, "span", -9, -1))
  assert isinstance(act, Act)
  engine.send("out", act, "out\n", "stdout", by=WORLD)
  engine.send("out", act, "err\n", "stderr", by=WORLD)
  engine.send("exited", act, 3, by=WORLD)
  assert exited(await act) == (3, (f"{act}/stdout", "out\n"), (f"{act}/stderr", "err\n"))


async def test_an_exit_is_the_value_that_a_command_completes_with() -> None:
  """An Exit is the value that a command completes with."""
  sand = Sand(stands=STANDS, words=BASH)
  log, root = life(sand)
  assert await engine.rung("x = await bash('echo hi')\nclose([type(x) is Exit, x.code])", on=root) == [True, 0]
  command = said(log, "bash")[0][1]
  assert exited(engine.peek(command, on=root))[0] == 0


async def test_the_streams_of_an_exit_are_its_stdout_and_its_stderr_each_a_text() -> None:
  """The streams of an Exit are its stdout and its stderr, each a Text."""
  sand = Sand(stands=STANDS, words=BASH)
  log, root = life(sand)
  word = "x = await bash('echo hi')\nclose([type(x.stdout) is Text, type(x.stderr) is Text])"
  assert await engine.rung(word, on=root) == [True, True]
  command = said(log, "bash")[0][1]
  got = exited(engine.peek(command, on=root))
  assert got[1:] == ((f"{command}/stdout", "ran echo hi\n"), (f"{command}/stderr", ""))

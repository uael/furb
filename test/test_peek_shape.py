"""Peek, the question of what an act came to."""

from conftest import STANDS, Sand, life, said, settle
from furb import engine
from furb.engine import OPERATOR, Exit, Text


async def test_a_peek_is_at_an_act_and_gives_what_the_act_came_to_as_the_record_stands() -> None:
  """A peek is at an act, and gives what the act came to as the record stands."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  got = await engine.bash("echo hi", on=root)
  command = said(log, "bash")[0][1]
  assert got == Exit(0, Text(f"{command}/stdout", "ran echo hi\n"), Text(f"{command}/stderr"))
  word, answer = engine.ask("peek", root, command)
  assert word == ("peek", "peek@operator.3", OPERATOR, root, command) and answer == got
  sand.script[root] = [f"seen = peek({command!r})\nassert isinstance(seen, Exit)\nclose(seen.code)"]
  assert await engine.prompt(int, "look", on=root) == 0
  await settle()
  assert engine.modules[root]["seen"] == got

"""Peek, the question of what an act came to."""

import pytest

from conftest import STANDS, Sand, attr, life, said, settle, tags
from furb import engine
from furb.engine import OPERATOR, Exit


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_peek_is_at_an_act_and_gives_what_the_act_came_to_as_the_record_stands() -> None:
  """A peek is at an act, and gives what the act came to as the record stands."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  got = await engine.bash("echo hi", on=root)
  command = said(log, "bash")[0][1]
  word, answer = engine.ask("peek", root, command)
  match word:
    case ("peek", _, by, on, at):
      asked = (by, on, at)
    case _:
      asked = None
  assert asked == (OPERATOR, root, command) and answer == got
  sand.script[root] = [f"seen = peek({command!r})\nclose(seen.code)"]
  assert await engine.prompt(int, "look", on=root) == 0
  await settle()
  assert isinstance(got, Exit)
  assert [attr(tag, "at") for tag in tags(engine.turns(on=root), "peek")] == [command]

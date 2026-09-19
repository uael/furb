"""started, the kind that has the World do an act the record does not hold."""

from conftest import STANDS, Sand, life, said, settle
from furb import engine
from furb.engine import OPERATOR, WORLD


async def test_an_act_the_world_does_says_start_at_its_birth() -> None:
  """An act the World does says start at its birth, and the facts of the World about it come after."""
  sand = Sand()
  log, root = life(sand)
  act = engine.bash("echo hi", on=root)
  await settle()
  about = [a for a in log if a[1] == act]
  assert [a[0] for a in about[:2]] == ["bash", "start"]
  assert [i for i, a in enumerate(about) if a[2] == WORLD] == [3, 4]


async def test_the_world_starts_a_command_at_its_start_which_the_command_says_at_its_birth() -> None:
  """The World starts a command at its start, which the command says at its birth."""
  sand = Sand()
  log, root = life(sand)
  act = engine.bash("echo hi", on=root)
  assert [a[1] for a in said(sand.calls, "start")] == [act]
  assert said(log, "start")[0][2] == act
  assert (await act).code == 0


async def test_a_prompt_to_a_model_is_the_engines_to_do_so_it_says_no_start() -> None:
  """A prompt to a model is the engine's to do, so it says no start."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  assert said(log, "start") == []
  asking = engine.prompt(int, "how many?", to=OPERATOR, on=root)
  await settle()
  assert [a[1] for a in said(log, "start")] == [asking]
  engine.close(3, asking)
  assert await asking == 3

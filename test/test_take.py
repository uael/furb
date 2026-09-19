"""take, the filter of the file."""

from conftest import STANDS, Sand, life, said, settle, tags
from furb import engine
from furb.engine import take


async def twice(sand: Sand) -> tuple[str, str, str, str, str]:
  """A life of two prompts, each with a rung of its own: the chain, both prompts and both rungs."""
  log, root = life(sand)
  sand.script[root] = ["a = 1\nclose(1)", "b = 2\nclose(2)"]
  first = engine.prompt(int, "first", on=root)
  assert await first == 1
  second = engine.prompt(int, "second", on=root)
  assert await second == 2
  await settle()
  one, two = said(log, "rung")
  return root, first, second, one[1], two[1]


def named(on: str) -> set[str]:
  """Every act that the turns of a chain name."""
  return {value for tag in tags(engine.turns(on=on)) for key, value in tag[1] if key == "id" and isinstance(value, str)}


async def test_take_keeps_the_acts_it_names_and_everything_they_made() -> None:
  """take keeps the acts it names and everything they made."""
  root, first, second, one, two = await twice(Sand(stands=STANDS))
  kept = engine.chain("kept", source=root, filter=take(first))
  await settle(300)
  assert {first, one} <= named(kept)
  assert second not in named(kept) and two not in named(kept)


async def test_take_is_given_ids_and_keeps_the_acts_with_those_ids() -> None:
  """take is given ids and keeps the acts with those ids."""
  root, first, second, one, two = await twice(Sand(stands=STANDS))
  _, held = engine.ask("transcript", root, root)
  assert isinstance(held, list)
  made = [a for a in held if engine.question(a)]
  assert [a[1] for a in take(first)(made)] == [first, one]
  assert [a[1] for a in take(second)(made)] == [second, two]
  assert [a[1] for a in take(first, second)(made)] == [first, one, second, two]


async def test_take_keeps_everything_that_the_acts_with_those_ids_caused() -> None:
  """take keeps everything that the acts with those ids caused."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose(1)"]
  first = engine.prompt(int, "first", on=root)
  assert await first == 1
  await settle()
  _, step, *_ = said(log, "rung")[0]
  _, command, *_ = said(log, "bash")[0]
  kept = engine.chain("kept", source=root, filter=take(first))
  await settle(300)
  assert {first, step, command} <= named(kept)


async def test_take_that_is_not_inside_keeps_every_other_act() -> None:
  """take that is not inside keeps every other act, and drops everything the ones it names made."""
  root, first, second, one, two = await twice(Sand(stands=STANDS))
  narrow = engine.chain("narrow", source=root, filter=take(first, inside=False))
  await settle(300)
  assert {second, two} <= named(narrow)
  assert first not in named(narrow) and one not in named(narrow)

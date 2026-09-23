"""take, the filter of the file."""

from conftest import STANDS, Sand, life, named, said, settle
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
  (one,), (two,) = [[a[1] for a in said(log, "rung") if a[2] == prompt] for prompt in (first, second)]
  return root, first, second, one, two


async def test_take_keeps_the_acts_it_names_and_everything_they_made() -> None:
  """take keeps the acts it names and everything they made."""
  root, first, second, one, two = await twice(Sand(stands=STANDS))
  assert named(engine.turns(on=root)) == [root, root, first, one, first, second, two, second]
  kept = engine.chain("kept", source=root, filter=take(first))
  await settle(300)
  assert named(engine.turns(on=kept)) == [root, root, first, one, first, kept]


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
  sand.script[root] = ["x = bash('echo hi')\nclose(1)", "close(None)"]
  first = engine.prompt(int, "first", on=root)
  assert await first == 1
  await settle()
  _, step, *_ = said(log, "rung")[0]
  _, command, *_ = said(log, "bash")[0]
  _, held = engine.ask("transcript", root, root)
  assert isinstance(held, list)
  made = [a for a in held if engine.question(a)]
  assert [a[1] for a in take(first)(made)] == [first, step, f"holds@{command}.1", command]
  kept = engine.chain("kept", source=root, filter=take(first))
  await settle(300)
  assert named(engine.turns(on=kept)) == [root, root, first, step, command, first, command, kept]


async def test_take_that_is_not_inside_keeps_every_other_act() -> None:
  """take that is not inside keeps every other act, and drops everything the ones it names made."""
  root, first, second, one, two = await twice(Sand(stands=STANDS))
  narrow = engine.chain("narrow", source=root, filter=take(first, inside=False))
  await settle(300)
  assert named(engine.turns(on=root)) == [root, root, first, one, first, second, two, second]
  assert named(engine.turns(on=narrow)) == [root, root, second, two, second, narrow]
